//! Git Command Wrapper
//!
//! Wraps git operations with branch validation and audit logging.
//! Prevents agents from executing git operations that would violate
//! their branch assignment.
//!
//! # Design
//!
//! - **Categorize operations**: Read (always allowed), Write (check intent), Branch switch (require coordination)
//! - **Validate before execution**: Check branch assignment and intent
//! - **Audit trail**: Log all git operations with agent ID
//! - **Fail safely**: Return clear errors when operations are blocked

use crate::error::{MnemosyneError, Result};
use crate::orchestration::branch_registry::{BranchRegistry, WorkIntent};
use crate::orchestration::identity::AgentId;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::{Arc, RwLock};

/// Git operation category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitOperationType {
    /// Read-only operations (status, log, diff, show)
    Read,

    /// Write operations (add, commit)
    Write,

    /// Branch operations (checkout, switch, branch)
    BranchSwitch,

    /// Other operations
    Other,
}

/// Git operation audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitAuditEntry {
    /// When operation occurred
    pub timestamp: chrono::DateTime<Utc>,

    /// Agent that performed operation
    pub agent_id: AgentId,

    /// Git command
    pub command: String,

    /// Arguments
    pub args: Vec<String>,

    /// Operation type
    pub operation_type: GitOperationType,

    /// Whether operation was allowed
    pub allowed: bool,

    /// Reason if blocked
    pub block_reason: Option<String>,

    /// Exit code (if executed)
    pub exit_code: Option<i32>,
}

/// Git command wrapper with validation
pub struct GitWrapper {
    /// Branch registry for validation
    registry: Arc<RwLock<BranchRegistry>>,

    /// Repository root directory
    repo_root: PathBuf,

    /// Audit log entries
    audit_log: Arc<RwLock<Vec<GitAuditEntry>>>,

    /// Path to persist audit log
    audit_log_path: Option<PathBuf>,
}

impl GitWrapper {
    /// Create a new git wrapper
    ///
    /// # Arguments
    ///
    /// * `registry` - Branch registry for validation
    /// * `repo_root` - Repository root directory
    pub fn new(registry: Arc<RwLock<BranchRegistry>>, repo_root: PathBuf) -> Self {
        Self {
            registry,
            repo_root,
            audit_log: Arc::new(RwLock::new(Vec::new())),
            audit_log_path: None,
        }
    }

    /// Create with audit log persistence
    pub fn with_audit_log(
        registry: Arc<RwLock<BranchRegistry>>,
        repo_root: PathBuf,
        audit_log_path: PathBuf,
    ) -> Self {
        let mut wrapper = Self::new(registry, repo_root);
        wrapper.audit_log_path = Some(audit_log_path);
        wrapper
    }

    /// Execute git command with validation
    ///
    /// # Arguments
    ///
    /// * `agent_id` - Agent attempting operation
    /// * `args` - Git command arguments (e.g., ["status", "--porcelain"])
    ///
    /// # Returns
    ///
    /// Command output if allowed, error if blocked
    pub fn execute(&self, agent_id: &AgentId, args: &[String]) -> Result<Output> {
        let operation_type = self.categorize_operation(&args);

        // Validate operation
        if let Err(e) = self.validate_operation(agent_id, &operation_type, args) {
            self.log_blocked_operation(agent_id, args, operation_type, e.to_string());
            return Err(e);
        }

        // Execute git command
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| MnemosyneError::Other(format!("Failed to execute git: {}", e)))?;

        // Log successful operation
        self.log_operation(agent_id, args, operation_type, true, None, Some(output.status.code()));

        Ok(output)
    }

    /// Categorize git operation
    fn categorize_operation(&self, args: &[String]) -> GitOperationType {
        if args.is_empty() {
            return GitOperationType::Other;
        }

        match args[0].as_str() {
            // Read operations
            "status" | "log" | "diff" | "show" | "blame" | "ls-files" | "ls-tree" => {
                GitOperationType::Read
            }

            // Write operations
            "add" | "commit" | "rm" | "mv" => GitOperationType::Write,

            // Branch operations
            "checkout" | "switch" | "branch" => GitOperationType::BranchSwitch,

            // Everything else
            _ => GitOperationType::Other,
        }
    }

    /// Validate operation is allowed for agent
    fn validate_operation(
        &self,
        agent_id: &AgentId,
        operation_type: &GitOperationType,
        args: &[String],
    ) -> Result<()> {
        match operation_type {
            // Read always allowed
            GitOperationType::Read => Ok(()),

            // Write requires checking intent
            GitOperationType::Write => self.validate_write_operation(agent_id, args),

            // Branch switch requires coordination
            GitOperationType::BranchSwitch => self.validate_branch_operation(agent_id, args),

            // Other operations allowed by default (can be restricted in config)
            GitOperationType::Other => Ok(()),
        }
    }

    /// Validate write operation
    fn validate_write_operation(&self, agent_id: &AgentId, args: &[String]) -> Result<()> {
        let registry = self.registry.read().map_err(|e| {
            MnemosyneError::Other(format!("Failed to read registry: {}", e))
        })?;

        let assignment = registry
            .get_agent_assignment(agent_id)
            .ok_or_else(|| MnemosyneError::NotFound(format!("No assignment for agent {}", agent_id)))?;

        // Extract file paths from git command
        let files = self.extract_file_paths(args);

        // Check if intent allows writing to these files
        for file in &files {
            if !assignment.intent.allows_write(file) {
                return Err(MnemosyneError::BranchConflict(format!(
                    "Agent {} not permitted to write to {:?} (intent: {:?})",
                    agent_id, file, assignment.intent
                )));
            }
        }

        Ok(())
    }

    /// Validate branch operation
    fn validate_branch_operation(&self, agent_id: &AgentId, args: &[String]) -> Result<()> {
        // Detect if this is a branch switch
        if !self.is_branch_switch(args) {
            return Ok(()); // Not a switch, allow (e.g., "git branch" to list)
        }

        let registry = self.registry.read().map_err(|e| {
            MnemosyneError::Other(format!("Failed to read registry: {}", e))
        })?;

        let assignment = registry
            .get_agent_assignment(agent_id)
            .ok_or_else(|| MnemosyneError::NotFound(format!("No assignment for agent {}", agent_id)))?;

        let target_branch = self.extract_target_branch(args);

        // If switching to different branch, block
        if let Some(target) = target_branch {
            if target != assignment.branch {
                return Err(MnemosyneError::BranchConflict(format!(
                    "Cannot switch to '{}': Agent {} assigned to '{}' in {:?} mode. Use coordination protocol to switch branches.",
                    target, agent_id, assignment.branch, assignment.mode
                )));
            }
        }

        Ok(())
    }

    /// Check if command is a branch switch
    fn is_branch_switch(&self, args: &[String]) -> bool {
        if args.is_empty() {
            return false;
        }

        match args[0].as_str() {
            "checkout" | "switch" => {
                // "git checkout -b new-branch" creates, doesn't switch to existing
                // "git checkout existing-branch" switches
                args.len() > 1 && !args.contains(&"-b".to_string())
            }
            _ => false,
        }
    }

    /// Extract target branch from checkout/switch command
    fn extract_target_branch(&self, args: &[String]) -> Option<String> {
        if args.len() < 2 {
            return None;
        }

        match args[0].as_str() {
            "checkout" | "switch" => {
                // Find first arg that's not a flag
                for arg in &args[1..] {
                    if !arg.starts_with('-') {
                        return Some(arg.clone());
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Extract file paths from git command arguments
    fn extract_file_paths(&self, args: &[String]) -> Vec<PathBuf> {
        if args.is_empty() {
            return vec![];
        }

        match args[0].as_str() {
            "add" | "rm" | "mv" => {
                // Skip flags, collect paths
                args[1..]
                    .iter()
                    .filter(|arg| !arg.starts_with('-'))
                    .map(PathBuf::from)
                    .collect()
            }
            "commit" => {
                // Commit operates on staged files, which requires separate query
                // For now, assume FullBranch intent needed for commit
                vec![PathBuf::from(".")]
            }
            _ => vec![],
        }
    }

    /// Log git operation
    fn log_operation(
        &self,
        agent_id: &AgentId,
        args: &[String],
        operation_type: GitOperationType,
        allowed: bool,
        block_reason: Option<String>,
        exit_code: Option<Option<i32>>,
    ) {
        let entry = GitAuditEntry {
            timestamp: Utc::now(),
            agent_id: agent_id.clone(),
            command: "git".to_string(),
            args: args.to_vec(),
            operation_type,
            allowed,
            block_reason,
            exit_code: exit_code.flatten(),
        };

        if let Ok(mut log) = self.audit_log.write() {
            log.push(entry);

            // Persist if configured
            if self.audit_log_path.is_some() {
                let _ = self.persist_audit_log();
            }
        }
    }

    /// Log blocked operation
    fn log_blocked_operation(
        &self,
        agent_id: &AgentId,
        args: &[String],
        operation_type: GitOperationType,
        reason: String,
    ) {
        self.log_operation(agent_id, args, operation_type, false, Some(reason), None);
    }

    /// Get audit log entries
    pub fn get_audit_log(&self) -> Result<Vec<GitAuditEntry>> {
        self.audit_log
            .read()
            .map(|log| log.clone())
            .map_err(|e| MnemosyneError::Other(format!("Failed to read audit log: {}", e)))
    }

    /// Get audit log entries for specific agent
    pub fn get_agent_audit_log(&self, agent_id: &AgentId) -> Result<Vec<GitAuditEntry>> {
        let log = self.get_audit_log()?;
        Ok(log
            .into_iter()
            .filter(|entry| &entry.agent_id == agent_id)
            .collect())
    }

    /// Clear audit log
    pub fn clear_audit_log(&self) -> Result<()> {
        if let Ok(mut log) = self.audit_log.write() {
            log.clear();
            if self.audit_log_path.is_some() {
                self.persist_audit_log()?;
            }
        }
        Ok(())
    }

    /// Persist audit log to disk
    fn persist_audit_log(&self) -> Result<()> {
        if let Some(path) = &self.audit_log_path {
            let log = self.audit_log.read().map_err(|e| {
                MnemosyneError::Other(format!("Failed to read audit log: {}", e))
            })?;

            let json = serde_json::to_string_pretty(&*log)
                .map_err(|e| MnemosyneError::Other(format!("Failed to serialize audit log: {}", e)))?;

            std::fs::write(path, json).map_err(|e| {
                MnemosyneError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to write audit log: {}", e),
                ))
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::branch_registry::{BranchRegistry, CoordinationMode};
    use tempfile::TempDir;

    fn setup_test_repo() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        Command::new("git")
            .arg("init")
            .current_dir(repo_path)
            .output()
            .expect("Failed to init git");

        Command::new("git")
            .args(&["config", "user.name", "Test"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to config git");

        Command::new("git")
            .args(&["config", "user.email", "test@example.com"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to config git");

        std::fs::write(repo_path.join("README.md"), "Test").unwrap();
        Command::new("git")
            .args(&["add", "README.md"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to add");

        Command::new("git")
            .args(&["commit", "-m", "Initial"])
            .current_dir(repo_path)
            .output()
            .expect("Failed to commit");

        temp_dir
    }

    #[test]
    fn test_categorize_operation() {
        let registry = Arc::new(RwLock::new(BranchRegistry::new()));
        let wrapper = GitWrapper::new(registry, PathBuf::from("."));

        assert_eq!(
            wrapper.categorize_operation(&["status".to_string()]),
            GitOperationType::Read
        );
        assert_eq!(
            wrapper.categorize_operation(&["add".to_string()]),
            GitOperationType::Write
        );
        assert_eq!(
            wrapper.categorize_operation(&["checkout".to_string()]),
            GitOperationType::BranchSwitch
        );
    }

    #[test]
    fn test_read_operation_allowed() {
        let temp_dir = setup_test_repo();
        let registry = Arc::new(RwLock::new(BranchRegistry::new()));
        let wrapper = GitWrapper::new(registry.clone(), temp_dir.path().to_path_buf());

        let agent_id = AgentId::new();

        // Even without assignment, read should work
        let result = wrapper.execute(&agent_id, &["status".to_string(), "--porcelain".to_string()]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_write_without_assignment_blocked() {
        let temp_dir = setup_test_repo();
        let registry = Arc::new(RwLock::new(BranchRegistry::new()));
        let wrapper = GitWrapper::new(registry, temp_dir.path().to_path_buf());

        let agent_id = AgentId::new();

        let result = wrapper.execute(&agent_id, &["add".to_string(), "file.txt".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_write_with_valid_intent() {
        let temp_dir = setup_test_repo();
        let mut registry = BranchRegistry::new();
        let agent_id = AgentId::new();

        registry
            .assign_agent(
                agent_id.clone(),
                "main".to_string(),
                WorkIntent::FullBranch,
                CoordinationMode::Isolated,
            )
            .unwrap();

        let registry = Arc::new(RwLock::new(registry));
        let wrapper = GitWrapper::new(registry, temp_dir.path().to_path_buf());

        // Create file first
        std::fs::write(temp_dir.path().join("file.txt"), "content").unwrap();

        let result = wrapper.execute(&agent_id, &["add".to_string(), "file.txt".to_string()]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_branch_switch_blocked() {
        let temp_dir = setup_test_repo();
        let mut registry = BranchRegistry::new();
        let agent_id = AgentId::new();

        registry
            .assign_agent(
                agent_id.clone(),
                "main".to_string(),
                WorkIntent::FullBranch,
                CoordinationMode::Isolated,
            )
            .unwrap();

        let registry = Arc::new(RwLock::new(registry));
        let wrapper = GitWrapper::new(registry, temp_dir.path().to_path_buf());

        // Create another branch
        Command::new("git")
            .args(&["branch", "feature"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        let result = wrapper.execute(&agent_id, &["checkout".to_string(), "feature".to_string()]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot switch to 'feature'"));
    }

    #[test]
    fn test_audit_log() {
        let temp_dir = setup_test_repo();
        let registry = Arc::new(RwLock::new(BranchRegistry::new()));
        let wrapper = GitWrapper::new(registry, temp_dir.path().to_path_buf());

        let agent_id = AgentId::new();

        // Execute some operations
        let _ = wrapper.execute(&agent_id, &["status".to_string()]);

        let log = wrapper.get_audit_log().unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].agent_id, agent_id);
        assert_eq!(log[0].operation_type, GitOperationType::Read);
        assert!(log[0].allowed);
    }
}
