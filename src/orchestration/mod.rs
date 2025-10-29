//! Multi-Agent Orchestration System
//!
//! This module implements a three-layer agent communication architecture:
//! - **Layer 1 (Ractor)**: Process-local actor messaging with supervision
//! - **Layer 2 (Mnemosyne)**: Persistent event sourcing for durable execution
//! - **Layer 3 (Iroh)**: Distributed P2P networking between agent processes
//!
//! # Architecture
//!
//! Four primary agents coordinate through Ractor messages locally and Iroh
//! streams for distributed communication:
//!
//! - **Orchestrator**: Central coordinator managing work queue and dependencies
//! - **Optimizer**: Context optimization and skill discovery specialist
//! - **Reviewer**: Quality assurance with blocking quality gates
//! - **Executor**: Primary work agent with sub-agent spawning capability
//!
//! All agent state changes are persisted to Mnemosyne as events, enabling:
//! - Deterministic replay after crashes
//! - Cross-session continuity
//! - Complete audit trail
//! - State recovery

pub mod actors;
pub mod branch_coordinator;
pub mod branch_guard;
pub mod branch_registry;
pub mod conflict_detector;
pub mod conflict_notifier;
pub mod cross_process;
pub mod events;
pub mod file_tracker;
pub mod git_state;
pub mod git_wrapper;
pub mod identity;
pub mod integrations;
pub mod messages;
pub mod network;
pub mod notification_task;
pub mod state;
pub mod supervision;

#[cfg(test)]
mod coordination_tests;

// Re-export key types
pub use actors::{ExecutorActor, OrchestratorActor, OptimizerActor, ReviewerActor};
pub use branch_coordinator::{BranchCoordinator, BranchCoordinatorConfig, JoinRequest, JoinResponse};
pub use branch_guard::{BranchGuard, BranchGuardConfig};
pub use branch_registry::{
    AgentAssignment, BranchRegistry, ConflictReport, CoordinationMode, SharedBranchRegistry,
    WorkIntent,
};
pub use conflict_detector::{ConflictAction, ConflictAssessment, ConflictDetector, ConflictSeverity};
pub use conflict_notifier::{ConflictNotification, ConflictNotifier, NotificationConfig, NotificationType};
pub use cross_process::{CoordinationMessage, CrossProcessCoordinator, MessageType, ProcessRegistration};
pub use events::{AgentEvent, EventPersistence, EventReplay};
pub use file_tracker::{ActiveConflict, FileModification, FileTracker, ModificationType};
pub use git_state::{GitState, GitStateTracker};
pub use git_wrapper::{GitAuditEntry, GitOperationType, GitWrapper};
pub use identity::{AgentId, AgentIdentity};
pub use messages::{AgentMessage, ExecutorMessage, OptimizerMessage, OrchestratorMessage, ReviewerMessage};
pub use network::{AgentEndpoint, AgentProtocol, MessageRouter};
pub use notification_task::NotificationTaskHandle;
pub use state::{AgentState, Phase, WorkItem, WorkQueue};
pub use supervision::{SupervisionConfig, SupervisionTree};

use crate::error::Result;
use std::sync::Arc;

/// Orchestration engine managing all four agents and their coordination
pub struct OrchestrationEngine {
    /// Supervision tree managing agent lifecycle
    supervision: SupervisionTree,

    /// Network layer for distributed communication
    network: Arc<network::NetworkLayer>,

    /// Storage backend for event persistence
    storage: Arc<dyn crate::storage::StorageBackend>,
}

impl OrchestrationEngine {
    /// Create a new orchestration engine
    pub async fn new(
        storage: Arc<dyn crate::storage::StorageBackend>,
        config: SupervisionConfig,
    ) -> Result<Self> {
        // Initialize network layer
        let network = Arc::new(network::NetworkLayer::new().await?);

        // Create supervision tree
        let supervision = SupervisionTree::new(config, storage.clone(), network.clone()).await?;

        Ok(Self {
            supervision,
            network,
            storage,
        })
    }

    /// Create a new orchestration engine with explicit namespace
    pub async fn new_with_namespace(
        storage: Arc<dyn crate::storage::StorageBackend>,
        config: SupervisionConfig,
        namespace: crate::types::Namespace,
    ) -> Result<Self> {
        // Initialize network layer
        let network = Arc::new(network::NetworkLayer::new().await?);

        // Create supervision tree with explicit namespace
        let supervision = SupervisionTree::new_with_namespace(
            config,
            storage.clone(),
            network.clone(),
            namespace,
        )
        .await?;

        Ok(Self {
            supervision,
            network,
            storage,
        })
    }

    /// Start the orchestration engine
    pub async fn start(&mut self) -> Result<()> {
        tracing::info!("Starting orchestration engine");

        // Start network layer
        self.network.start().await?;

        // Start supervision tree (spawns all agents)
        self.supervision.start().await?;

        tracing::info!("Orchestration engine started");
        Ok(())
    }

    /// Stop the orchestration engine gracefully
    pub async fn stop(&mut self) -> Result<()> {
        tracing::info!("Stopping orchestration engine");

        // Stop supervision tree (graceful agent shutdown)
        self.supervision.stop().await?;

        // Stop network layer
        self.network.stop().await?;

        tracing::info!("Orchestration engine stopped");
        Ok(())
    }

    /// Get reference to orchestrator actor
    pub fn orchestrator(&self) -> &ractor::ActorRef<OrchestratorMessage> {
        self.supervision.orchestrator()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConnectionMode, LibsqlStorage};

    #[tokio::test]
    async fn test_engine_lifecycle() {
        let storage = Arc::new(
            LibsqlStorage::new(ConnectionMode::InMemory)
                .await
                .unwrap(),
        );

        let config = SupervisionConfig::default();
        let mut engine = OrchestrationEngine::new(storage, config).await.unwrap();

        // Test start/stop cycle
        engine.start().await.unwrap();
        engine.stop().await.unwrap();
    }
}
