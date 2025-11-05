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

#[cfg(feature = "python")]
pub mod dspy_bridge;
#[cfg(feature = "python")]
pub mod dspy_module_loader;
#[cfg(feature = "python")]
pub mod dspy_ab_testing;
#[cfg(feature = "python")]
pub mod dspy_telemetry;
#[cfg(feature = "python")]
pub mod dspy_production_logger;
#[cfg(feature = "python")]
pub mod dspy_instrumentation;
pub mod branch_guard;
pub mod branch_registry;
pub mod cli;
pub mod config;
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
pub mod prompts;
pub mod proposal_queue;
pub mod registry;
pub mod skills;
pub mod state;
pub mod status_line;
pub mod supervision;

#[cfg(test)]
mod coordination_tests;

// Re-export key types
pub use actors::{ExecutorActor, OptimizerActor, OrchestratorActor, ReviewerActor};

#[cfg(feature = "python")]
pub use dspy_bridge::DSpyBridge;
#[cfg(feature = "python")]
pub use dspy_module_loader::{DSpyModuleLoader, ModuleMetadata, ModuleVersion};
#[cfg(feature = "python")]
pub use dspy_ab_testing::{
    ABTestConfig, ABTestMetrics, ABTestRouter, RollbackEvent, RollbackPolicy, VersionMetrics,
};
#[cfg(feature = "python")]
pub use dspy_telemetry::{
    CostCalculator, DSpyEvent, EventType, ModuleMetrics as TelemetryModuleMetrics,
    TelemetryCollector, TokenUsage,
};
#[cfg(feature = "python")]
pub use dspy_production_logger::{
    InteractionLog, LogConfig, LogSink, LoggerStats, ProductionLogger, TrainingDataEntry,
};
pub use branch_coordinator::{
    BranchCoordinator, BranchCoordinatorConfig, JoinRequest, JoinResponse,
};
pub use branch_guard::{BranchGuard, BranchGuardConfig};
pub use branch_registry::{
    AgentAssignment, BranchRegistry, ConflictReport, CoordinationMode, SharedBranchRegistry,
    WorkIntent,
};
pub use cli::{parse_args, CliCommand, CliHandler, CliResult};
pub use config::{
    BranchIsolationConfig, BranchIsolationSettings, ConflictDetectionSettings,
    CrossProcessSettings, NotificationSettings,
};
pub use conflict_detector::{
    ConflictAction, ConflictAssessment, ConflictDetector, ConflictSeverity,
};
pub use conflict_notifier::{
    ConflictNotification, ConflictNotifier, NotificationConfig, NotificationType,
};
pub use cross_process::{
    CoordinationMessage, CrossProcessCoordinator, MessageType, ProcessRegistration,
};
pub use events::{AgentEvent, EventPersistence, EventReplay};
pub use file_tracker::{ActiveConflict, FileModification, FileTracker, ModificationType};
pub use git_state::{GitState, GitStateTracker};
pub use git_wrapper::{GitAuditEntry, GitOperationType, GitWrapper};
pub use identity::{AgentId, AgentIdentity};
pub use messages::{
    AgentMessage, ExecutorMessage, OptimizerMessage, OrchestratorMessage, ReviewerMessage,
};
pub use network::{AgentEndpoint, AgentProtocol, MessageRouter};
pub use notification_task::NotificationTaskHandle;
pub use prompts::{
    ConflictDecision, ConflictPrompt, InteractivePrompter, JoinDecision, JoinRequestPrompt,
};
pub use proposal_queue::{ProposalQueue, ProposalSender, SendError};
pub use registry::AgentRegistry;
pub use skills::{get_skills_directory, SkillMatch, SkillMetadata, SkillsDiscovery};
pub use state::{AgentState, Phase, WorkItem, WorkQueue};
pub use status_line::{ShellIntegration, StatusLine, StatusLineFormat, StatusLineProvider};
pub use supervision::{SupervisionConfig, SupervisionTree};

use crate::error::Result;
use std::sync::Arc;

/// Orchestration engine managing all four agents and their coordination
pub struct OrchestrationEngine {
    /// Supervision tree managing agent lifecycle
    supervision: SupervisionTree,

    /// Network layer for distributed communication
    network: Arc<network::NetworkLayer>,

    /// Storage backend for event persistence (WIP)
    #[allow(dead_code)]
    storage: Arc<dyn crate::storage::StorageBackend>,

    /// Optional event broadcaster for real-time API updates
    event_broadcaster: Option<crate::api::EventBroadcaster>,

    /// Optional state manager for dashboard state tracking
    #[allow(dead_code)]
    state_manager: Option<Arc<crate::api::StateManager>>,
}

impl OrchestrationEngine {
    /// Create a new orchestration engine
    pub async fn new(
        storage: Arc<dyn crate::storage::StorageBackend>,
        config: SupervisionConfig,
    ) -> Result<Self> {
        Self::new_with_events(storage, config, None).await
    }

    /// Create a new orchestration engine with event broadcasting
    pub async fn new_with_events(
        storage: Arc<dyn crate::storage::StorageBackend>,
        config: SupervisionConfig,
        event_broadcaster: Option<crate::api::EventBroadcaster>,
    ) -> Result<Self> {
        Self::new_with_state(storage, config, event_broadcaster, None).await
    }

    /// Create a new orchestration engine with event broadcasting and state management
    pub async fn new_with_state(
        storage: Arc<dyn crate::storage::StorageBackend>,
        config: SupervisionConfig,
        event_broadcaster: Option<crate::api::EventBroadcaster>,
        state_manager: Option<Arc<crate::api::StateManager>>,
    ) -> Result<Self> {
        // Initialize network layer
        let network = Arc::new(network::NetworkLayer::new().await?);

        // Create supervision tree with state management
        let supervision = SupervisionTree::new_with_state(
            config,
            storage.clone(),
            network.clone(),
            event_broadcaster.clone(),
            state_manager.clone(),
        )
        .await?;

        Ok(Self {
            supervision,
            network,
            storage,
            event_broadcaster,
            state_manager,
        })
    }

    /// Create a new orchestration engine with explicit namespace
    pub async fn new_with_namespace(
        storage: Arc<dyn crate::storage::StorageBackend>,
        config: SupervisionConfig,
        namespace: crate::types::Namespace,
    ) -> Result<Self> {
        Self::new_with_namespace_and_events(storage, config, namespace, None).await
    }

    /// Create a new orchestration engine with explicit namespace and event broadcasting
    pub async fn new_with_namespace_and_events(
        storage: Arc<dyn crate::storage::StorageBackend>,
        config: SupervisionConfig,
        namespace: crate::types::Namespace,
        event_broadcaster: Option<crate::api::EventBroadcaster>,
    ) -> Result<Self> {
        Self::new_with_namespace_and_state(storage, config, namespace, event_broadcaster, None).await
    }

    /// Create a new orchestration engine with explicit namespace, event broadcasting, and state management
    pub async fn new_with_namespace_and_state(
        storage: Arc<dyn crate::storage::StorageBackend>,
        config: SupervisionConfig,
        namespace: crate::types::Namespace,
        event_broadcaster: Option<crate::api::EventBroadcaster>,
        state_manager: Option<Arc<crate::api::StateManager>>,
    ) -> Result<Self> {
        // Initialize network layer
        let network = Arc::new(network::NetworkLayer::new().await?);

        // Create supervision tree with explicit namespace and state management
        let supervision = SupervisionTree::new_with_namespace_and_state(
            config,
            storage.clone(),
            network.clone(),
            namespace,
            event_broadcaster.clone(),
            state_manager.clone(),
        )
        .await?;

        Ok(Self {
            supervision,
            network,
            storage,
            event_broadcaster,
            state_manager,
        })
    }

    /// Start the orchestration engine
    pub async fn start(&mut self) -> Result<()> {
        tracing::debug!("Starting orchestration engine");

        // Start network layer
        self.network.start().await?;

        // Start supervision tree (spawns all agents)
        self.supervision.start().await?;

        tracing::debug!("Orchestration engine started");
        Ok(())
    }

    /// Stop the orchestration engine gracefully
    pub async fn stop(&mut self) -> Result<()> {
        tracing::debug!("Stopping orchestration engine");

        // Stop supervision tree (graceful agent shutdown)
        self.supervision.stop().await?;

        // Stop network layer
        self.network.stop().await?;

        tracing::debug!("Orchestration engine stopped");
        Ok(())
    }

    /// Get reference to orchestrator actor
    pub fn orchestrator(&self) -> &ractor::ActorRef<OrchestratorMessage> {
        self.supervision.orchestrator()
    }

    /// Get reference to event broadcaster
    pub fn event_broadcaster(&self) -> Option<&crate::api::EventBroadcaster> {
        self.event_broadcaster.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConnectionMode, LibsqlStorage};

    #[tokio::test]
    async fn test_engine_lifecycle() {
        let storage = Arc::new(LibsqlStorage::new(ConnectionMode::InMemory).await.unwrap());

        let config = SupervisionConfig::default();
        let mut engine = OrchestrationEngine::new(storage, config).await.unwrap();

        // Test start/stop cycle
        engine.start().await.unwrap();
        engine.stop().await.unwrap();
    }
}
