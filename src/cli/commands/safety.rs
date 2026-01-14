//! Safety command - DCG safety gate status, logs, and command checking.

use clap::{Args, Subcommand};
use colored::Colorize;
use serde_json::json;

use crate::app::AppContext;
use crate::core::safety::SafetyTier;
use crate::error::Result;
use crate::security::SafetyGate;

#[derive(Args, Debug)]
pub struct SafetyArgs {
    #[command(subcommand)]
    pub command: SafetyCommand,
}

#[derive(Subcommand, Debug)]
pub enum SafetyCommand {
    /// Show DCG safety gate status
    Status,

    /// Show recent safety events log
    Log(LogArgs),

    /// Check a command through the safety gate
    Check(CheckArgs),
}

#[derive(Args, Debug)]
pub struct LogArgs {
    /// Maximum number of events to show
    #[arg(short, long, default_value = "20")]
    pub limit: usize,

    /// Filter by session ID
    #[arg(long)]
    pub session: Option<String>,

    /// Show only blocked events
    #[arg(long)]
    pub blocked_only: bool,
}

#[derive(Args, Debug)]
pub struct CheckArgs {
    /// The command to check
    pub command: String,

    /// Session ID for audit logging
    #[arg(long)]
    pub session_id: Option<String>,

    /// Dry run - don't log the event
    #[arg(long)]
    pub dry_run: bool,
}

pub fn run(ctx: &AppContext, args: &SafetyArgs) -> Result<()> {
    match &args.command {
        SafetyCommand::Status => run_status(ctx),
        SafetyCommand::Log(log_args) => run_log(ctx, log_args),
        SafetyCommand::Check(check_args) => run_check(ctx, check_args),
    }
}

/// Show DCG safety gate status.
fn run_status(ctx: &AppContext) -> Result<()> {
    let gate = SafetyGate::from_context(ctx);
    let status = gate.status();

    if ctx.robot_mode {
        let output = json!({
            "dcg_available": status.dcg_version.is_some(),
            "dcg_version": status.dcg_version,
            "dcg_bin": status.dcg_bin.display().to_string(),
            "packs": status.packs,
            "require_verbatim_approval": ctx.config.safety.require_verbatim_approval,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("{}", "Safety Gate Status".bold());
        println!("{}", "─".repeat(30));
        println!();

        match &status.dcg_version {
            Some(version) => {
                println!("  {} DCG Available", "✓".green());
                println!("    Version: {}", version.cyan());
            }
            None => {
                println!("  {} DCG Not Available", "✗".red());
                println!("    Commands will be allowed with warnings");
            }
        }
        println!("    Binary: {}", status.dcg_bin.display());

        if !status.packs.is_empty() {
            println!("    Packs: {}", status.packs.join(", ").dimmed());
        }

        println!();
        println!(
            "  Verbatim Approval: {}",
            if ctx.config.safety.require_verbatim_approval {
                "Required".yellow()
            } else {
                "Disabled".dimmed()
            }
        );
    }

    Ok(())
}

/// Show recent safety events log.
fn run_log(ctx: &AppContext, args: &LogArgs) -> Result<()> {
    let events = ctx.db.list_command_safety_events(args.limit)?;

    // Filter events if needed
    let events: Vec<_> = events
        .into_iter()
        .filter(|e| {
            if let Some(session) = &args.session {
                e.session_id.as_deref() == Some(session.as_str())
            } else {
                true
            }
        })
        .filter(|e| {
            if args.blocked_only {
                !e.decision.allowed
            } else {
                true
            }
        })
        .collect();

    if ctx.robot_mode {
        let output = json!({
            "events": events,
            "count": events.len(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        if events.is_empty() {
            println!("No safety events found.");
            return Ok(());
        }

        println!("{}", "Safety Event Log".bold());
        println!("{}", "─".repeat(60));
        println!();

        for event in &events {
            let status_icon = if event.decision.allowed {
                if event.decision.approved {
                    "✓".green()
                } else {
                    "○".green()
                }
            } else {
                "✗".red()
            };

            let tier_label = format_tier(&event.decision.tier);

            println!(
                "{} [{}] {} {}",
                status_icon,
                tier_label,
                event.created_at.dimmed(),
                event.session_id.as_deref().unwrap_or("-").dimmed()
            );
            println!("    {}", truncate_command(&event.command, 70));

            if !event.decision.allowed {
                println!("    Reason: {}", event.decision.reason.yellow());
                if let Some(remediation) = &event.decision.remediation {
                    println!("    Fix: {}", remediation.cyan());
                }
            }
            println!();
        }

        println!(
            "Showing {} of {} events",
            events.len().to_string().cyan(),
            args.limit.to_string().dimmed()
        );
    }

    Ok(())
}

/// Check a command through the safety gate.
fn run_check(ctx: &AppContext, args: &CheckArgs) -> Result<()> {
    let gate = SafetyGate::from_context(ctx);

    // Get DCG decision without logging
    let status = gate.status();
    if status.dcg_version.is_none() {
        if ctx.robot_mode {
            let output = json!({
                "error": true,
                "code": "dcg_unavailable",
                "message": "DCG is not available",
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("{} DCG is not available", "!".yellow());
            println!("  Configure dcg_bin in your ms config");
        }
        return Ok(());
    }

    // Use the core safety module directly to avoid logging on dry run
    use crate::core::safety::DcgGuard;
    let guard = DcgGuard::new(
        ctx.config.safety.dcg_bin.clone(),
        ctx.config.safety.dcg_packs.clone(),
        ctx.config.safety.dcg_explain_format.clone(),
    );

    let decision = match guard.evaluate_command(&args.command) {
        Ok(d) => d,
        Err(e) => {
            if ctx.robot_mode {
                let output = json!({
                    "error": true,
                    "code": "dcg_error",
                    "message": e.to_string(),
                });
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                println!("{} DCG error: {}", "✗".red(), e);
            }
            return Ok(());
        }
    };

    // Determine if approval would be required
    let approval_required = !decision.allowed
        && ctx.config.safety.require_verbatim_approval
        && decision.tier >= SafetyTier::Danger;

    if ctx.robot_mode {
        let output = json!({
            "command": args.command,
            "allowed": decision.allowed,
            "tier": format!("{:?}", decision.tier).to_lowercase(),
            "reason": decision.reason,
            "remediation": decision.remediation,
            "rule_id": decision.rule_id,
            "pack": decision.pack,
            "approval_required": approval_required,
            "approval_hint": if approval_required {
                Some(format!("MS_APPROVE_COMMAND=\"{}\"", args.command))
            } else {
                None
            },
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("{}", "Safety Check Result".bold());
        println!("{}", "─".repeat(40));
        println!();

        println!("  Command: {}", args.command.cyan());
        println!();

        let status_icon = if decision.allowed {
            "✓".green()
        } else {
            "✗".red()
        };
        let tier_label = format_tier(&decision.tier);

        println!("  {} {} - {}", status_icon, tier_label, decision.reason);

        if let Some(rule_id) = &decision.rule_id {
            println!("    Rule: {}", rule_id.dimmed());
        }
        if let Some(pack) = &decision.pack {
            println!("    Pack: {}", pack.dimmed());
        }

        if !decision.allowed {
            if let Some(remediation) = &decision.remediation {
                println!();
                println!("  {} {}", "Suggestion:".yellow(), remediation);
            }

            if approval_required {
                println!();
                println!(
                    "  {} Approval required. Set environment variable:",
                    "!".yellow()
                );
                println!(
                    "    {}=\"{}\"",
                    "MS_APPROVE_COMMAND".cyan(),
                    args.command
                );
            }
        }
    }

    // Log the event if not dry run
    if !args.dry_run {
        let _session_id = args.session_id.as_deref();
        // Note: In a full implementation, we would log the event here
        // using gate.enforce() or a similar mechanism
    }

    Ok(())
}

fn format_tier(tier: &SafetyTier) -> colored::ColoredString {
    match tier {
        SafetyTier::Safe => "SAFE".green(),
        SafetyTier::Caution => "CAUTION".yellow(),
        SafetyTier::Danger => "DANGER".red(),
        SafetyTier::Critical => "CRITICAL".red().bold(),
    }
}

fn truncate_command(cmd: &str, max_len: usize) -> String {
    if cmd.len() <= max_len {
        cmd.to_string()
    } else {
        format!("{}...", &cmd[..max_len - 3])
    }
}
