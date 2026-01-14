//! ms security - Prompt injection defense and quarantine controls

use clap::{Args, Subcommand};
use serde::Serialize;

use crate::app::AppContext;
use crate::cli::output::emit_json;
use crate::error::{MsError, Result};
use crate::security::{AcipEngine, ContentSource};
use crate::security::acip::prompt_version;

#[derive(Args, Debug)]
pub struct SecurityArgs {
    #[command(subcommand)]
    pub command: SecurityCommand,
}

#[derive(Subcommand, Debug)]
pub enum SecurityCommand {
    /// Show ACIP status and prompt health
    Status,
    /// Show effective ACIP config
    Config,
    /// Show ACIP version (config + detected)
    Version,
    /// Test ACIP classification on a single input
    Test {
        /// Input text to classify
        input: String,
        /// Content source (user|assistant|tool|file)
        #[arg(long, default_value = "user")]
        source: String,
    },
    /// Scan sessions for injection attempts (stub)
    Scan,
    /// Quarantine management
    Quarantine(QuarantineArgs),
}

#[derive(Args, Debug)]
pub struct QuarantineArgs {
    #[command(subcommand)]
    pub command: QuarantineCommand,
}

#[derive(Subcommand, Debug)]
pub enum QuarantineCommand {
    /// List recent quarantine records
    List {
        /// Max records to return
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
    /// Show a specific quarantine record
    Show {
        /// Quarantine record id
        id: String,
    },
}

#[derive(Serialize)]
struct StatusOutput {
    ok: bool,
    enabled: bool,
    acip_version: String,
    detected_version: Option<String>,
    audit_mode: bool,
    prompt_path: String,
    error: Option<String>,
}

#[derive(Serialize)]
struct VersionOutput {
    configured: String,
    detected: Option<String>,
}

pub fn run(ctx: &AppContext, args: &SecurityArgs) -> Result<()> {
    match &args.command {
        SecurityCommand::Status => status(ctx),
        SecurityCommand::Config => config(ctx),
        SecurityCommand::Version => version(ctx),
        SecurityCommand::Test { input, source } => test(ctx, input, source),
        SecurityCommand::Scan => not_implemented(ctx, "ms security scan not implemented yet"),
        SecurityCommand::Quarantine(cmd) => quarantine(ctx, cmd),
    }
}

fn status(ctx: &AppContext) -> Result<()> {
    let cfg = &ctx.config.security.acip;
    let detected = prompt_version(&cfg.prompt_path).ok().flatten();
    let (ok, error) = if cfg.enabled {
        match AcipEngine::load(cfg.clone()) {
            Ok(_) => (true, None),
            Err(err) => (false, Some(err.to_string())),
        }
    } else {
        (false, Some("ACIP disabled".to_string()))
    };

    let payload = StatusOutput {
        ok,
        enabled: cfg.enabled,
        acip_version: cfg.version.clone(),
        detected_version: detected,
        audit_mode: cfg.audit_mode,
        prompt_path: cfg.prompt_path.display().to_string(),
        error,
    };

    emit_output(ctx, &payload)
}

fn config(ctx: &AppContext) -> Result<()> {
    emit_output(ctx, &ctx.config.security.acip)
}

fn version(ctx: &AppContext) -> Result<()> {
    let cfg = &ctx.config.security.acip;
    let detected = prompt_version(&cfg.prompt_path).ok().flatten();
    let payload = VersionOutput {
        configured: cfg.version.clone(),
        detected,
    };
    emit_output(ctx, &payload)
}

fn test(ctx: &AppContext, input: &str, source: &str) -> Result<()> {
    let cfg = ctx.config.security.acip.clone();
    let engine = AcipEngine::load(cfg)?;
    let source = parse_source(source)?;
    let analysis = engine.analyze(input, source)?;
    emit_output(ctx, &analysis)
}

fn quarantine(ctx: &AppContext, args: &QuarantineArgs) -> Result<()> {
    match &args.command {
        QuarantineCommand::List { limit } => {
            let records = ctx.db.list_quarantine_records(*limit)?;
            emit_output(ctx, &records)
        }
        QuarantineCommand::Show { id } => {
            let record = ctx.db.get_quarantine_record(id)?;
            if ctx.robot_mode {
                emit_output(ctx, &record)
            } else {
                match record {
                    Some(rec) => emit_output(ctx, &rec),
                    None => Err(MsError::Config(format!("quarantine record not found: {id}"))),
                }
            }
        }
    }
}

fn parse_source(raw: &str) -> Result<ContentSource> {
    match raw.to_lowercase().as_str() {
        "user" => Ok(ContentSource::User),
        "assistant" => Ok(ContentSource::Assistant),
        "tool" | "tool_output" => Ok(ContentSource::ToolOutput),
        "file" | "file_contents" => Ok(ContentSource::File),
        _ => Err(MsError::Config(format!(
            "invalid source {raw} (expected user|assistant|tool|file)"
        ))),
    }
}

fn not_implemented(ctx: &AppContext, message: &str) -> Result<()> {
    if ctx.robot_mode {
        let payload = serde_json::json!({
            "ok": false,
            "error": "not_implemented",
            "message": message,
        });
        emit_json(&payload)
    } else {
        println!("{message}");
        Ok(())
    }
}

fn emit_output<T: Serialize>(ctx: &AppContext, payload: &T) -> Result<()> {
    if ctx.robot_mode {
        emit_json(payload)
    } else {
        let pretty = serde_json::to_string_pretty(payload)
            .map_err(|err| MsError::Config(format!("serialize output: {err}")))?;
        println!("{pretty}");
        Ok(())
    }
}
