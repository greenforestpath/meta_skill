//! Git utilities

use crate::error::Result;

/// Get current branch name
pub fn current_branch() -> Result<Option<String>> {
    // TODO: Implement
    Ok(None)
}

/// Check if directory is a git repository
pub fn is_repo(path: impl AsRef<std::path::Path>) -> bool {
    path.as_ref().join(".git").exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // =========================================================================
    // current_branch tests
    // =========================================================================

    #[test]
    fn current_branch_returns_none_stub() {
        // Note: This is a stub that returns None
        let result = current_branch().unwrap();
        assert!(result.is_none());
    }

    // =========================================================================
    // is_repo tests
    // =========================================================================

    #[test]
    fn is_repo_with_git_directory() {
        let temp = TempDir::new().unwrap();
        let git_dir = temp.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();

        assert!(is_repo(temp.path()));
    }

    #[test]
    fn is_repo_without_git_directory() {
        let temp = TempDir::new().unwrap();

        assert!(!is_repo(temp.path()));
    }

    #[test]
    fn is_repo_with_git_file_not_dir() {
        let temp = TempDir::new().unwrap();
        let git_file = temp.path().join(".git");
        // Create a file named .git instead of a directory
        std::fs::write(&git_file, "gitdir: ../other").unwrap();

        // A .git file (worktree) should also count as a repo
        assert!(is_repo(temp.path()));
    }

    #[test]
    fn is_repo_nonexistent_path() {
        let temp = TempDir::new().unwrap();
        let nonexistent = temp.path().join("nonexistent");

        assert!(!is_repo(&nonexistent));
    }
}
