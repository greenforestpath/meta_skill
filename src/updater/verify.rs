//! SHA256 verification

use crate::error::Result;

/// Verify binary checksum
pub fn verify(_data: &[u8], _expected_sha256: &str) -> Result<bool> {
    // TODO: Compute SHA256 and compare
    Ok(false)
}
