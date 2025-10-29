//! Supervision Tree - Erlang-Style Actor Supervision
//!
//! Manages the lifecycle of all 4 agents:
//! - Orchestrator (root supervisor)
//! - Optimizer, Reviewer, Executor (supervised children)
//!
//! Provides:
//! - Automatic restart on failure
//! - Graceful shutdown
//! - Actor registry

use crate::error::Result;
use crate::launcher::agents::AgentRole;
use crate::orchestration::actors::{
    ExecutorActor, OptimizerActor, OrchestratorActor, ReviewerActor,
};
use crate::orchestration::messages::{
    ExecutorMessage, OptimizerMessage, OrchestratorMessage, ReviewerMessage,
};
use crate::orchestration::network;
use crate::storage::StorageBackend;
use crate::types::Namespace;
use ractor::{Actor, ActorRef, SupervisionEvent};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Supervision configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisionConfig {
    /// Max restart attempts before giving up
    pub max_restarts: usize,

    /// Time window for restart counting
    pub restart_window_secs: u64,

    /// Enable sub-agent spawning
    pub enable_subagents: bool,

    /// Max concurrent agents
    pub max_concurrent_agents: usize,
}

impl Default for SupervisionConfig {
    fn default() -> Self {
        Self {
            max_restarts: 3,
            restart_window_secs: 60,
            enable_subagents: true,
            max_concurrent_agents: 4,
        }
    }
}

/// Supervision tree managing all agents
pub struct SupervisionTree {
    /// Configuration
    config: SupervisionConfig,

    /// Storage backend
    storage: Arc<dyn StorageBackend>,

    /// Network layer
    network: Arc<network::NetworkLayer>,

    /// Namespace for this session
    namespace: Namespace,

    /// Orchestrator actor
    orchestrator: Option<ActorRef<OrchestratorMessage>>,

    /// Optimizer actor
    optimizer: Option<ActorRef<OptimizerMessage>>,

    /// Reviewer actor
    reviewer: Option<ActorRef<ReviewerMessage>>,

    /// Executor actor
    executor: Option<ActorRef<ExecutorMessage>>,
}

impl SupervisionTree {
    /// Create a new supervision tree
    pub async fn new(
        config: SupervisionConfig,
        storage: Arc<dyn StorageBackend>,
        network: Arc<network::NetworkLayer>,
    ) -> Result<Self> {
        // Detect namespace
        let namespace = Namespace::Session {
            project: "orchestration".to_string(),
            session_id: uuid::Uuid::new_v4().to_string(),
        };

        Ok(Self {
            config,
            storage,
            network,
            namespace,
            orchestrator: None,
            optimizer: None,
            reviewer: None,
            executor: None,
        })
    }

    /// Start all agents in the supervision tree
    pub async fn start(&mut self) -> Result<()> {
        tracing::info!("Starting supervision tree");

        // Spawn Optimizer
        let (optimizer_ref, _) = Actor::spawn(
            Some("optimizer".to_string()),
            OptimizerActor::new(self.storage.clone(), self.namespace.clone()),
            (self.storage.clone(), self.namespace.clone()),
        )
        .await
        .map_err(|e| crate::error::MnemosyneError::ActorError(e.to_string()))?;

        optimizer_ref
            .cast(OptimizerMessage::Initialize)
            .map_err(|e| crate::error::MnemosyneError::ActorError(e.to_string()))?;

        self.optimizer = Some(optimizer_ref);

        // Spawn Reviewer
        let (reviewer_ref, _) = Actor::spawn(
            Some("reviewer".to_string()),
            ReviewerActor::new(self.storage.clone(), self.namespace.clone()),
            (self.storage.clone(), self.namespace.clone()),
        )
        .await
        .map_err(|e| crate::error::MnemosyneError::ActorError(e.to_string()))?;

        reviewer_ref
            .cast(ReviewerMessage::Initialize)
            .map_err(|e| crate::error::MnemosyneError::ActorError(e.to_string()))?;

        self.reviewer = Some(reviewer_ref);

        // Spawn Executor
        let (executor_ref, _) = Actor::spawn(
            Some("executor".to_string()),
            ExecutorActor::new(self.storage.clone(), self.namespace.clone()),
            (self.storage.clone(), self.namespace.clone()),
        )
        .await
        .map_err(|e| crate::error::MnemosyneError::ActorError(e.to_string()))?;

        executor_ref
            .cast(ExecutorMessage::Initialize)
            .map_err(|e| crate::error::MnemosyneError::ActorError(e.to_string()))?;

        self.executor = Some(executor_ref);

        // Spawn Orchestrator (root supervisor)
        let (orchestrator_ref, _) = Actor::spawn(
            Some("orchestrator".to_string()),
            OrchestratorActor::new(self.storage.clone(), self.namespace.clone()),
            (self.storage.clone(), self.namespace.clone()),
        )
        .await
        .map_err(|e| crate::error::MnemosyneError::ActorError(e.to_string()))?;

        orchestrator_ref
            .cast(OrchestratorMessage::Initialize)
            .map_err(|e| crate::error::MnemosyneError::ActorError(e.to_string()))?;

        self.orchestrator = Some(orchestrator_ref);

        tracing::info!("Supervision tree started with {} agents", 4);

        Ok(())
    }

    /// Stop all agents gracefully
    pub async fn stop(&mut self) -> Result<()> {
        tracing::info!("Stopping supervision tree");

        // Stop in reverse order (children first, then supervisor)
        if let Some(executor) = self.executor.take() {
            executor.stop(None);
        }

        if let Some(reviewer) = self.reviewer.take() {
            reviewer.stop(None);
        }

        if let Some(optimizer) = self.optimizer.take() {
            optimizer.stop(None);
        }

        if let Some(orchestrator) = self.orchestrator.take() {
            orchestrator.stop(None);
        }

        // Wait for actors to stop
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        tracing::info!("Supervision tree stopped");

        Ok(())
    }

    /// Get reference to orchestrator
    pub fn orchestrator(&self) -> &ActorRef<OrchestratorMessage> {
        self.orchestrator
            .as_ref()
            .expect("Orchestrator not started")
    }

    /// Get reference to optimizer
    pub fn optimizer(&self) -> Option<&ActorRef<OptimizerMessage>> {
        self.optimizer.as_ref()
    }

    /// Get reference to reviewer
    pub fn reviewer(&self) -> Option<&ActorRef<ReviewerMessage>> {
        self.reviewer.as_ref()
    }

    /// Get reference to executor
    pub fn executor(&self) -> Option<&ActorRef<ExecutorMessage>> {
        self.executor.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConnectionMode, LibsqlStorage};

    #[tokio::test]
    async fn test_supervision_tree() {
        let storage = Arc::new(
            LibsqlStorage::new(ConnectionMode::InMemory)
                .await
                .unwrap(),
        );

        let network = Arc::new(network::NetworkLayer::new().await.unwrap());

        let config = SupervisionConfig::default();
        let mut tree = SupervisionTree::new(config, storage, network)
            .await
            .unwrap();

        // Start and stop
        tree.start().await.unwrap();
        assert!(tree.orchestrator.is_some());
        assert!(tree.optimizer.is_some());
        assert!(tree.reviewer.is_some());
        assert!(tree.executor.is_some());

        tree.stop().await.unwrap();
    }
}
