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
use crate::orchestration::state::{
    AgentState, Phase, SharedWorkQueue, WorkItem, WorkItemId, WorkQueue,
};
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

    /// Deadlock check interval (WIP)
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    storage: Arc<dyn StorageBackend>,
    #[allow(dead_code)]
    namespace: Namespace,
}

impl OrchestratorActor {
    pub fn new(storage: Arc<dyn StorageBackend>, namespace: Namespace) -> Self {
        Self { storage, namespace }
    }

    /// Handle work submission
    async fn handle_submit_work(state: &mut OrchestratorState, item: WorkItem) -> Result<()> {
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

    /// Handle work completion - transitions to PendingReview
    async fn handle_work_completed(
        state: &mut OrchestratorState,
        item_id: crate::orchestration::state::WorkItemId,
        result: WorkResult,
    ) -> Result<()> {
        tracing::info!("Work completed, sending for review: {:?}", item_id);

        // Get work item and update execution memories
        let work_item = {
            let mut queue = state.work_queue.write().await;
            if let Some(item) = queue.get_mut(&item_id) {
                // Transition to PendingReview
                item.transition(AgentState::PendingReview);

                // Store execution memory IDs for context consolidation
                item.execution_memory_ids = result.memory_ids.clone();

                Some(item.clone())
            } else {
                None
            }
        };

        if let Some(work_item) = work_item {
            // Send to Reviewer
            if let Some(ref reviewer) = state.reviewer {
                reviewer
                    .cast(ReviewerMessage::ReviewWork {
                        item_id: item_id.clone(),
                        result: result.clone(),
                        work_item,
                    })
                    .map_err(|e| {
                        tracing::error!("Failed to send work to Reviewer: {:?}", e);
                        crate::error::MnemosyneError::Other(format!(
                            "Failed to send work to Reviewer: {:?}",
                            e
                        ))
                    })?;

                tracing::info!("Work sent to Reviewer for quality gates: {:?}", item_id);
            } else {
                tracing::warn!("No reviewer available, marking as completed");

                // Fallback: mark as completed if no reviewer
                let mut queue = state.work_queue.write().await;
                queue.mark_completed(&item_id);
            }
        } else {
            tracing::warn!("Work item not found: {:?}", item_id);
        }

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
        let deadlocked = {
            let queue = state.work_queue.read().await;
            queue.detect_deadlocks()
        };

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

            // Resolve deadlock using priority-based preemption
            Self::resolve_deadlock(state, deadlocked).await?;
        }

        Ok(())
    }

    /// Resolve deadlock using priority-based preemption
    ///
    /// Strategy:
    /// 1. Sort deadlocked items by priority (lowest first)
    /// 2. Cancel lowest-priority items until deadlock is broken
    /// 3. Reset canceled items to Ready state for retry
    /// 4. Notify affected agents
    async fn resolve_deadlock(
        state: &mut OrchestratorState,
        deadlocked_ids: Vec<WorkItemId>,
    ) -> Result<()> {
        tracing::info!("Resolving deadlock for {} items", deadlocked_ids.len());

        // Get items with their priorities for sorting
        let mut deadlocked_items: Vec<(WorkItemId, u8, String)> = {
            let queue = state.work_queue.read().await;
            deadlocked_ids
                .iter()
                .filter_map(|id| {
                    queue
                        .get(id)
                        .map(|item| (id.clone(), item.priority, item.description.clone()))
                })
                .collect()
        };

        // Sort by priority (lowest first) - these will be preempted
        deadlocked_items.sort_by_key(|(_, priority, _)| *priority);

        // Cancel lower-priority items (bottom 50%)
        let cancel_count = deadlocked_items.len().div_ceil(2);
        let to_cancel: Vec<_> = deadlocked_items
            .iter()
            .take(cancel_count)
            .cloned()
            .collect();

        tracing::info!(
            "Preempting {} lower-priority items out of {}",
            to_cancel.len(),
            deadlocked_items.len()
        );

        // Cancel and reset items
        let mut preempted_ids = Vec::new();
        {
            let mut queue = state.work_queue.write().await;
            for (id, priority, description) in to_cancel {
                if let Some(item) = queue.get_mut(&id) {
                    tracing::info!(
                        "Preempting item {} (priority {}): {}",
                        id,
                        priority,
                        description
                    );

                    // Reset to Ready state for retry
                    item.transition(AgentState::Ready);
                    item.started_at = None;
                    item.error = Some(format!("Preempted due to deadlock (priority {})", priority));

                    preempted_ids.push(id);
                }
            }
        }

        // Persist resolution event
        state
            .events
            .persist(AgentEvent::DeadlockResolved {
                blocked_items: preempted_ids,
                resolution: format!("Preempted {} lower-priority items", cancel_count),
            })
            .await?;

        tracing::info!("Deadlock resolved via priority-based preemption");

        // Items reset to Ready state will be picked up by normal work assignment
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
            queue
                .transition_phase(to)
                .map_err(crate::error::MnemosyneError::Other)?;
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
        tracing::warn!("Context threshold reached: {:.1}%", current_pct * 100.0);

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

    /// Handle review completion from Reviewer
    async fn handle_review_completed(
        state: &mut OrchestratorState,
        item_id: WorkItemId,
        passed: bool,
        feedback: crate::orchestration::messages::ReviewFeedback,
    ) -> Result<()> {
        tracing::info!(
            "Review completed for {:?}: {}",
            item_id,
            if passed { "PASS" } else { "FAIL" }
        );

        if passed {
            // Review passed - mark as complete
            {
                let mut queue = state.work_queue.write().await;
                queue.mark_completed(&item_id);
            }

            // Persist completion event
            state
                .events
                .persist(AgentEvent::WorkItemCompleted {
                    agent: AgentRole::Executor,
                    item_id: item_id.clone(),
                    duration_ms: 0, // Duration tracked separately
                    memory_ids: feedback.execution_context,
                })
                .await?;

            tracing::info!("Work item passed all quality gates: {:?}", item_id);

            // Dispatch next items
            Self::dispatch_work(state).await?;
        } else {
            // Review failed - consolidate context and re-queue
            tracing::warn!(
                "Work item failed review ({} issues): {:?}",
                feedback.issues.len(),
                item_id
            );

            // Get work item for context consolidation
            let work_item = {
                let queue = state.work_queue.read().await;
                queue.get(&item_id).cloned()
            };

            if let Some(mut work_item) = work_item {
                // Increment review attempt
                work_item.review_attempt += 1;

                // Store review feedback
                let mut all_feedback = work_item.review_feedback.unwrap_or_default();
                all_feedback.extend(feedback.issues.clone());
                work_item.review_feedback = Some(all_feedback);

                // Store suggested tests
                let mut all_tests = work_item.suggested_tests.unwrap_or_default();
                all_tests.extend(feedback.suggested_tests.clone());
                work_item.suggested_tests = Some(all_tests);

                // Send to Optimizer for context consolidation
                if let Some(ref optimizer) = state.optimizer {
                    optimizer
                        .cast(OptimizerMessage::ConsolidateWorkItemContext {
                            item_id: item_id.clone(),
                            execution_memory_ids: work_item.execution_memory_ids.clone(),
                            review_feedback: feedback.issues,
                            suggested_tests: feedback.suggested_tests,
                            review_attempt: work_item.review_attempt,
                        })
                        .map_err(|e| {
                            tracing::error!("Failed to send to Optimizer: {:?}", e);
                            crate::error::MnemosyneError::Other(format!(
                                "Failed to send to Optimizer: {:?}",
                                e
                            ))
                        })?;

                    tracing::info!(
                        "Sent work item to Optimizer for context consolidation (attempt {})",
                        work_item.review_attempt
                    );

                    // Update work item in queue with review feedback
                    {
                        let mut queue = state.work_queue.write().await;
                        if let Some(item) = queue.get_mut(&item_id) {
                            item.review_feedback = work_item.review_feedback.clone();
                            item.suggested_tests = work_item.suggested_tests.clone();
                            item.review_attempt = work_item.review_attempt;
                        }
                    }
                } else {
                    tracing::error!("No optimizer available for context consolidation");
                }
            } else {
                tracing::error!("Work item not found for review feedback: {:?}", item_id);
            }
        }

        Ok(())
    }

    /// Handle context consolidation from Optimizer
    async fn handle_context_consolidated(
        state: &mut OrchestratorState,
        item_id: WorkItemId,
        consolidated_memory_id: crate::types::MemoryId,
        estimated_tokens: usize,
    ) -> Result<()> {
        tracing::info!(
            "Context consolidated for {:?}: {} tokens",
            item_id,
            estimated_tokens
        );

        // Update work item with consolidated context
        let work_item = {
            let mut queue = state.work_queue.write().await;
            if let Some(item) = queue.get_mut(&item_id) {
                item.consolidated_context_id = Some(consolidated_memory_id);
                item.estimated_context_tokens = estimated_tokens;

                // Reset to Ready for re-execution
                item.transition(AgentState::Ready);
                item.started_at = None;

                Some(item.clone())
            } else {
                None
            }
        };

        if let Some(work_item) = work_item {
            // Re-enqueue the work item
            {
                let mut queue = state.work_queue.write().await;
                queue
                    .re_enqueue(work_item.clone())
                    .map_err(crate::error::MnemosyneError::Other)?;
            }

            // Persist event
            state
                .events
                .persist(AgentEvent::WorkItemRequeued {
                    item_id: item_id.clone(),
                    reason: format!(
                        "Review failed (attempt {}), context consolidated",
                        work_item.review_attempt
                    ),
                    review_attempt: work_item.review_attempt,
                })
                .await?;

            tracing::info!(
                "Work item re-queued with consolidated context: {:?} (attempt {})",
                item_id,
                work_item.review_attempt
            );

            // Dispatch work (will pick up re-queued item)
            Self::dispatch_work(state).await?;
        } else {
            tracing::error!("Work item not found for context consolidation: {:?}", item_id);
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
        _state: &mut Self::State,
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
            OrchestratorMessage::RegisterAgents {
                optimizer,
                reviewer,
                executor,
            } => {
                tracing::info!("Registering agent references with Orchestrator");
                state.register_agents(optimizer, reviewer, executor);
                tracing::info!("Agents wired: Optimizer, Reviewer, Executor");
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
            OrchestratorMessage::ReviewCompleted {
                item_id,
                passed,
                feedback,
            } => {
                Self::handle_review_completed(state, item_id, passed, feedback)
                    .await
                    .map_err(|e| ActorProcessingErr::from(e.to_string()))?;
            }
            OrchestratorMessage::ContextConsolidated {
                item_id,
                consolidated_memory_id,
                estimated_tokens,
            } => {
                Self::handle_context_consolidated(
                    state,
                    item_id,
                    consolidated_memory_id,
                    estimated_tokens,
                )
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
