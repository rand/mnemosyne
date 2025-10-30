//! Agent Registry
//!
//! Centralized registry for tracking active agents and their status.
//! Provides thread-safe access to agent information for UI display.

use crate::ics::agent_status::{AgentActivity, AgentInfo};
use crate::launcher::agents::AgentRole;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;

/// Agent status information
#[derive(Debug, Clone)]
struct AgentStatus {
    /// Agent unique identifier
    id: String,
    /// Agent display name
    name: String,
    /// Agent role
    role: AgentRole,
    /// Current activity
    activity: AgentActivity,
    /// Last activity timestamp
    last_active: SystemTime,
    /// Activity message
    message: Option<String>,
}

impl AgentStatus {
    /// Convert to AgentInfo for UI display
    fn to_info(&self) -> AgentInfo {
        AgentInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            activity: self.activity.clone(),
            last_active: self.last_active,
            message: self.message.clone(),
        }
    }
}

/// Thread-safe agent registry
#[derive(Clone)]
pub struct AgentRegistry {
    /// Map of agent ID to status
    agents: Arc<RwLock<HashMap<String, AgentStatus>>>,
}

impl AgentRegistry {
    /// Create new empty registry
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new agent
    pub async fn register(&self, id: String, name: String, role: AgentRole) {
        let mut agents = self.agents.write().await;
        agents.insert(
            id.clone(),
            AgentStatus {
                id,
                name,
                role,
                activity: AgentActivity::Idle,
                last_active: SystemTime::now(),
                message: None,
            },
        );
    }

    /// Update agent activity
    pub async fn update_activity(
        &self,
        id: &str,
        activity: AgentActivity,
        message: Option<String>,
    ) {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(id) {
            agent.activity = activity;
            agent.last_active = SystemTime::now();
            agent.message = message;
        }
    }

    /// Mark agent as idle
    pub async fn mark_idle(&self, id: &str) {
        self.update_activity(id, AgentActivity::Idle, None).await;
    }

    /// Mark agent as analyzing
    pub async fn mark_analyzing(&self, id: &str, description: String) {
        self.update_activity(id, AgentActivity::Analyzing, Some(description))
            .await;
    }

    /// Mark agent as proposing
    pub async fn mark_proposing(&self, id: &str, description: String) {
        self.update_activity(id, AgentActivity::Proposing, Some(description))
            .await;
    }

    /// Mark agent as waiting
    pub async fn mark_waiting(&self, id: &str, reason: String) {
        self.update_activity(id, AgentActivity::Waiting, Some(reason))
            .await;
    }

    /// Mark agent as errored
    pub async fn mark_error(&self, id: &str, error: String) {
        self.update_activity(id, AgentActivity::Error(error.clone()), Some(error))
            .await;
    }

    /// Unregister an agent
    pub async fn unregister(&self, id: &str) {
        let mut agents = self.agents.write().await;
        agents.remove(id);
    }

    /// Get all active agents
    pub async fn list_agents(&self) -> Vec<AgentInfo> {
        let agents = self.agents.read().await;
        agents
            .values()
            .map(|status| status.to_info())
            .collect()
    }

    /// Get agent by ID
    pub async fn get_agent(&self, id: &str) -> Option<AgentInfo> {
        let agents = self.agents.read().await;
        agents.get(id).map(|status| status.to_info())
    }

    /// Get count of active agents
    pub async fn count(&self) -> usize {
        let agents = self.agents.read().await;
        agents.len()
    }

    /// Get agents by role
    pub async fn get_by_role(&self, role: AgentRole) -> Vec<AgentInfo> {
        let agents = self.agents.read().await;
        agents
            .values()
            .filter(|status| status.role == role)
            .map(|status| status.to_info())
            .collect()
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_agent() {
        let registry = AgentRegistry::new();

        registry
            .register(
                "agent-1".to_string(),
                "Orchestrator".to_string(),
                AgentRole::Orchestrator,
            )
            .await;

        assert_eq!(registry.count().await, 1);

        let agent = registry.get_agent("agent-1").await;
        assert!(agent.is_some());
        let agent = agent.unwrap();
        assert_eq!(agent.id, "agent-1");
        assert_eq!(agent.name, "Orchestrator");
    }

    #[tokio::test]
    async fn test_update_activity() {
        let registry = AgentRegistry::new();

        registry
            .register(
                "agent-1".to_string(),
                "Executor".to_string(),
                AgentRole::Executor,
            )
            .await;

        registry
            .mark_analyzing("agent-1", "Processing task".to_string())
            .await;

        let agent = registry.get_agent("agent-1").await.unwrap();
        assert!(matches!(agent.activity, AgentActivity::Analyzing));
        assert_eq!(agent.message.unwrap(), "Processing task");
    }

    #[tokio::test]
    async fn test_unregister() {
        let registry = AgentRegistry::new();

        registry
            .register(
                "agent-1".to_string(),
                "Test".to_string(),
                AgentRole::Executor,
            )
            .await;
        assert_eq!(registry.count().await, 1);

        registry.unregister("agent-1").await;
        assert_eq!(registry.count().await, 0);
    }

    #[tokio::test]
    async fn test_list_agents() {
        let registry = AgentRegistry::new();

        registry
            .register(
                "agent-1".to_string(),
                "Orchestrator".to_string(),
                AgentRole::Orchestrator,
            )
            .await;
        registry
            .register(
                "agent-2".to_string(),
                "Executor".to_string(),
                AgentRole::Executor,
            )
            .await;

        let agents = registry.list_agents().await;
        assert_eq!(agents.len(), 2);
    }

    #[tokio::test]
    async fn test_get_by_role() {
        let registry = AgentRegistry::new();

        registry
            .register(
                "agent-1".to_string(),
                "Executor 1".to_string(),
                AgentRole::Executor,
            )
            .await;
        registry
            .register(
                "agent-2".to_string(),
                "Executor 2".to_string(),
                AgentRole::Executor,
            )
            .await;
        registry
            .register(
                "agent-3".to_string(),
                "Optimizer".to_string(),
                AgentRole::Optimizer,
            )
            .await;

        let executors = registry.get_by_role(AgentRole::Executor).await;
        assert_eq!(executors.len(), 2);

        let optimizers = registry.get_by_role(AgentRole::Optimizer).await;
        assert_eq!(optimizers.len(), 1);
    }
}
