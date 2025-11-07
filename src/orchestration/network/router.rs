//! Message Router - Hybrid Local/Remote Routing
//!
//! Routes messages to the appropriate destination:
//! - Local agents: Use Ractor message passing
//! - Remote agents: Use Iroh QUIC streams
//!
//! Maintains agent registry for discovery.

use crate::launcher::agents::AgentRole;
use crate::orchestration::messages::{
    AgentMessage, ExecutorMessage, OptimizerMessage, OrchestratorMessage, ReviewerMessage,
};
use ractor::ActorRef;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Agent location (local or remote)
#[derive(Debug, Clone)]
pub enum AgentLocation {
    /// Local actor reference
    Local(LocalAgent),

    /// Remote node ID
    Remote(String),
}

/// Local agent references
#[derive(Debug, Clone)]
pub enum LocalAgent {
    Orchestrator(ActorRef<OrchestratorMessage>),
    Optimizer(ActorRef<OptimizerMessage>),
    Reviewer(ActorRef<ReviewerMessage>),
    Executor(ActorRef<ExecutorMessage>),
}

/// Message router for hybrid local/remote routing
pub struct MessageRouter {
    /// Agent registry mapping role to location
    registry: Arc<RwLock<HashMap<AgentRole, AgentLocation>>>,
}

impl MessageRouter {
    /// Create a new message router
    pub fn new() -> Self {
        Self {
            registry: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a local agent
    pub async fn register_local(&self, role: AgentRole, agent: LocalAgent) {
        let mut registry = self.registry.write().await;
        registry.insert(role, AgentLocation::Local(agent));
        tracing::debug!("Registered local agent: {:?}", role);
    }

    /// Register a remote agent
    pub async fn register_remote(&self, role: AgentRole, node_id: String) {
        let mut registry = self.registry.write().await;
        registry.insert(role, AgentLocation::Remote(node_id.clone()));
        tracing::debug!("Registered remote agent: {:?} at {}", role, node_id);
    }

    /// Route a message to the appropriate agent
    pub async fn route(&self, to: AgentRole, message: AgentMessage) -> Result<(), String> {
        let registry = self.registry.read().await;

        match registry.get(&to) {
            Some(AgentLocation::Local(agent)) => {
                // Route to local actor via Ractor
                self.route_local(agent, message).await
            }
            Some(AgentLocation::Remote(node_id)) => {
                // Route to remote agent via Iroh
                tracing::debug!("Routing to remote agent {} at {}", to.as_str(), node_id);
                // TODO: Implement remote routing via Iroh
                Err("Remote routing not yet implemented".to_string())
            }
            None => {
                tracing::warn!("No route found for agent: {:?}", to);
                Err(format!("Agent not registered: {:?}", to))
            }
        }
    }

    /// Route message to local actor
    async fn route_local(&self, agent: &LocalAgent, message: AgentMessage) -> Result<(), String> {
        match (agent, message) {
            (LocalAgent::Orchestrator(actor), AgentMessage::Orchestrator(msg)) => {
                actor
                    .cast(msg)
                    .map_err(|e| format!("Failed to route to orchestrator: {:?}", e))?;
            }
            (LocalAgent::Optimizer(actor), AgentMessage::Optimizer(msg)) => {
                actor
                    .cast(msg)
                    .map_err(|e| format!("Failed to route to optimizer: {:?}", e))?;
            }
            (LocalAgent::Reviewer(actor), AgentMessage::Reviewer(msg)) => {
                actor
                    .cast(msg)
                    .map_err(|e| format!("Failed to route to reviewer: {:?}", e))?;
            }
            (LocalAgent::Executor(actor), AgentMessage::Executor(msg)) => {
                actor
                    .cast(*msg)
                    .map_err(|e| format!("Failed to route to executor: {:?}", e))?;
            }
            _ => {
                return Err("Message type mismatch with agent type".to_string());
            }
        }

        Ok(())
    }

    /// Get all registered agents
    pub async fn list_agents(&self) -> Vec<(AgentRole, AgentLocation)> {
        let registry = self.registry.read().await;
        registry
            .iter()
            .map(|(role, location)| (*role, location.clone()))
            .collect()
    }

    /// Check if an agent is registered
    pub async fn is_registered(&self, role: &AgentRole) -> bool {
        let registry = self.registry.read().await;
        registry.contains_key(role)
    }
}

impl Default for MessageRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_router_registration() {
        let router = MessageRouter::new();

        // Register a remote agent
        router
            .register_remote(AgentRole::Executor, "node-123".to_string())
            .await;

        assert!(router.is_registered(&AgentRole::Executor).await);
        assert!(!router.is_registered(&AgentRole::Optimizer).await);

        let agents = router.list_agents().await;
        assert_eq!(agents.len(), 1);
    }
}
