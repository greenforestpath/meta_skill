//! Filesystem utilities.
//!
//! Helper functions for file operations.

use std::path::Path;

use crate::error::Result;

/// Ensure a directory exists, creating it if necessary.
pub fn ensure_dir(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}

/// Read a file to string, returning None if it doesn't exist.
pub fn read_optional(path: impl AsRef<Path>) -> Result<Option<String>> {
    let path = path.as_ref();
    if path.exists() {
        Ok(Some(std::fs::read_to_string(path)?))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // =========================================================================
    // ensure_dir tests
    // =========================================================================

    #[test]
    fn ensure_dir_creates_new_directory() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join("new_dir");

        assert!(!dir.exists());
        ensure_dir(&dir).unwrap();
        assert!(dir.exists());
        assert!(dir.is_dir());
    }

    #[test]
    fn ensure_dir_creates_nested_directories() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join("a").join("b").join("c");

        assert!(!dir.exists());
        ensure_dir(&dir).unwrap();
        assert!(dir.exists());
        assert!(dir.is_dir());
    }

    #[test]
    fn ensure_dir_noop_if_exists() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join("existing");
        std::fs::create_dir(&dir).unwrap();

        // Should not fail if directory exists
        ensure_dir(&dir).unwrap();
        assert!(dir.exists());
    }

    #[test]
    fn ensure_dir_idempotent() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join("idem");

        // Call multiple times
        ensure_dir(&dir).unwrap();
        ensure_dir(&dir).unwrap();
        ensure_dir(&dir).unwrap();
        assert!(dir.exists());
    }

    // =========================================================================
    // read_optional tests
    // =========================================================================

    #[test]
    fn read_optional_existing_file() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("test.txt");
        std::fs::write(&file, "hello world").unwrap();

        let result = read_optional(&file).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "hello world");
    }

    #[test]
    fn read_optional_nonexistent_file() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("nonexistent.txt");

        let result = read_optional(&file).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn read_optional_empty_file() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("empty.txt");
        std::fs::write(&file, "").unwrap();

        let result = read_optional(&file).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "");
    }

    #[test]
    fn read_optional_with_unicode() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("unicode.txt");
        std::fs::write(&file, "日本語🚀").unwrap();

        let result = read_optional(&file).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "日本語🚀");
    }

    #[test]
    fn read_optional_multiline() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("multiline.txt");
        std::fs::write(&file, "line1\nline2\nline3").unwrap();

        let result = read_optional(&file).unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains('\n'));
    }
}
