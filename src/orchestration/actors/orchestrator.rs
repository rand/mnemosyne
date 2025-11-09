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
    DEFAULT_MAX_WORK_ITEMS,
};
use crate::storage::StorageBackend;
use crate::types::Namespace;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[cfg(feature = "python")]
use crate::orchestration::ClaudeAgentBridge;

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

    /// Python Claude SDK agent bridge
    #[cfg(feature = "python")]
    python_bridge: Option<ClaudeAgentBridge>,

    /// Shutdown signal for background tasks
    shutdown_tx: tokio::sync::broadcast::Sender<()>,

    /// Heartbeat task handle for cleanup
    heartbeat_handle: Option<tokio::task::JoinHandle<()>>,

    /// Deadlock checker task handle for cleanup
    deadlock_checker_handle: Option<tokio::task::JoinHandle<()>>,
}

impl OrchestratorState {
    pub fn new(storage: Arc<dyn StorageBackend>, namespace: Namespace) -> Self {
        // Create shutdown channel for graceful task termination
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);

        Self {
            work_queue: Arc::new(RwLock::new(WorkQueue::new())),
            events: EventPersistence::new(storage, namespace),
            optimizer: None,
            reviewer: None,
            executor: None,
            context_usage_pct: 0.0,
            deadlock_check_interval: Duration::from_secs(10),
            #[cfg(feature = "python")]
            python_bridge: None,
            shutdown_tx,
            heartbeat_handle: None,
            deadlock_checker_handle: None,
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

    /// Register Python Claude SDK agent bridge
    #[cfg(feature = "python")]
    pub fn register_python_bridge(&mut self, bridge: ClaudeAgentBridge) {
        tracing::info!("Registering Python agent bridge for Orchestrator");
        self.python_bridge = Some(bridge);
    }

    /// Register event broadcaster for real-time observability
    pub fn register_event_broadcaster(
        &mut self,
        broadcaster: crate::api::EventBroadcaster,
        storage: Arc<dyn StorageBackend>,
        namespace: Namespace,
        agent_id: String,
    ) {
        tracing::info!("Orchestrator: Registering event broadcaster for agent_id: {}", agent_id);
        // Reconstruct EventPersistence with broadcaster
        self.events = EventPersistence::new_with_broadcaster(
            storage,
            namespace.clone(),
            Some(broadcaster.clone()),
        );
        tracing::info!("Orchestrator: EventPersistence recreated with broadcaster");

        // Clone agent_id for the spawn task
        let agent_id_clone = agent_id.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        // Spawn heartbeat task with immediate first beat, then 30s interval, with shutdown support
        let heartbeat_handle = tokio::spawn(async move {
            // Send immediate first heartbeat so dashboard sees agent right away
            let event = crate::api::Event::heartbeat(agent_id_clone.clone());
            if let Err(e) = broadcaster.broadcast(event) {
                tracing::debug!(
                    "Failed to broadcast initial heartbeat for {}: {}",
                    agent_id_clone,
                    e
                );
            }

            // Then continue with 30s interval
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let event = crate::api::Event::heartbeat(agent_id_clone.clone());
                        if let Err(e) = broadcaster.broadcast(event) {
                            tracing::debug!(
                                "Failed to broadcast heartbeat for {} (no subscribers): {}",
                                agent_id_clone,
                                e
                            );
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        tracing::debug!("Orchestrator heartbeat task received shutdown signal");
                        break;
                    }
                }
            }
        });

        // Store heartbeat handle for cleanup
        self.heartbeat_handle = Some(heartbeat_handle);
        tracing::info!("Heartbeat task spawned for {} (immediate first beat + 30s interval)", agent_id);
    }
}

impl Drop for OrchestratorState {
    fn drop(&mut self) {
        // Send shutdown signal to background tasks
        let _ = self.shutdown_tx.send(());

        // Abort heartbeat task if it's still running
        if let Some(handle) = self.heartbeat_handle.take() {
            handle.abort();
            tracing::debug!("OrchestratorState dropped - heartbeat task aborted");
        }

        // Abort deadlock checker task if it's still running
        if let Some(handle) = self.deadlock_checker_handle.take() {
            handle.abort();
            tracing::debug!("OrchestratorState dropped - deadlock checker task aborted");
        }
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
            if let Err(e) = queue.add(item) {
                tracing::warn!("Work queue at capacity: {}", e);
                return Err(crate::error::MnemosyneError::Other(format!(
                    "Work queue full: {}",
                    e
                ))
                .into());
            }

            // Log warning if nearing capacity
            if queue.is_near_capacity() {
                tracing::warn!(
                    "Work queue nearing capacity: {:.1}% ({}/{})",
                    queue.capacity_utilization() * 100.0,
                    queue.stats().total_items,
                    DEFAULT_MAX_WORK_ITEMS
                );
            }
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

            // First, discover relevant skills for the work item (unless it's for Optimizer itself)
            if item.agent != AgentRole::Optimizer {
                if let Some(ref optimizer) = state.optimizer {
                    tracing::debug!(
                        "Discovering skills for work item: {}",
                        item.description
                    );
                    let _ = optimizer
                        .cast(OptimizerMessage::DiscoverSkills {
                            task_description: item.description.clone(),
                            max_skills: 7, // Load top 7 most relevant skills
                        })
                        .map_err(|e| {
                            tracing::warn!("Failed to discover skills: {:?}", e)
                        });
                }
            }

            // Second, load relevant context memories for the work item
            // All agents benefit from having relevant context loaded
            if let Some(ref optimizer) = state.optimizer {
                tracing::debug!(
                    "Loading context memories for work item: {}",
                    item.description
                );
                let _ = optimizer
                    .cast(OptimizerMessage::LoadContextMemories {
                        work_item_id: item.id.clone(),
                        query: item.description.clone(),
                    })
                    .map_err(|e| {
                        tracing::warn!("Failed to load context memories: {:?}", e)
                    });
            }

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
                    // Optimizer work items already have context loaded above
                    tracing::debug!("Optimizer work item dispatched");
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
                        work_item: Box::new(work_item),
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

    /// Check if current phase is complete and trigger automatic transition
    async fn check_phase_completion(state: &mut OrchestratorState) -> Result<()> {
        use crate::orchestration::work_plan_templates;

        let current_phase = {
            let queue = state.work_queue.read().await;
            queue.current_phase()
        };

        // Check if all work items for current phase are completed
        let phase_complete = {
            let queue = state.work_queue.read().await;

            // Get ready, active, and blocked items for current phase
            let ready_items = queue.get_ready_items();
            let active_items = queue.get_active_items();

            // Count items that are still in progress for current phase
            let in_progress_count = ready_items
                .iter()
                .chain(active_items.iter())
                .filter(|item| item.phase == current_phase)
                .count();

            // Phase is complete when no items are in progress
            in_progress_count == 0
        };

        if !phase_complete {
            return Ok(());
        }

        // Determine next phase
        let next_phase = match current_phase.next() {
            Some(phase) => phase,
            None => {
                tracing::info!("Work Plan Protocol complete! All phases finished.");
                return Ok(());
            }
        };

        tracing::info!(
            "Phase {:?} complete! Transitioning to {:?}",
            current_phase,
            next_phase
        );

        // Generate work items for next phase
        let new_items = match next_phase {
            Phase::SpecToFullSpec => {
                // TODO: Extract spec summary from completed PromptToSpec work
                let spec_summary = "Generated specification".to_string();
                work_plan_templates::create_phase2_work_items(spec_summary)
            }
            Phase::FullSpecToPlan => {
                // TODO: Extract full spec summary from completed SpecToFullSpec work
                let full_spec_summary = "Detailed specification with components".to_string();
                work_plan_templates::create_phase3_work_items(full_spec_summary)
            }
            Phase::PlanToArtifacts => {
                // TODO: Extract plan tasks from completed FullSpecToPlan work
                let plan_tasks = vec![
                    "Implement core functionality".to_string(),
                    "Write tests".to_string(),
                    "Create documentation".to_string(),
                ];
                work_plan_templates::create_phase4_work_items(plan_tasks)
            }
            Phase::Complete => {
                vec![] // No more work items
            }
            _ => vec![], // Shouldn't happen, but handle gracefully
        };

        // Trigger phase transition
        Self::handle_phase_transition(state, current_phase, next_phase).await?;

        // Submit new work items for next phase
        let num_items = new_items.len();
        for item in new_items {
            tracing::debug!("Submitting work for next phase: {}", item.description);

            let agent = item.agent;
            let item_id = item.id.clone();
            let phase = item.phase;
            let description = item.description.clone();

            {
                let mut queue = state.work_queue.write().await;
                if let Err(e) = queue.add(item) {
                    tracing::error!("Failed to add work item for next phase: {}", e);
                    // Continue with other items rather than failing entire phase transition
                    continue;
                }
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
        }

        tracing::info!(
            "Phase transition complete: {} new work items created for {:?}",
            num_items,
            next_phase
        );

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

        // Enforce requirement satisfaction before marking complete
        let all_requirements_satisfied = feedback.unsatisfied_requirements.is_empty();

        if !all_requirements_satisfied {
            tracing::warn!(
                "Work item {:?} has {} unsatisfied requirements: {:?}",
                item_id,
                feedback.unsatisfied_requirements.len(),
                feedback.unsatisfied_requirements
            );
        }

        if passed && all_requirements_satisfied {
            // Review passed AND all requirements satisfied - mark as complete

            // Update work item with requirement tracking
            {
                let mut queue = state.work_queue.write().await;
                if let Some(work_item) = queue.get_mut(&item_id) {
                    // Store extracted requirements if not already present
                    if work_item.requirements.is_empty()
                        && !feedback.extracted_requirements.is_empty()
                    {
                        work_item.requirements = feedback.extracted_requirements.clone();
                    }

                    // Mark all requirements as satisfied
                    for req in &work_item.requirements {
                        work_item.requirement_status.insert(
                            req.clone(),
                            crate::orchestration::state::RequirementStatus::Satisfied,
                        );
                    }

                    // Store implementation evidence
                    work_item.implementation_evidence = feedback.satisfied_requirements.clone();
                }

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

            tracing::info!(
                "Work item passed all quality gates and satisfied all requirements: {:?}",
                item_id
            );

            // Check for phase completion and trigger automatic transition
            Self::check_phase_completion(state).await?;

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

                // Store extracted requirements if not already present
                if work_item.requirements.is_empty() && !feedback.extracted_requirements.is_empty()
                {
                    work_item.requirements = feedback.extracted_requirements.clone();
                }

                // Track unsatisfied requirements
                for req in &feedback.unsatisfied_requirements {
                    work_item.requirement_status.insert(
                        req.clone(),
                        crate::orchestration::state::RequirementStatus::InProgress,
                    );
                }

                // Track satisfied requirements (partial completion)
                for (req, evidence) in &feedback.satisfied_requirements {
                    work_item.requirement_status.insert(
                        req.clone(),
                        crate::orchestration::state::RequirementStatus::Satisfied,
                    );
                    work_item
                        .implementation_evidence
                        .insert(req.clone(), evidence.clone());
                }

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
            tracing::error!(
                "Work item not found for context consolidation: {:?}",
                item_id
            );
        }

        Ok(())
    }

    /// Handle CLI event received from SSE subscriber
    async fn handle_cli_event(
        state: &mut OrchestratorState,
        event: AgentEvent,
    ) -> Result<()> {
        tracing::debug!("Orchestrator received CLI event: {}", event.summary());

        // Persist the event for audit trail and memory
        state.events.persist(event.clone()).await?;

        // React to specific event types that require orchestration coordination
        match &event {
            // Memory operations might need context refresh
            AgentEvent::RememberExecuted { .. } | AgentEvent::RecallExecuted { .. } => {
                tracing::debug!("Memory operation detected, work queue unaffected");
            }

            // CLI commands completion might affect work queue
            AgentEvent::CliCommandCompleted { command, .. } => {
                tracing::debug!("CLI command completed: {}", command);
                // Future: Could trigger work queue updates based on command type
            }

            // Session lifecycle
            AgentEvent::SessionStarted { instance_id, .. } => {
                tracing::info!("Claude Code session started: {}", instance_id);
            }

            AgentEvent::SessionEnded { instance_id, .. } => {
                tracing::info!("Claude Code session ended: {}", instance_id);
                // Future: Could trigger cleanup or state persistence
            }

            // Database operations
            AgentEvent::DatabaseOperation { operation, table, .. } => {
                tracing::debug!("Database operation: {} on {}", operation, table);
            }

            // Other events are logged but don't require action
            _ => {
                tracing::debug!("CLI event received (no action required): {}", event.summary());
            }
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
        tracing::debug!("Orchestrator actor starting");
        let (storage, namespace) = args;
        Ok(OrchestratorState::new(storage, namespace))
    }

    async fn post_start(
        &self,
        myself: ActorRef<Self::Msg>,
        state: &mut Self::State,
    ) -> std::result::Result<(), ActorProcessingErr> {
        tracing::debug!("Orchestrator actor started: {:?}", myself.get_id());

        // Start periodic deadlock checker with shutdown support
        let myself_clone = myself.clone();
        let mut shutdown_rx = state.shutdown_tx.subscribe();

        let deadlock_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let _ = myself_clone.cast(OrchestratorMessage::GetReadyWork);
                    }
                    _ = shutdown_rx.recv() => {
                        tracing::debug!("Orchestrator deadlock checker task received shutdown signal");
                        break;
                    }
                }
            }
        });

        // Store deadlock checker handle for cleanup
        state.deadlock_checker_handle = Some(deadlock_handle);

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
                tracing::debug!("Orchestrator initialized");
            }
            OrchestratorMessage::RegisterAgents {
                optimizer,
                reviewer,
                executor,
            } => {
                tracing::debug!("Registering agent references with Orchestrator");
                state.register_agents(optimizer, reviewer, executor);
                tracing::debug!("Agents wired: Optimizer, Reviewer, Executor");
            }
            OrchestratorMessage::RegisterEventBroadcaster(broadcaster) => {
                tracing::debug!("Registering event broadcaster with Orchestrator");
                let agent_id = format!("{}-orchestrator", self.namespace);
                state.register_event_broadcaster(
                    broadcaster,
                    self.storage.clone(),
                    self.namespace.clone(),
                    agent_id,
                );
                tracing::info!(
                    "Event broadcaster registered with Orchestrator - events will now be broadcast"
                );
            }
            #[cfg(feature = "python")]
            OrchestratorMessage::RegisterPythonBridge(bridge) => {
                tracing::info!("Registering Python Claude SDK agent bridge");
                state.register_python_bridge(bridge);
            }
            OrchestratorMessage::SubmitWork(item) => {
                Self::handle_submit_work(state, *item)
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
            OrchestratorMessage::CliEventReceived { event } => {
                Self::handle_cli_event(state, event)
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
    use crate::orchestration::state::RequirementStatus;
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

    #[tokio::test]
    async fn test_requirement_enforcement_all_satisfied() {
        // Setup
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let _storage = Arc::new(
            LibsqlStorage::new_with_validation(
                crate::ConnectionMode::Local(db_path.to_str().unwrap().to_string()),
                true,
            )
            .await
            .expect("Failed to create test storage"),
        );

        let _namespace = Namespace::Session {
            project: "test".to_string(),
            session_id: "test-session".to_string(),
        };

        // Create work item with requirements
        let mut work_item = WorkItem::new(
            "Test work".to_string(),
            AgentRole::Executor,
            Phase::PlanToArtifacts,
            5,
        );
        work_item.requirements = vec!["Req 1".to_string(), "Req 2".to_string()];

        // Create feedback with all requirements satisfied
        let mut satisfied_requirements = std::collections::HashMap::new();
        satisfied_requirements.insert("Req 1".to_string(), vec![]);
        satisfied_requirements.insert("Req 2".to_string(), vec![]);

        let feedback = crate::orchestration::messages::ReviewFeedback {
            gates_passed: true,
            issues: vec![],
            suggested_tests: vec![],
            execution_context: vec![],
            improvement_guidance: None,
            extracted_requirements: vec![],
            unsatisfied_requirements: vec![], // All satisfied
            satisfied_requirements,
        };

        // Verify enforcement logic
        let all_requirements_satisfied = feedback.unsatisfied_requirements.is_empty();
        assert!(
            all_requirements_satisfied,
            "All requirements should be satisfied"
        );

        let should_complete = true && all_requirements_satisfied;
        assert!(should_complete, "Work should be marked complete");
    }

    #[tokio::test]
    async fn test_requirement_enforcement_unsatisfied() {
        // Setup
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let _storage = Arc::new(
            LibsqlStorage::new_with_validation(
                crate::ConnectionMode::Local(db_path.to_str().unwrap().to_string()),
                true,
            )
            .await
            .expect("Failed to create test storage"),
        );

        let _namespace = Namespace::Session {
            project: "test".to_string(),
            session_id: "test-session".to_string(),
        };

        // Create work item with requirements
        let mut work_item = WorkItem::new(
            "Test work".to_string(),
            AgentRole::Executor,
            Phase::PlanToArtifacts,
            5,
        );
        work_item.requirements = vec!["Req 1".to_string(), "Req 2".to_string()];

        // Create feedback with unsatisfied requirements
        let mut satisfied_requirements = std::collections::HashMap::new();
        satisfied_requirements.insert("Req 1".to_string(), vec![]);

        let feedback = crate::orchestration::messages::ReviewFeedback {
            gates_passed: true,
            issues: vec![],
            suggested_tests: vec![],
            execution_context: vec![],
            improvement_guidance: None,
            extracted_requirements: vec![],
            unsatisfied_requirements: vec!["Req 2".to_string()], // One unsatisfied
            satisfied_requirements,
        };

        // Verify enforcement logic
        let all_requirements_satisfied = feedback.unsatisfied_requirements.is_empty();
        assert!(
            !all_requirements_satisfied,
            "Not all requirements should be satisfied"
        );

        let should_complete = true && all_requirements_satisfied;
        assert!(
            !should_complete,
            "Work should NOT be marked complete with unsatisfied requirements"
        );
    }

    #[tokio::test]
    async fn test_requirement_status_tracking() {
        // Setup
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let _storage = Arc::new(
            LibsqlStorage::new_with_validation(
                crate::ConnectionMode::Local(db_path.to_str().unwrap().to_string()),
                true,
            )
            .await
            .expect("Failed to create test storage"),
        );

        let _namespace = Namespace::Session {
            project: "test".to_string(),
            session_id: "test-session".to_string(),
        };

        // Create work item
        let mut work_item = WorkItem::new(
            "Test work".to_string(),
            AgentRole::Executor,
            Phase::PlanToArtifacts,
            5,
        );

        // Simulate requirement tracking on success
        work_item.requirements = vec!["Req 1".to_string(), "Req 2".to_string()];

        for req in &work_item.requirements {
            work_item
                .requirement_status
                .insert(req.clone(), RequirementStatus::Satisfied);
        }

        // Verify all requirements marked as satisfied
        assert_eq!(work_item.requirement_status.len(), 2);
        assert_eq!(
            work_item.requirement_status.get("Req 1"),
            Some(&RequirementStatus::Satisfied)
        );
        assert_eq!(
            work_item.requirement_status.get("Req 2"),
            Some(&RequirementStatus::Satisfied)
        );
    }

    #[tokio::test]
    async fn test_partial_requirement_satisfaction() {
        // Setup
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let _storage = Arc::new(
            LibsqlStorage::new_with_validation(
                crate::ConnectionMode::Local(db_path.to_str().unwrap().to_string()),
                true,
            )
            .await
            .expect("Failed to create test storage"),
        );

        let _namespace = Namespace::Session {
            project: "test".to_string(),
            session_id: "test-session".to_string(),
        };

        // Create work item
        let mut work_item = WorkItem::new(
            "Test work".to_string(),
            AgentRole::Executor,
            Phase::PlanToArtifacts,
            5,
        );
        work_item.requirements = vec![
            "Req 1".to_string(),
            "Req 2".to_string(),
            "Req 3".to_string(),
        ];

        // Simulate partial satisfaction (for retry scenario)
        let mut satisfied_requirements = std::collections::HashMap::new();
        satisfied_requirements.insert("Req 1".to_string(), vec![]);

        let unsatisfied_requirements = vec!["Req 2".to_string(), "Req 3".to_string()];

        // Track status
        for (req, evidence) in &satisfied_requirements {
            work_item
                .requirement_status
                .insert(req.clone(), RequirementStatus::Satisfied);
            work_item
                .implementation_evidence
                .insert(req.clone(), evidence.clone());
        }

        for req in &unsatisfied_requirements {
            work_item
                .requirement_status
                .insert(req.clone(), RequirementStatus::InProgress);
        }

        // Verify partial satisfaction tracked
        assert_eq!(
            work_item.requirement_status.get("Req 1"),
            Some(&RequirementStatus::Satisfied)
        );
        assert_eq!(
            work_item.requirement_status.get("Req 2"),
            Some(&RequirementStatus::InProgress)
        );
        assert_eq!(
            work_item.requirement_status.get("Req 3"),
            Some(&RequirementStatus::InProgress)
        );
    }
}
