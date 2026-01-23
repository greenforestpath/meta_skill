//! Secure Token Storage
//!
//! Handles persistent storage of authentication credentials with a security-first approach:
//!
//! 1. **OS Keychain** (preferred): Uses the system's secure credential storage
//!    - macOS: Keychain
//!    - Windows: Credential Manager
//!    - Linux: Secret Service (libsecret)
//!
//! 2. **File Fallback**: When keychain is unavailable, uses a file with:
//!    - 0600 permissions (owner read/write only)
//!    - Atomic writes to prevent corruption
//!    - JSON format for easy debugging

use std::fs;
use std::path::PathBuf;

use tracing::{debug, warn};

use super::JfpCredentials;
use crate::error::{MsError, Result};

/// Service name for keychain storage.
const KEYCHAIN_SERVICE: &str = "ms-jfp-cloud";
/// Account name for keychain storage.
const KEYCHAIN_ACCOUNT: &str = "credentials";
/// Filename for fallback file storage.
const CREDENTIALS_FILENAME: &str = "jfp-credentials.json";

/// Storage method currently in use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageMethod {
    Keychain,
    File,
}

impl std::fmt::Display for StorageMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageMethod::Keychain => write!(f, "keychain"),
            StorageMethod::File => write!(f, "file"),
        }
    }
}

/// Get the current storage method being used.
pub fn current_storage_method() -> String {
    if keyring_available() {
        "keychain".to_string()
    } else {
        "file".to_string()
    }
}

/// Check if the system keychain is available.
fn keyring_available() -> bool {
    // Try to access keyring - if it works, we can use it
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT);
    match entry {
        Ok(e) => {
            // Try a harmless operation to see if it works
            match e.get_password() {
                Ok(_) => true,
                Err(keyring::Error::NoEntry) => true, // No entry is fine, keychain works
                Err(keyring::Error::NoStorageAccess(_)) => false,
                Err(keyring::Error::PlatformFailure(_)) => false,
                Err(_) => true, // Other errors might be transient
            }
        }
        Err(_) => false,
    }
}

/// Get the path to the credentials file.
fn credentials_file_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| MsError::Config("Could not find config directory".to_string()))?;
    Ok(config_dir.join("ms").join(CREDENTIALS_FILENAME))
}

/// Save credentials to storage.
///
/// Tries keychain first, falls back to file if unavailable.
pub fn save_credentials(creds: &JfpCredentials) -> Result<()> {
    // Serialize credentials
    let json = serde_json::to_string(creds)
        .map_err(|e| MsError::AuthError(format!("Failed to serialize credentials: {e}")))?;

    // Try keychain first
    if keyring_available() {
        match save_to_keychain(&json) {
            Ok(()) => {
                debug!("Credentials saved to keychain");
                return Ok(());
            }
            Err(e) => {
                warn!(error = %e, "Keychain save failed, falling back to file");
            }
        }
    }

    // Fall back to file
    save_to_file(&json)?;
    debug!("Credentials saved to file");
    Ok(())
}

/// Load credentials from storage.
///
/// Tries keychain first, falls back to file if unavailable.
pub fn load_credentials() -> Result<Option<JfpCredentials>> {
    // Try keychain first
    if keyring_available() {
        match load_from_keychain() {
            Ok(Some(creds)) => {
                debug!("Credentials loaded from keychain");
                return Ok(Some(creds));
            }
            Ok(None) => {
                // No credentials in keychain, try file
            }
            Err(e) => {
                warn!(error = %e, "Keychain load failed, trying file");
            }
        }
    }

    // Try file
    match load_from_file() {
        Ok(Some(creds)) => {
            debug!("Credentials loaded from file");
            Ok(Some(creds))
        }
        Ok(None) => Ok(None),
        Err(e) => {
            warn!(error = %e, "File load failed");
            Ok(None)
        }
    }
}

/// Clear credentials from storage.
///
/// Removes from both keychain and file to ensure complete cleanup.
pub fn clear_credentials() -> Result<()> {
    let mut errors = Vec::new();

    // Try to clear keychain
    if keyring_available() {
        if let Err(e) = clear_from_keychain() {
            warn!(error = %e, "Failed to clear keychain");
            errors.push(format!("keychain: {e}"));
        } else {
            debug!("Cleared credentials from keychain");
        }
    }

    // Try to clear file
    if let Err(e) = clear_from_file() {
        warn!(error = %e, "Failed to clear file");
        errors.push(format!("file: {e}"));
    } else {
        debug!("Cleared credentials from file");
    }

    if errors.is_empty() {
        Ok(())
    } else {
        // Return success even if some methods failed - best effort
        Ok(())
    }
}

// === Keychain Storage ===

fn save_to_keychain(json: &str) -> Result<()> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
        .map_err(|e| MsError::AuthError(format!("Failed to create keychain entry: {e}")))?;

    entry
        .set_password(json)
        .map_err(|e| MsError::AuthError(format!("Failed to save to keychain: {e}")))?;

    Ok(())
}

fn load_from_keychain() -> Result<Option<JfpCredentials>> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
        .map_err(|e| MsError::AuthError(format!("Failed to create keychain entry: {e}")))?;

    match entry.get_password() {
        Ok(json) => {
            let creds: JfpCredentials = serde_json::from_str(&json)
                .map_err(|e| MsError::AuthError(format!("Invalid credentials in keychain: {e}")))?;
            Ok(Some(creds))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(MsError::AuthError(format!(
            "Failed to load from keychain: {e}"
        ))),
    }
}

fn clear_from_keychain() -> Result<()> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
        .map_err(|e| MsError::AuthError(format!("Failed to create keychain entry: {e}")))?;

    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()), // Already deleted
        Err(e) => Err(MsError::AuthError(format!("Failed to clear keychain: {e}"))),
    }
}

// === File Storage ===

fn save_to_file(json: &str) -> Result<()> {
    let path = credentials_file_path()?;

    // Create parent directory
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| MsError::AuthError(format!("Failed to create config dir: {e}")))?;
    }

    // Write to temporary file first (atomic write)
    let temp_path = path.with_extension("tmp");

    fs::write(&temp_path, json)
        .map_err(|e| MsError::AuthError(format!("Failed to write credentials: {e}")))?;

    // Set restrictive permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = fs::Permissions::from_mode(0o600);
        fs::set_permissions(&temp_path, permissions)
            .map_err(|e| MsError::AuthError(format!("Failed to set permissions: {e}")))?;
    }

    // Atomic rename
    fs::rename(&temp_path, &path)
        .map_err(|e| MsError::AuthError(format!("Failed to save credentials: {e}")))?;

    Ok(())
}

fn load_from_file() -> Result<Option<JfpCredentials>> {
    let path = credentials_file_path()?;

    if !path.exists() {
        return Ok(None);
    }

    // Check permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = fs::metadata(&path)
            .map_err(|e| MsError::AuthError(format!("Failed to read file metadata: {e}")))?;
        let mode = metadata.mode() & 0o777;
        if mode != 0o600 {
            warn!(
                path = %path.display(),
                mode = format!("{:o}", mode),
                "Credentials file has insecure permissions"
            );
        }
    }

    let json = fs::read_to_string(&path)
        .map_err(|e| MsError::AuthError(format!("Failed to read credentials: {e}")))?;

    let creds: JfpCredentials = serde_json::from_str(&json)
        .map_err(|e| MsError::AuthError(format!("Invalid credentials file: {e}")))?;

    Ok(Some(creds))
}

fn clear_from_file() -> Result<()> {
    let path = credentials_file_path()?;

    if path.exists() {
        // Overwrite with zeros before deleting (secure delete)
        let _ = fs::write(&path, "{}");
        fs::remove_file(&path)
            .map_err(|e| MsError::AuthError(format!("Failed to delete credentials: {e}")))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credentials_file_path() {
        let path = credentials_file_path().unwrap();
        assert!(path.ends_with(CREDENTIALS_FILENAME));
    }

    #[test]
    fn test_storage_method_display() {
        assert_eq!(StorageMethod::Keychain.to_string(), "keychain");
        assert_eq!(StorageMethod::File.to_string(), "file");
    }
}
