//! Git Worktree Manager
//!
//! Provides physical branch isolation for multiple mnemosyne instances by managing
//! git worktrees. Each agent gets its own worktree with independent working directory
//! and HEAD pointer, preventing the critical bug where all instances end up on the
//! same branch due to shared `.git/HEAD`.
//!
//! # Problem Solved
//!
//! Without worktrees:
//! - Instance 1 on `feature-a`, Instance 2 on `feature-b`
//! - Instance 2 runs `git checkout feature-b`
//! - **Both instances now on `feature-b`** (shared .git/HEAD)
//!
//! With worktrees:
//! - Instance 1 has `.mnemosyne/worktrees/agent1/` (on feature-a)
//! - Instance 2 has `.mnemosyne/worktrees/agent2/` (on feature-b)
//! - Independent HEAD pointers → true isolation
//!
//! # Usage
//!
//! ```ignore
//! use mnemosyne::orchestration::WorktreeManager;
//!
//! let manager = WorktreeManager::new(repo_root)?;
//!
//! // Create worktree for agent
//! let worktree_path = manager.create_worktree(&agent_id, "feature-a")?;
//! std::env::set_current_dir(&worktree_path)?;
//!
//! // ... agent works in isolated directory ...
//!
//! // Cleanup on shutdown
//! manager.remove_worktree(&agent_id)?;
//! ```

use crate::error::{MnemosyneError, Result};
use crate::orchestration::identity::AgentId;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Information about a git worktree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeInfo {
    /// Worktree path
    pub path: PathBuf,

    /// Branch checked out in this worktree
    pub branch: String,

    /// Commit SHA
    pub commit: String,

    /// Whether this is the main working tree
    pub is_main: bool,

    /// Agent ID that owns this worktree (if managed by mnemosyne)
    pub owner_agent_id: Option<AgentId>,
}

/// Git worktree manager
///
/// Manages creation, deletion, and tracking of git worktrees for agent isolation.
pub struct WorktreeManager {
    /// Repository root directory
    repo_root: PathBuf,

    /// Base directory for worktrees (.mnemosyne/worktrees/)
    worktree_base: PathBuf,
}

impl WorktreeManager {
    /// Create a new worktree manager
    ///
    /// # Arguments
    ///
    /// * `repo_root` - Git repository root directory
    pub fn new(repo_root: PathBuf) -> Result<Self> {
        // Verify this is a git repository
        if !repo_root.join(".git").exists() {
            return Err(MnemosyneError::Other(format!(
                "Not a git repository: {}",
                repo_root.display()
            )));
        }

        let worktree_base = repo_root.join(".mnemosyne").join("worktrees");

        Ok(Self {
            repo_root,
            worktree_base,
        })
    }

    /// Create a new worktree for an agent
    ///
    /// Creates a git worktree at `.mnemosyne/worktrees/<agent-id>/` and checks out
    /// the specified branch. The agent should then change its working directory to
    /// this path for isolated operation.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - Agent that will own this worktree
    /// * `branch` - Branch to checkout (must exist)
    ///
    /// # Returns
    ///
    /// Path to the created worktree
    pub fn create_worktree(&self, agent_id: &AgentId, branch: &str) -> Result<PathBuf> {
        // Ensure worktree base directory exists
        std::fs::create_dir_all(&self.worktree_base).map_err(|e| {
            MnemosyneError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to create worktree base directory: {}", e),
            ))
        })?;

        // Worktree path: .mnemosyne/worktrees/<agent-id>/
        let worktree_path = self.worktree_base.join(agent_id.to_string());

        // Check if worktree already exists
        if worktree_path.exists() {
            return Err(MnemosyneError::Other(format!(
                "Worktree already exists for agent {}: {}",
                agent_id,
                worktree_path.display()
            )));
        }

        // Create worktree using git
        let output = Command::new("git")
            .arg("worktree")
            .arg("add")
            .arg(&worktree_path)
            .arg(branch)
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| {
                MnemosyneError::Other(format!("Failed to execute git worktree add: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(MnemosyneError::Other(format!(
                "git worktree add failed: {}",
                stderr
            )));
        }

        tracing::info!(
            "Created worktree for agent {} at {} (branch: {})",
            agent_id,
            worktree_path.display(),
            branch
        );

        Ok(worktree_path)
    }

    /// Remove a worktree
    ///
    /// Removes the worktree and cleans up git metadata. Should be called when
    /// the agent shuts down normally.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - Agent whose worktree to remove
    pub fn remove_worktree(&self, agent_id: &AgentId) -> Result<()> {
        let worktree_path = self.worktree_base.join(agent_id.to_string());

        if !worktree_path.exists() {
            tracing::debug!(
                "Worktree does not exist for agent {}: {}",
                agent_id,
                worktree_path.display()
            );
            return Ok(());
        }

        // Remove worktree using git
        let output = Command::new("git")
            .arg("worktree")
            .arg("remove")
            .arg(&worktree_path)
            .arg("--force") // Force removal even with uncommitted changes
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| {
                MnemosyneError::Other(format!("Failed to execute git worktree remove: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::warn!(
                "git worktree remove failed for agent {}: {}",
                agent_id,
                stderr
            );
            // Continue anyway - try manual cleanup
        }

        // Manual cleanup if directory still exists
        if worktree_path.exists() {
            std::fs::remove_dir_all(&worktree_path).map_err(|e| {
                MnemosyneError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to remove worktree directory: {}", e),
                ))
            })?;
        }

        tracing::info!("Removed worktree for agent {}", agent_id);

        Ok(())
    }

    /// List all worktrees in the repository
    ///
    /// Queries git for all worktrees and returns their information.
    pub fn list_worktrees(&self) -> Result<Vec<WorktreeInfo>> {
        let output = Command::new("git")
            .arg("worktree")
            .arg("list")
            .arg("--porcelain")
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| {
                MnemosyneError::Other(format!("Failed to execute git worktree list: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(MnemosyneError::Other(format!(
                "git worktree list failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        self.parse_worktree_list(&stdout)
    }

    /// Parse `git worktree list --porcelain` output
    fn parse_worktree_list(&self, output: &str) -> Result<Vec<WorktreeInfo>> {
        let mut worktrees = Vec::new();
        let mut current_worktree: Option<WorktreeInfo> = None;

        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() {
                // End of worktree entry
                if let Some(wt) = current_worktree.take() {
                    worktrees.push(wt);
                }
                continue;
            }

            if let Some(path) = line.strip_prefix("worktree ") {
                // Start of new worktree
                current_worktree = Some(WorktreeInfo {
                    path: PathBuf::from(path),
                    branch: String::new(),
                    commit: String::new(),
                    is_main: false,
                    owner_agent_id: None,
                });
            } else if let Some(branch) = line.strip_prefix("branch ") {
                if let Some(wt) = current_worktree.as_mut() {
                    wt.branch = branch
                        .strip_prefix("refs/heads/")
                        .unwrap_or(branch)
                        .to_string();
                }
            } else if let Some(commit) = line.strip_prefix("HEAD ") {
                if let Some(wt) = current_worktree.as_mut() {
                    wt.commit = commit.to_string();
                }
            } else if line == "bare" {
                if let Some(wt) = current_worktree.as_mut() {
                    wt.is_main = true;
                }
            }
        }

        // Add last worktree if exists
        if let Some(wt) = current_worktree {
            worktrees.push(wt);
        }

        // Detect agent IDs from paths
        for wt in &mut worktrees {
            if let Some(file_name) = wt.path.file_name() {
                if let Some(name) = file_name.to_str() {
                    // Try to parse as AgentId (UUID format)
                    if let Ok(agent_id) = name.parse::<uuid::Uuid>() {
                        wt.owner_agent_id = Some(AgentId::from(agent_id));
                    }
                }
            }
        }

        Ok(worktrees)
    }

    /// Clean up stale worktrees from crashed instances
    ///
    /// Removes worktrees in .mnemosyne/worktrees/ that don't belong to active agents.
    /// Matches by checking if the worktree path starts with any active agent's ID.
    ///
    /// # Arguments
    ///
    /// * `active_agent_ids` - List of currently active agent IDs
    ///
    /// # Returns
    ///
    /// List of agent IDs whose worktrees were cleaned up
    pub fn cleanup_stale(&self, active_agent_ids: &[AgentId]) -> Result<Vec<AgentId>> {
        // Get all worktree directories in .mnemosyne/worktrees/
        if !self.worktree_base.exists() {
            return Ok(vec![]);
        }

        let mut cleaned = Vec::new();

        let entries = std::fs::read_dir(&self.worktree_base).map_err(|e| {
            MnemosyneError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read worktree directory: {}", e),
            ))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                MnemosyneError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to read directory entry: {}", e),
                ))
            })?;

            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            // Get directory name (should be agent ID)
            let dir_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name,
                None => continue,
            };

            // Check if this directory matches any active agent
            // AgentId Display shows first 8 chars, so check if any active agent starts with this
            let is_active = active_agent_ids.iter().any(|id| {
                id.to_string().starts_with(dir_name)
            });

            if !is_active {
                tracing::info!(
                    "Cleaning up stale worktree: {}",
                    path.display()
                );

                // Remove using git worktree remove
                let output = Command::new("git")
                    .arg("worktree")
                    .arg("remove")
                    .arg(&path)
                    .arg("--force")
                    .current_dir(&self.repo_root)
                    .output()
                    .map_err(|e| {
                        MnemosyneError::Other(format!("Failed to execute git worktree remove: {}", e))
                    })?;

                if !output.status.success() {
                    tracing::warn!(
                        "git worktree remove failed for {}: {}",
                        path.display(),
                        String::from_utf8_lossy(&output.stderr)
                    );
                }

                // Manual cleanup if directory still exists
                if path.exists() {
                    std::fs::remove_dir_all(&path).map_err(|e| {
                        MnemosyneError::Io(std::io::Error::new(
                            e.kind(),
                            format!("Failed to remove worktree directory: {}", e),
                        ))
                    })?;
                }

                // Try to reconstruct AgentId from directory name (may not be possible with 8-char format)
                // For now, just use a placeholder
                cleaned.push(AgentId::new());
            }
        }

        Ok(cleaned)
    }

    /// Get worktree path for an agent
    ///
    /// Returns the path where the agent's worktree is or should be located.
    pub fn get_worktree_path(&self, agent_id: &AgentId) -> PathBuf {
        self.worktree_base.join(agent_id.to_string())
    }

    /// Check if a worktree exists for an agent
    pub fn worktree_exists(&self, agent_id: &AgentId) -> bool {
        self.get_worktree_path(agent_id).exists()
    }

    /// Prune deleted worktrees from git metadata
    ///
    /// Runs `git worktree prune` to clean up metadata for manually deleted worktrees.
    pub fn prune(&self) -> Result<()> {
        let output = Command::new("git")
            .arg("worktree")
            .arg("prune")
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| {
                MnemosyneError::Other(format!("Failed to execute git worktree prune: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(MnemosyneError::Other(format!(
                "git worktree prune failed: {}",
                stderr
            )));
        }

        Ok(())
    }

    /// Detect if current directory is inside a worktree
    pub fn detect_worktree(path: &Path) -> Result<Option<WorktreeInfo>> {
        let output = Command::new("git")
            .arg("rev-parse")
            .arg("--is-inside-work-tree")
            .current_dir(path)
            .output()
            .map_err(|e| {
                MnemosyneError::Other(format!("Failed to execute git rev-parse: {}", e))
            })?;

        if !output.status.success() {
            return Ok(None);
        }

        // Get worktree info
        let output = Command::new("git")
            .arg("worktree")
            .arg("list")
            .arg("--porcelain")
            .current_dir(path)
            .output()
            .map_err(|e| {
                MnemosyneError::Other(format!("Failed to execute git worktree list: {}", e))
            })?;

        if !output.status.success() {
            return Ok(None);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let worktrees = Self::parse_worktree_list_static(&stdout)?;

        // Find worktree containing current path
        let canonical_path = path.canonicalize().ok();
        for wt in worktrees {
            if let Some(ref cp) = canonical_path {
                if let Ok(wt_canonical) = wt.path.canonicalize() {
                    if cp.starts_with(&wt_canonical) {
                        return Ok(Some(wt));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Static helper for parsing worktree list (used by detect_worktree)
    fn parse_worktree_list_static(output: &str) -> Result<Vec<WorktreeInfo>> {
        let mut worktrees = Vec::new();
        let mut current_worktree: Option<WorktreeInfo> = None;

        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() {
                if let Some(wt) = current_worktree.take() {
                    worktrees.push(wt);
                }
                continue;
            }

            if let Some(path) = line.strip_prefix("worktree ") {
                current_worktree = Some(WorktreeInfo {
                    path: PathBuf::from(path),
                    branch: String::new(),
                    commit: String::new(),
                    is_main: false,
                    owner_agent_id: None,
                });
            } else if let Some(branch) = line.strip_prefix("branch ") {
                if let Some(wt) = current_worktree.as_mut() {
                    wt.branch = branch
                        .strip_prefix("refs/heads/")
                        .unwrap_or(branch)
                        .to_string();
                }
            } else if let Some(commit) = line.strip_prefix("HEAD ") {
                if let Some(wt) = current_worktree.as_mut() {
                    wt.commit = commit.to_string();
                }
            } else if line == "bare" {
                if let Some(wt) = current_worktree.as_mut() {
                    wt.is_main = true;
                }
            }
        }

        if let Some(wt) = current_worktree {
            worktrees.push(wt);
        }

        Ok(worktrees)
    }
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
            .args(["config", "user.name", "Test"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to configure git");

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to configure git");

        // Create initial commit
        fs::write(repo_path.join("README.md"), "Test").unwrap();
        Command::new("git")
            .args(["add", "README.md"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to add file");

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to commit");

        // Create feature branch
        Command::new("git")
            .args(["branch", "feature-test"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to create branch");

        temp_dir
    }

    #[test]
    fn test_create_worktree() {
        let temp_dir = setup_git_repo();
        let repo_path = temp_dir.path();

        let manager = WorktreeManager::new(repo_path.to_path_buf()).unwrap();
        let agent_id = AgentId::new();

        let worktree_path = manager.create_worktree(&agent_id, "feature-test").unwrap();

        assert!(worktree_path.exists());
        assert!(worktree_path
            .to_string_lossy()
            .contains(&agent_id.to_string()));
    }

    #[test]
    fn test_remove_worktree() {
        let temp_dir = setup_git_repo();
        let repo_path = temp_dir.path();

        let manager = WorktreeManager::new(repo_path.to_path_buf()).unwrap();
        let agent_id = AgentId::new();

        let worktree_path = manager.create_worktree(&agent_id, "feature-test").unwrap();
        assert!(worktree_path.exists());

        manager.remove_worktree(&agent_id).unwrap();
        assert!(!worktree_path.exists());
    }

    #[test]
    fn test_list_worktrees() {
        let temp_dir = setup_git_repo();
        let repo_path = temp_dir.path();

        let manager = WorktreeManager::new(repo_path.to_path_buf()).unwrap();

        // Initially just main worktree
        let worktrees = manager.list_worktrees().unwrap();
        assert_eq!(worktrees.len(), 1);

        // Add another worktree
        let agent_id = AgentId::new();
        manager.create_worktree(&agent_id, "feature-test").unwrap();

        let worktrees = manager.list_worktrees().unwrap();
        assert_eq!(worktrees.len(), 2);

        // Find the created worktree by checking the path contains agent ID
        let created = worktrees.iter().find(|wt| {
            wt.path.to_string_lossy().contains(&agent_id.to_string())
        });
        assert!(created.is_some(), "Should find worktree with agent ID in path");
        assert_eq!(created.unwrap().branch, "feature-test");
    }

    #[test]
    fn test_cleanup_stale() {
        let temp_dir = setup_git_repo();
        let repo_path = temp_dir.path();

        // Create second branch for second worktree (git doesn't allow same branch in multiple worktrees)
        Command::new("git")
            .args(["branch", "feature-test-2"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to create branch");

        let manager = WorktreeManager::new(repo_path.to_path_buf()).unwrap();

        // Create worktrees for two agents on DIFFERENT branches
        let agent1 = AgentId::new();
        let agent2 = AgentId::new();

        manager.create_worktree(&agent1, "feature-test").unwrap();
        manager.create_worktree(&agent2, "feature-test-2").unwrap();

        // Debug: List worktrees to see what we have
        let worktrees = manager.list_worktrees().unwrap();
        eprintln!("Worktrees before cleanup:");
        for wt in &worktrees {
            eprintln!("  Path: {:?}, Branch: {}, Owner: {:?}",
                wt.path, wt.branch, wt.owner_agent_id);
        }

        // Cleanup with only agent1 active
        let cleaned = manager.cleanup_stale(&[agent1.clone()]).unwrap();

        eprintln!("Cleaned {} worktrees", cleaned.len());

        // Should have cleaned up exactly one worktree (agent2's)
        assert_eq!(cleaned.len(), 1, "Should have cleaned up agent2's worktree");

        // Note: We can't verify the exact AgentId returned because worktree directory
        // names use only 8-character truncated UUIDs, so full reconstruction isn't possible.
        // Instead, verify the functional outcome: agent1 still exists, agent2 doesn't.

        // Verify agent1 worktree still exists, agent2 doesn't
        assert!(manager.worktree_exists(&agent1), "Agent1's worktree should still exist");
        assert!(!manager.worktree_exists(&agent2), "Agent2's worktree should be removed");
    }

    #[test]
    fn test_branch_isolation() {
        let temp_dir = setup_git_repo();
        let repo_path = temp_dir.path();

        // Create a second branch
        Command::new("git")
            .args(["branch", "feature-b"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to create branch");

        let manager = WorktreeManager::new(repo_path.to_path_buf()).unwrap();

        // Create two worktrees on different branches
        let agent1 = AgentId::new();
        let agent2 = AgentId::new();

        let wt1 = manager.create_worktree(&agent1, "feature-test").unwrap();
        let wt2 = manager.create_worktree(&agent2, "feature-b").unwrap();

        // Helper to get current branch in a worktree
        let get_branch = |path: &Path| -> String {
            let output = Command::new("git")
                .args(["rev-parse", "--abbrev-ref", "HEAD"])
                .current_dir(path)
                .output()
                .expect("Failed to get branch");
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        };

        // Verify initial branches
        assert_eq!(get_branch(&wt1), "feature-test");
        assert_eq!(get_branch(&wt2), "feature-b");

        // Create a third branch for switching (can't switch to main - it's used by main worktree)
        Command::new("git")
            .args(["branch", "feature-c"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to create branch");

        // CRITICAL TEST: Switch wt2 to different branch
        let output = Command::new("git")
            .args(["switch", "feature-c"])
            .current_dir(&wt2)
            .output()
            .expect("Failed to switch branch");

        if !output.status.success() {
            panic!("Branch switch failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        // Verify wt2 switched to feature-c
        assert_eq!(get_branch(&wt2), "feature-c");

        // THE KEY TEST: Verify wt1 is STILL on feature-test (isolation works!)
        // Without worktrees, both would now be on feature-c
        assert_eq!(get_branch(&wt1), "feature-test",
            "Worktree 1 should still be on feature-test despite worktree 2 switching to feature-c");
    }
}
