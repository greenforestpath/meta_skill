//! Suggestion cooldown cache
//!
//! Stub module - full implementation pending.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Cache for suggestion cooldowns
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SuggestionCooldownCache {
    entries: Vec<()>,
}

/// Stats about the cooldown cache
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CooldownStats {
    pub total_entries: usize,
    pub active_cooldowns: usize,
    pub expired_pending_cleanup: usize,
}

impl SuggestionCooldownCache {
    /// Create a new empty cache
    pub fn new() -> Self {
        Self::default()
    }

    /// Load cache from file
    pub fn load(_path: &Path) -> Result<Self> {
        Ok(Self::default())
    }

    /// Save cache to file
    pub fn save(&self, _path: &Path) -> Result<()> {
        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> CooldownStats {
        CooldownStats::default()
    }
}
