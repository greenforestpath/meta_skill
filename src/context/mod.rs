//! Context capture for skill suggestions
//!
//! Stub module - full implementation pending.

use std::path::PathBuf;

use crate::error::Result;

/// Captured context from the working environment
#[derive(Debug, Clone, Default)]
pub struct ContextCapture {
    pub cwd: PathBuf,
}

impl ContextCapture {
    /// Capture context from the current directory
    pub fn capture_current(cwd: Option<PathBuf>) -> Result<Self> {
        Ok(Self {
            cwd: cwd.unwrap_or_else(|| std::env::current_dir().unwrap_or_default()),
        })
    }
}

/// Fingerprint of captured context for caching
#[derive(Debug, Clone, Copy, Default)]
pub struct ContextFingerprint(u64);

impl ContextFingerprint {
    /// Create fingerprint from captured context
    pub fn capture(_capture: &ContextCapture) -> Self {
        Self(0)
    }

    /// Get fingerprint as u64
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}
