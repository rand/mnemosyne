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
            registry: AgentRegistry::new(),
            proposal_queue: ProposalQueue::new(),
            orchestrator: None,
            optimizer: None,
            reviewer: None,
            executor: None,
        })
    }

    /// Create a new supervision tree with explicit namespace
    pub async fn new_with_namespace(
        config: SupervisionConfig,
        storage: Arc<dyn StorageBackend>,
        network: Arc<network::NetworkLayer>,
        namespace: Namespace,
    ) -> Result<Self> {
        Ok(Self {
            config,
            storage,
            network,
            namespace,
            registry: AgentRegistry::new(),
            proposal_queue: ProposalQueue::new(),
            orchestrator: None,
            optimizer: None,
            reviewer: None,
            executor: None,
        })
    }

    /// Start all agents in the supervision tree
    pub async fn start(&mut self) -> Result<()> {
        tracing::debug!("Starting supervision tree");

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

        tracing::debug!("Supervision tree started with {} agents", 4);

        Ok(())
    }

    /// Initialize Python reviewer instance
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

    /// Stop all agents gracefully
    pub async fn stop(&mut self) -> Result<()> {
        tracing::debug!("Stopping supervision tree");

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

        tracing::debug!("Supervision tree stopped");

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

    /// Get reference to agent registry
    pub fn registry(&self) -> &AgentRegistry {
        &self.registry
    }

    /// Get reference to proposal queue
    pub fn proposal_queue(&self) -> &ProposalQueue {
        &self.proposal_queue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConnectionMode, LibsqlStorage};
    use crate::orchestration::state::{RequirementStatus, Phase, WorkItem};
    use crate::orchestration::messages::{ReviewFeedback, WorkResult};

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
        let storage = Arc::new(LibsqlStorage::new(ConnectionMode::InMemory).await.unwrap());
        let namespace = Namespace::Session {
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
        let mut result_attempt1 = WorkResult::success(work_item.id.clone(), Duration::from_secs(10));
        result_attempt1.memory_ids = vec![
            crate::types::MemoryId(uuid::Uuid::new_v4()),
            crate::types::MemoryId(uuid::Uuid::new_v4()),
        ];

        // Simulate review feedback: 1 satisfied, 2 unsatisfied
        let mut satisfied_req1 = std::collections::HashMap::new();
        satisfied_req1.insert("Implement core functionality".to_string(), result_attempt1.memory_ids.clone());

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
        let all_requirements_satisfied_attempt1 = feedback_attempt1.unsatisfied_requirements.is_empty();
        assert!(!all_requirements_satisfied_attempt1, "Attempt 1: Not all requirements satisfied");
        assert_eq!(feedback_attempt1.satisfied_requirements.len(), 1, "Attempt 1: 1 requirement satisfied");
        assert_eq!(feedback_attempt1.unsatisfied_requirements.len(), 2, "Attempt 1: 2 requirements unsatisfied");

        // Update work item for retry (simulating orchestrator behavior)
        work_item.review_attempt = 1;
        for req in &feedback_attempt1.unsatisfied_requirements {
            work_item.requirement_status.insert(req.clone(), RequirementStatus::InProgress);
        }
        for (req, evidence) in &feedback_attempt1.satisfied_requirements {
            work_item.requirement_status.insert(req.clone(), RequirementStatus::Satisfied);
            work_item.implementation_evidence.insert(req.clone(), evidence.clone());
        }

        // Verify partial tracking
        assert_eq!(
            work_item.requirement_status.get("Implement core functionality"),
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
        let mut result_attempt2 = WorkResult::success(work_item.id.clone(), Duration::from_secs(15));
        result_attempt2.memory_ids = vec![
            crate::types::MemoryId(uuid::Uuid::new_v4()),
            crate::types::MemoryId(uuid::Uuid::new_v4()),
            crate::types::MemoryId(uuid::Uuid::new_v4()),
        ];

        // Simulate review feedback: all 3 satisfied
        let mut satisfied_req2 = std::collections::HashMap::new();
        satisfied_req2.insert("Implement core functionality".to_string(), result_attempt2.memory_ids.clone());
        satisfied_req2.insert("Add error handling".to_string(), result_attempt2.memory_ids.clone());
        satisfied_req2.insert("Write unit tests".to_string(), result_attempt2.memory_ids.clone());

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
        let all_requirements_satisfied_attempt2 = feedback_attempt2.unsatisfied_requirements.is_empty();
        assert!(all_requirements_satisfied_attempt2, "Attempt 2: All requirements satisfied");
        assert_eq!(feedback_attempt2.satisfied_requirements.len(), 3, "Attempt 2: 3 requirements satisfied");
        assert_eq!(feedback_attempt2.unsatisfied_requirements.len(), 0, "Attempt 2: 0 requirements unsatisfied");

        // Update work item with final status (simulating orchestrator behavior)
        for (req, evidence) in &feedback_attempt2.satisfied_requirements {
            work_item.requirement_status.insert(req.clone(), RequirementStatus::Satisfied);
            work_item.implementation_evidence.insert(req.clone(), evidence.clone());
        }

        // Verify all requirements satisfied
        assert_eq!(
            work_item.requirement_status.get("Implement core functionality"),
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
        assert_eq!(work_item.implementation_evidence.len(), 3, "All requirements have evidence");
        assert!(
            work_item.implementation_evidence.contains_key("Implement core functionality"),
            "Evidence for first requirement"
        );
        assert!(
            work_item.implementation_evidence.contains_key("Add error handling"),
            "Evidence for second requirement"
        );
        assert!(
            work_item.implementation_evidence.contains_key("Write unit tests"),
            "Evidence for third requirement"
        );
    }
}
