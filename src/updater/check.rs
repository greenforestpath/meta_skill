//! Version checking

use crate::error::Result;

/// Check for available updates
pub fn check_updates() -> Result<Option<UpdateInfo>> {
    // TODO: Check GitHub releases
    Ok(None)
}

/// Information about an available update
#[derive(Debug)]
pub struct UpdateInfo {
    pub version: String,
    pub url: String,
    pub changelog: Option<String>,
}
