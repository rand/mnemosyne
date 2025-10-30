//! Branch Coordinator
//!
//! High-level orchestration of branch isolation and multi-agent coordination.
//!
//! # Responsibilities
//!
//! - Handle agent join requests for branches
//! - Auto-approve read-only access (per user requirement)
//! - Enforce isolation or enable coordination based on intent
//! - Notify agents of conflicts via ConflictNotifier
//! - Coordinate with external agents via CrossProcessCoordinator
//! - Make intelligent BLOCK vs WARN decisions
//! - Integrate all components: BranchRegistry, BranchGuard, ConflictNotifier, CrossProcessCoordinator

use crate::error::{MnemosyneError, Result};
use crate::orchestration::branch_guard::BranchGuard;
use crate::orchestration::branch_registry::{
    AgentAssignment, CoordinationMode, SharedBranchRegistry, WorkIntent,
};
use crate::orchestration::conflict_notifier::ConflictNotifier;
use crate::orchestration::cross_process::{
    CoordinationMessage, CrossProcessCoordinator, MessageType,
};
use crate::orchestration::git_wrapper::GitWrapper;
use crate::orchestration::identity::{AgentId, AgentIdentity};
use crate::orchestration::state::WorkItemId;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Branch coordinator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchCoordinatorConfig {
    /// Enable cross-process coordination
    pub enable_cross_process: bool,

    /// Auto-approve read-only access (per user requirement)
    pub auto_approve_readonly: bool,

    /// Default coordination mode
    pub default_mode: CoordinationMode,

    /// Mnemosyne directory for cross-process state
    pub mnemosyne_dir: Option<PathBuf>,
}

impl Default for BranchCoordinatorConfig {
    fn default() -> Self {
        Self {
            enable_cross_process: true,
            auto_approve_readonly: true, // Per user requirement
            default_mode: CoordinationMode::Isolated, // Per user requirement: default to isolation
            mnemosyne_dir: Some(PathBuf::from(".mnemosyne")),
        }
    }
}

/// Join request from an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRequest {
    /// Agent identity
    pub agent_identity: AgentIdentity,

    /// Target branch
    pub target_branch: String,

    /// Work intent
    pub intent: WorkIntent,

    /// Desired coordination mode
    pub mode: CoordinationMode,

    /// Work items for this request
    pub work_items: Vec<WorkItemId>,
}

/// Join response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JoinResponse {
    /// Approved - agent can proceed
    Approved {
        assignment_id: String,
        message: String,
    },

    /// Denied - agent cannot proceed
    Denied {
        reason: String,
        suggestions: Vec<String>,
    },

    /// Requires coordination - agent can proceed but must coordinate
    RequiresCoordination {
        assignment_id: String,
        other_agents: Vec<AgentId>,
        message: String,
    },
}

/// Branch coordinator - orchestrates all branch isolation components
pub struct BranchCoordinator {
    /// Branch registry
    registry: SharedBranchRegistry,

    /// Branch guard for access validation
    guard: Arc<BranchGuard>,

    /// Conflict notifier
    notifier: Arc<ConflictNotifier>,

    /// Cross-process coordinator (optional)
    cross_process: Option<Arc<RwLock<CrossProcessCoordinator>>>,

    /// Git wrapper
    git_wrapper: Arc<GitWrapper>,

    /// Configuration
    config: BranchCoordinatorConfig,
}

impl BranchCoordinator {
    /// Create a new branch coordinator
    pub fn new(
        registry: SharedBranchRegistry,
        guard: Arc<BranchGuard>,
        notifier: Arc<ConflictNotifier>,
        git_wrapper: Arc<GitWrapper>,
        config: BranchCoordinatorConfig,
    ) -> Result<Self> {
        let cross_process = if config.enable_cross_process {
            if let Some(ref mnemosyne_dir) = config.mnemosyne_dir {
                // Cross-process coordinator will be initialized per-agent
                // For now, store None and initialize on first use
                None
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            registry,
            guard,
            notifier,
            cross_process,
            git_wrapper,
            config,
        })
    }

    /// Handle a join request from an agent
    pub async fn handle_join_request(&self, request: JoinRequest) -> Result<JoinResponse> {
        tracing::info!(
            "Processing join request from agent {} for branch {}",
            request.agent_identity.id,
            request.target_branch
        );

        // 1. Auto-approve read-only access (per user requirement)
        if self.config.auto_approve_readonly && matches!(request.intent, WorkIntent::ReadOnly) {
            return self.approve_readonly_access(request).await;
        }

        // 2. Check orchestrator bypass (special permissions)
        if request.agent_identity.has_coordinator_permissions() {
            return self.approve_coordinator_access(request).await;
        }

        // 3. Validate branch access using branch guard
        match self.guard.validate_branch_access(
            &request.agent_identity,
            &request.target_branch,
            &request.intent,
        ) {
            Ok(_) => {
                // Access allowed, create assignment
                self.create_assignment(request).await
            }
            Err(e) => {
                // Access denied or requires coordination
                self.handle_access_denial(request, e).await
            }
        }
    }

    /// Auto-approve read-only access
    async fn approve_readonly_access(&self, request: JoinRequest) -> Result<JoinResponse> {
        let mut registry = self.registry.write().map_err(|e| {
            MnemosyneError::Other(format!("Failed to acquire registry lock: {}", e))
        })?;

        registry.assign_agent(
            request.agent_identity.id.clone(),
            request.target_branch.clone(),
            request.intent,
            CoordinationMode::Coordinated, // Read-only is always coordinated
        )?;

        // Update work items if provided
        if !request.work_items.is_empty() {
            registry.update_work_items(&request.agent_identity.id, request.work_items)?;
        }

        tracing::info!(
            "Auto-approved read-only access for agent {} on branch {}",
            request.agent_identity.id,
            request.target_branch
        );

        Ok(JoinResponse::Approved {
            assignment_id: request.agent_identity.id.to_string(),
            message: "Read-only access auto-approved".to_string(),
        })
    }

    /// Approve coordinator access (orchestrator bypass)
    async fn approve_coordinator_access(&self, request: JoinRequest) -> Result<JoinResponse> {
        let mut registry = self.registry.write().map_err(|e| {
            MnemosyneError::Other(format!("Failed to acquire registry lock: {}", e))
        })?;

        registry.assign_agent(
            request.agent_identity.id.clone(),
            request.target_branch.clone(),
            request.intent,
            request.mode,
        )?;

        // Update work items if provided
        if !request.work_items.is_empty() {
            registry.update_work_items(&request.agent_identity.id, request.work_items)?;
        }

        tracing::info!(
            "Approved coordinator access for agent {} on branch {}",
            request.agent_identity.id,
            request.target_branch
        );

        Ok(JoinResponse::Approved {
            assignment_id: request.agent_identity.id.to_string(),
            message: "Coordinator access approved".to_string(),
        })
    }

    /// Create assignment after validation succeeds
    async fn create_assignment(&self, request: JoinRequest) -> Result<JoinResponse> {
        let mut registry = self.registry.write().map_err(|e| {
            MnemosyneError::Other(format!("Failed to acquire registry lock: {}", e))
        })?;

        // Check if there are other agents on this branch
        let existing = registry
            .get_assignments(&request.target_branch)
            .into_iter()
            .filter(|a| a.agent_id != request.agent_identity.id)
            .collect::<Vec<_>>();

        registry.assign_agent(
            request.agent_identity.id.clone(),
            request.target_branch.clone(),
            request.intent,
            request.mode,
        )?;

        // Update work items if provided
        if !request.work_items.is_empty() {
            registry.update_work_items(&request.agent_identity.id, request.work_items)?;
        }

        if existing.is_empty() {
            // No other agents, proceed
            Ok(JoinResponse::Approved {
                assignment_id: request.agent_identity.id.to_string(),
                message: format!("Assigned to branch '{}'", request.target_branch),
            })
        } else {
            // Other agents present, requires coordination
            let other_agent_ids: Vec<AgentId> =
                existing.iter().map(|a| a.agent_id.clone()).collect();

            // Send coordination notifications
            self.notify_coordination_required(&request.agent_identity.id, &other_agent_ids)
                .await?;

            Ok(JoinResponse::RequiresCoordination {
                assignment_id: request.agent_identity.id.to_string(),
                other_agents: other_agent_ids.clone(),
                message: format!(
                    "Assigned to branch '{}'. Coordination required with {} other agent(s).",
                    request.target_branch,
                    other_agent_ids.len()
                ),
            })
        }
    }

    /// Handle access denial from branch guard
    async fn handle_access_denial(
        &self,
        request: JoinRequest,
        error: MnemosyneError,
    ) -> Result<JoinResponse> {
        tracing::warn!(
            "Access denied for agent {} on branch {}: {}",
            request.agent_identity.id,
            request.target_branch,
            error
        );

        // Generate helpful suggestions based on the error
        let suggestions = self.generate_suggestions(&request, &error).await;

        Ok(JoinResponse::Denied {
            reason: error.to_string(),
            suggestions,
        })
    }

    /// Generate suggestions for denied access
    async fn generate_suggestions(
        &self,
        request: &JoinRequest,
        _error: &MnemosyneError,
    ) -> Vec<String> {
        let mut suggestions = Vec::new();

        // Check if coordinated mode would help
        if matches!(request.mode, CoordinationMode::Isolated) {
            suggestions.push(
                "Try requesting CoordinationMode::Coordinated to work alongside other agents"
                    .to_string(),
            );
        }

        // Check if read-only would help
        if !matches!(request.intent, WorkIntent::ReadOnly) {
            suggestions.push(
                "Consider using WorkIntent::ReadOnly if you only need to read the code".to_string(),
            );
        }

        // Check if waiting would help
        if let Ok(registry) = self.registry.read() {
            let assignments = registry.get_assignments(&request.target_branch);
            if !assignments.is_empty() {
                suggestions.push(format!(
                    "Wait for {} active agent(s) to complete their work",
                    assignments.len()
                ));
            }
        }

        // Suggest creating a new branch
        suggestions.push(format!(
            "Create a new branch from '{}' for independent work",
            request.target_branch
        ));

        suggestions
    }

    /// Notify agents that coordination is required
    async fn notify_coordination_required(
        &self,
        new_agent: &AgentId,
        existing_agents: &[AgentId],
    ) -> Result<()> {
        // Use conflict notifier to send notifications
        for agent_id in existing_agents {
            tracing::info!(
                "Notifying agent {} about new agent {} requiring coordination",
                agent_id,
                new_agent
            );
        }

        // If cross-process coordination enabled, send messages
        if let Some(ref coordinator) = self.cross_process {
            let coordinator = coordinator.read().map_err(|e| {
                MnemosyneError::Other(format!("Failed to acquire cross-process lock: {}", e))
            })?;
            for agent_id in existing_agents {
                let message = CoordinationMessage {
                    id: uuid::Uuid::new_v4().to_string(),
                    from_agent: new_agent.clone(),
                    to_agent: Some(agent_id.clone()),
                    message_type: MessageType::JoinRequest,
                    timestamp: Utc::now(),
                    payload: serde_json::json!({
                        "new_agent": new_agent.to_string(),
                        "message": "New agent requires coordination on shared branch"
                    }),
                };

                coordinator.send_message(message)?;
            }
        }

        Ok(())
    }

    /// Release an agent's assignment
    pub async fn release_assignment(&self, agent_id: &AgentId) -> Result<()> {
        let mut registry = self.registry.write().map_err(|e| {
            MnemosyneError::Other(format!("Failed to acquire registry lock: {}", e))
        })?;
        registry.release_assignment(agent_id)?;

        tracing::info!("Released assignment for agent {}", agent_id);

        Ok(())
    }

    /// Get active assignments for a branch
    pub async fn get_branch_assignments(&self, branch: &str) -> Result<Vec<AgentAssignment>> {
        let registry = self.registry.read().map_err(|e| {
            MnemosyneError::Other(format!("Failed to acquire registry lock: {}", e))
        })?;
        Ok(registry.get_assignments(branch))
    }

    /// Get list of all active branches
    ///
    /// Returns branches that currently have at least one agent assigned.
    pub fn get_active_branches(&self) -> Result<Vec<String>> {
        let registry = self.registry.read().map_err(|e| {
            MnemosyneError::Other(format!("Failed to acquire registry lock: {}", e))
        })?;
        Ok(registry.active_branches())
    }

    /// Get the total count of active conflicts across all branches
    pub fn get_conflict_count(&self) -> Result<usize> {
        self.notifier.get_conflict_count()
    }

    /// Get the count of conflicts for a specific agent
    pub fn get_agent_conflict_count(&self, agent_id: &AgentId) -> Result<usize> {
        self.notifier.get_agent_conflict_count(agent_id)
    }

    /// Get all active conflicts
    pub fn get_all_conflicts(
        &self,
    ) -> Result<Vec<crate::orchestration::file_tracker::ActiveConflict>> {
        self.notifier.get_all_conflicts()
    }

    /// Get conflicts for a specific agent
    pub fn get_agent_conflicts(
        &self,
        agent_id: &AgentId,
    ) -> Result<Vec<crate::orchestration::file_tracker::ActiveConflict>> {
        self.notifier.get_agent_conflicts(agent_id)
    }

    /// Initialize cross-process coordinator for an agent
    pub async fn initialize_cross_process(&mut self, agent_id: AgentId) -> Result<()> {
        if !self.config.enable_cross_process {
            return Ok(());
        }

        if let Some(ref mnemosyne_dir) = self.config.mnemosyne_dir {
            let coordinator = CrossProcessCoordinator::new(mnemosyne_dir, agent_id)?;
            self.cross_process = Some(Arc::new(RwLock::new(coordinator)));

            tracing::info!("Initialized cross-process coordinator for agent");
        }

        Ok(())
    }

    /// Process incoming cross-process messages
    pub async fn process_cross_process_messages(&self) -> Result<Vec<CoordinationMessage>> {
        if let Some(ref coordinator) = self.cross_process {
            let coordinator = coordinator.read().map_err(|e| {
                MnemosyneError::Other(format!("Failed to acquire cross-process lock: {}", e))
            })?;
            coordinator.receive_messages()
        } else {
            Ok(vec![])
        }
    }

    /// Send heartbeat (if cross-process enabled)
    pub async fn send_heartbeat(&self) -> Result<()> {
        if let Some(ref coordinator) = self.cross_process {
            let mut coordinator = coordinator.write().map_err(|e| {
                MnemosyneError::Other(format!("Failed to acquire cross-process lock: {}", e))
            })?;
            coordinator.heartbeat()?;
        }
        Ok(())
    }

    /// Cleanup stale processes (if cross-process enabled)
    pub async fn cleanup_stale_processes(&self) -> Result<Vec<AgentId>> {
        if let Some(ref coordinator) = self.cross_process {
            let coordinator = coordinator.read().map_err(|e| {
                MnemosyneError::Other(format!("Failed to acquire cross-process lock: {}", e))
            })?;
            coordinator.cleanup_stale_processes()
        } else {
            Ok(vec![])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launcher::agents::AgentRole;
    use crate::orchestration::branch_registry::BranchRegistry;
    use crate::orchestration::conflict_detector::ConflictDetector;
    use crate::orchestration::conflict_notifier::NotificationConfig;
    use crate::orchestration::file_tracker::FileTracker;
    
    use crate::types::Namespace;
    use std::sync::Arc;

    async fn setup_coordinator() -> BranchCoordinator {
        let registry = Arc::new(RwLock::new(BranchRegistry::new()));

        let guard = Arc::new(BranchGuard::new(registry.clone(), PathBuf::from(".")));

        let conflict_detector = Arc::new(ConflictDetector::new());
        let file_tracker = Arc::new(FileTracker::new(conflict_detector));

        let notifier_config = NotificationConfig {
            enabled: true,
            notify_on_save: true,
            periodic_interval_minutes: 20,
            session_end_summary: true,
        };

        let notifier = Arc::new(ConflictNotifier::new(notifier_config, file_tracker));

        let git_wrapper = Arc::new(GitWrapper::new(registry.clone(), PathBuf::from(".")));

        let config = BranchCoordinatorConfig::default();

        BranchCoordinator::new(registry, guard, notifier, git_wrapper, config).unwrap()
    }

    #[tokio::test]
    async fn test_auto_approve_readonly() {
        let coordinator = setup_coordinator().await;

        let agent_identity = AgentIdentity {
            id: AgentId::new(),
            role: AgentRole::Executor,
            namespace: Namespace::Global,
            branch: "main".to_string(),
            working_dir: PathBuf::from("."),
            spawned_at: Utc::now(),
            parent_id: None,
            is_coordinator: false,
        };

        let request = JoinRequest {
            agent_identity,
            target_branch: "main".to_string(),
            intent: WorkIntent::ReadOnly,
            mode: CoordinationMode::Coordinated,
            work_items: vec![],
        };

        let response = coordinator.handle_join_request(request).await.unwrap();

        match response {
            JoinResponse::Approved { .. } => {
                // Success
            }
            _ => panic!("Expected Approved response for read-only access"),
        }
    }

    #[tokio::test]
    async fn test_coordinator_bypass() {
        let coordinator = setup_coordinator().await;

        let agent_identity = AgentIdentity {
            id: AgentId::new(),
            role: AgentRole::Orchestrator,
            namespace: Namespace::Global,
            branch: "main".to_string(),
            working_dir: PathBuf::from("."),
            spawned_at: Utc::now(),
            parent_id: None,
            is_coordinator: true,
        };

        let request = JoinRequest {
            agent_identity,
            target_branch: "main".to_string(),
            intent: WorkIntent::FullBranch,
            mode: CoordinationMode::Isolated,
            work_items: vec![],
        };

        let response = coordinator.handle_join_request(request).await.unwrap();

        match response {
            JoinResponse::Approved { .. } => {
                // Success
            }
            _ => panic!("Expected Approved response for coordinator access"),
        }
    }
}
