//! ms doctor - Health checks and repairs

use std::path::Path;

use clap::Args;
use colored::Colorize;

use crate::app::AppContext;
use crate::error::Result;
use crate::storage::tx::GlobalLock;

#[derive(Args, Debug)]
pub struct DoctorArgs {
    /// Attempt to fix issues automatically
    #[arg(long)]
    pub fix: bool,

    /// Show verbose output
    #[arg(long)]
    pub verbose: bool,

    /// Check lock status
    #[arg(long)]
    pub check_lock: bool,

    /// Break a stale lock (use with caution)
    #[arg(long)]
    pub break_lock: bool,
}

pub fn run(ctx: &AppContext, args: &DoctorArgs) -> Result<()> {
    let mut issues_found = 0;
    let mut issues_fixed = 0;

    println!("{}", "ms doctor - Health Checks".bold());
    println!();

    // Check lock status if requested or as part of general health check
    if args.check_lock || !args.break_lock {
        issues_found += check_lock_status(ctx, args.verbose)?;
    }

    // Break lock if requested
    if args.break_lock {
        if break_stale_lock(ctx)? {
            issues_fixed += 1;
            println!("{} Stale lock broken", "✓".green());
        }
    }

    // Check database integrity
    issues_found += check_database(ctx, args.verbose)?;

    // Check Git archive integrity
    issues_found += check_git_archive(ctx, args.verbose)?;

    // Check for incomplete transactions
    issues_found += check_transactions(ctx, args.fix, args.verbose, &mut issues_fixed)?;

    // Summary
    println!();
    if issues_found == 0 {
        println!("{} All checks passed", "✓".green().bold());
    } else if args.fix && issues_fixed == issues_found {
        println!(
            "{} Found {} issues, fixed {}",
            "✓".green().bold(),
            issues_found,
            issues_fixed
        );
    } else {
        println!(
            "{} Found {} issues, fixed {}",
            "!".yellow().bold(),
            issues_found,
            issues_fixed
        );
        if !args.fix && issues_found > issues_fixed {
            println!("  Run with --fix to attempt automatic repairs");
        }
    }

    Ok(())
}

/// Check the global lock status
fn check_lock_status(ctx: &AppContext, verbose: bool) -> Result<usize> {
    print!("Checking lock status... ");

    let ms_root = &ctx.ms_root;

    match GlobalLock::status(ms_root)? {
        Some(holder) => {
            println!("{} Lock held", "!".yellow());
            println!("  PID: {}", holder.pid);
            println!("  Host: {}", holder.hostname);
            println!("  Since: {}", holder.acquired_at);

            // Check if process is still alive
            #[cfg(target_os = "linux")]
            {
                let proc_path = format!("/proc/{}", holder.pid);
                if !Path::new(&proc_path).exists() {
                    println!(
                        "  {} Process {} no longer exists - lock may be stale",
                        "!".yellow(),
                        holder.pid
                    );
                    println!("  Use --break-lock to remove stale lock");
                    return Ok(1);
                }
            }

            if verbose {
                println!("  Lock is held by an active process");
            }
            Ok(0) // Active lock is not an issue
        }
        None => {
            println!("{} No lock held", "✓".green());
            Ok(0)
        }
    }
}

/// Break a stale lock
fn break_stale_lock(ctx: &AppContext) -> Result<bool> {
    print!("Breaking stale lock... ");

    let ms_root = &ctx.ms_root;

    // First check if there's a lock to break
    match GlobalLock::status(ms_root)? {
        Some(holder) => {
            // Warn user about what we're doing
            println!();
            println!(
                "  {} Breaking lock held by PID {} on {} since {}",
                "!".yellow(),
                holder.pid,
                holder.hostname,
                holder.acquired_at
            );

            if GlobalLock::break_lock(ms_root)? {
                Ok(true)
            } else {
                println!("  Lock file not found");
                Ok(false)
            }
        }
        None => {
            println!("{} No lock to break", "✓".green());
            Ok(false)
        }
    }
}

/// Check database integrity
fn check_database(ctx: &AppContext, verbose: bool) -> Result<usize> {
    print!("Checking database... ");

    let db_path = ctx.ms_root.join("ms.db");
    if !db_path.exists() {
        println!("{} Database not found", "!".yellow());
        println!("  Run 'ms init' to create the database");
        return Ok(1);
    }

    // Try to open and run integrity check
    match crate::storage::Database::open(&db_path) {
        Ok(db) => {
            // Run SQLite integrity check
            match db.integrity_check() {
                Ok(true) => {
                    println!("{} OK", "✓".green());
                    if verbose {
                        println!("  Database path: {}", db_path.display());
                    }
                    Ok(0)
                }
                Ok(false) => {
                    println!("{} Integrity check failed", "✗".red());
                    Ok(1)
                }
                Err(e) => {
                    println!("{} Error: {}", "✗".red(), e);
                    Ok(1)
                }
            }
        }
        Err(e) => {
            println!("{} Cannot open: {}", "✗".red(), e);
            Ok(1)
        }
    }
}

/// Check Git archive integrity
fn check_git_archive(ctx: &AppContext, verbose: bool) -> Result<usize> {
    print!("Checking Git archive... ");

    let archive_path = ctx.ms_root.join("archive");
    if !archive_path.exists() {
        println!("{} Archive not found", "!".yellow());
        println!("  Run 'ms init' to create the archive");
        return Ok(1);
    }

    let git_dir = archive_path.join(".git");
    if !git_dir.exists() {
        println!("{} Not a Git repository", "✗".red());
        return Ok(1);
    }

    match crate::storage::GitArchive::open(&archive_path) {
        Ok(_git) => {
            println!("{} OK", "✓".green());
            if verbose {
                println!("  Archive path: {}", archive_path.display());
            }
            Ok(0)
        }
        Err(e) => {
            println!("{} Cannot open: {}", "✗".red(), e);
            Ok(1)
        }
    }
}

/// Check for incomplete transactions
fn check_transactions(
    ctx: &AppContext,
    fix: bool,
    verbose: bool,
    issues_fixed: &mut usize,
) -> Result<usize> {
    print!("Checking transactions... ");

    let db_path = ctx.ms_root.join("ms.db");
    let archive_path = ctx.ms_root.join("archive");

    if !db_path.exists() || !archive_path.exists() {
        println!("{} Skipped (database or archive not found)", "-".dimmed());
        return Ok(0);
    }

    let db = match crate::storage::Database::open(&db_path) {
        Ok(db) => std::sync::Arc::new(db),
        Err(_) => {
            println!("{} Skipped (cannot open database)", "-".dimmed());
            return Ok(0);
        }
    };

    let git = match crate::storage::GitArchive::open(&archive_path) {
        Ok(git) => std::sync::Arc::new(git),
        Err(_) => {
            println!("{} Skipped (cannot open archive)", "-".dimmed());
            return Ok(0);
        }
    };

    // Check for incomplete transactions
    let tx_mgr = crate::storage::TxManager::new(db.clone(), git.clone(), ctx.ms_root.clone())?;

    if fix {
        let report = tx_mgr.recover()?;
        if report.had_work() {
            println!("{} Recovered", "✓".green());
            if verbose {
                println!("  Rolled back: {}", report.rolled_back);
                println!("  Completed: {}", report.completed);
                println!("  Orphaned files cleaned: {}", report.orphaned_files);
            }
            *issues_fixed += report.rolled_back + report.completed + report.orphaned_files;
            Ok(report.rolled_back + report.completed + report.orphaned_files)
        } else {
            println!("{} OK", "✓".green());
            Ok(0)
        }
    } else {
        // Just check without fixing
        let incomplete = db.list_incomplete_transactions()?;
        if incomplete.is_empty() {
            println!("{} OK", "✓".green());
            Ok(0)
        } else {
            println!(
                "{} {} incomplete transactions",
                "!".yellow(),
                incomplete.len()
            );
            if verbose {
                for tx in &incomplete {
                    println!("  - {} ({}, phase: {})", tx.id, tx.entity_type, tx.phase);
                }
            }
            println!("  Run with --fix to recover transactions");
            Ok(incomplete.len())
        }
    }
}
