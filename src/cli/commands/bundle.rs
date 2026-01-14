//! ms bundle - Manage skill bundles

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use clap::{Args, Subcommand};

use crate::app::AppContext;
use crate::bundler::github::{download_bundle, download_url, publish_bundle, GitHubConfig};
use crate::bundler::install::InstallReport;
use crate::bundler::local_safety::{detect_modifications, ModificationStatus, SkillModificationReport};
use crate::bundler::registry::{BundleRegistry, InstallSource, InstalledBundle, ParsedSource};
use crate::bundler::{Bundle, BundleInfo, BundleManifest, BundledSkill};
use crate::cli::output::emit_json;
use crate::error::{MsError, Result};

#[derive(Args, Debug)]
pub struct BundleArgs {
    #[command(subcommand)]
    pub command: BundleCommand,
}

#[derive(Subcommand, Debug)]
pub enum BundleCommand {
    /// Create a new bundle
    Create(BundleCreateArgs),
    /// Publish a bundle to GitHub
    Publish(BundlePublishArgs),
    /// Install a bundle
    Install(BundleInstallArgs),
    /// Remove an installed bundle
    Remove(BundleRemoveArgs),
    /// List installed bundles
    List,
    /// Show details of a bundle
    Show(BundleShowArgs),
    /// Check for local modifications and conflicts
    Conflicts(BundleConflictsArgs),
}

#[derive(Args, Debug)]
pub struct BundleCreateArgs {
    /// Bundle name
    pub name: String,

    /// Skills to include
    #[arg(long)]
    pub skills: Vec<String>,

    /// Bundle id (defaults to slug of name)
    #[arg(long)]
    pub id: Option<String>,

    /// Bundle version
    #[arg(long, default_value = "0.1.0")]
    pub version: String,

    /// Output path for bundle file
    #[arg(long)]
    pub output: Option<PathBuf>,

    /// Write manifest TOML alongside bundle
    #[arg(long)]
    pub write_manifest: bool,
}

#[derive(Args, Debug)]
pub struct BundlePublishArgs {
    /// Bundle path
    pub path: String,

    /// GitHub repository
    #[arg(long)]
    pub repo: Option<String>,

    /// GitHub token (overrides env)
    #[arg(long)]
    pub token: Option<String>,

    /// Release tag (defaults to v<bundle version>)
    #[arg(long)]
    pub tag: Option<String>,

    /// Asset name (defaults to bundle filename)
    #[arg(long)]
    pub asset_name: Option<String>,

    /// Create release as draft
    #[arg(long)]
    pub draft: bool,

    /// Create release as prerelease
    #[arg(long)]
    pub prerelease: bool,
}

#[derive(Args, Debug)]
pub struct BundleInstallArgs {
    /// Bundle source (path or URL)
    pub source: String,

    /// Skills to install (defaults to all)
    #[arg(long)]
    pub skills: Vec<String>,

    /// GitHub token (overrides env)
    #[arg(long)]
    pub token: Option<String>,

    /// Release tag (defaults to latest)
    #[arg(long)]
    pub tag: Option<String>,

    /// Asset name to download
    #[arg(long)]
    pub asset_name: Option<String>,

    /// Skip signature and checksum verification (not recommended)
    #[arg(long)]
    pub no_verify: bool,
}

#[derive(Args, Debug)]
pub struct BundleShowArgs {
    /// Bundle source (path, URL, or repo)
    pub source: String,

    /// GitHub token (overrides env)
    #[arg(long)]
    pub token: Option<String>,

    /// Release tag (for repo sources)
    #[arg(long)]
    pub tag: Option<String>,
}

#[derive(Args, Debug)]
pub struct BundleRemoveArgs {
    /// Bundle ID to remove
    pub bundle_id: String,

    /// Remove installed skills as well
    #[arg(long)]
    pub remove_skills: bool,

    /// Force removal without confirmation
    #[arg(long, short = 'f')]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct BundleConflictsArgs {
    /// Skill to check (default: all installed skills)
    #[arg(long)]
    pub skill: Option<String>,

    /// Bundle to check against (default: detect from installed bundles)
    #[arg(long)]
    pub bundle: Option<String>,

    /// Show only modified files
    #[arg(long)]
    pub modified_only: bool,

    /// Show detailed diff information
    #[arg(long)]
    pub diff: bool,
}

pub fn run(_ctx: &AppContext, _args: &BundleArgs) -> Result<()> {
    let ctx = _ctx;
    let args = _args;

    match &args.command {
        BundleCommand::Create(create) => run_create(ctx, create),
        BundleCommand::Install(install) => run_install(ctx, install),
        BundleCommand::Remove(remove) => run_remove(ctx, remove),
        BundleCommand::Publish(publish) => run_publish(ctx, publish),
        BundleCommand::List => run_list(ctx),
        BundleCommand::Show(show) => run_show(ctx, show),
        BundleCommand::Conflicts(conflicts) => run_conflicts(ctx, conflicts),
    }
}

fn run_create(ctx: &AppContext, args: &BundleCreateArgs) -> Result<()> {
    let skills = normalize_skill_list(&args.skills);
    if skills.is_empty() {
        return Err(MsError::ValidationFailed(
            "bundle create requires --skills".to_string(),
        ));
    }

    let bundle_id = args.id.clone().unwrap_or_else(|| slugify(&args.name));
    let root = ctx.git.root().to_path_buf();

    let mut entries = Vec::new();
    for skill_id in skills {
        let skill_dir = ctx.git.skill_path(&skill_id).ok_or_else(|| {
            MsError::SkillNotFound(format!("invalid skill id: {}", skill_id))
        })?;
        if !skill_dir.exists() {
            return Err(MsError::SkillNotFound(format!(
                "skill not found in archive: {}",
                skill_id
            )));
        }

        let rel = skill_dir
            .strip_prefix(&root)
            .unwrap_or(&skill_dir)
            .to_path_buf();
        let metadata = ctx.git.read_metadata(&skill_id)?;
        let version = if metadata.version.trim().is_empty() {
            None
        } else {
            Some(metadata.version)
        };
        entries.push(BundledSkill {
            name: skill_id,
            path: rel,
            version,
            hash: None,
            optional: false,
        });
    }

    let manifest = BundleManifest {
        bundle: BundleInfo {
            id: bundle_id.clone(),
            name: args.name.clone(),
            version: args.version.clone(),
            description: None,
            authors: Vec::new(),
            license: None,
            repository: None,
            keywords: Vec::new(),
            ms_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        },
        skills: entries,
        dependencies: Vec::new(),
        checksum: None,
        signatures: Vec::new(),
    };
    manifest.validate()?;

    let bundle = Bundle::new(manifest, &root);
    let package = bundle.package()?;
    package.verify()?;

    let output = args
        .output
        .clone()
        .unwrap_or_else(|| PathBuf::from(format!("{bundle_id}.msb")));
    if output.exists() {
        return Err(MsError::ValidationFailed(format!(
            "bundle output already exists: {}",
            output.display()
        )));
    }
    let bytes = package.to_bytes()?;
    std::fs::write(&output, bytes).map_err(|err| {
        MsError::Config(format!("write {}: {err}", output.display()))
    })?;

    let mut manifest_path = None;
    if args.write_manifest {
        let path = output
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(format!("{bundle_id}.bundle.toml"));
        if path.exists() {
            return Err(MsError::ValidationFailed(format!(
                "manifest output already exists: {}",
                path.display()
            )));
        }
        let toml = package.manifest.to_toml_string()?;
        std::fs::write(&path, toml).map_err(|err| {
            MsError::Config(format!("write {}: {err}", path.display()))
        })?;
        manifest_path = Some(path);
    }

    if ctx.robot_mode {
        let report = BundleCreateReport {
            id: bundle_id,
            output: output.display().to_string(),
            manifest_path: manifest_path.map(|p| p.display().to_string()),
            checksum: package.manifest.checksum.clone(),
        };
        return emit_json(&report);
    }

    println!("Bundle created: {}", output.display());
    if let Some(path) = manifest_path {
        println!("Manifest written: {}", path.display());
    }

    Ok(())
}

fn run_install(ctx: &AppContext, args: &BundleInstallArgs) -> Result<()> {
    // Parse the source using the new ParsedSource
    let parsed = ParsedSource::parse(&args.source)?;

    // Override tag and asset from args if provided
    let source = match parsed.source {
        InstallSource::GitHub { repo, tag, asset } => InstallSource::GitHub {
            repo,
            tag: args.tag.clone().or(tag),
            asset: args.asset_name.clone().or(asset),
        },
        other => other,
    };

    // Download or read the bundle bytes
    let bytes = match &source {
        InstallSource::File { path } => {
            let local_path = PathBuf::from(path);
            if !local_path.exists() {
                return Err(MsError::ValidationFailed(format!(
                    "bundle source not found: {}",
                    local_path.display()
                )));
            }
            std::fs::read(&local_path).map_err(|err| {
                MsError::Config(format!("read {}: {err}", local_path.display()))
            })?
        }
        InstallSource::Url { url } => {
            download_url(url, args.token.clone())?
        }
        InstallSource::GitHub { repo, tag, asset } => {
            let download = download_bundle(
                repo,
                tag.as_deref(),
                asset.as_deref(),
                args.token.clone(),
            )?;
            download.bytes
        }
    };

    let package = crate::bundler::package::BundlePackage::from_bytes(&bytes)?;
    let bundle_id = package.manifest.bundle.id.clone();
    let bundle_version = package.manifest.bundle.version.clone();
    let checksum = package.manifest.checksum.clone();

    // Check if already installed
    let mut registry = BundleRegistry::open(ctx.git.root())?;
    if registry.is_installed(&bundle_id) && !args.no_verify {
        return Err(MsError::ValidationFailed(format!(
            "bundle {} is already installed; use ms bundle remove first",
            bundle_id
        )));
    }

    let only = normalize_skill_list(&args.skills);

    // Install with verification (unless --no-verify is specified)
    let options = if args.no_verify {
        crate::bundler::InstallOptions::<crate::bundler::manifest::NoopSignatureVerifier>::allow_unsigned()
    } else {
        crate::bundler::InstallOptions::default()
    };
    let report = crate::bundler::install_with_options(&package, ctx.git.root(), &only, &options)?;

    // Register the installation
    let installed = InstalledBundle {
        id: bundle_id,
        version: bundle_version,
        source: source.clone(),
        installed_at: chrono::Utc::now(),
        skills: report.installed.clone(),
        checksum,
    };
    registry.register(installed)?;

    if ctx.robot_mode {
        return emit_json(&report);
    }

    print_install_report(&report);
    Ok(())
}

fn run_publish(ctx: &AppContext, args: &BundlePublishArgs) -> Result<()> {
    let repo = args.repo.clone().ok_or_else(|| {
        MsError::ValidationFailed("bundle publish requires --repo".to_string())
    })?;
    let config = GitHubConfig {
        repo,
        token: args.token.clone(),
        tag: args.tag.clone(),
        asset_name: args.asset_name.clone(),
        draft: args.draft,
        prerelease: args.prerelease,
    };
    let result = publish_bundle(std::path::Path::new(&args.path), &config)?;

    if ctx.robot_mode {
        return emit_json(&result);
    }

    println!("Published bundle to {}", result.repo);
    println!("Release: {}", result.release_url);
    println!("Asset: {} (tag {})", result.asset_name, result.tag);
    Ok(())
}

fn run_conflicts(ctx: &AppContext, args: &BundleConflictsArgs) -> Result<()> {
    let skills_dir = ctx.git.root().join("skills");
    if !skills_dir.exists() {
        if ctx.robot_mode {
            return emit_json(&ConflictsReport {
                skills: vec![],
                total_modified: 0,
                total_conflicts: 0,
            });
        }
        println!("No skills installed.");
        return Ok(());
    }

    let mut reports = Vec::new();

    // If specific skill requested, check only that one
    let skill_ids: Vec<String> = if let Some(ref skill) = args.skill {
        vec![skill.clone()]
    } else {
        // List all installed skills
        let mut ids = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&skills_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                        ids.push(name.to_string());
                    }
                }
            }
        }
        ids.sort();
        ids
    };

    for skill_id in skill_ids {
        let skill_path = skills_dir.join(&skill_id);
        if !skill_path.exists() {
            if args.skill.is_some() {
                return Err(MsError::SkillNotFound(format!(
                    "skill not found: {}",
                    skill_id
                )));
            }
            continue;
        }

        // Try to load expected hashes from bundle metadata
        let meta_path = skill_path.join(".bundle_meta.json");
        let expected_hashes: HashMap<PathBuf, String> = if meta_path.exists() {
            match std::fs::read_to_string(&meta_path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => HashMap::new(),
            }
        } else {
            HashMap::new()
        };

        let report = detect_modifications(&skill_path, &skill_id, &expected_hashes)?;

        // Filter to modified only if requested
        if args.modified_only && report.status == ModificationStatus::Clean {
            continue;
        }

        reports.push(report);
    }

    let total_modified = reports
        .iter()
        .filter(|r| r.status != ModificationStatus::Clean)
        .count();
    let total_conflicts = reports.iter().map(|r| r.summary.conflict).sum();

    if ctx.robot_mode {
        return emit_json(&ConflictsReport {
            skills: reports,
            total_modified,
            total_conflicts,
        });
    }

    if reports.is_empty() {
        println!("No modifications detected.");
        return Ok(());
    }

    for report in &reports {
        println!("Skill: {}", report.skill_id);
        println!("  Status: {:?}", report.status);
        println!(
            "  Files: {} total, {} modified, {} new, {} deleted",
            report.summary.total(),
            report.summary.modified,
            report.summary.new,
            report.summary.deleted
        );

        if args.diff && !report.files.is_empty() {
            println!("  Changes:");
            for file in &report.files {
                let status_str = match file.status {
                    ModificationStatus::Clean => "clean",
                    ModificationStatus::Modified => "modified",
                    ModificationStatus::New => "new",
                    ModificationStatus::Deleted => "deleted",
                    ModificationStatus::Conflict => "conflict",
                };
                println!("    {} [{}]", file.path.display(), status_str);
            }
        }
        println!();
    }

    println!(
        "Summary: {} skill(s) with modifications, {} conflict(s)",
        total_modified, total_conflicts
    );

    Ok(())
}

fn run_list(ctx: &AppContext) -> Result<()> {
    let bundles_dir = ctx.git.root().join("bundles");

    if !bundles_dir.exists() {
        if ctx.robot_mode {
            return emit_json(&BundleListReport {
                bundles: vec![],
                count: 0,
            });
        }
        println!("No bundles installed.");
        return Ok(());
    }

    // List .msb files in bundles directory
    let mut bundles = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&bundles_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "msb") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    bundles.push(name.to_string());
                }
            }
        }
    }
    bundles.sort();

    if ctx.robot_mode {
        return emit_json(&BundleListReport {
            count: bundles.len(),
            bundles,
        });
    }

    if bundles.is_empty() {
        println!("No bundles installed.");
    } else {
        println!("Installed bundles:");
        for bundle in &bundles {
            println!("  - {}", bundle);
        }
        println!("\n{} bundle(s) total.", bundles.len());
    }
    Ok(())
}

fn run_show(ctx: &AppContext, args: &BundleShowArgs) -> Result<()> {
    let local_path = expand_local_path(&args.source);
    let bytes = if local_path.exists() {
        std::fs::read(&local_path).map_err(|err| {
            MsError::Config(format!("read {}: {err}", local_path.display()))
        })?
    } else if looks_like_path(&args.source) {
        return Err(MsError::ValidationFailed(format!(
            "bundle source not found: {}",
            local_path.display()
        )));
    } else if args.source.starts_with("http://") || args.source.starts_with("https://") {
        download_url(&args.source, args.token.clone())?
    } else if let Some((repo, tag)) = split_repo_tag(&args.source) {
        let tag = args.tag.as_deref().or(tag);
        let download = download_bundle(repo, tag, None, args.token.clone())?;
        download.bytes
    } else {
        return Err(MsError::ValidationFailed(format!(
            "bundle source not found: {}",
            args.source
        )));
    };
    let package = crate::bundler::package::BundlePackage::from_bytes(&bytes)?;
    let manifest = &package.manifest;

    if ctx.robot_mode {
        return emit_json(&BundleShowReport {
            id: manifest.bundle.id.clone(),
            name: manifest.bundle.name.clone(),
            version: manifest.bundle.version.clone(),
            description: manifest.bundle.description.clone(),
            authors: manifest.bundle.authors.clone(),
            license: manifest.bundle.license.clone(),
            repository: manifest.bundle.repository.clone(),
            keywords: manifest.bundle.keywords.clone(),
            ms_version: manifest.bundle.ms_version.clone(),
            skills: manifest.skills.iter().map(|s| s.name.clone()).collect(),
            skill_count: manifest.skills.len(),
            checksum: manifest.checksum.clone(),
            signed: !manifest.signatures.is_empty(),
        });
    }

    println!("Bundle: {} ({})", manifest.bundle.name, manifest.bundle.id);
    println!("Version: {}", manifest.bundle.version);
    if let Some(desc) = &manifest.bundle.description {
        println!("Description: {}", desc);
    }
    if !manifest.bundle.authors.is_empty() {
        println!("Authors: {}", manifest.bundle.authors.join(", "));
    }
    if let Some(license) = &manifest.bundle.license {
        println!("License: {}", license);
    }
    if let Some(repo) = &manifest.bundle.repository {
        println!("Repository: {}", repo);
    }
    if !manifest.bundle.keywords.is_empty() {
        println!("Keywords: {}", manifest.bundle.keywords.join(", "));
    }
    if let Some(ms_ver) = &manifest.bundle.ms_version {
        println!("MS Version: {}", ms_ver);
    }

    println!("\nSkills ({}):", manifest.skills.len());
    for skill in &manifest.skills {
        let version_str = skill.version.as_deref().unwrap_or("-");
        let optional_str = if skill.optional { " (optional)" } else { "" };
        println!("  - {} v{}{}", skill.name, version_str, optional_str);
    }

    if let Some(checksum) = &manifest.checksum {
        println!("\nChecksum: {}", checksum);
    }
    if manifest.signatures.is_empty() {
        println!("Signed: no");
    } else {
        println!("Signed: yes ({} signature(s))", manifest.signatures.len());
    }

    Ok(())
}

fn normalize_skill_list(values: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for value in values {
        for part in value.split(',') {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                continue;
            }
            if seen.insert(trimmed.to_string()) {
                out.push(trimmed.to_string());
            }
        }
    }
    out
}

fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in input.chars() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            out.push(lower);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "bundle".to_string()
    } else {
        trimmed.to_string()
    }
}

fn print_install_report(report: &InstallReport) {
    println!("Bundle installed: {}", report.bundle_id);
    if !report.installed.is_empty() {
        println!("Installed:");
        for skill in &report.installed {
            println!("  - {}", skill);
        }
    }
    if !report.skipped.is_empty() {
        println!("Skipped:");
        for skill in &report.skipped {
            println!("  - {}", skill);
        }
    }
    println!("Blobs written: {}", report.blobs_written);
}

fn split_repo_tag(input: &str) -> Option<(&str, Option<&str>)> {
    if let Some((repo, tag)) = input.split_once('@') {
        return Some((repo, Some(tag)));
    }
    if input.contains('/') {
        return Some((input, None));
    }
    None
}

fn looks_like_path(input: &str) -> bool {
    input == "~"
        || input.starts_with("~/")
        || input.starts_with("./")
        || input.starts_with("../")
        || input.starts_with('/')
}

fn expand_local_path(input: &str) -> PathBuf {
    if input == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }
    if let Some(stripped) = input.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }
    PathBuf::from(input)
}

#[derive(serde::Serialize)]
struct BundleCreateReport {
    id: String,
    output: String,
    manifest_path: Option<String>,
    checksum: Option<String>,
}

#[derive(serde::Serialize)]
struct BundleListReport {
    bundles: Vec<String>,
    count: usize,
}

#[derive(serde::Serialize)]
struct BundleShowReport {
    id: String,
    name: String,
    version: String,
    description: Option<String>,
    authors: Vec<String>,
    license: Option<String>,
    repository: Option<String>,
    keywords: Vec<String>,
    ms_version: Option<String>,
    skills: Vec<String>,
    skill_count: usize,
    checksum: Option<String>,
    signed: bool,
}

#[derive(serde::Serialize)]
struct ConflictsReport {
    skills: Vec<SkillModificationReport>,
    total_modified: usize,
    total_conflicts: usize,
}
