//! Agent Identity System
//!
//! Provides unique identification and tracking for agents in the orchestration system.
//! Each agent gets a unique ID, role, namespace, and branch assignment.
//!
//! # Key Concepts
//!
//! - **AgentId**: Unique identifier for each agent instance
//! - **AgentIdentity**: Complete metadata including role, namespace, branch, working directory
//! - **Parent tracking**: Sub-agents maintain reference to parent agent
//! - **Persistence**: Identities can be serialized for cross-session durability

use crate::launcher::agents::AgentRole;
use crate::types::Namespace;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use uuid::Uuid;

/// Unique agent identifier
///
/// Each agent instance gets a unique UUID-based ID that persists throughout
/// its lifecycle. The ID can be used to track agent state, branch assignments,
/// and coordination activities.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(Uuid);

impl AgentId {
    /// Create a new unique agent ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the inner UUID
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Create from existing UUID (for deserialization)
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Use short UUID format for readability (first 8 chars)
        write!(f, "{}", &self.0.to_string()[..8])
    }
}

impl From<Uuid> for AgentId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

/// Complete agent identity with metadata
///
/// Contains all information needed to identify and track an agent:
/// - Unique ID and role
/// - Namespace isolation
/// - Branch assignment
/// - Working directory
/// - Spawn time and parent relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIdentity {
    /// Unique identifier for this agent
    pub id: AgentId,

    /// Agent's role in the orchestration system
    pub role: AgentRole,

    /// Namespace for memory isolation
    pub namespace: Namespace,

    /// Currently assigned git branch
    pub branch: String,

    /// Working directory (may differ for sub-agents with isolated workdirs)
    pub working_dir: PathBuf,

    /// When this agent was spawned
    pub spawned_at: DateTime<Utc>,

    /// Parent agent ID if this is a sub-agent
    pub parent_id: Option<AgentId>,

    /// Whether this agent has special coordinator permissions
    pub is_coordinator: bool,
}

impl AgentIdentity {
    /// Create a new agent identity
    ///
    /// # Arguments
    ///
    /// * `role` - The agent's role (Orchestrator, Optimizer, Reviewer, Executor)
    /// * `namespace` - Namespace for memory isolation
    /// * `branch` - Initial git branch assignment
    /// * `working_dir` - Working directory for this agent
    pub fn new(
        role: AgentRole,
        namespace: Namespace,
        branch: String,
        working_dir: PathBuf,
    ) -> Self {
        let is_coordinator = matches!(role, AgentRole::Orchestrator);

        Self {
            id: AgentId::new(),
            role,
            namespace,
            branch,
            working_dir,
            spawned_at: Utc::now(),
            parent_id: None,
            is_coordinator,
        }
    }

    /// Create a sub-agent identity
    ///
    /// Sub-agents inherit parent's namespace and working directory by default,
    /// but can be assigned to different branches.
    ///
    /// # Arguments
    ///
    /// * `parent` - Parent agent identity
    /// * `role` - Sub-agent's role
    /// * `branch` - Branch for sub-agent (can be same as parent or different)
    pub fn new_subagent(parent: &AgentIdentity, role: AgentRole, branch: String) -> Self {
        Self {
            id: AgentId::new(),
            role,
            namespace: parent.namespace.clone(),
            branch,
            working_dir: parent.working_dir.clone(),
            spawned_at: Utc::now(),
            parent_id: Some(parent.id.clone()),
            is_coordinator: false, // Sub-agents are never coordinators
        }
    }

    /// Get a human-readable name for this agent
    ///
    /// Format: `{role}-{short_id}` (e.g., "executor-a1b2c3d4")
    pub fn name(&self) -> String {
        format!("{}-{}", self.role.as_str(), self.id)
    }

    /// Check if this is a sub-agent
    pub fn is_subagent(&self) -> bool {
        self.parent_id.is_some()
    }

    /// Update branch assignment
    ///
    /// This should only be called through the branch registry coordination system.
    pub fn update_branch(&mut self, new_branch: String) {
        self.branch = new_branch;
    }

    /// Get age of this agent (duration since spawn)
    pub fn age(&self) -> chrono::Duration {
        Utc::now().signed_duration_since(self.spawned_at)
    }

    /// Check if agent has coordinator permissions
    ///
    /// Coordinators (Orchestrator role) have special permissions:
    /// - Can join any branch without approval
    /// - Can force branch isolation
    /// - Can override conflicts
    pub fn has_coordinator_permissions(&self) -> bool {
        self.is_coordinator
    }
}

impl fmt::Display for AgentIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} [{}@{}{}]",
            self.name(),
            self.role.as_str(),
            self.branch,
            if self.is_subagent() {
                " (sub-agent)"
            } else {
                ""
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration as StdDuration;

    #[test]
    fn test_agent_id_uniqueness() {
        let id1 = AgentId::new();
        let id2 = AgentId::new();

        assert_ne!(id1, id2);
        assert_ne!(id1.to_string(), id2.to_string());
    }

    #[test]
    fn test_agent_id_display() {
        let id = AgentId::new();
        let display = id.to_string();

        // Should be 8 characters (short UUID)
        assert_eq!(display.len(), 8);
    }

    #[test]
    fn test_agent_identity_creation() {
        let identity = AgentIdentity::new(
            AgentRole::Executor,
            Namespace::Global,
            "main".to_string(),
            PathBuf::from("/test"),
        );

        assert_eq!(identity.role, AgentRole::Executor);
        assert_eq!(identity.branch, "main");
        assert!(!identity.is_subagent());
        assert!(!identity.has_coordinator_permissions());
    }

    #[test]
    fn test_orchestrator_has_coordinator_permissions() {
        let identity = AgentIdentity::new(
            AgentRole::Orchestrator,
            Namespace::Global,
            "main".to_string(),
            PathBuf::from("/test"),
        );

        assert!(identity.has_coordinator_permissions());
    }

    #[test]
    fn test_subagent_creation() {
        let parent = AgentIdentity::new(
            AgentRole::Executor,
            Namespace::Project {
                name: "test".to_string(),
            },
            "feature/parent".to_string(),
            PathBuf::from("/test"),
        );

        let subagent = AgentIdentity::new_subagent(
            &parent,
            AgentRole::Reviewer,
            "feature/child".to_string(),
        );

        assert!(subagent.is_subagent());
        assert_eq!(subagent.parent_id, Some(parent.id.clone()));
        assert_eq!(subagent.namespace, parent.namespace);
        assert_eq!(subagent.working_dir, parent.working_dir);
        assert_eq!(subagent.branch, "feature/child");
        assert!(!subagent.has_coordinator_permissions()); // Sub-agents never coordinators
    }

    #[test]
    fn test_agent_name() {
        let identity = AgentIdentity::new(
            AgentRole::Optimizer,
            Namespace::Global,
            "main".to_string(),
            PathBuf::from("/test"),
        );

        let name = identity.name();
        assert!(name.starts_with("optimizer-"));
        assert_eq!(name.len(), "optimizer-".len() + 8); // role + dash + 8 char ID
    }

    #[test]
    fn test_branch_update() {
        let mut identity = AgentIdentity::new(
            AgentRole::Executor,
            Namespace::Global,
            "main".to_string(),
            PathBuf::from("/test"),
        );

        identity.update_branch("feature/new".to_string());
        assert_eq!(identity.branch, "feature/new");
    }

    #[test]
    fn test_agent_age() {
        let identity = AgentIdentity::new(
            AgentRole::Executor,
            Namespace::Global,
            "main".to_string(),
            PathBuf::from("/test"),
        );

        // Age should be very small (just created)
        let age = identity.age();
        assert!(age.num_seconds() < 1);

        // Simulate passage of time
        thread::sleep(StdDuration::from_millis(10));
        let age_after = identity.age();
        assert!(age_after > age);
    }

    #[test]
    fn test_identity_display() {
        let identity = AgentIdentity::new(
            AgentRole::Executor,
            Namespace::Global,
            "feature/test".to_string(),
            PathBuf::from("/test"),
        );

        let display = identity.to_string();
        assert!(display.contains("executor-"));
        assert!(display.contains("feature/test"));
        assert!(!display.contains("sub-agent"));

        // Test sub-agent display
        let subagent = AgentIdentity::new_subagent(&identity, AgentRole::Reviewer, "main".to_string());
        let sub_display = subagent.to_string();
        assert!(sub_display.contains("(sub-agent)"));
    }

    #[test]
    fn test_identity_serialization() {
        let identity = AgentIdentity::new(
            AgentRole::Executor,
            Namespace::Project {
                name: "mnemosyne".to_string(),
            },
            "feature/test".to_string(),
            PathBuf::from("/test"),
        );

        // Serialize
        let json = serde_json::to_string(&identity).unwrap();
        assert!(!json.is_empty());

        // Deserialize
        let deserialized: AgentIdentity = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, identity.id);
        assert_eq!(deserialized.role, identity.role);
        assert_eq!(deserialized.branch, identity.branch);
    }
}
