use std::collections::HashMap;
use std::path::PathBuf;

use clap::Args;

use crate::app::AppContext;
use crate::cli::formatters::{SuggestionContext, SuggestionItem, SuggestionOutput};
use crate::cli::output::Formattable;
use crate::context::collector::{CollectedContext, ContextCollector, ContextCollectorConfig};
use crate::context::{ContextCapture, ContextFingerprint};
use crate::error::Result;
use crate::storage::sqlite::SkillRecord;
use crate::suggestions::bandit::contextual::ContextualBandit;
use crate::suggestions::bandit::features::{DefaultFeatureExtractor, FeatureExtractor, UserHistory, FEATURE_DIM};
use crate::suggestions::tracking::SuggestionTracker;
use crate::suggestions::SuggestionCooldownCache;

#[derive(Args, Debug)]
pub struct SuggestArgs {
    /// Maximum number of suggestions to return
    #[arg(long, short, default_value = "5")]
    pub limit: usize,

    /// Include discovery suggestions (exploration of novel skills)
    #[arg(long)]
    pub discover: bool,

    /// Weight historical preferences heavily
    #[arg(long)]
    pub personal: bool,

    /// Show explanation for each suggestion
    #[arg(long)]
    pub explain: bool,

    /// Filter by domain/tag
    #[arg(long)]
    pub domain: Option<String>,

    /// Automatically load suggested skills
    #[arg(long)]
    pub load: bool,

    /// Number of skills to auto-load (used with --load)
    #[arg(long, default_value = "3")]
    pub top: usize,

    /// Working directory context
    #[arg(long)]
    pub cwd: Option<String>,

    /// Budget for packed output
    #[arg(long)]
    pub budget: Option<usize>,

    /// Ignore suggestion cooldowns
    #[arg(long)]
    pub ignore_cooldowns: bool,

    /// Clear cooldown cache before suggesting
    #[arg(long)]
    pub reset_cooldowns: bool,

    /// Disable bandit-based weighting
    #[arg(long)]
    pub no_bandit: bool,

    /// Override bandit exploration factor
    #[arg(long)]
    pub bandit_exploration: Option<f64>,

    /// Reset bandit state before suggesting
    #[arg(long)]
    pub reset_bandit: bool,
}

/// A suggestion with score and metadata.
#[derive(Debug, Clone)]
pub struct Suggestion {
    pub skill_id: String,
    pub name: String,
    pub description: String,
    pub score: f32,
    pub breakdown: ScoreBreakdown,
    pub is_discovery: bool,
    pub tags: Vec<String>,
}

/// Score breakdown for explanation mode.
#[derive(Debug, Clone, Default)]
pub struct ScoreBreakdown {
    pub contextual_score: f32,
    pub thompson_score: f32,
    pub exploration_bonus: f32,
    pub personal_boost: f32,
    pub pull_count: u64,
    pub avg_reward: f64,
}

pub fn run(ctx: &AppContext, args: &SuggestArgs) -> Result<()> {
    // 1. Capture working context
    let cwd_path: Option<PathBuf> = args.cwd.as_ref().map(PathBuf::from);
    let capture = ContextCapture::capture_current(cwd_path.clone())?;
    let fingerprint = ContextFingerprint::capture(&capture);

    // 2. Load cooldown cache
    let cache_path = cooldown_path();
    let mut cache = if args.reset_cooldowns {
        SuggestionCooldownCache::new()
    } else {
        SuggestionCooldownCache::load(&cache_path).unwrap_or_else(|e| {
            if !ctx.output_format.is_machine_readable() {
                eprintln!("Warning: Failed to load cooldown cache: {e}. Starting fresh.");
            }
            SuggestionCooldownCache::new()
        })
    };

    if args.reset_cooldowns {
        cache.save(&cache_path)?;
    }

    // 3. Load contextual bandit
    let contextual_bandit_path = contextual_bandit_path();
    let mut contextual_bandit = if args.reset_bandit {
        let bandit = ContextualBandit::with_feature_dim(FEATURE_DIM);
        bandit.save(&contextual_bandit_path)?;
        bandit
    } else {
        ContextualBandit::load(&contextual_bandit_path).unwrap_or_else(|e| {
            if !ctx.output_format.is_machine_readable() {
                eprintln!("Warning: Failed to load bandit state: {e}. Starting fresh.");
            }
            ContextualBandit::with_feature_dim(FEATURE_DIM)
        })
    };

    // 4. Collect context for feature extraction
    let collector_config = ContextCollectorConfig::default();
    let collector = ContextCollector::new(collector_config);
    let working_dir = cwd_path.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let collected_context = collector.collect(&working_dir)?;

    // 5. Extract context features
    let feature_extractor = DefaultFeatureExtractor::new();
    let user_history = load_user_history();
    let context_features = feature_extractor.extract_from_collected(&collected_context, &user_history);

    // 6. Get all skills from database
    let all_skills = ctx.db.list_skills(1000, 0)?;
    if all_skills.is_empty() {
        return output_empty_suggestions(ctx, args, &fingerprint, &cache);
    }

    // Register all skills with the bandit
    let skill_ids: Vec<String> = all_skills.iter().map(|s| s.id.clone()).collect();
    contextual_bandit.register_skills(&skill_ids);

    // 7. Get recommendations from bandit
    let fetch_limit = args.limit * 2; // Fetch extra for filtering
    let recommendations = contextual_bandit.recommend(&context_features, fetch_limit);

    // 8. Build suggestions with metadata
    let skill_map: HashMap<String, &SkillRecord> = all_skills.iter().map(|s| (s.id.clone(), s)).collect();
    let mut suggestions: Vec<Suggestion> = recommendations
        .iter()
        .filter_map(|rec| {
            let skill = skill_map.get(&rec.skill_id)?;
            let tags = parse_tags_from_metadata(&skill.metadata_json);

            Some(Suggestion {
                skill_id: rec.skill_id.clone(),
                name: skill.name.clone(),
                description: skill.description.clone(),
                score: rec.score,
                breakdown: ScoreBreakdown {
                    contextual_score: rec.components.contextual_score,
                    thompson_score: rec.components.thompson_score,
                    exploration_bonus: rec.components.exploration_bonus,
                    personal_boost: 0.0,
                    pull_count: rec.components.pull_count,
                    avg_reward: rec.components.avg_reward,
                },
                is_discovery: rec.components.pull_count < 5,
                tags,
            })
        })
        .collect();

    // 9. Apply domain filter if specified
    if let Some(ref domain) = args.domain {
        let domain_lower = domain.to_lowercase();
        suggestions.retain(|s| {
            s.tags.iter().any(|t| t.to_lowercase().contains(&domain_lower))
                || s.name.to_lowercase().contains(&domain_lower)
                || s.description.to_lowercase().contains(&domain_lower)
        });
    }

    // 10. Apply personal mode (boost historical preferences)
    if args.personal {
        for suggestion in &mut suggestions {
            let frequency = user_history.skill_frequency(&suggestion.skill_id);
            let recency = user_history.skill_recency(&suggestion.skill_id);
            let personal_boost = frequency * 0.3 + recency * 0.2;
            suggestion.breakdown.personal_boost = personal_boost;
            suggestion.score = (suggestion.score + personal_boost).clamp(0.0, 1.0);
        }
        // Re-sort after personal boost
        suggestions.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    }

    // 11. Apply cooldown filter (unless ignored)
    let fp = fingerprint.as_u64();
    if !args.ignore_cooldowns {
        use crate::suggestions::CooldownStatus;
        suggestions.retain(|s| {
            !matches!(cache.status(fp, &s.skill_id), CooldownStatus::Active { .. })
        });
    }

    // 12. Truncate to limit
    suggestions.truncate(args.limit);

    // 13. Build discovery suggestions if requested
    let mut discovery_suggestions: Vec<Suggestion> = Vec::new();
    if args.discover {
        // Find skills not in main suggestions that are under-explored
        let suggested_ids: std::collections::HashSet<_> = suggestions.iter().map(|s| &s.skill_id).collect();
        let mut discovery_candidates: Vec<Suggestion> = all_skills
            .iter()
            .filter(|s| !suggested_ids.contains(&s.id))
            .filter_map(|skill| {
                let rec = recommendations.iter().find(|r| r.skill_id == skill.id);
                let components = rec.map(|r| &r.components);
                let pull_count = components.map(|c| c.pull_count).unwrap_or(0);

                // Only include under-explored skills
                if pull_count >= 10 {
                    return None;
                }

                let tags = parse_tags_from_metadata(&skill.metadata_json);
                let base_score = rec.map(|r| r.score).unwrap_or(0.3);

                Some(Suggestion {
                    skill_id: skill.id.clone(),
                    name: skill.name.clone(),
                    description: skill.description.clone(),
                    score: base_score,
                    breakdown: ScoreBreakdown {
                        contextual_score: components.map(|c| c.contextual_score).unwrap_or(0.0),
                        thompson_score: components.map(|c| c.thompson_score).unwrap_or(0.5),
                        exploration_bonus: components.map(|c| c.exploration_bonus).unwrap_or(0.1),
                        personal_boost: 0.0,
                        pull_count,
                        avg_reward: components.map(|c| c.avg_reward).unwrap_or(0.5),
                    },
                    is_discovery: true,
                    tags,
                })
            })
            .collect();

        // Sort by exploration potential
        discovery_candidates.sort_by(|a, b| {
            let a_potential = a.breakdown.exploration_bonus + (1.0 - a.breakdown.pull_count as f32 / 10.0).max(0.0) * 0.2;
            let b_potential = b.breakdown.exploration_bonus + (1.0 - b.breakdown.pull_count as f32 / 10.0).max(0.0) * 0.2;
            b_potential.partial_cmp(&a_potential).unwrap_or(std::cmp::Ordering::Equal)
        });

        discovery_suggestions = discovery_candidates.into_iter().take(3).collect();
    }

    // 14. Record suggestions for learning
    let mut suggestion_tracker = SuggestionTracker::new();
    let all_suggested_ids: Vec<String> = suggestions
        .iter()
        .chain(discovery_suggestions.iter())
        .map(|s| s.skill_id.clone())
        .collect();
    suggestion_tracker.record_suggestions(&all_suggested_ids, Some(fingerprint.as_u64()));

    // 15. Update cooldowns for shown suggestions (default 5 minute cooldown)
    let cooldown_seconds = 300; // 5 minutes
    for suggestion in &suggestions {
        cache.record(fp, suggestion.skill_id.clone(), cooldown_seconds);
    }
    cache.save(&cache_path)?;

    // 16. Output results
    output_suggestions(ctx, args, &fingerprint, &suggestions, &discovery_suggestions, &collected_context, &contextual_bandit)
}

/// Output when no skills are available.
fn output_empty_suggestions(
    ctx: &AppContext,
    _args: &SuggestArgs,
    fingerprint: &ContextFingerprint,
    _cache: &SuggestionCooldownCache,
) -> Result<()> {
    let output = SuggestionOutput::new().with_fingerprint(fingerprint.as_u64());
    println!("{}", output.format(ctx.output_format));
    Ok(())
}

/// Unified output function using the new formatter system.
fn output_suggestions(
    ctx: &AppContext,
    _args: &SuggestArgs,
    fingerprint: &ContextFingerprint,
    suggestions: &[Suggestion],
    discovery_suggestions: &[Suggestion],
    context: &CollectedContext,
    _bandit: &ContextualBandit,
) -> Result<()> {
    // Build context
    let suggestion_context = SuggestionContext {
        cwd: std::env::current_dir()
            .ok()
            .map(|p| p.display().to_string()),
        git_branch: context.git_context.as_ref().map(|g| g.branch.clone()),
        recent_files: context
            .recent_files
            .iter()
            .take(10)
            .map(|f| f.path.display().to_string())
            .collect(),
        fingerprint: Some(fingerprint.as_u64()),
    };

    // Build output
    let mut output = SuggestionOutput::new().with_context(suggestion_context);

    // Add main suggestions
    for s in suggestions {
        let reason = if s.breakdown.contextual_score > 0.5 {
            Some(format!(
                "High context match ({:.0}%)",
                s.breakdown.contextual_score * 100.0
            ))
        } else if s.breakdown.pull_count > 10 {
            Some(format!("Historically useful ({} uses)", s.breakdown.pull_count))
        } else {
            None
        };

        output.add_suggestion(SuggestionItem {
            skill_id: s.skill_id.clone(),
            name: s.name.clone(),
            description: s.description.clone(),
            confidence: s.score,
            reason,
            is_discovery: false,
            tags: s.tags.clone(),
        });
    }

    // Add discovery suggestions
    for s in discovery_suggestions {
        output.add_suggestion(SuggestionItem {
            skill_id: s.skill_id.clone(),
            name: s.name.clone(),
            description: s.description.clone(),
            confidence: s.score,
            reason: Some(format!(
                "Under-explored ({} uses), high exploration potential",
                s.breakdown.pull_count
            )),
            is_discovery: true,
            tags: s.tags.clone(),
        });
    }

    println!("{}", output.format(ctx.output_format));
    Ok(())
}

/// Parse tags from skill metadata JSON.
fn parse_tags_from_metadata(metadata_json: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(metadata_json) else {
        return vec![];
    };
    value
        .get("tags")
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// Load user history from persistence.
fn load_user_history() -> UserHistory {
    let path = user_history_path();
    if !path.exists() {
        return UserHistory::default();
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn user_history_path() -> std::path::PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    base.join("ms").join("user_history.json")
}

fn cooldown_path() -> std::path::PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    base.join("ms").join("cooldowns.json")
}

fn contextual_bandit_path() -> std::path::PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    base.join("ms").join("contextual_bandit.json")
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
        suggest: SuggestArgs,
    }

    #[test]
    fn parse_suggest_defaults() {
        let cli = TestCli::try_parse_from(["test"]).unwrap();
        assert!(cli.suggest.cwd.is_none());
        assert!(cli.suggest.budget.is_none());
        assert!(!cli.suggest.ignore_cooldowns);
        assert!(!cli.suggest.reset_cooldowns);
        assert!(!cli.suggest.no_bandit);
        assert!(cli.suggest.bandit_exploration.is_none());
        assert!(!cli.suggest.reset_bandit);
    }

    #[test]
    fn parse_suggest_with_cwd() {
        let cli = TestCli::try_parse_from(["test", "--cwd", "/path/to/dir"]).unwrap();
        assert_eq!(cli.suggest.cwd, Some("/path/to/dir".to_string()));
    }

    #[test]
    fn parse_suggest_with_budget() {
        let cli = TestCli::try_parse_from(["test", "--budget", "1000"]).unwrap();
        assert_eq!(cli.suggest.budget, Some(1000));
    }

    #[test]
    fn parse_suggest_ignore_cooldowns() {
        let cli = TestCli::try_parse_from(["test", "--ignore-cooldowns"]).unwrap();
        assert!(cli.suggest.ignore_cooldowns);
    }

    #[test]
    fn parse_suggest_reset_cooldowns() {
        let cli = TestCli::try_parse_from(["test", "--reset-cooldowns"]).unwrap();
        assert!(cli.suggest.reset_cooldowns);
    }

    #[test]
    fn parse_suggest_no_bandit() {
        let cli = TestCli::try_parse_from(["test", "--no-bandit"]).unwrap();
        assert!(cli.suggest.no_bandit);
    }

    #[test]
    fn parse_suggest_bandit_exploration() {
        let cli = TestCli::try_parse_from(["test", "--bandit-exploration", "0.5"]).unwrap();
        assert_eq!(cli.suggest.bandit_exploration, Some(0.5));
    }

    #[test]
    fn parse_suggest_bandit_exploration_zero() {
        let cli = TestCli::try_parse_from(["test", "--bandit-exploration", "0.0"]).unwrap();
        assert_eq!(cli.suggest.bandit_exploration, Some(0.0));
    }

    #[test]
    fn parse_suggest_reset_bandit() {
        let cli = TestCli::try_parse_from(["test", "--reset-bandit"]).unwrap();
        assert!(cli.suggest.reset_bandit);
    }

    #[test]
    fn parse_suggest_all_options() {
        let cli = TestCli::try_parse_from([
            "test",
            "--cwd",
            "/home/user/project",
            "--budget",
            "2000",
            "--ignore-cooldowns",
            "--reset-cooldowns",
            "--no-bandit",
            "--bandit-exploration",
            "1.5",
            "--reset-bandit",
        ])
        .unwrap();

        assert_eq!(cli.suggest.cwd, Some("/home/user/project".to_string()));
        assert_eq!(cli.suggest.budget, Some(2000));
        assert!(cli.suggest.ignore_cooldowns);
        assert!(cli.suggest.reset_cooldowns);
        assert!(cli.suggest.no_bandit);
        assert_eq!(cli.suggest.bandit_exploration, Some(1.5));
        assert!(cli.suggest.reset_bandit);
    }

    // =========================================================================
    // Error case tests
    // =========================================================================

    #[test]
    fn parse_suggest_invalid_budget() {
        let result = TestCli::try_parse_from(["test", "--budget", "not-a-number"]);
        assert!(result.is_err());
    }

    #[test]
    fn parse_suggest_invalid_bandit_exploration() {
        let result = TestCli::try_parse_from(["test", "--bandit-exploration", "abc"]);
        assert!(result.is_err());
    }

    // =========================================================================
    // Path function tests
    // =========================================================================

    #[test]
    fn cooldown_path_ends_with_expected() {
        let path = cooldown_path();
        assert!(path.ends_with("ms/cooldowns.json"));
    }

    #[test]
    fn contextual_bandit_path_ends_with_expected() {
        let path = contextual_bandit_path();
        assert!(path.ends_with("ms/contextual_bandit.json"));
    }

    #[test]
    fn paths_are_in_same_directory() {
        let cooldown = cooldown_path();
        let bandit = contextual_bandit_path();

        // Both should be in the same parent directory
        assert_eq!(cooldown.parent(), bandit.parent());
    }
}
