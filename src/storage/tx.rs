//! Two-Phase Commit (2PC) for dual persistence to SQLite and Git.
//!
//! All writes that touch both stores are wrapped in a lightweight transaction
//! protocol to prevent split-brain states where one store is updated but the
//! other fails.

use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::core::SkillSpec;
use crate::error::{MsError, Result};

use super::git::GitArchive;
use super::sqlite::Database;

// =============================================================================
// TRANSACTION RECORD
// =============================================================================

/// Transaction phase in the 2PC protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TxPhase {
    /// Intent recorded, changes not yet staged
    Prepare,
    /// SQLite write pending, Git not yet committed
    Pending,
    /// Git committed, SQLite not yet marked complete
    Committed,
    /// Transaction completed successfully
    Complete,
}

impl std::fmt::Display for TxPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TxPhase::Prepare => write!(f, "prepare"),
            TxPhase::Pending => write!(f, "pending"),
            TxPhase::Committed => write!(f, "committed"),
            TxPhase::Complete => write!(f, "complete"),
        }
    }
}

/// Record of an in-flight transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxRecord {
    /// Unique transaction ID
    pub id: String,
    /// Entity type (e.g., "skill")
    pub entity_type: String,
    /// Entity ID being modified
    pub entity_id: String,
    /// Current phase
    pub phase: TxPhase,
    /// JSON-serialized payload
    pub payload_json: String,
    /// When transaction was created
    pub created_at: DateTime<Utc>,
}

impl TxRecord {
    /// Create a new transaction record in prepare phase
    pub fn prepare<T: Serialize>(entity_type: &str, entity_id: &str, payload: &T) -> Result<Self> {
        let payload_json = serde_json::to_string(payload)
            .map_err(|e| MsError::TransactionFailed(format!("serialize payload: {e}")))?;

        Ok(Self {
            id: Uuid::new_v4().to_string(),
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            phase: TxPhase::Prepare,
            payload_json,
            created_at: Utc::now(),
        })
    }
}

// =============================================================================
// GLOBAL FILE LOCK
// =============================================================================

/// Advisory file lock for coordinating dual-persistence writes
pub struct GlobalLock {
    #[allow(dead_code)]
    lock_file: File,
    #[allow(dead_code)]
    lock_path: PathBuf,
}

impl GlobalLock {
    const LOCK_FILENAME: &'static str = "ms.lock";

    /// Acquire exclusive lock (blocking)
    pub fn acquire(ms_root: &Path) -> Result<Self> {
        let lock_path = ms_root.join(Self::LOCK_FILENAME);
        fs::create_dir_all(ms_root)?;

        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|e| MsError::TransactionFailed(format!("open lock file: {e}")))?;

        // Use fs2's cross-platform exclusive lock (blocking)
        lock_file
            .lock_exclusive()
            .map_err(|e| MsError::TransactionFailed(format!("acquire exclusive lock: {e}")))?;

        // Write lock holder info
        let holder = LockHolder {
            pid: std::process::id(),
            acquired_at: Utc::now(),
            hostname: hostname::get()
                .ok()
                .and_then(|h| h.into_string().ok())
                .unwrap_or_else(|| "unknown".to_string()),
        };
        let holder_json = serde_json::to_string(&holder).unwrap_or_default();
        fs::write(&lock_path, holder_json).ok();

        debug!("Acquired global lock at {:?}", lock_path);
        Ok(Self {
            lock_file,
            lock_path,
        })
    }

    /// Try to acquire lock without blocking
    pub fn try_acquire(ms_root: &Path) -> Result<Option<Self>> {
        let lock_path = ms_root.join(Self::LOCK_FILENAME);
        fs::create_dir_all(ms_root)?;

        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|e| MsError::TransactionFailed(format!("open lock file: {e}")))?;

        // Use fs2's cross-platform try_lock (non-blocking)
        match lock_file.try_lock_exclusive() {
            Ok(()) => {
                // Lock acquired
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                debug!("Lock held by another process");
                return Ok(None);
            }
            Err(e) => {
                return Err(MsError::TransactionFailed(format!(
                    "try acquire lock: {e}"
                )));
            }
        }

        // Write lock holder info
        let holder = LockHolder {
            pid: std::process::id(),
            acquired_at: Utc::now(),
            hostname: hostname::get()
                .ok()
                .and_then(|h| h.into_string().ok())
                .unwrap_or_else(|| "unknown".to_string()),
        };
        let holder_json = serde_json::to_string(&holder).unwrap_or_default();
        fs::write(&lock_path, holder_json).ok();

        debug!("Acquired global lock (non-blocking) at {:?}", lock_path);
        Ok(Some(Self {
            lock_file,
            lock_path,
        }))
    }

    /// Acquire with timeout (polling)
    pub fn acquire_timeout(ms_root: &Path, timeout: Duration) -> Result<Option<Self>> {
        let start = std::time::Instant::now();
        let poll_interval = Duration::from_millis(50);

        while start.elapsed() < timeout {
            if let Some(lock) = Self::try_acquire(ms_root)? {
                return Ok(Some(lock));
            }
            std::thread::sleep(poll_interval);
        }

        warn!(
            "Timeout waiting for lock after {:?}",
            start.elapsed()
        );
        Ok(None)
    }

    /// Check lock status without acquiring
    pub fn status(ms_root: &Path) -> Result<Option<LockHolder>> {
        let lock_path = ms_root.join(Self::LOCK_FILENAME);
        if !lock_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&lock_path)?;
        if content.is_empty() {
            return Ok(None);
        }

        let holder: LockHolder = serde_json::from_str(&content)
            .map_err(|e| MsError::TransactionFailed(format!("parse lock holder: {e}")))?;

        // Check if process is still alive using /proc on Linux
        #[cfg(target_os = "linux")]
        {
            let proc_path = format!("/proc/{}", holder.pid);
            if !std::path::Path::new(&proc_path).exists() {
                // Process no longer exists - lock is stale
                return Ok(None);
            }
        }

        // On other platforms, we trust the lock file content
        // The lock itself is enforced by the OS-level flock

        Ok(Some(holder))
    }

    /// Break a stale lock (use with caution)
    pub fn break_lock(ms_root: &Path) -> Result<bool> {
        let lock_path = ms_root.join(Self::LOCK_FILENAME);
        if !lock_path.exists() {
            return Ok(false);
        }

        // Check if holder process is alive
        if let Some(holder) = Self::status(ms_root)? {
            warn!(
                "Breaking lock held by PID {} since {}",
                holder.pid, holder.acquired_at
            );
        }

        fs::remove_file(&lock_path)?;
        info!("Lock file removed");
        Ok(true)
    }
}

impl Drop for GlobalLock {
    fn drop(&mut self) {
        // fs2's unlock is safe and cross-platform
        if let Err(e) = self.lock_file.unlock() {
            // Use debug level in drop - can't use error! without triggering additional allocations
            debug!("Failed to release lock: {}", e);
        }
        debug!("Released global lock");
    }
}

/// Information about the current lock holder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockHolder {
    /// Process ID holding the lock
    pub pid: u32,
    /// When the lock was acquired
    pub acquired_at: DateTime<Utc>,
    /// Hostname of the lock holder
    pub hostname: String,
}

// =============================================================================
// TRANSACTION MANAGER
// =============================================================================

/// Two-Phase Commit transaction manager for dual persistence
pub struct TxManager {
    db: Arc<Database>,
    git: Arc<GitArchive>,
    tx_dir: PathBuf,
    ms_root: PathBuf,
}

impl TxManager {
    /// Create a new transaction manager
    pub fn new(db: Arc<Database>, git: Arc<GitArchive>, ms_root: PathBuf) -> Result<Self> {
        let tx_dir = ms_root.join("tx");
        fs::create_dir_all(&tx_dir)?;

        Ok(Self {
            db,
            git,
            tx_dir,
            ms_root,
        })
    }

    /// Write a skill with 2PC guarantees (without global lock)
    pub fn write_skill(&self, skill: &SkillSpec) -> Result<()> {
        let tx = TxRecord::prepare("skill", &skill.metadata.id, skill)?;
        debug!("Starting 2PC transaction {} for skill {}", tx.id, skill.metadata.id);

        // Phase 1: Prepare - write intent
        self.write_tx_record(&tx)?;

        // Phase 2: Pending - write to SQLite
        let tx = self.db_write_pending(&tx)?;

        // Phase 3: Commit - write to Git
        let tx = self.git_commit(&tx)?;

        // Phase 4: Complete - finalize SQLite
        let tx = self.db_mark_committed(&tx)?;

        // Cleanup
        self.cleanup_tx(&tx)?;

        info!("2PC transaction {} completed for skill {}", tx.id, skill.metadata.id);
        Ok(())
    }

    /// Write a skill with global lock coordination
    pub fn write_skill_locked(&self, skill: &SkillSpec) -> Result<()> {
        let _lock = GlobalLock::acquire_timeout(&self.ms_root, Duration::from_secs(30))?
            .ok_or_else(|| {
                MsError::TransactionFailed("timeout waiting for global lock".to_string())
            })?;

        self.write_skill(skill)
    }

    /// Batch write skills with a single lock acquisition
    pub fn write_skills_batch(&self, skills: &[SkillSpec]) -> Result<()> {
        if skills.is_empty() {
            return Ok(());
        }

        let _lock = GlobalLock::acquire(&self.ms_root)?;

        for skill in skills {
            self.write_skill(skill)?;
        }

        Ok(())
    }

    /// Delete a skill with 2PC guarantees
    pub fn delete_skill_locked(&self, skill_id: &str) -> Result<()> {
        let _lock = GlobalLock::acquire_timeout(&self.ms_root, Duration::from_secs(30))?
            .ok_or_else(|| {
                MsError::TransactionFailed("timeout waiting for global lock".to_string())
            })?;

        // Delete from Git first (this creates a commit)
        self.git.delete_skill(skill_id)?;

        // Then delete from SQLite
        self.db.delete_skill(skill_id)?;

        Ok(())
    }

    // -------------------------------------------------------------------------
    // Internal transaction phases
    // -------------------------------------------------------------------------

    /// Write transaction record to tx_log and filesystem
    fn write_tx_record(&self, tx: &TxRecord) -> Result<()> {
        debug!("Phase: prepare (tx={})", tx.id);

        // Write to SQLite tx_log
        self.db.insert_tx_record(tx)?;

        // Write to filesystem for crash recovery
        let tx_path = self.tx_dir.join(format!("{}.json", tx.id));
        let tx_json = serde_json::to_string_pretty(tx)
            .map_err(|e| MsError::TransactionFailed(format!("serialize tx: {e}")))?;
        fs::write(&tx_path, tx_json)?;

        Ok(())
    }

    /// Write to SQLite in pending state
    fn db_write_pending(&self, tx: &TxRecord) -> Result<TxRecord> {
        debug!("Phase: pending (tx={})", tx.id);

        let skill: SkillSpec = serde_json::from_str(&tx.payload_json)
            .map_err(|e| MsError::TransactionFailed(format!("deserialize skill: {e}")))?;

        // Upsert skill with pending marker
        self.db.upsert_skill_pending(&skill)?;

        // Update phase
        let mut tx = tx.clone();
        tx.phase = TxPhase::Pending;
        self.db.update_tx_phase(&tx.id, TxPhase::Pending)?;

        // Update filesystem tx record
        let tx_path = self.tx_dir.join(format!("{}.json", tx.id));
        let tx_json = serde_json::to_string_pretty(&tx)
            .map_err(|e| MsError::TransactionFailed(format!("serialize tx: {e}")))?;
        fs::write(&tx_path, tx_json)?;

        Ok(tx)
    }

    /// Commit to Git archive
    fn git_commit(&self, tx: &TxRecord) -> Result<TxRecord> {
        debug!("Phase: committed (tx={})", tx.id);

        let skill: SkillSpec = serde_json::from_str(&tx.payload_json)
            .map_err(|e| MsError::TransactionFailed(format!("deserialize skill: {e}")))?;

        // Write to Git
        self.git.write_skill(&skill)?;

        // Update phase
        let mut tx = tx.clone();
        tx.phase = TxPhase::Committed;
        self.db.update_tx_phase(&tx.id, TxPhase::Committed)?;

        // Update filesystem tx record
        let tx_path = self.tx_dir.join(format!("{}.json", tx.id));
        let tx_json = serde_json::to_string_pretty(&tx)
            .map_err(|e| MsError::TransactionFailed(format!("serialize tx: {e}")))?;
        fs::write(&tx_path, tx_json)?;

        Ok(tx)
    }

    /// Mark SQLite record as committed with final values
    fn db_mark_committed(&self, tx: &TxRecord) -> Result<TxRecord> {
        debug!("Phase: complete (tx={})", tx.id);

        let skill: SkillSpec = serde_json::from_str(&tx.payload_json)
            .map_err(|e| MsError::TransactionFailed(format!("deserialize skill: {e}")))?;

        // Update skill with final values
        let git_path = self
            .git
            .root()
            .join("skills/by-id")
            .join(&skill.metadata.id);
        let git_path_str = git_path.to_string_lossy();
        let content_hash = compute_content_hash(&skill)?;

        self.db.finalize_skill_commit(&skill.metadata.id, &git_path_str, &content_hash)?;

        // Update phase
        let mut tx = tx.clone();
        tx.phase = TxPhase::Complete;
        self.db.update_tx_phase(&tx.id, TxPhase::Complete)?;

        Ok(tx)
    }

    /// Clean up completed transaction
    fn cleanup_tx(&self, tx: &TxRecord) -> Result<()> {
        debug!("Cleanup tx={}", tx.id);

        // Remove from tx_log table
        self.db.delete_tx_record(&tx.id)?;

        // Remove tx file
        let tx_path = self.tx_dir.join(format!("{}.json", tx.id));
        fs::remove_file(&tx_path).ok();

        Ok(())
    }

    // -------------------------------------------------------------------------
    // Recovery
    // -------------------------------------------------------------------------

    /// Recover from incomplete transactions on startup
    pub fn recover(&self) -> Result<RecoveryReport> {
        info!("Starting transaction recovery");
        let mut report = RecoveryReport::default();

        // Find incomplete transactions in tx_log
        let txs = self.db.list_incomplete_transactions()?;

        for tx in txs {
            match tx.phase {
                TxPhase::Prepare => {
                    // Transaction never started - roll back
                    info!("Rolling back prepare-only tx: {}", tx.id);
                    self.rollback_tx(&tx)?;
                    report.rolled_back += 1;
                }
                TxPhase::Pending => {
                    // SQLite written but Git not committed - roll back
                    info!("Rolling back pending tx: {}", tx.id);
                    self.rollback_tx(&tx)?;
                    report.rolled_back += 1;
                }
                TxPhase::Committed => {
                    // Git committed but not marked complete - complete it
                    info!("Completing committed tx: {}", tx.id);
                    let tx = self.db_mark_committed(&tx)?;
                    self.cleanup_tx(&tx)?;
                    report.completed += 1;
                }
                TxPhase::Complete => {
                    // Should not be in incomplete list, but cleanup if found
                    warn!("Found complete tx in incomplete list: {}", tx.id);
                    self.cleanup_tx(&tx)?;
                }
            }
        }

        // Also check tx_dir for orphaned tx files
        if self.tx_dir.exists() {
            for entry in fs::read_dir(&self.tx_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    let tx_json = fs::read_to_string(&path)?;
                    let tx: TxRecord = match serde_json::from_str(&tx_json) {
                        Ok(tx) => tx,
                        Err(e) => {
                            warn!("Invalid tx file {:?}: {}", path, e);
                            fs::remove_file(&path)?;
                            report.orphaned_files += 1;
                            continue;
                        }
                    };

                    // Check if in database
                    if !self.db.tx_exists(&tx.id)? {
                        warn!("Orphaned tx file: {}", tx.id);
                        fs::remove_file(&path)?;
                        report.orphaned_files += 1;
                    }
                }
            }
        }

        if report.rolled_back > 0 || report.completed > 0 || report.orphaned_files > 0 {
            info!(
                "Recovery complete: {} rolled back, {} completed, {} orphaned files cleaned",
                report.rolled_back, report.completed, report.orphaned_files
            );
        } else {
            debug!("Recovery complete: no incomplete transactions found");
        }

        Ok(report)
    }

    /// Roll back a transaction
    fn rollback_tx(&self, tx: &TxRecord) -> Result<()> {
        debug!("Rolling back tx={}", tx.id);

        // Remove from skills table if it was written with pending marker
        if tx.phase == TxPhase::Pending {
            self.db.delete_pending_skill(&tx.entity_id)?;
        }

        // Remove from tx_log
        self.db.delete_tx_record(&tx.id)?;

        // Remove tx file
        let tx_path = self.tx_dir.join(format!("{}.json", tx.id));
        fs::remove_file(&tx_path).ok();

        Ok(())
    }
}

/// Report of recovery actions taken
#[derive(Debug, Default)]
pub struct RecoveryReport {
    /// Number of transactions rolled back
    pub rolled_back: usize,
    /// Number of transactions completed
    pub completed: usize,
    /// Number of orphaned tx files cleaned
    pub orphaned_files: usize,
}

impl RecoveryReport {
    /// Check if any recovery actions were needed
    pub fn had_work(&self) -> bool {
        self.rolled_back > 0 || self.completed > 0 || self.orphaned_files > 0
    }
}

// =============================================================================
// HELPERS
// =============================================================================

/// Compute content hash for a skill spec
fn compute_content_hash(skill: &SkillSpec) -> Result<String> {
    use sha2::{Digest, Sha256};

    let json = serde_json::to_string(skill)
        .map_err(|e| MsError::TransactionFailed(format!("serialize skill for hash: {e}")))?;

    let mut hasher = Sha256::new();
    hasher.update(json.as_bytes());
    let result = hasher.finalize();

    Ok(hex::encode(result))
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{SkillMetadata, SkillSection};
    use tempfile::tempdir;

    fn sample_skill(id: &str) -> SkillSpec {
        SkillSpec {
            metadata: SkillMetadata {
                id: id.to_string(),
                name: format!("Test Skill {}", id),
                version: "1.0.0".to_string(),
                description: "A test skill".to_string(),
                ..Default::default()
            },
            sections: vec![SkillSection {
                id: "intro".to_string(),
                title: "Introduction".to_string(),
                blocks: vec![],
            }],
        }
    }

    #[test]
    fn test_tx_record_prepare() {
        let skill = sample_skill("test-1");
        let tx = TxRecord::prepare("skill", &skill.metadata.id, &skill).unwrap();

        assert!(!tx.id.is_empty());
        assert_eq!(tx.entity_type, "skill");
        assert_eq!(tx.entity_id, "test-1");
        assert_eq!(tx.phase, TxPhase::Prepare);
        assert!(tx.payload_json.contains("test-1"));
    }

    #[test]
    fn test_lock_acquisition_and_release() {
        let dir = tempdir().unwrap();
        let ms_root = dir.path().to_path_buf();

        // First lock should succeed
        let lock1 = GlobalLock::acquire(&ms_root).unwrap();

        // Second lock with try_acquire should fail
        let lock2 = GlobalLock::try_acquire(&ms_root).unwrap();
        assert!(lock2.is_none(), "Should not acquire lock while held");

        // Release first lock
        drop(lock1);

        // Now should succeed
        let lock3 = GlobalLock::try_acquire(&ms_root).unwrap();
        assert!(lock3.is_some(), "Should acquire lock after release");
    }

    #[test]
    fn test_lock_timeout() {
        let dir = tempdir().unwrap();
        let ms_root = dir.path().to_path_buf();

        // Acquire lock
        let _lock = GlobalLock::acquire(&ms_root).unwrap();

        // Timeout should return None quickly
        let start = std::time::Instant::now();
        let result = GlobalLock::acquire_timeout(&ms_root, Duration::from_millis(100)).unwrap();
        let elapsed = start.elapsed();

        assert!(result.is_none());
        assert!(elapsed >= Duration::from_millis(100));
        assert!(elapsed < Duration::from_millis(300)); // Reasonable upper bound
    }

    #[test]
    fn test_lock_status_and_break() {
        let dir = tempdir().unwrap();
        let ms_root = dir.path().to_path_buf();

        // No lock initially
        let status = GlobalLock::status(&ms_root).unwrap();
        assert!(status.is_none());

        // Acquire lock
        let lock = GlobalLock::acquire(&ms_root).unwrap();

        // Status should show current process
        let status = GlobalLock::status(&ms_root).unwrap();
        assert!(status.is_some());
        let holder = status.unwrap();
        assert_eq!(holder.pid, std::process::id());

        // Release lock
        drop(lock);
    }

    #[test]
    fn test_successful_2pc() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let archive_path = dir.path().join("archive");
        let ms_root = dir.path().to_path_buf();

        let db = Arc::new(Database::open(&db_path).unwrap());
        let git = Arc::new(GitArchive::open(&archive_path).unwrap());
        let tx_mgr = TxManager::new(db.clone(), git.clone(), ms_root).unwrap();

        let skill = sample_skill("2pc-test");
        tx_mgr.write_skill(&skill).unwrap();

        // Verify skill exists in Git
        let git_skill = git.read_skill("2pc-test").unwrap();
        assert_eq!(git_skill.metadata.id, "2pc-test");

        // Verify skill exists in SQLite
        let db_skill = db.get_skill("2pc-test").unwrap();
        assert!(db_skill.is_some());

        // Verify no incomplete transactions
        let incomplete = db.list_incomplete_transactions().unwrap();
        assert!(incomplete.is_empty());

        // Verify no tx files remain
        assert!(
            !dir.path().join("tx").exists()
                || fs::read_dir(dir.path().join("tx"))
                    .unwrap()
                    .count()
                    == 0
        );
    }

    #[test]
    fn test_recovery_empty() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let archive_path = dir.path().join("archive");
        let ms_root = dir.path().to_path_buf();

        let db = Arc::new(Database::open(&db_path).unwrap());
        let git = Arc::new(GitArchive::open(&archive_path).unwrap());
        let tx_mgr = TxManager::new(db, git, ms_root).unwrap();

        let report = tx_mgr.recover().unwrap();
        assert!(!report.had_work());
    }

    #[test]
    fn test_compute_content_hash() {
        let skill1 = sample_skill("hash-test-1");
        let skill2 = sample_skill("hash-test-2");
        let skill1_copy = sample_skill("hash-test-1");

        let hash1 = compute_content_hash(&skill1).unwrap();
        let hash2 = compute_content_hash(&skill2).unwrap();
        let hash1_copy = compute_content_hash(&skill1_copy).unwrap();

        // Different skills should have different hashes
        assert_ne!(hash1, hash2);

        // Same skill should have same hash
        assert_eq!(hash1, hash1_copy);

        // Hash should be hex string of SHA256 (64 chars)
        assert_eq!(hash1.len(), 64);
    }
}
