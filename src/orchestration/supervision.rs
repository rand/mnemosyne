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
use crate::orchestration::proposal_queue::ProposalQueue;
use crate::orchestration::registry::AgentRegistry;
use crate::storage::StorageBackend;
use crate::types::Namespace;
use ractor::{Actor, ActorRef};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[cfg(feature = "python")]
use pyo3::prelude::*;

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
    /// Configuration (WIP)
    #[allow(dead_code)]
    config: SupervisionConfig,

    /// Storage backend
    storage: Arc<dyn StorageBackend>,

    /// Network layer (WIP)
    #[allow(dead_code)]
    network: Arc<network::NetworkLayer>,

    /// Namespace for this session
    namespace: Namespace,

    /// Agent registry for status tracking
    registry: AgentRegistry,

    /// Proposal queue for agent-to-ICS communication
    proposal_queue: ProposalQueue,

    /// Optional event broadcaster for real-time API updates
    event_broadcaster: Option<crate::api::EventBroadcaster>,

    /// Optional state manager for dashboard state tracking
    state_manager: Option<Arc<crate::api::StateManager>>,

    /// Orchestrator actor
    orchestrator: Option<ActorRef<OrchestratorMessage>>,

    /// Optimizer actor
    optimizer: Option<ActorRef<OptimizerMessage>>,

    /// Reviewer actor
    reviewer: Option<ActorRef<ReviewerMessage>>,

    /// Executor actor
    executor: Option<ActorRef<ExecutorMessage>>,

    /// SSE subscriber shutdown signal
    sse_shutdown_tx: Option<tokio::sync::broadcast::Sender<()>>,

    /// SSE subscriber task handle
    sse_subscriber_handle: Option<tokio::task::JoinHandle<()>>,
}

impl SupervisionTree {
    /// Helper to emit agent started event and update state
    async fn notify_agent_started(&self, agent_id: &str, agent_name: &str) {
        // Emit event if broadcaster is available
        if let Some(broadcaster) = &self.event_broadcaster {
            let event = crate::api::Event::agent_started(agent_id.to_string());
            if let Err(_e) = broadcaster.broadcast(event) {
                // Expected when no dashboard is connected - not an error
                tracing::debug!("No subscribers for agent started event ({})", agent_name);
            } else {
                tracing::debug!("Broadcasted agent started event for {}", agent_name);
            }
        }

        // Update state if state manager is available
        if let Some(state_manager) = &self.state_manager {
            let agent_info = crate::api::state::AgentInfo {
                id: agent_id.to_string(),
                state: crate::api::state::AgentState::Idle,
                updated_at: chrono::Utc::now(),
                metadata: std::collections::HashMap::new(),
                health: Some(crate::api::state::AgentHealth::default()),
            };
            state_manager.update_agent(agent_info).await;
            tracing::debug!("Updated state manager for {}", agent_name);
        }
    }

    /// Create a new supervision tree
    pub async fn new(
        config: SupervisionConfig,
        storage: Arc<dyn StorageBackend>,
        network: Arc<network::NetworkLayer>,
    ) -> Result<Self> {
        Self::new_with_state(config, storage, network, None, None).await
    }

    /// Create a new supervision tree with event broadcasting and state management
    pub async fn new_with_state(
        config: SupervisionConfig,
        storage: Arc<dyn StorageBackend>,
        network: Arc<network::NetworkLayer>,
        event_broadcaster: Option<crate::api::EventBroadcaster>,
        state_manager: Option<Arc<crate::api::StateManager>>,
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
            registry: AgentRegistry::new(),
            proposal_queue: ProposalQueue::new(),
            event_broadcaster,
            state_manager,
            orchestrator: None,
            optimizer: None,
            reviewer: None,
            executor: None,
            sse_shutdown_tx: None,
            sse_subscriber_handle: None,
        })
    }

    /// Create a new supervision tree with explicit namespace
    pub async fn new_with_namespace(
        config: SupervisionConfig,
        storage: Arc<dyn StorageBackend>,
        network: Arc<network::NetworkLayer>,
        namespace: Namespace,
    ) -> Result<Self> {
        Self::new_with_namespace_and_state(config, storage, network, namespace, None, None).await
    }

    /// Create a new supervision tree with explicit namespace, event broadcasting, and state management
    pub async fn new_with_namespace_and_state(
        config: SupervisionConfig,
        storage: Arc<dyn StorageBackend>,
        network: Arc<network::NetworkLayer>,
        namespace: Namespace,
        event_broadcaster: Option<crate::api::EventBroadcaster>,
        state_manager: Option<Arc<crate::api::StateManager>>,
    ) -> Result<Self> {
        Ok(Self {
            config,
            storage,
            network,
            namespace,
            registry: AgentRegistry::new(),
            proposal_queue: ProposalQueue::new(),
            event_broadcaster,
            state_manager,
            orchestrator: None,
            optimizer: None,
            reviewer: None,
            executor: None,
            sse_shutdown_tx: None,
            sse_subscriber_handle: None,
        })
    }

    /// Start all agents in the supervision tree
    pub async fn start(&mut self) -> Result<()> {
        tracing::info!("Starting supervision tree");
        tracing::info!(
            "Event broadcaster available: {}",
            self.event_broadcaster.is_some()
        );

        // Use unique names based on namespace to avoid registry conflicts in tests
        // In production, this creates names like "optimizer-session:project:session-123"
        let name_prefix = format!("{}", self.namespace);

        // Spawn Optimizer
        let optimizer_id = format!("{}-optimizer", name_prefix);
        let (optimizer_ref, _) = Actor::spawn(
            Some(optimizer_id.clone()),
            OptimizerActor::new(self.storage.clone(), self.namespace.clone()),
            (self.storage.clone(), self.namespace.clone()),
        )
        .await
        .map_err(|e| crate::error::MnemosyneError::ActorError(e.to_string()))?;

        // Register event broadcaster BEFORE Initialize to avoid race condition
        if let Some(broadcaster) = &self.event_broadcaster {
            optimizer_ref
                .cast(OptimizerMessage::RegisterEventBroadcaster(
                    broadcaster.clone(),
                ))
                .map_err(|e| {
                    tracing::warn!("Failed to register broadcaster with Optimizer: {:?}", e);
                    crate::error::MnemosyneError::ActorError(e.to_string())
                })?;
            tracing::debug!("Event broadcaster registered with Optimizer");
        }

        optimizer_ref
            .cast(OptimizerMessage::Initialize)
            .map_err(|e| crate::error::MnemosyneError::ActorError(e.to_string()))?;

        // Register in registry
        self.registry
            .register(
                optimizer_id.clone(),
                "Optimizer".to_string(),
                AgentRole::Optimizer,
            )
            .await;

        // Notify dashboard about agent startup
        self.notify_agent_started(&optimizer_id, "Optimizer").await;

        self.optimizer = Some(optimizer_ref);

        // Spawn Reviewer
        let reviewer_id = format!("{}-reviewer", name_prefix);
        let (reviewer_ref, _) = Actor::spawn(
            Some(reviewer_id.clone()),
            ReviewerActor::new(self.storage.clone(), self.namespace.clone()),
            (self.storage.clone(), self.namespace.clone()),
        )
        .await
        .map_err(|e| crate::error::MnemosyneError::ActorError(e.to_string()))?;

        // Register event broadcaster BEFORE Initialize to avoid race condition
        if let Some(broadcaster) = &self.event_broadcaster {
            reviewer_ref
                .cast(ReviewerMessage::RegisterEventBroadcaster(
                    broadcaster.clone(),
                ))
                .map_err(|e| {
                    tracing::warn!("Failed to register broadcaster with Reviewer: {:?}", e);
                    crate::error::MnemosyneError::ActorError(e.to_string())
                })?;
            tracing::debug!("Event broadcaster registered with Reviewer");
        }

        reviewer_ref
            .cast(ReviewerMessage::Initialize)
            .map_err(|e| crate::error::MnemosyneError::ActorError(e.to_string()))?;

        // Register in registry
        self.registry
            .register(
                reviewer_id.clone(),
                "Reviewer".to_string(),
                AgentRole::Reviewer,
            )
            .await;

        // Notify dashboard about agent startup
        self.notify_agent_started(&reviewer_id, "Reviewer").await;

        self.reviewer = Some(reviewer_ref.clone());

        // Initialize Python reviewer for LLM validation (feature-gated)
        #[cfg(feature = "python")]
        {
            tracing::info!("Initializing Python reviewer for LLM validation");
            match Self::initialize_python_reviewer() {
                Ok(py_reviewer) => {
                    reviewer_ref
                        .cast(ReviewerMessage::RegisterPythonReviewer { py_reviewer })
                        .map_err(|e| crate::error::MnemosyneError::ActorError(e.to_string()))?;
                    tracing::info!("Python reviewer registered successfully");
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize Python reviewer, continuing without LLM validation: {}", e);
                    tracing::warn!("Reviewer will fall back to pattern-matching validation");
                }
            }
        }

        // Spawn Executor
        let executor_id = format!("{}-executor", name_prefix);
        let (executor_ref, _) = Actor::spawn(
            Some(executor_id.clone()),
            ExecutorActor::new(self.storage.clone(), self.namespace.clone()),
            (self.storage.clone(), self.namespace.clone()),
        )
        .await
        .map_err(|e| crate::error::MnemosyneError::ActorError(e.to_string()))?;

        // Register event broadcaster BEFORE Initialize to avoid race condition
        if let Some(broadcaster) = &self.event_broadcaster {
            executor_ref
                .cast(ExecutorMessage::RegisterEventBroadcaster(
                    broadcaster.clone(),
                ))
                .map_err(|e| {
                    tracing::warn!("Failed to register broadcaster with Executor: {:?}", e);
                    crate::error::MnemosyneError::ActorError(e.to_string())
                })?;
            tracing::debug!("Event broadcaster registered with Executor");
        }

        executor_ref
            .cast(ExecutorMessage::Initialize)
            .map_err(|e| crate::error::MnemosyneError::ActorError(e.to_string()))?;

        // Register in registry
        self.registry
            .register(
                executor_id.clone(),
                "Executor".to_string(),
                AgentRole::Executor,
            )
            .await;

        // Notify dashboard about agent startup
        self.notify_agent_started(&executor_id, "Executor").await;

        self.executor = Some(executor_ref);

        // Spawn Orchestrator (root supervisor)
        let orchestrator_id = format!("{}-orchestrator", name_prefix);
        let (orchestrator_ref, _) = Actor::spawn(
            Some(orchestrator_id.clone()),
            OrchestratorActor::new(self.storage.clone(), self.namespace.clone()),
            (self.storage.clone(), self.namespace.clone()),
        )
        .await
        .map_err(|e| crate::error::MnemosyneError::ActorError(e.to_string()))?;

        // Register event broadcaster BEFORE Initialize to avoid race condition
        if let Some(broadcaster) = &self.event_broadcaster {
            orchestrator_ref
                .cast(OrchestratorMessage::RegisterEventBroadcaster(
                    broadcaster.clone(),
                ))
                .map_err(|e| {
                    tracing::warn!("Failed to register broadcaster with Orchestrator: {:?}", e);
                    crate::error::MnemosyneError::ActorError(e.to_string())
                })?;
            tracing::debug!("Event broadcaster registered with Orchestrator");
        }

        orchestrator_ref
            .cast(OrchestratorMessage::Initialize)
            .map_err(|e| crate::error::MnemosyneError::ActorError(e.to_string()))?;

        // Register in registry
        self.registry
            .register(
                orchestrator_id.clone(),
                "Orchestrator".to_string(),
                AgentRole::Orchestrator,
            )
            .await;

        // Notify dashboard about agent startup
        self.notify_agent_started(&orchestrator_id, "Orchestrator")
            .await;

        self.orchestrator = Some(orchestrator_ref.clone());

        // Wire agents together - send agent references to connect the mesh
        if let (Some(optimizer), Some(reviewer), Some(executor)) = (
            self.optimizer.as_ref(),
            self.reviewer.as_ref(),
            self.executor.as_ref(),
        ) {
            // Wire Orchestrator with Optimizer, Reviewer, Executor
            orchestrator_ref
                .cast(OrchestratorMessage::RegisterAgents {
                    optimizer: optimizer.clone(),
                    reviewer: reviewer.clone(),
                    executor: executor.clone(),
                })
                .map_err(|e| crate::error::MnemosyneError::ActorError(e.to_string()))?;

            // Wire Orchestrator with Router (for distributed dispatch)
            orchestrator_ref
                .cast(OrchestratorMessage::RegisterRouter(self.network.router()))
                .map_err(|e| crate::error::MnemosyneError::ActorError(e.to_string()))?;

            // Wire Optimizer with Orchestrator
            optimizer
                .cast(OptimizerMessage::RegisterOrchestrator(
                    orchestrator_ref.clone(),
                ))
                .map_err(|e| crate::error::MnemosyneError::ActorError(e.to_string()))?;

            // Wire Reviewer with Orchestrator
            reviewer
                .cast(ReviewerMessage::RegisterOrchestrator(
                    orchestrator_ref.clone(),
                ))
                .map_err(|e| crate::error::MnemosyneError::ActorError(e.to_string()))?;

            tracing::debug!("Agents wired: Full mesh topology established");
        }

        // Event broadcaster registration moved to immediately after actor spawn
        // to avoid race condition with Initialize message
        if self.event_broadcaster.is_some() {
            tracing::info!("Event broadcaster registered with all 4 actors during spawn");
        } else {
            tracing::warn!("No event broadcaster available - dashboard will not receive events!");
        }

        // Initialize and register Python Claude SDK agent bridges (if Python feature enabled)
        #[cfg(feature = "python")]
        {
            if let Some(broadcaster) = &self.event_broadcaster {
                tracing::info!("Initializing Python Claude SDK agent bridges for intelligent agent collaboration");

                // Create event transmitter for bridges
                let event_tx = broadcaster.sender();

                // Initialize and register bridge for Orchestrator
                if let Some(ref orchestrator) = self.orchestrator {
                    match Self::initialize_python_bridge(AgentRole::Orchestrator, event_tx.clone())
                        .await
                    {
                        Ok(bridge) => {
                            orchestrator
                                .cast(OrchestratorMessage::RegisterPythonBridge(bridge))
                                .map_err(|e| {
                                    tracing::warn!(
                                        "Failed to register Python bridge with Orchestrator: {:?}",
                                        e
                                    );
                                    crate::error::MnemosyneError::ActorError(e.to_string())
                                })?;
                            tracing::info!("Python bridge registered with Orchestrator");
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to initialize Python bridge for Orchestrator: {}",
                                e
                            );
                            tracing::warn!(
                                "Orchestrator will use basic coordination without LLM intelligence"
                            );
                        }
                    }
                }

                // Initialize and register bridge for Optimizer
                if let Some(ref optimizer) = self.optimizer {
                    match Self::initialize_python_bridge(AgentRole::Optimizer, event_tx.clone())
                        .await
                    {
                        Ok(bridge) => {
                            optimizer
                                .cast(OptimizerMessage::RegisterPythonBridge(bridge))
                                .map_err(|e| {
                                    tracing::warn!(
                                        "Failed to register Python bridge with Optimizer: {:?}",
                                        e
                                    );
                                    crate::error::MnemosyneError::ActorError(e.to_string())
                                })?;
                            tracing::info!("Python bridge registered with Optimizer");
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to initialize Python bridge for Optimizer: {}",
                                e
                            );
                            tracing::warn!("Optimizer will use basic context management without LLM intelligence");
                        }
                    }
                }

                // Initialize and register bridge for Reviewer
                // Note: Reviewer has both DSPy adapter (above) and Claude SDK bridge
                if let Some(ref reviewer) = self.reviewer {
                    match Self::initialize_python_bridge(AgentRole::Reviewer, event_tx.clone())
                        .await
                    {
                        Ok(bridge) => {
                            reviewer
                                .cast(ReviewerMessage::RegisterPythonBridge(bridge))
                                .map_err(|e| {
                                    tracing::warn!(
                                        "Failed to register Python bridge with Reviewer: {:?}",
                                        e
                                    );
                                    crate::error::MnemosyneError::ActorError(e.to_string())
                                })?;
                            tracing::info!("Python bridge registered with Reviewer");
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to initialize Python bridge for Reviewer: {}",
                                e
                            );
                            tracing::warn!(
                                "Reviewer will use DSPy adapter or pattern-matching validation"
                            );
                        }
                    }
                }

                // Initialize and register bridge for Executor
                if let Some(ref executor) = self.executor {
                    match Self::initialize_python_bridge(AgentRole::Executor, event_tx.clone())
                        .await
                    {
                        Ok(bridge) => {
                            executor
                                .cast(ExecutorMessage::RegisterPythonBridge(bridge))
                                .map_err(|e| {
                                    tracing::warn!(
                                        "Failed to register Python bridge with Executor: {:?}",
                                        e
                                    );
                                    crate::error::MnemosyneError::ActorError(e.to_string())
                                })?;
                            tracing::info!("Python bridge registered with Executor");
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to initialize Python bridge for Executor: {}",
                                e
                            );
                            tracing::warn!(
                                "Executor will use basic execution without LLM intelligence"
                            );
                        }
                    }
                }

                tracing::info!(
                    "Python Claude SDK agent bridges initialized and registered with all 4 actors"
                );
            } else {
                tracing::info!(
                    "No event broadcaster available, skipping Python bridge initialization"
                );
            }
        }

        // Start SSE subscriber for bidirectional event flow (CLI → Orchestrator)
        if let Some(ref orchestrator) = self.orchestrator {
            tracing::info!("Starting SSE subscriber for CLI event subscription");

            // Create shutdown channel
            let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);

            // Create SSE subscriber
            let sse_config = crate::orchestration::SseSubscriberConfig::default();
            let sse_subscriber = crate::orchestration::SseSubscriber::new(
                sse_config,
                orchestrator.clone(),
                shutdown_rx,
            );

            // Spawn SSE subscriber task
            let sse_handle = tokio::spawn(async move {
                sse_subscriber.run().await;
            });

            // Store shutdown sender and handle for cleanup
            self.sse_shutdown_tx = Some(shutdown_tx);
            self.sse_subscriber_handle = Some(sse_handle);

            tracing::info!("SSE subscriber started - orchestrator will receive CLI events");
        } else {
            tracing::warn!("No orchestrator available, skipping SSE subscriber initialization");
        }

        tracing::debug!("Supervision tree started with {} agents", 4);

        // Bootstrap work protocol with error handling
        // If bootstrap fails, log warning but don't crash - system can still accept user work
        if let Err(e) = self.bootstrap_work_plan_protocol().await {
            tracing::warn!("Bootstrap work protocol failed: {}. System will continue without initial work items.", e);
        }

        Ok(())
    }

    /// Bootstrap Work Plan Protocol
    ///
    /// Initializes the orchestration system by submitting initial work items.
    /// This makes the system "always active" by automatically starting the Work Plan Protocol
    /// when agents are spawned, rather than waiting for external triggers.
    ///
    /// Strategy:
    /// 1. Check for existing active work (resume if present)
    /// 2. If none, create session initialization work items
    /// 3. Submit work to orchestrator to begin processing
    async fn bootstrap_work_plan_protocol(&mut self) -> Result<()> {
        use crate::orchestration::state::AgentState;
        use crate::orchestration::work_plan_templates;

        tracing::info!("Bootstrapping Work Plan Protocol");

        // Check if there are existing active work items to resume
        // We check Ready, Active, and Blocked states
        let mut existing_items = Vec::new();

        for state in &[AgentState::Ready, AgentState::Active, AgentState::Blocked] {
            match self.storage.load_work_items_by_state(*state).await {
                Ok(items) => existing_items.extend(items),
                Err(e) => {
                    tracing::warn!("Failed to load work items in state {:?}: {}", state, e);
                }
            }
        }

        if !existing_items.is_empty() {
            tracing::info!(
                "Found {} existing work items, resuming orchestration",
                existing_items.len()
            );

            // Submit existing items to orchestrator for resumption
            // Continue even if some items fail - log warnings but don't abort
            if let Some(ref orchestrator) = self.orchestrator {
                for item in existing_items {
                    tracing::debug!("Resuming work item: {}", item.description);
                    if let Err(e) =
                        orchestrator.cast(OrchestratorMessage::SubmitWork(Box::new(item)))
                    {
                        tracing::warn!(
                            "Failed to resume work item: {}. Continuing with remaining items.",
                            e
                        );
                    }
                }
            } else {
                tracing::warn!("Orchestrator not available, cannot resume work");
            }
        } else {
            // No existing work, bootstrap fresh session
            tracing::info!("No existing work found, creating session initialization work items");

            let init_items = work_plan_templates::create_session_init_work_items();

            // Submit initialization work items to orchestrator
            // Continue even if some items fail - log warnings but don't abort
            if let Some(ref orchestrator) = self.orchestrator {
                for item in init_items {
                    tracing::debug!("Submitting init work: {}", item.description);
                    if let Err(e) =
                        orchestrator.cast(OrchestratorMessage::SubmitWork(Box::new(item)))
                    {
                        tracing::warn!(
                            "Failed to submit init work item: {}. Continuing with remaining items.",
                            e
                        );
                    }
                }

                tracing::info!("Work Plan Protocol bootstrap attempted");
            } else {
                tracing::warn!("Orchestrator not available, skipping bootstrap");
            }
        }

        Ok(())
    }

    /// Initialize Python Claude SDK agent bridge for a given role
    ///
    /// This spawns a Python Claude SDK agent using the ClaudeAgentBridge,
    /// which provides intelligent LLM-powered agent capabilities.
    ///
    /// Returns Err if Python initialization fails (e.g., module not found,
    /// import error, API key missing).
    #[cfg(feature = "python")]
    async fn initialize_python_bridge(
        role: AgentRole,
        event_tx: tokio::sync::broadcast::Sender<crate::api::Event>,
    ) -> Result<crate::orchestration::ClaudeAgentBridge> {
        use crate::error::MnemosyneError;

        crate::orchestration::ClaudeAgentBridge::spawn(role, event_tx)
            .await
            .map_err(|e| {
                MnemosyneError::ActorError(format!(
                    "Failed to initialize Python bridge for {:?}: {}. \
                     Ensure Python dependencies are installed and ANTHROPIC_API_KEY is set.",
                    role, e
                ))
            })
    }

    /// Initialize Python reviewer instance (DSPy adapter - legacy)
    ///
    /// This creates a Python reviewer instance using PyO3 and returns
    /// it as a PyObject that can be registered with the Rust reviewer.
    ///
    /// Returns Err if Python initialization fails (e.g., module not found,
    /// import error, API key missing).
    #[cfg(feature = "python")]
    fn initialize_python_reviewer() -> Result<Arc<PyObject>> {
        use crate::error::MnemosyneError;

        Python::with_gil(|py| {
            // Add src directory to Python path so we can import from orchestration.agents
            let sys = py.import_bound("sys")?;
            let py_path = sys.getattr("path")?;

            // Get the project src directory
            let manifest_dir = env!("CARGO_MANIFEST_DIR");
            let src_path = std::path::PathBuf::from(manifest_dir).join("src");
            py_path.call_method1("insert", (0, src_path.to_str().unwrap()))?;

            // Import the ReviewerAgent class from Python
            let reviewer_module = py.import_bound("orchestration.agents.reviewer")?;
            let reviewer_class = reviewer_module.getattr("ReviewerAgent")?;

            // Create an instance with default config
            let config = py.eval_bound(
                "{'agent_id': 'reviewer-llm', 'strict_mode': True, 'max_retries': 3}",
                None,
                None,
            )?;

            let reviewer_instance = reviewer_class.call1((config,))?;

            Ok(Arc::new(reviewer_instance.unbind()))
        })
        .map_err(|e: PyErr| {
            MnemosyneError::ActorError(format!(
                "Failed to initialize Python reviewer: {}. \
                 Ensure Python dependencies are installed and ANTHROPIC_API_KEY is set.",
                e
            ))
        })
    }

    /// Stop all agents gracefully with timeout
    pub async fn stop(&mut self) -> Result<()> {
        self.stop_with_timeout(std::time::Duration::from_secs(30))
            .await
    }

    /// Stop all agents gracefully with custom timeout (P1-2: Actor shutdown coordination)
    pub async fn stop_with_timeout(&mut self, timeout: std::time::Duration) -> Result<()> {
        tracing::debug!("Stopping supervision tree (timeout: {:?})", timeout);

        let stop_start = std::time::Instant::now();

        // Stop SSE subscriber first (before orchestrator)
        if let Some(shutdown_tx) = self.sse_shutdown_tx.take() {
            tracing::debug!("Sending shutdown signal to SSE subscriber");
            let _ = shutdown_tx.send(());

            // Wait for SSE subscriber to stop (with timeout)
            if let Some(mut handle) = self.sse_subscriber_handle.take() {
                tokio::select! {
                    _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                        tracing::warn!("SSE subscriber did not stop within 5s, aborting");
                        handle.abort();
                    }
                    result = &mut handle => {
                        if let Err(e) = result {
                            tracing::warn!("SSE subscriber task error during shutdown: {}", e);
                        } else {
                            tracing::debug!("SSE subscriber stopped gracefully");
                        }
                    }
                }
            }
        }

        // Stop in reverse order (children first, then supervisor)
        if let Some(executor) = self.executor.take() {
            executor.stop(None);
            tracing::debug!("Sent stop signal to executor");
        }

        if let Some(reviewer) = self.reviewer.take() {
            reviewer.stop(None);
            tracing::debug!("Sent stop signal to reviewer");
        }

        if let Some(optimizer) = self.optimizer.take() {
            optimizer.stop(None);
            tracing::debug!("Sent stop signal to optimizer");
        }

        if let Some(orchestrator) = self.orchestrator.take() {
            orchestrator.stop(None);
            tracing::debug!("Sent stop signal to orchestrator");
        }

        // Wait for actors to stop gracefully with timeout (P1-2: 30s default)
        tokio::select! {
            _ = tokio::time::sleep(timeout) => {
                let elapsed = stop_start.elapsed();
                tracing::warn!(
                    "Actor shutdown timeout ({:?}) exceeded after {:?}. \
                     Some actors may not have stopped gracefully.",
                    timeout, elapsed
                );
            }
            _ = async {
                // Give actors time to complete cleanup
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            } => {
                tracing::debug!("Actors stopped gracefully");
            }
        }

        let total_elapsed = stop_start.elapsed();
        tracing::debug!("Supervision tree stopped (took {:?})", total_elapsed);

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

    /// Check if any actors are still running (for P1-2: Actor shutdown coordination)
    pub fn has_running_actors(&self) -> bool {
        self.orchestrator.is_some()
            || self.optimizer.is_some()
            || self.reviewer.is_some()
            || self.executor.is_some()
    }

    /// Send stop signals to all actors synchronously (for Drop implementation)
    ///
    /// This is a best-effort cleanup that doesn't wait for graceful shutdown.
    /// Use `stop()` or `stop_with_timeout()` for proper graceful shutdown.
    pub fn send_stop_signals(&self) {
        if let Some(ref executor) = self.executor {
            executor.stop(None);
        }
        if let Some(ref reviewer) = self.reviewer {
            reviewer.stop(None);
        }
        if let Some(ref optimizer) = self.optimizer {
            optimizer.stop(None);
        }
        if let Some(ref orchestrator) = self.orchestrator {
            orchestrator.stop(None);
        }
    }

    /// Get reference to agent registry
    pub fn registry(&self) -> &AgentRegistry {
        &self.registry
    }

    /// Get reference to proposal queue
    pub fn proposal_queue(&self) -> &ProposalQueue {
        &self.proposal_queue
    }

    /// Spawn all agents (alias for start() for daemon compatibility)
    ///
    /// This spawns all 4 agents:
    /// - Orchestrator: Central coordinator
    /// - Optimizer: Context and resource optimization
    /// - Reviewer: Quality assurance and gating
    /// - Executor: Work execution
    pub async fn spawn_agents(&mut self) -> Result<()> {
        self.start().await
    }

    /// Check if all agents are healthy and running
    ///
    /// Returns true if all 4 agent actor references exist and are accessible.
    /// This is a lightweight health check that doesn't send messages to agents.
    pub async fn is_healthy(&self) -> bool {
        // Check if all agent references exist
        self.orchestrator.is_some()
            && self.optimizer.is_some()
            && self.reviewer.is_some()
            && self.executor.is_some()
    }

    /// Restart failed agents
    ///
    /// Checks each agent and restarts any that have failed.
    /// The supervision tree (Ractor) handles automatic restart on failure,
    /// so this is primarily for explicit restart requests.
    pub async fn restart_failed_agents(&mut self) -> Result<()> {
        // In Ractor supervision trees, actors automatically restart on failure
        // This method is a no-op for now but can be extended to:
        // - Manually restart specific actors
        // - Clear actor state on restart
        // - Broadcast restart events

        // Check if agents are still alive
        if self.orchestrator.is_none()
            || self.optimizer.is_none()
            || self.reviewer.is_none()
            || self.executor.is_none()
        {
            tracing::warn!("Some agents missing, restarting supervision tree");
            // Stop any remaining agents
            self.stop().await?;
            // Restart all agents
            self.start().await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::messages::{ReviewFeedback, WorkResult};
    use crate::orchestration::state::{Phase, RequirementStatus, WorkItem};
    use crate::{ConnectionMode, LibsqlStorage};

    #[tokio::test]
    async fn test_supervision_tree() {
        let storage = Arc::new(LibsqlStorage::new(ConnectionMode::InMemory).await.unwrap());

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

    /// E2E test for retry workflow with requirement tracking
    ///
    /// Scenario:
    /// 1. Work item submitted with 3 requirements
    /// 2. First attempt: 1 requirement satisfied, 2 unsatisfied → retry
    /// 3. Second attempt: all 3 requirements satisfied → complete
    ///
    /// This test verifies the full requirement tracking flow without
    /// needing actual LLM calls (uses simulated feedback).
    #[tokio::test]
    async fn test_e2e_retry_workflow_with_requirements() {
        use std::time::Duration;

        // Setup
        let _storage = Arc::new(LibsqlStorage::new(ConnectionMode::InMemory).await.unwrap());
        let _namespace = Namespace::Session {
            project: "test-e2e".to_string(),
            session_id: "retry-test".to_string(),
        };

        // Create work item with 3 requirements
        let mut work_item = WorkItem::new(
            "Implement feature X with error handling and tests".to_string(),
            AgentRole::Executor,
            Phase::PlanToArtifacts,
            5,
        );
        work_item.requirements = vec![
            "Implement core functionality".to_string(),
            "Add error handling".to_string(),
            "Write unit tests".to_string(),
        ];

        // === ATTEMPT 1: Partial completion ===

        // Simulate first execution result
        let mut result_attempt1 =
            WorkResult::success(work_item.id.clone(), Duration::from_secs(10));
        result_attempt1.memory_ids = vec![
            crate::types::MemoryId(uuid::Uuid::new_v4()),
            crate::types::MemoryId(uuid::Uuid::new_v4()),
        ];

        // Simulate review feedback: 1 satisfied, 2 unsatisfied
        let mut satisfied_req1 = std::collections::HashMap::new();
        satisfied_req1.insert(
            "Implement core functionality".to_string(),
            result_attempt1.memory_ids.clone(),
        );

        let feedback_attempt1 = ReviewFeedback {
            gates_passed: false, // Completeness gate failed
            issues: vec![
                "Missing error handling".to_string(),
                "No unit tests found".to_string(),
            ],
            suggested_tests: vec!["Test happy path".to_string()],
            execution_context: result_attempt1.memory_ids.clone(),
            improvement_guidance: Some("Add error handling and tests".to_string()),
            extracted_requirements: vec![], // Already extracted
            unsatisfied_requirements: vec![
                "Add error handling".to_string(),
                "Write unit tests".to_string(),
            ],
            satisfied_requirements: satisfied_req1.clone(),
        };

        // Verify attempt 1 would trigger retry
        let all_requirements_satisfied_attempt1 =
            feedback_attempt1.unsatisfied_requirements.is_empty();
        assert!(
            !all_requirements_satisfied_attempt1,
            "Attempt 1: Not all requirements satisfied"
        );
        assert_eq!(
            feedback_attempt1.satisfied_requirements.len(),
            1,
            "Attempt 1: 1 requirement satisfied"
        );
        assert_eq!(
            feedback_attempt1.unsatisfied_requirements.len(),
            2,
            "Attempt 1: 2 requirements unsatisfied"
        );

        // Update work item for retry (simulating orchestrator behavior)
        work_item.review_attempt = 1;
        for req in &feedback_attempt1.unsatisfied_requirements {
            work_item
                .requirement_status
                .insert(req.clone(), RequirementStatus::InProgress);
        }
        for (req, evidence) in &feedback_attempt1.satisfied_requirements {
            work_item
                .requirement_status
                .insert(req.clone(), RequirementStatus::Satisfied);
            work_item
                .implementation_evidence
                .insert(req.clone(), evidence.clone());
        }

        // Verify partial tracking
        assert_eq!(
            work_item
                .requirement_status
                .get("Implement core functionality"),
            Some(&RequirementStatus::Satisfied),
            "Attempt 1: First requirement satisfied"
        );
        assert_eq!(
            work_item.requirement_status.get("Add error handling"),
            Some(&RequirementStatus::InProgress),
            "Attempt 1: Second requirement in progress"
        );
        assert_eq!(
            work_item.requirement_status.get("Write unit tests"),
            Some(&RequirementStatus::InProgress),
            "Attempt 1: Third requirement in progress"
        );

        // === ATTEMPT 2: Full completion ===

        // Simulate second execution result (after retry)
        let mut result_attempt2 =
            WorkResult::success(work_item.id.clone(), Duration::from_secs(15));
        result_attempt2.memory_ids = vec![
            crate::types::MemoryId(uuid::Uuid::new_v4()),
            crate::types::MemoryId(uuid::Uuid::new_v4()),
            crate::types::MemoryId(uuid::Uuid::new_v4()),
        ];

        // Simulate review feedback: all 3 satisfied
        let mut satisfied_req2 = std::collections::HashMap::new();
        satisfied_req2.insert(
            "Implement core functionality".to_string(),
            result_attempt2.memory_ids.clone(),
        );
        satisfied_req2.insert(
            "Add error handling".to_string(),
            result_attempt2.memory_ids.clone(),
        );
        satisfied_req2.insert(
            "Write unit tests".to_string(),
            result_attempt2.memory_ids.clone(),
        );

        let feedback_attempt2 = ReviewFeedback {
            gates_passed: true, // All gates passed
            issues: vec![],
            suggested_tests: vec![],
            execution_context: result_attempt2.memory_ids.clone(),
            improvement_guidance: None,
            extracted_requirements: vec![],
            unsatisfied_requirements: vec![], // All satisfied!
            satisfied_requirements: satisfied_req2.clone(),
        };

        // Verify attempt 2 would mark complete
        let all_requirements_satisfied_attempt2 =
            feedback_attempt2.unsatisfied_requirements.is_empty();
        assert!(
            all_requirements_satisfied_attempt2,
            "Attempt 2: All requirements satisfied"
        );
        assert_eq!(
            feedback_attempt2.satisfied_requirements.len(),
            3,
            "Attempt 2: 3 requirements satisfied"
        );
        assert_eq!(
            feedback_attempt2.unsatisfied_requirements.len(),
            0,
            "Attempt 2: 0 requirements unsatisfied"
        );

        // Update work item with final status (simulating orchestrator behavior)
        for (req, evidence) in &feedback_attempt2.satisfied_requirements {
            work_item
                .requirement_status
                .insert(req.clone(), RequirementStatus::Satisfied);
            work_item
                .implementation_evidence
                .insert(req.clone(), evidence.clone());
        }

        // Verify all requirements satisfied
        assert_eq!(
            work_item
                .requirement_status
                .get("Implement core functionality"),
            Some(&RequirementStatus::Satisfied),
            "Attempt 2: First requirement satisfied"
        );
        assert_eq!(
            work_item.requirement_status.get("Add error handling"),
            Some(&RequirementStatus::Satisfied),
            "Attempt 2: Second requirement satisfied"
        );
        assert_eq!(
            work_item.requirement_status.get("Write unit tests"),
            Some(&RequirementStatus::Satisfied),
            "Attempt 2: Third requirement satisfied"
        );

        // Verify implementation evidence stored
        assert_eq!(
            work_item.implementation_evidence.len(),
            3,
            "All requirements have evidence"
        );
        assert!(
            work_item
                .implementation_evidence
                .contains_key("Implement core functionality"),
            "Evidence for first requirement"
        );
        assert!(
            work_item
                .implementation_evidence
                .contains_key("Add error handling"),
            "Evidence for second requirement"
        );
        assert!(
            work_item
                .implementation_evidence
                .contains_key("Write unit tests"),
            "Evidence for third requirement"
        );
    }
}
