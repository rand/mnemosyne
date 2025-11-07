//! Branch Guard
//!
//! Prevents agents from accidentally switching branches or performing operations
//! outside their assigned branch and work intent.
//!
//! # Key Features
//!
//! - **Default Isolated mode**: Blocks accidental branch switches
//! - **Orchestrator bypass**: Coordinator role can access any branch
//! - **Intent validation**: Ensures writes stay within declared scope
//! - **Intelligent conflict detection**: Uses heuristics for BLOCK vs WARN decisions
//! - **Audit trail**: Logs all attempts for debugging

use crate::error::{MnemosyneError, Result};
use crate::orchestration::branch_registry::{BranchRegistry, CoordinationMode, WorkIntent};
use crate::orchestration::conflict_detector::{ConflictAction, ConflictDetector};
use crate::orchestration::git_state::GitState;
use crate::orchestration::git_wrapper::{GitOperationType, GitWrapper};
use crate::orchestration::identity::{AgentId, AgentIdentity};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Branch guard configuration
#[derive(Debug, Clone)]
pub struct BranchGuardConfig {
    /// Enable branch isolation (default: true)
    pub enabled: bool,

    /// Allow orchestrator to bypass restrictions (default: true)
    pub orchestrator_bypass: bool,

    /// Auto-approve read-only access (default: true)
    pub auto_approve_readonly: bool,

    /// Enable conflict detection (default: true)
    pub conflict_detection: bool,
}

impl Default for BranchGuardConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            orchestrator_bypass: true,
            auto_approve_readonly: true,
            conflict_detection: true,
        }
    }
}

/// Branch guard - enforces branch isolation and coordination rules
pub struct BranchGuard {
    /// Configuration
    config: BranchGuardConfig,

    /// Branch registry
    registry: Arc<RwLock<BranchRegistry>>,

    /// Conflict detector
    conflict_detector: ConflictDetector,

    /// Git wrapper for command execution
    git_wrapper: GitWrapper,

    /// Current repository root
    repo_root: PathBuf,
}

impl BranchGuard {
    /// Create a new branch guard
    ///
    /// # Arguments
    ///
    /// * `registry` - Branch registry for assignments
    /// * `repo_root` - Repository root directory
    pub fn new(registry: Arc<RwLock<BranchRegistry>>, repo_root: PathBuf) -> Self {
        let git_wrapper = GitWrapper::new(registry.clone(), repo_root.clone());

        Self {
            config: BranchGuardConfig::default(),
            registry,
            conflict_detector: ConflictDetector::new(),
            git_wrapper,
            repo_root,
        }
    }

    /// Create with custom configuration
    pub fn with_config(
        config: BranchGuardConfig,
        registry: Arc<RwLock<BranchRegistry>>,
        repo_root: PathBuf,
    ) -> Self {
        let git_wrapper = GitWrapper::new(registry.clone(), repo_root.clone());

        Self {
            config,
            registry,
            conflict_detector: ConflictDetector::new(),
            git_wrapper,
            repo_root,
        }
    }

    /// Validate branch access for agent
    ///
    /// Checks if agent can access the target branch with given intent.
    /// Returns Ok(()) if allowed, Err with reason if blocked.
    ///
    /// # Arguments
    ///
    /// * `agent_identity` - Full agent identity
    /// * `target_branch` - Branch to access
    /// * `intent` - Proposed work intent
    pub fn validate_branch_access(
        &self,
        agent_identity: &AgentIdentity,
        target_branch: &str,
        intent: &WorkIntent,
    ) -> Result<()> {
        // If disabled, allow everything
        if !self.config.enabled {
            return Ok(());
        }

        // Orchestrator bypass
        if self.config.orchestrator_bypass && agent_identity.has_coordinator_permissions() {
            return Ok(());
        }

        let registry = self
            .registry
            .read()
            .map_err(|e| MnemosyneError::Other(format!("Failed to read registry: {}", e)))?;

        // Check if agent already assigned
        if let Some(current_assignment) = registry.get_agent_assignment(&agent_identity.id) {
            // Trying to switch branches?
            if current_assignment.branch != target_branch {
                return Err(MnemosyneError::BranchConflict(format!(
                    "Cannot access '{}': Agent {} assigned to '{}' in {:?} mode. Use coordination protocol to switch.",
                    target_branch,
                    agent_identity.name(),
                    current_assignment.branch,
                    current_assignment.mode
                )));
            }

            // On same branch, check intent compatibility
            return self.validate_intent_compatibility(agent_identity, intent);
        }

        // No assignment yet - check if target branch is available
        let existing_assignments = registry.get_assignments(target_branch);

        if existing_assignments.is_empty() {
            return Ok(()); // Branch empty, allow
        }

        // Check conflicts with existing assignments
        if self.config.conflict_detection {
            if let Some(assessment) = self.conflict_detector.assess_conflict(
                &existing_assignments,
                intent,
                &agent_identity.id,
            ) {
                match assessment.action {
                    ConflictAction::Proceed | ConflictAction::NotifyAndProceed => Ok(()),
                    ConflictAction::RequireApproval | ConflictAction::RequireCoordination => {
                        Err(MnemosyneError::BranchConflict(format!(
                            "Conflict detected: {} - {}",
                            assessment.reason,
                            assessment.suggestions.join(", ")
                        )))
                    }
                    ConflictAction::Block => Err(MnemosyneError::BranchConflict(format!(
                        "BLOCKED: {} - {}",
                        assessment.reason,
                        assessment.suggestions.join(", ")
                    ))),
                }
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    /// Validate intent compatibility for agent already on branch
    fn validate_intent_compatibility(
        &self,
        agent_identity: &AgentIdentity,
        new_intent: &WorkIntent,
    ) -> Result<()> {
        let registry = self
            .registry
            .read()
            .map_err(|e| MnemosyneError::Other(format!("Failed to read registry: {}", e)))?;

        let current_assignment = registry
            .get_agent_assignment(&agent_identity.id)
            .ok_or_else(|| {
                MnemosyneError::NotFound(format!(
                    "No assignment for agent {}",
                    agent_identity.name()
                ))
            })?;

        // Check if new intent is compatible with current mode
        match current_assignment.mode {
            CoordinationMode::Isolated => {
                // In isolated mode, agent can expand intent freely
                Ok(())
            }
            CoordinationMode::Coordinated => {
                // In coordinated mode, check for conflicts with other agents
                let other_assignments: Vec<_> = registry
                    .get_assignments(&current_assignment.branch)
                    .into_iter()
                    .filter(|a| a.agent_id != agent_identity.id)
                    .collect();

                if let Some(assessment) = self.conflict_detector.assess_conflict(
                    &other_assignments,
                    new_intent,
                    &agent_identity.id,
                ) {
                    if matches!(assessment.action, ConflictAction::Block) {
                        return Err(MnemosyneError::BranchConflict(format!(
                            "Cannot change intent: {}",
                            assessment.reason
                        )));
                    }
                }

                Ok(())
            }
        }
    }

    /// Execute git command with validation
    ///
    /// Wraps git command execution with branch guard validation.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - Agent executing command
    /// * `args` - Git command arguments
    pub fn execute_git(&self, agent_id: &AgentId, args: &[String]) -> Result<std::process::Output> {
        if !self.config.enabled {
            // If disabled, execute directly without validation
            return std::process::Command::new("git")
                .args(args)
                .current_dir(&self.repo_root)
                .output()
                .map_err(|e| MnemosyneError::Other(format!("Git command failed: {}", e)));
        }

        // Use git wrapper for validation
        self.git_wrapper.execute(agent_id, args)
    }

    /// Validate file write operation
    ///
    /// Check if agent's intent allows writing to specific file.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - Agent attempting write
    /// * `file_path` - Path to file
    pub fn validate_file_write(&self, agent_id: &AgentId, file_path: &Path) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let registry = self
            .registry
            .read()
            .map_err(|e| MnemosyneError::Other(format!("Failed to read registry: {}", e)))?;

        let assignment = registry.get_agent_assignment(agent_id).ok_or_else(|| {
            MnemosyneError::NotFound(format!("No assignment for agent {}", agent_id))
        })?;

        if assignment.intent.allows_write(file_path) {
            Ok(())
        } else {
            Err(MnemosyneError::BranchConflict(format!(
                "Agent {} not permitted to write to {:?} (intent: {:?})",
                agent_id, file_path, assignment.intent
            )))
        }
    }

    /// Validate current branch matches assignment
    ///
    /// Ensures agent hasn't accidentally switched branches outside of coordination protocol.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - Agent to validate
    pub fn validate_current_branch(&self, agent_id: &AgentId) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let registry = self
            .registry
            .read()
            .map_err(|e| MnemosyneError::Other(format!("Failed to read registry: {}", e)))?;

        let assignment = registry.get_agent_assignment(agent_id).ok_or_else(|| {
            MnemosyneError::NotFound(format!("No assignment for agent {}", agent_id))
        })?;

        let current_state = GitState::from_repo_root(&self.repo_root)?;

        if current_state.current_branch != assignment.branch {
            return Err(MnemosyneError::BranchConflict(format!(
                "Branch mismatch: Agent {} assigned to '{}', but currently on '{}'",
                agent_id, assignment.branch, current_state.current_branch
            )));
        }

        Ok(())
    }

    /// Check if agent can perform operation type
    ///
    /// Used by MCP tools and other integrations to validate operations.
    pub fn can_perform_operation(
        &self,
        agent_id: &AgentId,
        operation: GitOperationType,
    ) -> Result<bool> {
        if !self.config.enabled {
            return Ok(true);
        }

        match operation {
            GitOperationType::Read => Ok(true), // Always allowed
            GitOperationType::Write => {
                // Check if agent has write intent
                let registry = self.registry.read().map_err(|e| {
                    MnemosyneError::Other(format!("Failed to read registry: {}", e))
                })?;

                if let Some(assignment) = registry.get_agent_assignment(agent_id) {
                    Ok(!assignment.intent.is_readonly())
                } else {
                    Ok(false)
                }
            }
            GitOperationType::BranchSwitch => {
                // Never allowed without coordination protocol
                Ok(false)
            }
            GitOperationType::Other => Ok(true), // Allow by default
        }
    }

    /// Get audit log from git wrapper
    pub fn get_audit_log(&self) -> Result<Vec<crate::orchestration::git_wrapper::GitAuditEntry>> {
        self.git_wrapper.get_audit_log()
    }

    /// Get conflict detector (for customization)
    pub fn conflict_detector_mut(&mut self) -> &mut ConflictDetector {
        &mut self.conflict_detector
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launcher::agents::AgentRole;
    use crate::types::Namespace;
    use tempfile::TempDir;

    fn setup_test_repo() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        std::process::Command::new("git")
            .arg("init")
            .current_dir(repo_path)
            .output()
            .unwrap();

        std::process::Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        std::fs::write(repo_path.join("README.md"), "Test").unwrap();
        std::process::Command::new("git")
            .args(["add", "README.md"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        std::process::Command::new("git")
            .args(["commit", "-m", "Initial"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        temp_dir
    }

    #[test]
    fn test_orchestrator_bypass() {
        let temp_dir = setup_test_repo();
        let registry = Arc::new(RwLock::new(BranchRegistry::new()));
        let guard = BranchGuard::new(registry, temp_dir.path().to_path_buf());

        let orchestrator = AgentIdentity::new(
            AgentRole::Orchestrator,
            Namespace::Global,
            "main".to_string(),
            temp_dir.path().to_path_buf(),
        );

        // Orchestrator should bypass restrictions
        let result =
            guard.validate_branch_access(&orchestrator, "any-branch", &WorkIntent::FullBranch);

        assert!(result.is_ok());
    }

    #[test]
    fn test_regular_agent_blocked_without_assignment() {
        let temp_dir = setup_test_repo();
        let mut registry = BranchRegistry::new();

        // Create agent identity first
        let agent = AgentIdentity::new(
            AgentRole::Executor,
            Namespace::Global,
            "main".to_string(),
            temp_dir.path().to_path_buf(),
        );

        // Use the agent's ID for registry assignment
        let agent_id = agent.id.clone();

        // Assign agent to main
        registry
            .assign_agent(
                agent_id,
                "main".to_string(),
                WorkIntent::FullBranch,
                CoordinationMode::Isolated,
            )
            .unwrap();

        let registry = Arc::new(RwLock::new(registry));
        let guard = BranchGuard::new(registry, temp_dir.path().to_path_buf());

        // Create feature branch
        std::process::Command::new("git")
            .args(["branch", "feature"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        // Should be blocked from accessing feature branch (agent assigned to main, trying to access feature)
        let result = guard.validate_branch_access(&agent, "feature", &WorkIntent::FullBranch);

        assert!(result.is_err());
    }

    #[test]
    fn test_readonly_auto_approved() {
        let temp_dir = setup_test_repo();
        let registry = Arc::new(RwLock::new(BranchRegistry::new()));
        let guard = BranchGuard::new(registry, temp_dir.path().to_path_buf());

        let agent = AgentIdentity::new(
            AgentRole::Executor,
            Namespace::Global,
            "main".to_string(),
            temp_dir.path().to_path_buf(),
        );

        // ReadOnly should be allowed without assignment
        let result = guard.validate_branch_access(&agent, "main", &WorkIntent::ReadOnly);

        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_file_write() {
        let temp_dir = setup_test_repo();
        let mut registry = BranchRegistry::new();
        let agent_id = AgentId::new();

        registry
            .assign_agent(
                agent_id.clone(),
                "main".to_string(),
                WorkIntent::Write(vec![PathBuf::from("src/")]),
                CoordinationMode::Isolated,
            )
            .unwrap();

        let registry = Arc::new(RwLock::new(registry));
        let guard = BranchGuard::new(registry, temp_dir.path().to_path_buf());

        // Should allow write to src/
        assert!(guard
            .validate_file_write(&agent_id, &PathBuf::from("src/main.rs"))
            .is_ok());

        // Should block write to tests/
        assert!(guard
            .validate_file_write(&agent_id, &PathBuf::from("tests/test.rs"))
            .is_err());
    }

    #[test]
    fn test_disabled_guard_allows_everything() {
        let temp_dir = setup_test_repo();
        let registry = Arc::new(RwLock::new(BranchRegistry::new()));
        let config = BranchGuardConfig {
            enabled: false,
            ..Default::default()
        };
        let guard = BranchGuard::with_config(config, registry, temp_dir.path().to_path_buf());

        let agent = AgentIdentity::new(
            AgentRole::Executor,
            Namespace::Global,
            "main".to_string(),
            temp_dir.path().to_path_buf(),
        );

        // Everything should be allowed when disabled
        assert!(guard
            .validate_branch_access(&agent, "any-branch", &WorkIntent::FullBranch)
            .is_ok());
    }
}
