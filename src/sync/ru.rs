//! RU (Repo Updater) integration for skill repository sync.
//!
//! Wraps the `ru` CLI tool to provide repository synchronization
//! for skill repositories distributed via GitHub.

use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::error::{MsError, Result};

/// Exit codes from ru (see AGENTS.md)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuExitCode {
    /// Success - all operations completed
    Ok = 0,
    /// Partial success - some repos had issues
    Partial = 1,
    /// Conflicts detected - manual intervention needed
    Conflicts = 2,
    /// System error - git/network failure
    SystemError = 3,
    /// Bad arguments - invalid CLI usage
    BadArgs = 4,
    /// Interrupted - can resume with --resume
    Interrupted = 5,
}

impl RuExitCode {
    pub fn from_code(code: i32) -> Self {
        match code {
            0 => Self::Ok,
            1 => Self::Partial,
            2 => Self::Conflicts,
            3 => Self::SystemError,
            4 => Self::BadArgs,
            5 => Self::Interrupted,
            _ => Self::SystemError,
        }
    }

    pub fn is_success(self) -> bool {
        matches!(self, Self::Ok | Self::Partial)
    }
}

/// Result from an ru sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuSyncResult {
    pub exit_code: i32,
    pub cloned: Vec<String>,
    pub pulled: Vec<String>,
    pub conflicts: Vec<RuConflict>,
    pub errors: Vec<RuError>,
    pub skipped: Vec<String>,
}

/// A conflict detected by ru
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuConflict {
    pub repo: String,
    pub reason: String,
    #[serde(default)]
    pub resolution_hint: Option<String>,
}

/// An error reported by ru
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuError {
    pub repo: String,
    pub error: String,
}

/// Repository status from ru
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuRepoStatus {
    pub path: PathBuf,
    pub name: String,
    pub clean: bool,
    pub ahead: u32,
    pub behind: u32,
    #[serde(default)]
    pub branch: Option<String>,
}

/// Options for ru sync
#[derive(Debug, Clone, Default)]
pub struct RuSyncOptions {
    pub dry_run: bool,
    pub clone_only: bool,
    pub pull_only: bool,
    pub autostash: bool,
    pub rebase: bool,
    pub parallel: Option<u32>,
    pub resume: bool,
}

/// Client for interacting with the ru CLI
pub struct RuClient {
    /// Path to ru binary (None = auto-detect)
    ru_path: Option<PathBuf>,
    /// Cached detection result
    available: Option<bool>,
}

impl Default for RuClient {
    fn default() -> Self {
        Self::new()
    }
}

impl RuClient {
    /// Create a new RuClient with auto-detection
    pub fn new() -> Self {
        Self {
            ru_path: None,
            available: None,
        }
    }

    /// Create an RuClient with explicit path
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            ru_path: Some(path),
            available: None,
        }
    }

    /// Check if ru is available
    pub fn is_available(&mut self) -> bool {
        if let Some(available) = self.available {
            return available;
        }

        let result = self.detect_ru();
        self.available = Some(result);
        result
    }

    /// Get the ru binary path
    pub fn ru_path(&self) -> &str {
        self.ru_path
            .as_ref()
            .map(|p| p.to_str().unwrap_or("ru"))
            .unwrap_or("ru")
    }

    /// Detect if ru is installed and working
    fn detect_ru(&self) -> bool {
        let output = Command::new(self.ru_path())
            .arg("--version")
            .output();

        match output {
            Ok(out) => out.status.success(),
            Err(_) => false,
        }
    }

    /// Sync all configured repositories
    pub fn sync(&mut self, options: &RuSyncOptions) -> Result<RuSyncResult> {
        if !self.is_available() {
            return Err(MsError::Config(
                "ru is not available; install from /data/projects/repo_updater".to_string(),
            ));
        }

        let mut cmd = Command::new(self.ru_path());
        cmd.arg("sync")
            .arg("--json")
            .arg("--non-interactive");

        if options.dry_run {
            cmd.arg("--dry-run");
        }
        if options.clone_only {
            cmd.arg("--clone-only");
        }
        if options.pull_only {
            cmd.arg("--pull-only");
        }
        if options.autostash {
            cmd.arg("--autostash");
        }
        if options.rebase {
            cmd.arg("--rebase");
        }
        if let Some(parallel) = options.parallel {
            cmd.arg("-j").arg(parallel.to_string());
        }
        if options.resume {
            cmd.arg("--resume");
        }

        let output = cmd.output().map_err(|err| {
            MsError::Config(format!("failed to execute ru sync: {err}"))
        })?;

        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Parse JSON output
        let mut result: RuSyncResult = serde_json::from_str(&stdout).unwrap_or_else(|_| {
            // Fallback for non-JSON output
            RuSyncResult {
                exit_code,
                cloned: Vec::new(),
                pulled: Vec::new(),
                conflicts: Vec::new(),
                errors: vec![RuError {
                    repo: "unknown".to_string(),
                    error: stdout.to_string(),
                }],
                skipped: Vec::new(),
            }
        });
        // Ensure exit code matches actual process status
        result.exit_code = exit_code;

        if !output.status.success() && !stderr.trim().is_empty() {
            result.errors.push(RuError {
                repo: "unknown".to_string(),
                error: stderr.trim().to_string(),
            });
        }

        Ok(result)
    }

    /// Get status of all repositories without making changes
    pub fn status(&mut self) -> Result<Vec<RuRepoStatus>> {
        if !self.is_available() {
            return Err(MsError::Config(
                "ru is not available; install from /data/projects/repo_updater".to_string(),
            ));
        }

        let output = Command::new(self.ru_path())
            .args(["status", "--no-fetch", "--json"])
            .output()
            .map_err(|err| MsError::Config(format!("failed to execute ru status: {err}")))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(MsError::Config(format!(
                "ru status failed: {}",
                stderr.trim()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let statuses: Vec<RuRepoStatus> = serde_json::from_str(&stdout).unwrap_or_default();
        Ok(statuses)
    }

    /// List all configured repository paths
    pub fn list_paths(&mut self) -> Result<Vec<PathBuf>> {
        if !self.is_available() {
            return Err(MsError::Config(
                "ru is not available; install from /data/projects/repo_updater".to_string(),
            ));
        }

        let output = Command::new(self.ru_path())
            .args(["list", "--paths"])
            .output()
            .map_err(|err| MsError::Config(format!("failed to execute ru list: {err}")))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(MsError::Config(format!(
                "ru list failed: {}",
                stderr.trim()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let paths: Vec<PathBuf> = stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(PathBuf::from)
            .collect();

        Ok(paths)
    }

    /// Run ru doctor to check system health
    pub fn doctor(&mut self) -> Result<bool> {
        if !self.is_available() {
            return Err(MsError::Config(
                "ru is not available; install from /data/projects/repo_updater".to_string(),
            ));
        }

        let output = Command::new(self.ru_path())
            .arg("doctor")
            .output()
            .map_err(|err| MsError::Config(format!("failed to execute ru doctor: {err}")))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(MsError::Config(format!(
                "ru doctor failed: {}",
                stderr.trim()
            )));
        }

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_code_from_code_maps_correctly() {
        assert_eq!(RuExitCode::from_code(0), RuExitCode::Ok);
        assert_eq!(RuExitCode::from_code(1), RuExitCode::Partial);
        assert_eq!(RuExitCode::from_code(2), RuExitCode::Conflicts);
        assert_eq!(RuExitCode::from_code(3), RuExitCode::SystemError);
        assert_eq!(RuExitCode::from_code(4), RuExitCode::BadArgs);
        assert_eq!(RuExitCode::from_code(5), RuExitCode::Interrupted);
        assert_eq!(RuExitCode::from_code(99), RuExitCode::SystemError);
    }

    #[test]
    fn exit_code_is_success() {
        assert!(RuExitCode::Ok.is_success());
        assert!(RuExitCode::Partial.is_success());
        assert!(!RuExitCode::Conflicts.is_success());
        assert!(!RuExitCode::SystemError.is_success());
    }

    #[test]
    fn ru_sync_options_default() {
        let opts = RuSyncOptions::default();
        assert!(!opts.dry_run);
        assert!(!opts.clone_only);
        assert!(!opts.pull_only);
        assert!(!opts.autostash);
        assert!(!opts.rebase);
        assert!(opts.parallel.is_none());
        assert!(!opts.resume);
    }

    #[test]
    fn ru_client_default_path() {
        let client = RuClient::new();
        assert_eq!(client.ru_path(), "ru");
    }

    #[test]
    fn ru_client_with_explicit_path() {
        let client = RuClient::with_path(PathBuf::from("/usr/local/bin/ru"));
        assert_eq!(client.ru_path(), "/usr/local/bin/ru");
    }
}
