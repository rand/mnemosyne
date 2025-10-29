//! Git State Tracker
//!
//! Tracks git repository state for branch isolation:
//! - Current branch detection
//! - Working directory status
//! - Commit tracking for conflict detection
//! - Branch validation
//!
//! This module provides utilities for monitoring git state without modifying
//! the repository. It's used by the branch guard to ensure agents stay on
//! their assigned branches.

use crate::error::{MnemosyneError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Git repository state
///
/// Captures the current state of a git repository including:
/// - Active branch
/// - Working directory status
/// - Last commit SHA
/// - Repository root path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitState {
    /// Current branch name (or "HEAD" if detached)
    pub current_branch: String,

    /// Repository root directory
    pub repo_root: PathBuf,

    /// Last commit SHA on current branch
    pub last_commit: String,

    /// Whether working directory is clean (no uncommitted changes)
    pub is_clean: bool,

    /// Modified files (if not clean)
    pub modified_files: Vec<PathBuf>,
}

impl GitState {
    /// Detect git state for current directory
    ///
    /// Walks up directory tree to find `.git` folder, then queries git state.
    pub fn detect() -> Result<Self> {
        let repo_root = Self::find_repo_root(".")?;
        Self::from_repo_root(&repo_root)
    }

    /// Detect git state for specific directory
    pub fn from_path(path: &Path) -> Result<Self> {
        let repo_root = Self::find_repo_root(path)?;
        Self::from_repo_root(&repo_root)
    }

    /// Get git state from known repository root
    pub fn from_repo_root(repo_root: &Path) -> Result<Self> {
        let current_branch = Self::get_current_branch(repo_root)?;
        let last_commit = Self::get_last_commit(repo_root)?;
        let (is_clean, modified_files) = Self::get_working_dir_status(repo_root)?;

        Ok(Self {
            current_branch,
            repo_root: repo_root.to_path_buf(),
            last_commit,
            is_clean,
            modified_files,
        })
    }

    /// Find git repository root by walking up directory tree
    fn find_repo_root(start_path: impl AsRef<Path>) -> Result<PathBuf> {
        let mut current = std::fs::canonicalize(start_path).map_err(|e| {
            MnemosyneError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to canonicalize path: {}", e),
            ))
        })?;

        loop {
            let git_dir = current.join(".git");
            if git_dir.exists() {
                return Ok(current);
            }

            match current.parent() {
                Some(parent) => current = parent.to_path_buf(),
                None => {
                    return Err(MnemosyneError::Other(
                        "Not in a git repository".to_string(),
                    ))
                }
            }
        }
    }

    /// Get current branch name
    fn get_current_branch(repo_root: &Path) -> Result<String> {
        let output = Command::new("git")
            .arg("rev-parse")
            .arg("--abbrev-ref")
            .arg("HEAD")
            .current_dir(repo_root)
            .output()
            .map_err(|e| {
                MnemosyneError::Other(format!("Failed to execute git rev-parse: {}", e))
            })?;

        if !output.status.success() {
            return Err(MnemosyneError::Other(format!(
                "git rev-parse failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let branch = String::from_utf8(output.stdout)
            .map_err(|e| MnemosyneError::Other(format!("Invalid UTF-8 in git output: {}", e)))?
            .trim()
            .to_string();

        Ok(branch)
    }

    /// Get last commit SHA
    fn get_last_commit(repo_root: &Path) -> Result<String> {
        let output = Command::new("git")
            .arg("rev-parse")
            .arg("HEAD")
            .current_dir(repo_root)
            .output()
            .map_err(|e| {
                MnemosyneError::Other(format!("Failed to execute git rev-parse: {}", e))
            })?;

        if !output.status.success() {
            return Err(MnemosyneError::Other(format!(
                "git rev-parse HEAD failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let commit = String::from_utf8(output.stdout)
            .map_err(|e| MnemosyneError::Other(format!("Invalid UTF-8 in git output: {}", e)))?
            .trim()
            .to_string();

        Ok(commit)
    }

    /// Get working directory status
    fn get_working_dir_status(repo_root: &Path) -> Result<(bool, Vec<PathBuf>)> {
        let output = Command::new("git")
            .arg("status")
            .arg("--porcelain")
            .current_dir(repo_root)
            .output()
            .map_err(|e| {
                MnemosyneError::Other(format!("Failed to execute git status: {}", e))
            })?;

        if !output.status.success() {
            return Err(MnemosyneError::Other(format!(
                "git status failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let status_output = String::from_utf8(output.stdout)
            .map_err(|e| MnemosyneError::Other(format!("Invalid UTF-8 in git output: {}", e)))?;

        if status_output.trim().is_empty() {
            return Ok((true, vec![]));
        }

        // Parse modified files from porcelain output
        let modified_files: Vec<PathBuf> = status_output
            .lines()
            .filter_map(|line| {
                // Porcelain format: "XY file"
                if line.len() > 3 {
                    Some(PathBuf::from(line[3..].trim()))
                } else {
                    None
                }
            })
            .collect();

        Ok((false, modified_files))
    }

    /// Check if a specific branch exists
    pub fn branch_exists(&self, branch_name: &str) -> Result<bool> {
        let output = Command::new("git")
            .arg("rev-parse")
            .arg("--verify")
            .arg(branch_name)
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| {
                MnemosyneError::Other(format!("Failed to execute git rev-parse: {}", e))
            })?;

        Ok(output.status.success())
    }

    /// Get list of all branches
    pub fn list_branches(&self) -> Result<Vec<String>> {
        let output = Command::new("git")
            .arg("branch")
            .arg("--format=%(refname:short)")
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| MnemosyneError::Other(format!("Failed to execute git branch: {}", e)))?;

        if !output.status.success() {
            return Err(MnemosyneError::Other(format!(
                "git branch failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let branches_output = String::from_utf8(output.stdout)
            .map_err(|e| MnemosyneError::Other(format!("Invalid UTF-8 in git output: {}", e)))?;

        let branches: Vec<String> = branches_output
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();

        Ok(branches)
    }

    /// Check if file has been modified
    pub fn is_file_modified(&self, file_path: &Path) -> bool {
        self.modified_files.iter().any(|f| f == file_path)
    }

    /// Refresh state (re-query git)
    pub fn refresh(&mut self) -> Result<()> {
        let new_state = Self::from_repo_root(&self.repo_root)?;
        *self = new_state;
        Ok(())
    }
}

/// Git state tracker with caching
///
/// Wraps GitState with caching to avoid excessive git command execution.
/// Automatically refreshes state when it becomes stale.
pub struct GitStateTracker {
    state: GitState,
    last_refresh: std::time::Instant,
    cache_duration: std::time::Duration,
}

impl GitStateTracker {
    /// Create a new git state tracker
    pub fn new() -> Result<Self> {
        let state = GitState::detect()?;
        Ok(Self {
            state,
            last_refresh: std::time::Instant::now(),
            cache_duration: std::time::Duration::from_secs(2), // Refresh every 2 seconds
        })
    }

    /// Create with specific cache duration
    pub fn with_cache_duration(cache_duration: std::time::Duration) -> Result<Self> {
        let state = GitState::detect()?;
        Ok(Self {
            state,
            last_refresh: std::time::Instant::now(),
            cache_duration,
        })
    }

    /// Get current git state (cached)
    pub fn state(&mut self) -> Result<&GitState> {
        if self.last_refresh.elapsed() > self.cache_duration {
            self.refresh()?;
        }
        Ok(&self.state)
    }

    /// Force refresh state
    pub fn refresh(&mut self) -> Result<()> {
        self.state.refresh()?;
        self.last_refresh = std::time::Instant::now();
        Ok(())
    }

    /// Get current branch (cached)
    pub fn current_branch(&mut self) -> Result<String> {
        Ok(self.state()?.current_branch.clone())
    }

    /// Check if working directory is clean (cached)
    pub fn is_clean(&mut self) -> Result<bool> {
        Ok(self.state()?.is_clean)
    }

    /// Get modified files (cached)
    pub fn modified_files(&mut self) -> Result<Vec<PathBuf>> {
        Ok(self.state()?.modified_files.clone())
    }
}

/// Validate that current branch matches expected branch
///
/// Used by branch guard to ensure agent hasn't accidentally switched branches.
pub fn validate_branch(expected: &str) -> Result<()> {
    let state = GitState::detect()?;

    if state.current_branch != expected {
        return Err(MnemosyneError::BranchConflict(format!(
            "Branch mismatch: expected '{}', but currently on '{}'",
            expected, state.current_branch
        )));
    }

    Ok(())
}

/// Check if path is in a git repository
pub fn is_git_repo(path: &Path) -> bool {
    GitState::find_repo_root(path).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_git_repo() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Initialize git repo
        Command::new("git")
            .arg("init")
            .current_dir(repo_path)
            .output()
            .expect("Failed to init git repo");

        // Configure git
        Command::new("git")
            .args(&["config", "user.name", "Test"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to configure git");

        Command::new("git")
            .args(&["config", "user.email", "test@example.com"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to configure git");

        // Create initial commit
        fs::write(repo_path.join("README.md"), "Test").unwrap();
        Command::new("git")
            .args(&["add", "README.md"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to add file");

        Command::new("git")
            .args(&["commit", "-m", "Initial commit"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to commit");

        temp_dir
    }

    #[test]
    fn test_find_repo_root() {
        let temp_dir = setup_git_repo();
        let repo_path = temp_dir.path();

        // Create nested directory
        let nested = repo_path.join("src").join("nested");
        fs::create_dir_all(&nested).unwrap();

        // Should find root from nested directory
        let root = GitState::find_repo_root(&nested).unwrap();
        assert_eq!(root, repo_path);
    }

    #[test]
    fn test_get_current_branch() {
        let temp_dir = setup_git_repo();
        let repo_path = temp_dir.path();

        let branch = GitState::get_current_branch(repo_path).unwrap();
        // Default branch could be "main" or "master" depending on git config
        assert!(branch == "main" || branch == "master");
    }

    #[test]
    fn test_git_state_detect() {
        let temp_dir = setup_git_repo();
        let _original_dir = std::env::current_dir().unwrap();

        // Change to repo directory
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let state = GitState::detect().unwrap();
        assert!(state.current_branch == "main" || state.current_branch == "master");
        assert!(state.is_clean);
        assert!(state.modified_files.is_empty());
        assert!(!state.last_commit.is_empty());

        // Restore directory
        std::env::set_current_dir(_original_dir).unwrap();
    }

    #[test]
    fn test_working_dir_status() {
        let temp_dir = setup_git_repo();
        let repo_path = temp_dir.path();

        // Initially clean
        let (is_clean, modified) = GitState::get_working_dir_status(repo_path).unwrap();
        assert!(is_clean);
        assert!(modified.is_empty());

        // Modify file
        fs::write(repo_path.join("README.md"), "Modified").unwrap();

        let (is_clean, modified) = GitState::get_working_dir_status(repo_path).unwrap();
        assert!(!is_clean);
        assert_eq!(modified.len(), 1);
        assert_eq!(modified[0], PathBuf::from("README.md"));
    }

    #[test]
    fn test_branch_exists() {
        let temp_dir = setup_git_repo();
        let repo_path = temp_dir.path();

        let state = GitState::from_repo_root(repo_path).unwrap();

        // Current branch should exist
        assert!(state.branch_exists(&state.current_branch).unwrap());

        // Non-existent branch
        assert!(!state.branch_exists("non-existent-branch").unwrap());
    }

    #[test]
    fn test_list_branches() {
        let temp_dir = setup_git_repo();
        let repo_path = temp_dir.path();

        let state = GitState::from_repo_root(repo_path).unwrap();
        let branches = state.list_branches().unwrap();

        assert_eq!(branches.len(), 1);
        assert!(branches[0] == "main" || branches[0] == "master");
    }

    #[test]
    fn test_validate_branch() {
        let temp_dir = setup_git_repo();
        let _original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let state = GitState::detect().unwrap();

        // Should succeed for current branch
        validate_branch(&state.current_branch).unwrap();

        // Should fail for different branch
        let result = validate_branch("non-existent");
        assert!(result.is_err());

        std::env::set_current_dir(_original_dir).unwrap();
    }

    #[test]
    fn test_is_git_repo() {
        let temp_dir = setup_git_repo();
        assert!(is_git_repo(temp_dir.path()));

        let non_repo = TempDir::new().unwrap();
        assert!(!is_git_repo(non_repo.path()));
    }

    #[test]
    fn test_git_state_tracker() {
        let temp_dir = setup_git_repo();
        let _original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let mut tracker = GitStateTracker::new().unwrap();

        let branch = tracker.current_branch().unwrap();
        assert!(branch == "main" || branch == "master");

        assert!(tracker.is_clean().unwrap());

        // Modify file
        fs::write("README.md", "Modified").unwrap();

        // Force refresh
        tracker.refresh().unwrap();
        assert!(!tracker.is_clean().unwrap());

        let modified = tracker.modified_files().unwrap();
        assert_eq!(modified.len(), 1);

        std::env::set_current_dir(_original_dir).unwrap();
    }
}
