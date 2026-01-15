//! ms prune - Prune tombstoned/outdated data
//!
//! Tombstones are created when files are "deleted" within ms-managed directories.
//! This command lists, purges, or restores tombstoned items. It also supports
//! skill pruning analysis to surface low-usage, low-quality, and high-similarity
//! candidates (proposal-first; no destructive actions).

use clap::{Args, Subcommand};
use colored::Colorize;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use which::which;

use crate::app::AppContext;
use crate::beads::{BeadsClient, CreateIssueRequest, IssueType, Priority};
use crate::error::{MsError, Result};
use crate::search::embeddings::VectorIndex;
use crate::security::SafetyGate;
use crate::storage::TombstoneManager;
use crate::storage::Database;
use crate::cli::output::{HumanLayout, emit_human};
use rusqlite::params;

#[derive(Args, Debug)]
pub struct PruneArgs {
    #[command(subcommand)]
    pub command: Option<PruneCommand>,

    /// Dry run - show what would be pruned (for list command)
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Older than N days (for list command)
    #[arg(long, global = true)]
    pub older_than: Option<u32>,
}

#[derive(Subcommand, Debug)]
pub enum PruneCommand {
    /// List tombstoned items (default if no subcommand)
    List,

    /// Purge (permanently delete) tombstoned items
    Purge(PurgeArgs),

    /// Restore a tombstoned item
    Restore(RestoreArgs),

    /// Show tombstone statistics
    Stats,

    /// Analyze skills for pruning candidates
    Analyze(AnalyzeArgs),

    /// Propose prune actions (merge/deprecate)
    Proposals(AnalyzeArgs),
}

#[derive(Args, Debug)]
pub struct AnalyzeArgs {
    /// Usage window in days
    #[arg(long, default_value = "30")]
    pub days: u32,

    /// Minimum usage within window before flagging
    #[arg(long, default_value = "5")]
    pub min_usage: u32,

    /// Maximum quality score before flagging
    #[arg(long, default_value = "0.3")]
    pub max_quality: f64,

    /// Similarity threshold (0-1)
    #[arg(long, default_value = "0.85")]
    pub similarity: f32,

    /// Max results per category
    #[arg(long, default_value = "10")]
    pub limit: usize,

    /// Similarity neighbors per skill
    #[arg(long, default_value = "5")]
    pub per_skill: usize,

    /// Emit beads issues for proposals
    #[arg(long)]
    pub emit_beads: bool,
}

#[derive(Args, Debug)]
pub struct PurgeArgs {
    /// Tombstone ID to purge (or "all" for all tombstones)
    pub id: String,

    /// Approve the purge (required for destructive operation)
    #[arg(long)]
    pub approve: bool,

    /// Purge all tombstones older than N days
    #[arg(long)]
    pub older_than: Option<u32>,
}

#[derive(Args, Debug)]
pub struct RestoreArgs {
    /// Tombstone ID to restore
    pub id: String,
}

pub fn run(ctx: &AppContext, args: &PruneArgs) -> Result<()> {
    let command = args.command.as_ref().unwrap_or(&PruneCommand::List);

    match command {
        PruneCommand::List => run_list(ctx, args),
        PruneCommand::Purge(purge_args) => run_purge(ctx, purge_args),
        PruneCommand::Restore(restore_args) => run_restore(ctx, restore_args),
        PruneCommand::Stats => run_stats(ctx),
        PruneCommand::Analyze(analyze_args) => run_analyze(ctx, analyze_args),
        PruneCommand::Proposals(proposals_args) => run_proposals(ctx, proposals_args),
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct UsageCandidate {
    skill_id: String,
    name: String,
    uses: u64,
    window_days: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
struct QualityCandidate {
    skill_id: String,
    name: String,
    quality_score: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
struct SimilarityCandidate {
    skill_a: String,
    skill_b: String,
    name_a: String,
    name_b: String,
    similarity: f32,
}

#[derive(Debug, Clone, serde::Serialize)]
struct ToolchainCandidate {
    skill_id: String,
    name: String,
    expected_tools: Vec<String>,
    missing_tools: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct DeprecateProposal {
    skill_id: String,
    name: String,
    rationale: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct MergeProposal {
    sources: Vec<String>,
    target_id: String,
    target_name: String,
    similarity: f32,
    rationale: String,
}

fn run_analyze(ctx: &AppContext, args: &AnalyzeArgs) -> Result<()> {
    let analysis = analyze_candidates(ctx, args)?;
    let low_usage = analysis.low_usage;
    let low_quality = analysis.low_quality;
    let similarity_pairs = analysis.similarity_pairs;
    let toolchain_mismatch = analysis.toolchain_mismatch;

    if ctx.robot_mode {
        let output = json!({
            "status": "analysis",
            "window_days": args.days,
            "min_usage": args.min_usage,
            "max_quality": args.max_quality,
            "similarity_threshold": args.similarity,
            "candidates": {
                "low_usage": low_usage,
                "low_quality": low_quality,
                "high_similarity": similarity_pairs,
                "toolchain_mismatch": toolchain_mismatch,
            },
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    let mut layout = HumanLayout::new();
    layout.title("Prune Analysis");
    layout
        .kv("Usage window", &format!("{} days", args.days))
        .kv("Min usage", &args.min_usage.to_string())
        .kv("Max quality", &format!("{:.2}", args.max_quality))
        .kv("Similarity", &format!("{:.2}", args.similarity))
        .blank();

    layout.section("Low Usage");
    if low_usage.is_empty() {
        layout.bullet("None");
    } else {
        for candidate in &low_usage {
            layout.bullet(&format!(
                "{} ({}) - {} uses",
                candidate.name, candidate.skill_id, candidate.uses
            ));
        }
    }

    layout.section("Low Quality");
    if low_quality.is_empty() {
        layout.bullet("None");
    } else {
        for candidate in &low_quality {
            layout.bullet(&format!(
                "{} ({}) - score {:.2}",
                candidate.name, candidate.skill_id, candidate.quality_score
            ));
        }
    }

    layout.section("High Similarity");
    if similarity_pairs.is_empty() {
        layout.bullet("None");
    } else {
        for pair in &similarity_pairs {
            layout.bullet(&format!(
                "{} ↔ {} ({} ↔ {}) score {:.2}",
                pair.name_a, pair.name_b, pair.skill_a, pair.skill_b, pair.similarity
            ));
        }
    }

    layout.section("Toolchain Mismatch");
    if toolchain_mismatch.is_empty() {
        layout.bullet("None");
    } else {
        for candidate in &toolchain_mismatch {
            layout.bullet(&format!(
                "{} ({}) missing {}",
                candidate.name,
                candidate.skill_id,
                candidate.missing_tools.join(", ")
            ));
        }
    }

    emit_human(layout);
    Ok(())
}

fn run_proposals(ctx: &AppContext, args: &AnalyzeArgs) -> Result<()> {
    let analysis = analyze_candidates(ctx, args)?;
    let mut rationale_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut candidate_ids: HashSet<String> = HashSet::new();
    let mut created_beads = Vec::new();

    for candidate in &analysis.low_usage {
        rationale_map
            .entry(candidate.skill_id.clone())
            .or_default()
            .push(format!(
                "usage {} in last {}d",
                candidate.uses, candidate.window_days
            ));
        candidate_ids.insert(candidate.skill_id.clone());
    }
    for candidate in &analysis.low_quality {
        rationale_map
            .entry(candidate.skill_id.clone())
            .or_default()
            .push(format!("quality {:.2} < {:.2}", candidate.quality_score, args.max_quality));
        candidate_ids.insert(candidate.skill_id.clone());
    }
    for candidate in &analysis.toolchain_mismatch {
        rationale_map
            .entry(candidate.skill_id.clone())
            .or_default()
            .push(format!(
                "missing tools: {}",
                candidate.missing_tools.join(", ")
            ));
        candidate_ids.insert(candidate.skill_id.clone());
    }

    let mut deprecate = Vec::new();
    for skill in &analysis.skills {
        let Some(rationales) = rationale_map.get(&skill.id) else {
            continue;
        };
        let rationale = rationales.join("; ");
        deprecate.push(DeprecateProposal {
            skill_id: skill.id.clone(),
            name: skill.name.clone(),
            rationale,
        });
    }

    deprecate.sort_by(|a, b| a.skill_id.cmp(&b.skill_id));
    if deprecate.len() > args.limit {
        deprecate.truncate(args.limit);
    }

    let mut merge = Vec::new();
    for pair in &analysis.similarity_pairs {
        let (target_id, target_name) =
            pick_merge_target(pair, &analysis.usage_map, &analysis.quality_map, &analysis.name_map);
        merge.push(MergeProposal {
            sources: vec![pair.skill_a.clone(), pair.skill_b.clone()],
            target_id,
            target_name,
            similarity: pair.similarity,
            rationale: format!("similarity {:.2} >= {:.2}", pair.similarity, args.similarity),
        });
        candidate_ids.insert(pair.skill_a.clone());
        candidate_ids.insert(pair.skill_b.clone());
    }
    merge.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
    if merge.len() > args.limit {
        merge.truncate(args.limit);
    }
    let candidate_count = candidate_ids.len();

    if args.emit_beads {
        created_beads = emit_beads(ctx, &deprecate, &merge)?;
    }

    if ctx.robot_mode {
        let mut output = json!({
            "status": "proposals_ready",
            "window_days": args.days,
            "min_usage": args.min_usage,
            "max_quality": args.max_quality,
            "similarity_threshold": args.similarity,
            "proposals": {
                "deprecate": deprecate,
                "merge": merge,
                "split": [],
            },
            "stats": {
                "total_skills": analysis.skills.len(),
                "candidates": candidate_count,
                "merge_proposals": merge.len(),
                "deprecate_proposals": deprecate.len(),
                "split_proposals": 0,
            },
        });
        if args.emit_beads {
            let beads_summary: Vec<_> = created_beads
                .iter()
                .map(|issue| json!({"id": issue.id, "title": issue.title}))
                .collect();
            if let Some(obj) = output.as_object_mut() {
                obj.insert(
                    "beads".to_string(),
                    json!({
                        "emitted": true,
                        "created": beads_summary,
                    }),
                );
            }
        }
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    let mut layout = HumanLayout::new();
    layout.title("Prune Proposals");
    layout
        .kv("Usage window", &format!("{} days", args.days))
        .kv("Min usage", &args.min_usage.to_string())
        .kv("Max quality", &format!("{:.2}", args.max_quality))
        .kv("Similarity", &format!("{:.2}", args.similarity))
        .kv("Candidates", &candidate_count.to_string())
        .kv("Deprecate proposals", &deprecate.len().to_string())
        .kv("Merge proposals", &merge.len().to_string())
        .blank();

    layout.section("Deprecate");
    if deprecate.is_empty() {
        layout.bullet("None");
    } else {
        for proposal in &deprecate {
            layout.bullet(&format!(
                "{} ({}) - {}",
                proposal.name, proposal.skill_id, proposal.rationale
            ));
        }
    }

    layout.section("Merge");
    if merge.is_empty() {
        layout.bullet("None");
    } else {
        for proposal in &merge {
            layout.bullet(&format!(
                "{} + {} -> {} ({}) score {:.2}",
                proposal.sources[0],
                proposal.sources[1],
                proposal.target_id,
                proposal.target_name,
                proposal.similarity
            ));
        }
    }

    if args.emit_beads {
        layout.section("Beads");
        if created_beads.is_empty() {
            layout.bullet("No proposals emitted");
        } else {
            for issue in &created_beads {
                layout.bullet(&format!("{} ({})", issue.title, issue.id));
            }
        }
    }

    emit_human(layout);
    Ok(())
}

struct PruneAnalysis {
    skills: Vec<crate::storage::sqlite::SkillRecord>,
    name_map: HashMap<String, String>,
    usage_map: HashMap<String, u64>,
    quality_map: HashMap<String, f64>,
    low_usage: Vec<UsageCandidate>,
    low_quality: Vec<QualityCandidate>,
    similarity_pairs: Vec<SimilarityCandidate>,
    toolchain_mismatch: Vec<ToolchainCandidate>,
}

fn analyze_candidates(ctx: &AppContext, args: &AnalyzeArgs) -> Result<PruneAnalysis> {
    let skills = load_all_skills(ctx.db.as_ref())?;
    let mut name_map = HashMap::new();
    let mut usage_map = HashMap::new();
    let mut quality_map = HashMap::new();
    for skill in &skills {
        name_map.insert(skill.id.clone(), skill.name.clone());
        quality_map.insert(skill.id.clone(), skill.quality_score);
    }

    let cutoff = (chrono::Utc::now() - chrono::Duration::days(args.days as i64)).to_rfc3339();

    let mut low_usage = Vec::new();
    let mut low_quality = Vec::new();
    let mut toolchain_mismatch = Vec::new();

    for skill in &skills {
        let uses = usage_since(ctx.db.as_ref(), &skill.id, &cutoff)?;
        usage_map.insert(skill.id.clone(), uses);
        if uses < args.min_usage as u64 {
            low_usage.push(UsageCandidate {
                skill_id: skill.id.clone(),
                name: skill.name.clone(),
                uses,
                window_days: args.days,
            });
        }

        if skill.quality_score < args.max_quality {
            low_quality.push(QualityCandidate {
                skill_id: skill.id.clone(),
                name: skill.name.clone(),
                quality_score: skill.quality_score,
            });
        }

        let expected_tools = parse_toolchain_tools(&skill.metadata_json);
        if !expected_tools.is_empty() {
            let mut missing_tools = Vec::new();
            for tool in &expected_tools {
                if which(tool).is_err() {
                    missing_tools.push(tool.clone());
                }
            }
            if !missing_tools.is_empty() {
                toolchain_mismatch.push(ToolchainCandidate {
                    skill_id: skill.id.clone(),
                    name: skill.name.clone(),
                    expected_tools,
                    missing_tools,
                });
            }
        }
    }

    low_usage.sort_by_key(|c| c.uses);
    low_usage.truncate(args.limit);
    low_quality.sort_by(|a, b| a.quality_score.partial_cmp(&b.quality_score).unwrap());
    low_quality.truncate(args.limit);

    let similarity_pairs = analyze_similarity(ctx, &name_map, args)?;
    toolchain_mismatch.sort_by(|a, b| a.skill_id.cmp(&b.skill_id));
    toolchain_mismatch.truncate(args.limit);

    Ok(PruneAnalysis {
        skills,
        name_map,
        usage_map,
        quality_map,
        low_usage,
        low_quality,
        similarity_pairs,
        toolchain_mismatch,
    })
}

fn load_all_skills(db: &Database) -> Result<Vec<crate::storage::sqlite::SkillRecord>> {
    let mut results = Vec::new();
    let mut offset = 0usize;
    let limit = 200usize;
    loop {
        let batch = db.list_skills(limit, offset)?;
        if batch.is_empty() {
            break;
        }
        results.extend(batch);
        offset += limit;
    }
    Ok(results)
}

fn usage_since(db: &Database, skill_id: &str, cutoff: &str) -> Result<u64> {
    let count: i64 = db.conn().query_row(
        "SELECT COUNT(*) FROM skill_usage WHERE skill_id = ? AND used_at >= ?",
        params![skill_id, cutoff],
        |row| row.get(0),
    )?;
    Ok(count.max(0) as u64)
}

fn analyze_similarity(
    ctx: &AppContext,
    name_map: &HashMap<String, String>,
    args: &AnalyzeArgs,
) -> Result<Vec<SimilarityCandidate>> {
    let embeddings = ctx.db.get_all_embeddings()?;
    if embeddings.len() < 2 {
        return Ok(Vec::new());
    }
    let dims = embeddings[0].1.len();
    let mut index = VectorIndex::new(dims);
    for (id, embedding) in &embeddings {
        index.insert(id.clone(), embedding.clone());
    }

    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut pairs = Vec::new();
    for (id, embedding) in &embeddings {
        let matches = index.search(embedding, args.per_skill + 1);
        for (other_id, score) in matches {
            if other_id == *id {
                continue;
            }
            if score < args.similarity {
                continue;
            }
            let (a, b) = if id < &other_id {
                (id.clone(), other_id.clone())
            } else {
                (other_id.clone(), id.clone())
            };
            if !seen.insert((a.clone(), b.clone())) {
                continue;
            }
            pairs.push(SimilarityCandidate {
                skill_a: a.clone(),
                skill_b: b.clone(),
                name_a: name_map.get(&a).cloned().unwrap_or_else(|| a.clone()),
                name_b: name_map.get(&b).cloned().unwrap_or_else(|| b.clone()),
                similarity: score,
            });
        }
    }

    pairs.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
    pairs.truncate(args.limit);
    Ok(pairs)
}

fn parse_toolchain_tools(metadata_json: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<Value>(metadata_json) else {
        return Vec::new();
    };
    let mut tools = Vec::new();
    let mut seen = HashSet::new();
    for key in [
        "tools",
        "toolchain",
        "toolchain_tools",
        "requires_tools",
        "requires-tools",
    ] {
        collect_tools(value.get(key), &mut tools, &mut seen);
    }
    tools
}

fn collect_tools(value: Option<&Value>, tools: &mut Vec<String>, seen: &mut HashSet<String>) {
    let Some(value) = value else {
        return;
    };
    match value {
        Value::Array(items) => {
            for item in items {
                if let Some(tool) = item.as_str() {
                    let tool = tool.trim();
                    if tool.is_empty() {
                        continue;
                    }
                    if seen.insert(tool.to_string()) {
                        tools.push(tool.to_string());
                    }
                }
            }
        }
        Value::String(text) => {
            for tool in text.split(',') {
                let tool = tool.trim();
                if tool.is_empty() {
                    continue;
                }
                if seen.insert(tool.to_string()) {
                    tools.push(tool.to_string());
                }
            }
        }
        _ => {}
    }
}

fn pick_merge_target(
    pair: &SimilarityCandidate,
    usage_map: &HashMap<String, u64>,
    quality_map: &HashMap<String, f64>,
    name_map: &HashMap<String, String>,
) -> (String, String) {
    let usage_a = usage_map.get(&pair.skill_a).copied().unwrap_or(0);
    let usage_b = usage_map.get(&pair.skill_b).copied().unwrap_or(0);
    let quality_a = quality_map.get(&pair.skill_a).copied().unwrap_or(0.0);
    let quality_b = quality_map.get(&pair.skill_b).copied().unwrap_or(0.0);

    let target = if usage_a == usage_b {
        if quality_a >= quality_b {
            pair.skill_a.clone()
        } else {
            pair.skill_b.clone()
        }
    } else if usage_a > usage_b {
        pair.skill_a.clone()
    } else {
        pair.skill_b.clone()
    };

    let target_name = name_map
        .get(&target)
        .cloned()
        .unwrap_or_else(|| target.clone());
    (target, target_name)
}

fn emit_beads(
    ctx: &AppContext,
    deprecate: &[DeprecateProposal],
    merge: &[MergeProposal],
) -> Result<Vec<crate::beads::Issue>> {
    let work_dir = beads_work_dir(&ctx.ms_root);
    let client = BeadsClient::new().with_work_dir(work_dir);
    if !client.is_available() {
        return Err(MsError::BeadsUnavailable(
            "bd not available (install beads or configure PATH)".to_string(),
        ));
    }

    let mut created = Vec::new();
    let mut index = 1usize;

    for proposal in merge {
        let title = format!(
            "ms-prune-{:03}: Merge {} + {}",
            index, proposal.sources[0], proposal.sources[1]
        );
        let description = format!(
            "Type: merge\nSources: {} + {}\nTarget: {} ({})\nSimilarity: {:.2}\nRationale: {}",
            proposal.sources[0],
            proposal.sources[1],
            proposal.target_id,
            proposal.target_name,
            proposal.similarity,
            proposal.rationale
        );
        let req = CreateIssueRequest::new(title)
            .with_type(IssueType::Task)
            .with_priority(2 as Priority)
            .with_description(description)
            .with_label("prune")
            .with_label("proposal")
            .with_label("merge");
        created.push(client.create(&req)?);
        index += 1;
    }

    for proposal in deprecate {
        let title = format!("ms-prune-{:03}: Deprecate {}", index, proposal.skill_id);
        let description = format!(
            "Type: deprecate\nSkill: {} ({})\nRationale: {}",
            proposal.skill_id, proposal.name, proposal.rationale
        );
        let req = CreateIssueRequest::new(title)
            .with_type(IssueType::Task)
            .with_priority(3 as Priority)
            .with_description(description)
            .with_label("prune")
            .with_label("proposal")
            .with_label("deprecate");
        created.push(client.create(&req)?);
        index += 1;
    }

    Ok(created)
}

fn beads_work_dir(ms_root: &PathBuf) -> PathBuf {
    let mut candidates = Vec::new();
    candidates.push(ms_root.clone());
    if let Some(parent) = ms_root.parent() {
        candidates.push(parent.to_path_buf());
    }
    for candidate in &candidates {
        if candidate.join(".beads").is_dir() {
            return candidate.clone();
        }
    }
    std::env::current_dir().unwrap_or_else(|_| ms_root.clone())
}

fn run_list(ctx: &AppContext, args: &PruneArgs) -> Result<()> {
    let manager = TombstoneManager::new(&ctx.ms_root);

    let records = if let Some(days) = args.older_than {
        manager.list_older_than(days)?
    } else {
        manager.list()?
    };

    if ctx.robot_mode {
        let output = json!({
            "tombstones": records,
            "count": records.len(),
            "total_size_bytes": records.iter().map(|r| r.size_bytes).sum::<u64>(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        if records.is_empty() {
            println!("No tombstones found.");
            return Ok(());
        }

        println!("{}", "Tombstoned Items".bold());
        println!("{}", "─".repeat(60));
        println!();

        for record in &records {
            let type_icon = if record.is_directory { "📁" } else { "📄" };
            let size = format_size(record.size_bytes);

            println!(
                "{} {} ({}) - {}",
                type_icon,
                record.original_path.cyan(),
                size.dimmed(),
                record
                    .tombstoned_at
                    .format("%Y-%m-%d %H:%M")
                    .to_string()
                    .dimmed()
            );
            println!("    ID: {}", record.id.dimmed());
            if let Some(reason) = &record.reason {
                println!("    Reason: {}", reason);
            }
            println!();
        }

        let total_size = records.iter().map(|r| r.size_bytes).sum::<u64>();
        println!(
            "Total: {} items, {}",
            records.len().to_string().cyan(),
            format_size(total_size).yellow()
        );

        if args.dry_run {
            println!();
            println!("  (dry run - no changes made)");
        }
    }

    Ok(())
}

fn run_purge(ctx: &AppContext, args: &PurgeArgs) -> Result<()> {
    let manager = TombstoneManager::new(&ctx.ms_root);
    let gate = SafetyGate::from_context(ctx);

    // Get the list of tombstones to purge
    let to_purge: Vec<_> = if args.id == "all" {
        if let Some(days) = args.older_than {
            manager.list_older_than(days)?
        } else {
            manager.list()?
        }
    } else {
        let all = manager.list()?;
        all.into_iter()
            .filter(|r| r.id == args.id || r.id.starts_with(&args.id))
            .collect()
    };

    if to_purge.is_empty() {
        if ctx.robot_mode {
            let output = json!({
                "error": true,
                "code": "not_found",
                "message": "No matching tombstones found",
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("No matching tombstones found.");
        }
        return Ok(());
    }

    // Require approval for purge
    if !args.approve {
        if ctx.robot_mode {
            let output = json!({
                "error": true,
                "code": "approval_required",
                "message": "Purge requires --approve flag",
                "items_to_purge": to_purge.len(),
                "total_bytes": to_purge.iter().map(|r| r.size_bytes).sum::<u64>(),
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("{}", "Approval Required".yellow().bold());
            println!();
            println!(
                "The following {} items will be permanently deleted:",
                to_purge.len()
            );
            for record in &to_purge {
                println!(
                    "  - {} ({})",
                    record.original_path,
                    format_size(record.size_bytes)
                );
            }
            println!();
            println!(
                "Total: {}",
                format_size(to_purge.iter().map(|r| r.size_bytes).sum::<u64>()).yellow()
            );
            println!();
            println!("Run with {} to confirm deletion.", "--approve".cyan());
        }
        return Ok(());
    }

    // Check with safety gate
    for record in &to_purge {
        let command = format!("ms prune purge {}", record.id);
        gate.enforce(&command, None)?;
    }

    // Perform the purge
    let mut results = Vec::new();
    let mut total_freed = 0u64;

    for record in &to_purge {
        match manager.purge(&record.id) {
            Ok(result) => {
                total_freed += result.bytes_freed;
                results.push(result);
            }
            Err(e) => {
                if !ctx.robot_mode {
                    println!("{} Failed to purge {}: {}", "✗".red(), record.id, e);
                }
            }
        }
    }

    if ctx.robot_mode {
        let output = json!({
            "purged": results,
            "count": results.len(),
            "bytes_freed": total_freed,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("{} Purged {} items", "✓".green(), results.len());
        println!("  Freed: {}", format_size(total_freed).yellow());
    }

    Ok(())
}

fn run_restore(ctx: &AppContext, args: &RestoreArgs) -> Result<()> {
    let manager = TombstoneManager::new(&ctx.ms_root);

    // Find matching tombstone
    let all = manager.list()?;
    let matching: Vec<_> = all
        .iter()
        .filter(|r| r.id == args.id || r.id.starts_with(&args.id))
        .collect();

    if matching.is_empty() {
        if ctx.robot_mode {
            let output = json!({
                "error": true,
                "code": "not_found",
                "message": format!("No tombstone found matching: {}", args.id),
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("No tombstone found matching: {}", args.id);
        }
        return Ok(());
    }

    if matching.len() > 1 {
        if ctx.robot_mode {
            let output = json!({
                "error": true,
                "code": "ambiguous",
                "message": "Multiple tombstones match. Please use a more specific ID.",
                "matches": matching.iter().map(|r| &r.id).collect::<Vec<_>>(),
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("Multiple tombstones match. Please use a more specific ID:");
            for record in matching {
                println!("  - {} ({})", record.id, record.original_path);
            }
        }
        return Ok(());
    }

    let record = matching[0];
    let result = manager.restore(&record.id)?;

    if ctx.robot_mode {
        let output = json!({
            "restored": result,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!(
            "{} Restored {} to {}",
            "✓".green(),
            record.id.dimmed(),
            result.restored_path.cyan()
        );
    }

    Ok(())
}

fn run_stats(ctx: &AppContext) -> Result<()> {
    let manager = TombstoneManager::new(&ctx.ms_root);
    let records = manager.list()?;

    let total_size = records.iter().map(|r| r.size_bytes).sum::<u64>();
    let file_count = records.iter().filter(|r| !r.is_directory).count();
    let dir_count = records.iter().filter(|r| r.is_directory).count();

    // Age statistics
    let now = chrono::Utc::now();
    let older_than_7d = records
        .iter()
        .filter(|r| (now - r.tombstoned_at).num_days() > 7)
        .count();
    let older_than_30d = records
        .iter()
        .filter(|r| (now - r.tombstoned_at).num_days() > 30)
        .count();

    if ctx.robot_mode {
        let output = json!({
            "count": records.len(),
            "files": file_count,
            "directories": dir_count,
            "total_size_bytes": total_size,
            "older_than_7_days": older_than_7d,
            "older_than_30_days": older_than_30d,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("{}", "Tombstone Statistics".bold());
        println!("{}", "─".repeat(30));
        println!();
        println!("  Total items:     {}", records.len().to_string().cyan());
        println!("  Files:           {}", file_count);
        println!("  Directories:     {}", dir_count);
        println!("  Total size:      {}", format_size(total_size).yellow());
        println!();
        println!("  Older than 7d:   {}", older_than_7d);
        println!("  Older than 30d:  {}", older_than_30d);

        if older_than_30d > 0 {
            println!();
            println!(
                "  {} Consider running: {} {}",
                "!".yellow(),
                "ms prune purge all --older-than 30 --approve".cyan(),
                ""
            );
        }
    }

    Ok(())
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    // =========================================================================
    // Argument parsing tests
    // =========================================================================

    #[derive(Parser, Debug)]
    #[command(name = "test")]
    struct TestCli {
        #[command(flatten)]
        prune: PruneArgs,
    }

    #[test]
    fn parse_prune_defaults() {
        let cli = TestCli::try_parse_from(["test"]).unwrap();
        assert!(cli.prune.command.is_none());
        assert!(!cli.prune.dry_run);
        assert!(cli.prune.older_than.is_none());
    }

    #[test]
    fn parse_prune_dry_run() {
        let cli = TestCli::try_parse_from(["test", "--dry-run"]).unwrap();
        assert!(cli.prune.dry_run);
    }

    #[test]
    fn parse_prune_older_than() {
        let cli = TestCli::try_parse_from(["test", "--older-than", "30"]).unwrap();
        assert_eq!(cli.prune.older_than, Some(30));
    }

    #[test]
    fn parse_prune_list_subcommand() {
        let cli = TestCli::try_parse_from(["test", "list"]).unwrap();
        assert!(matches!(cli.prune.command, Some(PruneCommand::List)));
    }

    #[test]
    fn parse_prune_list_with_flags() {
        let cli =
            TestCli::try_parse_from(["test", "list", "--dry-run", "--older-than", "7"]).unwrap();
        assert!(matches!(cli.prune.command, Some(PruneCommand::List)));
        assert!(cli.prune.dry_run);
        assert_eq!(cli.prune.older_than, Some(7));
    }

    #[test]
    fn parse_prune_stats_subcommand() {
        let cli = TestCli::try_parse_from(["test", "stats"]).unwrap();
        assert!(matches!(cli.prune.command, Some(PruneCommand::Stats)));
    }

    #[test]
    fn parse_prune_analyze_defaults() {
        let cli = TestCli::try_parse_from(["test", "analyze"]).unwrap();
        match cli.prune.command {
            Some(PruneCommand::Analyze(args)) => {
                assert_eq!(args.days, 30);
                assert_eq!(args.min_usage, 5);
                assert!((args.max_quality - 0.3).abs() < f64::EPSILON);
                assert!((args.similarity - 0.85).abs() < f32::EPSILON);
                assert_eq!(args.limit, 10);
                assert_eq!(args.per_skill, 5);
                assert!(!args.emit_beads);
            }
            _ => panic!("Expected Analyze subcommand"),
        }
    }

    #[test]
    fn parse_prune_analyze_overrides() {
        let cli = TestCli::try_parse_from([
            "test",
            "analyze",
            "--days",
            "60",
            "--min-usage",
            "2",
            "--max-quality",
            "0.5",
            "--similarity",
            "0.9",
            "--limit",
            "12",
            "--per-skill",
            "8",
        ])
        .unwrap();
        match cli.prune.command {
            Some(PruneCommand::Analyze(args)) => {
                assert_eq!(args.days, 60);
                assert_eq!(args.min_usage, 2);
                assert!((args.max_quality - 0.5).abs() < f64::EPSILON);
                assert!((args.similarity - 0.9).abs() < f32::EPSILON);
                assert_eq!(args.limit, 12);
                assert_eq!(args.per_skill, 8);
            }
            _ => panic!("Expected Analyze subcommand"),
        }
    }

    #[test]
    fn parse_prune_proposals_defaults() {
        let cli = TestCli::try_parse_from(["test", "proposals"]).unwrap();
        match cli.prune.command {
            Some(PruneCommand::Proposals(args)) => {
                assert_eq!(args.days, 30);
                assert_eq!(args.min_usage, 5);
                assert!((args.max_quality - 0.3).abs() < f64::EPSILON);
                assert!((args.similarity - 0.85).abs() < f32::EPSILON);
                assert_eq!(args.limit, 10);
                assert_eq!(args.per_skill, 5);
                assert!(!args.emit_beads);
            }
            _ => panic!("Expected Proposals subcommand"),
        }
    }

    #[test]
    fn parse_prune_proposals_overrides() {
        let cli = TestCli::try_parse_from([
            "test",
            "proposals",
            "--days",
            "14",
            "--min-usage",
            "1",
            "--max-quality",
            "0.2",
            "--similarity",
            "0.88",
            "--limit",
            "3",
            "--per-skill",
            "4",
        ])
        .unwrap();
        match cli.prune.command {
            Some(PruneCommand::Proposals(args)) => {
                assert_eq!(args.days, 14);
                assert_eq!(args.min_usage, 1);
                assert!((args.max_quality - 0.2).abs() < f64::EPSILON);
                assert!((args.similarity - 0.88).abs() < f32::EPSILON);
                assert_eq!(args.limit, 3);
                assert_eq!(args.per_skill, 4);
                assert!(!args.emit_beads);
            }
            _ => panic!("Expected Proposals subcommand"),
        }
    }

    #[test]
    fn parse_prune_proposals_emit_beads() {
        let cli = TestCli::try_parse_from(["test", "proposals", "--emit-beads"]).unwrap();
        match cli.prune.command {
            Some(PruneCommand::Proposals(args)) => {
                assert!(args.emit_beads);
            }
            _ => panic!("Expected Proposals subcommand"),
        }
    }

    #[test]
    fn parse_toolchain_tools_dedupes_and_trims() {
        let metadata = r#"{"tools":["git","rg",""],"toolchain":"cargo, git "}"#;
        let tools = parse_toolchain_tools(metadata);
        assert_eq!(tools, vec!["git", "rg", "cargo"]);
    }

    #[test]
    fn pick_merge_target_prefers_usage() {
        let pair = SimilarityCandidate {
            skill_a: "a".to_string(),
            skill_b: "b".to_string(),
            name_a: "Skill A".to_string(),
            name_b: "Skill B".to_string(),
            similarity: 0.9,
        };
        let mut usage_map = HashMap::new();
        usage_map.insert("a".to_string(), 10);
        usage_map.insert("b".to_string(), 3);
        let mut quality_map = HashMap::new();
        quality_map.insert("a".to_string(), 0.2);
        quality_map.insert("b".to_string(), 0.9);
        let mut name_map = HashMap::new();
        name_map.insert("a".to_string(), "Alpha".to_string());
        name_map.insert("b".to_string(), "Beta".to_string());

        let (target, target_name) =
            pick_merge_target(&pair, &usage_map, &quality_map, &name_map);
        assert_eq!(target, "a");
        assert_eq!(target_name, "Alpha");
    }

    #[test]
    fn pick_merge_target_breaks_usage_tie_by_quality() {
        let pair = SimilarityCandidate {
            skill_a: "a".to_string(),
            skill_b: "b".to_string(),
            name_a: "Skill A".to_string(),
            name_b: "Skill B".to_string(),
            similarity: 0.9,
        };
        let mut usage_map = HashMap::new();
        usage_map.insert("a".to_string(), 5);
        usage_map.insert("b".to_string(), 5);
        let mut quality_map = HashMap::new();
        quality_map.insert("a".to_string(), 0.2);
        quality_map.insert("b".to_string(), 0.9);
        let mut name_map = HashMap::new();
        name_map.insert("a".to_string(), "Alpha".to_string());
        name_map.insert("b".to_string(), "Beta".to_string());

        let (target, target_name) =
            pick_merge_target(&pair, &usage_map, &quality_map, &name_map);
        assert_eq!(target, "b");
        assert_eq!(target_name, "Beta");
    }

    #[test]
    fn parse_prune_purge_with_id() {
        let cli = TestCli::try_parse_from(["test", "purge", "abc123"]).unwrap();
        match cli.prune.command {
            Some(PruneCommand::Purge(args)) => {
                assert_eq!(args.id, "abc123");
                assert!(!args.approve);
                assert!(args.older_than.is_none());
            }
            _ => panic!("Expected Purge subcommand"),
        }
    }

    #[test]
    fn parse_prune_purge_with_approve() {
        let cli = TestCli::try_parse_from(["test", "purge", "abc123", "--approve"]).unwrap();
        match cli.prune.command {
            Some(PruneCommand::Purge(args)) => {
                assert_eq!(args.id, "abc123");
                assert!(args.approve);
            }
            _ => panic!("Expected Purge subcommand"),
        }
    }

    #[test]
    fn parse_prune_purge_all_with_older_than() {
        let cli =
            TestCli::try_parse_from(["test", "purge", "all", "--older-than", "30", "--approve"])
                .unwrap();
        match cli.prune.command {
            Some(PruneCommand::Purge(args)) => {
                assert_eq!(args.id, "all");
                assert!(args.approve);
                assert_eq!(args.older_than, Some(30));
            }
            _ => panic!("Expected Purge subcommand"),
        }
    }

    #[test]
    fn parse_prune_restore_with_id() {
        let cli = TestCli::try_parse_from(["test", "restore", "xyz789"]).unwrap();
        match cli.prune.command {
            Some(PruneCommand::Restore(args)) => {
                assert_eq!(args.id, "xyz789");
            }
            _ => panic!("Expected Restore subcommand"),
        }
    }

    #[test]
    fn parse_prune_purge_requires_id() {
        let result = TestCli::try_parse_from(["test", "purge"]);
        assert!(result.is_err());
    }

    #[test]
    fn parse_prune_restore_requires_id() {
        let result = TestCli::try_parse_from(["test", "restore"]);
        assert!(result.is_err());
    }

    #[test]
    fn parse_prune_invalid_older_than() {
        let result = TestCli::try_parse_from(["test", "--older-than", "not-a-number"]);
        assert!(result.is_err());
    }

    // =========================================================================
    // format_size tests
    // =========================================================================

    #[test]
    fn format_size_zero_bytes() {
        assert_eq!(format_size(0), "0 B");
    }

    #[test]
    fn format_size_small_bytes() {
        assert_eq!(format_size(512), "512 B");
    }

    #[test]
    fn format_size_one_byte() {
        assert_eq!(format_size(1), "1 B");
    }

    #[test]
    fn format_size_max_bytes() {
        // Just under 1 KB
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn format_size_exactly_one_kb() {
        assert_eq!(format_size(1024), "1.00 KB");
    }

    #[test]
    fn format_size_kilobytes() {
        assert_eq!(format_size(2048), "2.00 KB");
    }

    #[test]
    fn format_size_kilobytes_fractional() {
        assert_eq!(format_size(1536), "1.50 KB");
    }

    #[test]
    fn format_size_max_kilobytes() {
        // Just under 1 MB (1024 * 1024 - 1)
        let result = format_size(1024 * 1024 - 1);
        assert!(result.ends_with(" KB"));
    }

    #[test]
    fn format_size_exactly_one_mb() {
        assert_eq!(format_size(1024 * 1024), "1.00 MB");
    }

    #[test]
    fn format_size_megabytes() {
        assert_eq!(format_size(5 * 1024 * 1024), "5.00 MB");
    }

    #[test]
    fn format_size_megabytes_fractional() {
        // 1.5 MB
        assert_eq!(format_size(1024 * 1024 + 512 * 1024), "1.50 MB");
    }

    #[test]
    fn format_size_max_megabytes() {
        // Just under 1 GB
        let result = format_size(1024 * 1024 * 1024 - 1);
        assert!(result.ends_with(" MB"));
    }

    #[test]
    fn format_size_exactly_one_gb() {
        assert_eq!(format_size(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn format_size_gigabytes() {
        assert_eq!(format_size(2 * 1024 * 1024 * 1024), "2.00 GB");
    }

    #[test]
    fn format_size_gigabytes_fractional() {
        // 1.5 GB
        let gb: u64 = 1024 * 1024 * 1024;
        assert_eq!(format_size(gb + gb / 2), "1.50 GB");
    }

    #[test]
    fn format_size_large_gigabytes() {
        // 100 GB
        let gb: u64 = 1024 * 1024 * 1024;
        assert_eq!(format_size(100 * gb), "100.00 GB");
    }
}
