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
use async_trait::async_trait;
use ractor::ActorRef;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Trait for sending messages to remote nodes
#[async_trait]
pub trait RemoteTransport: Send + Sync {
    /// Send a message to a remote node
    async fn send(&self, node_id: &str, message: &AgentMessage) -> Result<(), String>;
}

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
    /// Agent registry mapping role to list of locations
    registry: Arc<RwLock<HashMap<AgentRole, Vec<AgentLocation>>>>,
    
    /// Round-robin indices for load balancing
    rr_indices: Arc<RwLock<HashMap<AgentRole, usize>>>,

    /// Remote transport for sending messages
    transport: Arc<RwLock<Option<Arc<dyn RemoteTransport>>>>,
}

impl MessageRouter {
    /// Create a new message router
    pub fn new() -> Self {
        Self {
            registry: Arc::new(RwLock::new(HashMap::new())),
            rr_indices: Arc::new(RwLock::new(HashMap::new())),
            transport: Arc::new(RwLock::new(None)),
        }
    }

    /// Set the remote transport
    pub async fn set_transport(&self, transport: Arc<dyn RemoteTransport>) {
        let mut t = self.transport.write().await;
        *t = Some(transport);
    }

    /// Register a local agent
    pub async fn register_local(&self, role: AgentRole, agent: LocalAgent) {
        let mut registry = self.registry.write().await;
        registry.entry(role).or_default().push(AgentLocation::Local(agent));
        tracing::debug!("Registered local agent: {:?}", role);
    }

    /// Register a remote agent
    pub async fn register_remote(&self, role: AgentRole, node_id: String) {
        let mut registry = self.registry.write().await;
        registry.entry(role).or_default().push(AgentLocation::Remote(node_id.clone()));
        tracing::debug!("Registered remote agent: {:?} at {}", role, node_id);
    }

    /// Route a message to the appropriate agent (Load Balanced)
    pub async fn route(&self, to: AgentRole, message: AgentMessage) -> Result<(), String> {
        let registry = self.registry.read().await;
        
        let locations = registry.get(&to).ok_or_else(|| {
            tracing::warn!("No route found for agent: {:?}", to);
            format!("Agent not registered: {:?}", to)
        })?;

        if locations.is_empty() {
            return Err(format!("No agents registered for role: {:?}", to));
        }

        // Round-robin selection
        let mut indices = self.rr_indices.write().await;
        let idx = indices.entry(to).or_insert(0);
        let location = &locations[*idx % locations.len()];
        *idx = (*idx + 1) % locations.len();

        match location {
            AgentLocation::Local(agent) => {
                self.route_local(agent, message).await
            }
            AgentLocation::Remote(node_id) => {
                tracing::debug!("Routing to remote agent {} at {}", to.as_str(), node_id);
                self.send_remote(node_id, &message).await
            }
        }
    }

    /// Route a message to a specific node ID (Direct Addressing)
    pub async fn route_to(&self, to: AgentRole, node_id: &str, message: AgentMessage) -> Result<(), String> {
        // Verify the agent exists at that location
        let registry = self.registry.read().await;
        let locations = registry.get(&to).ok_or_else(|| format!("Agent role not found: {:?}", to))?;
        
        let target_exists = locations.iter().any(|loc| {
            match loc {
                AgentLocation::Remote(id) => id == node_id,
                _ => false
            }
        });

        if target_exists {
            self.send_remote(node_id, &message).await
        } else {
            Err(format!("Agent {:?} not found at node {}", to, node_id))
        }
    }

    /// Send message to remote agent
    async fn send_remote(&self, node_id: &str, message: &AgentMessage) -> Result<(), String> {
        let transport_lock = self.transport.read().await;
        if let Some(transport) = transport_lock.as_ref() {
            transport.send(node_id, message).await
        } else {
            Err("Remote transport not initialized".to_string())
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
        let mut result = Vec::new();
        for (role, locations) in registry.iter() {
            for loc in locations {
                result.push((*role, loc.clone()));
            }
        }
        result
    }

    /// Check if an agent is registered
    pub async fn is_registered(&self, role: &AgentRole) -> bool {
        let registry = self.registry.read().await;
        registry.contains_key(role)
    }
}

impl std::fmt::Debug for MessageRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MessageRouter")
            .field("registry", &self.registry)
            .field("rr_indices", &self.rr_indices)
            .field("transport", &"RemoteTransport")
            .finish()
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
    use crate::orchestration::messages::OrchestratorMessage;

    struct MockTransport {
        sent_messages: Arc<RwLock<Vec<(String, AgentMessage)>>>,
    }

    #[async_trait]
    impl RemoteTransport for MockTransport {
        async fn send(&self, node_id: &str, message: &AgentMessage) -> Result<(), String> {
            let mut sent = self.sent_messages.write().await;
            sent.push((node_id.to_string(), message.clone()));
            Ok(())
        }
    }

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

    #[tokio::test]
    async fn test_remote_routing() {
        let router = MessageRouter::new();
        let sent_messages = Arc::new(RwLock::new(Vec::new()));
        let transport = Arc::new(MockTransport {
            sent_messages: sent_messages.clone(),
        });

        // Set transport
        router.set_transport(transport).await;

        // Register remote agent
        router
            .register_remote(AgentRole::Executor, "node-123".to_string())
            .await;

        // Route message
        let msg = AgentMessage::Orchestrator(OrchestratorMessage::Initialize);
        router.route(AgentRole::Executor, msg.clone()).await.unwrap();

        // Verify message was sent
        let sent = sent_messages.read().await;
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].0, "node-123");
        match &sent[0].1 {
            AgentMessage::Orchestrator(OrchestratorMessage::Initialize) => {},
            _ => panic!("Wrong message type"),
        }
    }
}
