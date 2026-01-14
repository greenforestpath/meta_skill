//! ms bundle - Manage skill bundles

use std::collections::HashSet;
use std::path::{PathBuf};

use clap::{Args, Subcommand};

use crate::app::AppContext;
use crate::bundler::{Bundle, BundleInfo, BundleManifest, BundledSkill};
use crate::bundler::install::{install as install_bundle, InstallReport};
use crate::error::{MsError, Result};
use crate::cli::output::emit_json;

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
    /// List installed bundles
    List,
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
}

#[derive(Args, Debug)]
pub struct BundleInstallArgs {
    /// Bundle source (path or URL)
    pub source: String,

    /// Skills to install (defaults to all)
    #[arg(long)]
    pub skills: Vec<String>,
}

pub fn run(_ctx: &AppContext, _args: &BundleArgs) -> Result<()> {
    let ctx = _ctx;
    let args = _args;

    match &args.command {
        BundleCommand::Create(create) => run_create(ctx, create),
        BundleCommand::Install(install) => run_install(ctx, install),
        _ => Ok(()),
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
        let skill_dir = ctx.git.skill_path(&skill_id);
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
    if args.source.starts_with("http://") || args.source.starts_with("https://") {
        return Err(MsError::ValidationFailed(
            "bundle install only supports local files for now".to_string(),
        ));
    }

    let bytes = std::fs::read(&args.source).map_err(|err| {
        MsError::Config(format!("read {}: {err}", args.source))
    })?;
    let package = crate::bundler::package::BundlePackage::from_bytes(&bytes)?;

    let only = normalize_skill_list(&args.skills);
    let report = install_bundle(&package, ctx.git.root(), &only)?;

    if ctx.robot_mode {
        return emit_json(&report);
    }

    print_install_report(&report);
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

#[derive(serde::Serialize)]
struct BundleCreateReport {
    id: String,
    output: String,
    manifest_path: Option<String>,
    checksum: Option<String>,
}
