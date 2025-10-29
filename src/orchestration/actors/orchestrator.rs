//! Orchestrator Actor
//!
//! Central coordinator managing:
//! - Work queue with dependency-aware scheduling
//! - Agent coordination and handoffs
//! - Deadlock detection (60s timeout)
//! - Phase transitions (Work Plan Protocol)
//! - Context preservation triggers (75% threshold)

use crate::error::Result;
use crate::launcher::agents::AgentRole;
use crate::orchestration::events::{AgentEvent, EventPersistence};
use crate::orchestration::messages::{
    ExecutorMessage, OptimizerMessage, OrchestratorMessage, ReviewerMessage, WorkResult,
};
use crate::orchestration::state::{AgentState, Phase, SharedWorkQueue, WorkItem, WorkQueue};
use crate::storage::StorageBackend;
use crate::types::Namespace;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Orchestrator actor state
pub struct OrchestratorState {
    /// Work queue
    work_queue: SharedWorkQueue,

    /// Event persistence layer
    events: EventPersistence,

    /// Reference to Optimizer actor
    optimizer: Option<ActorRef<OptimizerMessage>>,

    /// Reference to Reviewer actor
    reviewer: Option<ActorRef<ReviewerMessage>>,

    /// Reference to Executor actor
    executor: Option<ActorRef<ExecutorMessage>>,

    /// Context usage percentage
    context_usage_pct: f32,

    /// Deadlock check interval
    deadlock_check_interval: Duration,
}

impl OrchestratorState {
    pub fn new(storage: Arc<dyn StorageBackend>, namespace: Namespace) -> Self {
        Self {
            work_queue: Arc::new(RwLock::new(WorkQueue::new())),
            events: EventPersistence::new(storage, namespace),
            optimizer: None,
            reviewer: None,
            executor: None,
            context_usage_pct: 0.0,
            deadlock_check_interval: Duration::from_secs(10),
        }
    }

    /// Register agent references
    pub fn register_agents(
        &mut self,
        optimizer: ActorRef<OptimizerMessage>,
        reviewer: ActorRef<ReviewerMessage>,
        executor: ActorRef<ExecutorMessage>,
    ) {
        self.optimizer = Some(optimizer);
        self.reviewer = Some(reviewer);
        self.executor = Some(executor);
    }
}

/// Orchestrator actor implementation
pub struct OrchestratorActor {
    storage: Arc<dyn StorageBackend>,
    namespace: Namespace,
}

impl OrchestratorActor {
    pub fn new(storage: Arc<dyn StorageBackend>, namespace: Namespace) -> Self {
        Self { storage, namespace }
    }

    /// Handle work submission
    async fn handle_submit_work(
        state: &mut OrchestratorState,
        item: WorkItem,
    ) -> Result<()> {
        tracing::info!("Submitting work: {}", item.description);

        // Add to work queue
        let item_id = item.id.clone();
        let agent = item.agent;
        let phase = item.phase;
        let description = item.description.clone();

        {
            let mut queue = state.work_queue.write().await;
            queue.add(item);
        }

        // Persist event
        state
            .events
            .persist(AgentEvent::WorkItemAssigned {
                agent,
                item_id,
                description,
                phase,
            })
            .await?;

        // Dispatch to appropriate agent
        Self::dispatch_work(state).await?;

        Ok(())
    }

    /// Dispatch ready work items to agents
    async fn dispatch_work(state: &mut OrchestratorState) -> Result<()> {
        let queue = state.work_queue.read().await;
        let ready_items = queue.get_ready_items();

        for item in ready_items {
            let item = item.clone();

            // Send to appropriate agent
            match item.agent {
                AgentRole::Executor => {
                    if let Some(ref executor) = state.executor {
                        let _ = executor
                            .cast(ExecutorMessage::ExecuteWork(item))
                            .map_err(|e| tracing::warn!("Failed to cast to executor: {:?}", e));
                    }
                }
                AgentRole::Optimizer => {
                    if let Some(ref optimizer) = state.optimizer {
                        let _ = optimizer
                            .cast(OptimizerMessage::LoadContextMemories {
                                work_item_id: item.id.clone(),
                                query: item.description.clone(),
                            })
                            .map_err(|e| tracing::warn!("Failed to cast to optimizer: {:?}", e));
                    }
                }
                AgentRole::Reviewer => {
                    // Reviewer doesn't execute work directly
                    tracing::debug!("Reviewer work items handled via validation");
                }
                AgentRole::Orchestrator => {
                    tracing::debug!("Orchestrator self-work (internal)");
                }
            }
        }

        Ok(())
    }

    /// Handle work completion
    async fn handle_work_completed(
        state: &mut OrchestratorState,
        item_id: crate::orchestration::state::WorkItemId,
        result: WorkResult,
    ) -> Result<()> {
        tracing::info!("Work completed: {:?}", item_id);

        // Mark as completed in queue
        {
            let mut queue = state.work_queue.write().await;
            queue.mark_completed(&item_id);
        }

        // Persist event
        state
            .events
            .persist(AgentEvent::WorkItemCompleted {
                agent: AgentRole::Executor, // TODO: Track actual agent
                item_id,
                duration_ms: result.duration.as_millis() as u64,
                memory_ids: result.memory_ids,
            })
            .await?;

        // Dispatch next items
        Self::dispatch_work(state).await?;

        Ok(())
    }

    /// Handle work failure
    async fn handle_work_failed(
        state: &mut OrchestratorState,
        item_id: crate::orchestration::state::WorkItemId,
        error: String,
    ) -> Result<()> {
        tracing::warn!("Work failed: {:?} - {}", item_id, error);

        // Update item state
        {
            let mut queue = state.work_queue.write().await;
            if let Some(item) = queue.get_mut(&item_id) {
                item.transition(AgentState::Error);
                item.error = Some(error.clone());
            }
        }

        // Persist event
        state
            .events
            .persist(AgentEvent::WorkItemFailed {
                agent: AgentRole::Executor,
                item_id,
                error,
            })
            .await?;

        Ok(())
    }

    /// Check for deadlocks
    async fn check_deadlocks(state: &mut OrchestratorState) -> Result<()> {
        let queue = state.work_queue.read().await;
        let deadlocked = queue.detect_deadlocks();

        if !deadlocked.is_empty() {
            tracing::warn!("Deadlock detected: {} items", deadlocked.len());

            // Persist event
            state
                .events
                .persist(AgentEvent::DeadlockDetected {
                    blocked_items: deadlocked.clone(),
                    detected_at: chrono::Utc::now(),
                })
                .await?;

            // TODO: Implement deadlock resolution strategy
            // For now, just log and continue
        }

        Ok(())
    }

    /// Handle phase transition
    async fn handle_phase_transition(
        state: &mut OrchestratorState,
        from: Phase,
        to: Phase,
    ) -> Result<()> {
        tracing::info!("Phase transition: {:?} → {:?}", from, to);

        // Validate transition with Reviewer
        if let Some(ref reviewer) = state.reviewer {
            let _ = reviewer
                .cast(ReviewerMessage::ValidatePhaseTransition { from, to })
                .map_err(|e| tracing::warn!("Failed to validate phase transition: {:?}", e));
        }

        // Update work queue phase
        {
            let mut queue = state.work_queue.write().await;
            queue.transition_phase(to)
                .map_err(|e| crate::error::MnemosyneError::Other(e))?;
        }

        // Persist event
        state
            .events
            .persist(AgentEvent::PhaseTransition {
                from,
                to,
                approved_by: AgentRole::Reviewer,
            })
            .await?;

        Ok(())
    }

    /// Handle context threshold reached
    async fn handle_context_threshold(
        state: &mut OrchestratorState,
        current_pct: f32,
    ) -> Result<()> {
        tracing::warn!(
            "Context threshold reached: {:.1}%",
            current_pct * 100.0
        );

        state.context_usage_pct = current_pct;

        // Trigger optimizer to checkpoint context
        if let Some(ref optimizer) = state.optimizer {
            let _ = optimizer
                .cast(OptimizerMessage::CheckpointContext {
                    reason: format!("Context usage at {:.1}%", current_pct * 100.0),
                })
                .map_err(|e| tracing::warn!("Failed to trigger checkpoint: {:?}", e));
        }

        Ok(())
    }
}

#[ractor::async_trait]
impl Actor for OrchestratorActor {
    type Msg = OrchestratorMessage;
    type State = OrchestratorState;
    type Arguments = (Arc<dyn StorageBackend>, Namespace);

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> std::result::Result<Self::State, ActorProcessingErr> {
        tracing::info!("Orchestrator actor starting");
        let (storage, namespace) = args;
        Ok(OrchestratorState::new(storage, namespace))
    }

    async fn post_start(
        &self,
        myself: ActorRef<Self::Msg>,
        state: &mut Self::State,
    ) -> std::result::Result<(), ActorProcessingErr> {
        tracing::info!("Orchestrator actor started: {:?}", myself.get_id());

        // Start periodic deadlock checker
        let myself_clone = myself.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                let _ = myself_clone.cast(OrchestratorMessage::GetReadyWork);
            }
        });

        Ok(())
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> std::result::Result<(), ActorProcessingErr> {
        match message {
            OrchestratorMessage::Initialize => {
                tracing::info!("Orchestrator initialized");
            }
            OrchestratorMessage::SubmitWork(item) => {
                Self::handle_submit_work(state, item)
                    .await
                    .map_err(|e| ActorProcessingErr::from(e.to_string()))?;
            }
            OrchestratorMessage::WorkCompleted { item_id, result } => {
                Self::handle_work_completed(state, item_id, result)
                    .await
                    .map_err(|e| ActorProcessingErr::from(e.to_string()))?;
            }
            OrchestratorMessage::WorkFailed { item_id, error } => {
                Self::handle_work_failed(state, item_id, error)
                    .await
                    .map_err(|e| ActorProcessingErr::from(e.to_string()))?;
            }
            OrchestratorMessage::GetReadyWork => {
                Self::check_deadlocks(state)
                    .await
                    .map_err(|e| ActorProcessingErr::from(e.to_string()))?;
                Self::dispatch_work(state)
                    .await
                    .map_err(|e| ActorProcessingErr::from(e.to_string()))?;
            }
            OrchestratorMessage::DeadlockDetected { .. } => {
                // Already detected in check_deadlocks
                tracing::debug!("Deadlock notification received");
            }
            OrchestratorMessage::ContextThresholdReached { current_pct } => {
                Self::handle_context_threshold(state, current_pct)
                    .await
                    .map_err(|e| ActorProcessingErr::from(e.to_string()))?;
            }
            OrchestratorMessage::PhaseTransition { from, to } => {
                Self::handle_phase_transition(state, from, to)
                    .await
                    .map_err(|e| ActorProcessingErr::from(e.to_string()))?;
            }
        }

        Ok(())
    }

    async fn post_stop(
        &self,
        _myself: ActorRef<Self::Msg>,
        _state: &mut Self::State,
    ) -> std::result::Result<(), ActorProcessingErr> {
        tracing::info!("Orchestrator actor stopped");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LibsqlStorage;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_orchestrator_lifecycle() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let storage = Arc::new(
            LibsqlStorage::new_with_validation(
                crate::ConnectionMode::Local(db_path.to_str().unwrap().to_string()),
                true, // create_if_missing
            )
            .await
            .expect("Failed to create test storage"),
        );

        let namespace = Namespace::Session {
            project: "test".to_string(),
            session_id: "test-session".to_string(),
        };

        let (actor_ref, _handle) = Actor::spawn(
            None,
            OrchestratorActor::new(storage.clone(), namespace.clone()),
            (storage, namespace),
        )
        .await
        .unwrap();

        // Test initialization
        actor_ref.cast(OrchestratorMessage::Initialize).unwrap();

        // Stop actor
        actor_ref.stop(None);

        // Wait for actor to stop
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
