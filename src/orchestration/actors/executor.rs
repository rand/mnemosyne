//! Executor Actor
//!
//! Responsibilities:
//! - Primary work execution
//! - Sub-agent spawning for parallel work
//! - Deterministic workflow wrapping
//! - Work result reporting

use crate::error::Result;
use crate::launcher::agents::AgentRole;
use crate::orchestration::events::{AgentEvent, EventPersistence};
use crate::orchestration::messages::{ExecutorMessage, OrchestratorMessage, WorkResult};
use crate::orchestration::state::WorkItem;
use crate::storage::StorageBackend;
use crate::types::Namespace;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Executor actor state
pub struct ExecutorState {
    /// Event persistence
    events: EventPersistence,

    /// Storage backend
    storage: Arc<dyn StorageBackend>,

    /// Reference to Orchestrator
    orchestrator: Option<ActorRef<OrchestratorMessage>>,

    /// Currently executing work items
    active_work: HashMap<crate::orchestration::state::WorkItemId, Instant>,

    /// Sub-agent references
    sub_agents: Vec<ActorRef<ExecutorMessage>>,

    /// Max concurrent work items
    max_concurrent: usize,
}

impl ExecutorState {
    pub fn new(storage: Arc<dyn StorageBackend>, namespace: Namespace) -> Self {
        Self {
            events: EventPersistence::new(storage.clone(), namespace),
            storage,
            orchestrator: None,
            active_work: HashMap::new(),
            sub_agents: Vec::new(),
            max_concurrent: 4,
        }
    }

    pub fn register_orchestrator(&mut self, orchestrator: ActorRef<OrchestratorMessage>) {
        self.orchestrator = Some(orchestrator);
    }

    /// Register event broadcaster for real-time observability
    pub fn register_event_broadcaster(&mut self, broadcaster: crate::api::EventBroadcaster, namespace: Namespace) {
        // Reconstruct EventPersistence with broadcaster
        self.events = EventPersistence::new_with_broadcaster(
            self.storage.clone(),
            namespace,
            Some(broadcaster),
        );
    }
}

/// Executor actor implementation
pub struct ExecutorActor {
    #[allow(dead_code)]
    storage: Arc<dyn StorageBackend>,
    #[allow(dead_code)]
    namespace: Namespace,
}

impl ExecutorActor {
    pub fn new(storage: Arc<dyn StorageBackend>, namespace: Namespace) -> Self {
        Self { storage, namespace }
    }

    /// Execute a work item
    async fn execute_work(state: &mut ExecutorState, item: WorkItem) -> Result<()> {
        tracing::info!("Executing work: {}", item.description);

        let item_id = item.id.clone();
        let start_time = Instant::now();

        // Mark as active
        state.active_work.insert(item_id.clone(), start_time);

        // Persist start event
        state
            .events
            .persist(AgentEvent::WorkItemStarted {
                agent: AgentRole::Executor,
                item_id: item_id.clone(),
            })
            .await?;

        // Simulate work execution
        // In production, this would:
        // 1. Load relevant context
        // 2. Execute the actual work
        // 3. Persist intermediate results
        // 4. Handle errors gracefully

        tokio::time::sleep(Duration::from_millis(100)).await;

        // Create result
        let duration = start_time.elapsed();
        let result = WorkResult::success(item_id.clone(), duration);

        // Remove from active
        state.active_work.remove(&item_id);

        // Notify orchestrator
        if let Some(ref orchestrator) = state.orchestrator {
            let _ = orchestrator
                .cast(OrchestratorMessage::WorkCompleted {
                    item_id: item_id.clone(),
                    result,
                })
                .map_err(|e| tracing::warn!("Failed to notify orchestrator: {:?}", e));
        }

        // Persist completion event
        state
            .events
            .persist(AgentEvent::WorkItemCompleted {
                agent: AgentRole::Executor,
                item_id,
                duration_ms: duration.as_millis() as u64,
                memory_ids: Vec::new(),
            })
            .await?;

        Ok(())
    }

    /// Spawn a sub-agent for parallel work
    async fn spawn_sub_agent(state: &mut ExecutorState, work_item: WorkItem) -> Result<()> {
        tracing::info!("Spawning sub-agent for: {}", work_item.description);

        // Check if we can spawn more sub-agents
        if state.sub_agents.len() >= state.max_concurrent {
            tracing::warn!("Max sub-agents reached, falling back to inline execution");
            // Fall back to inline execution when at capacity
            return Self::execute_work(state, work_item).await;
        }

        // Persist spawn event
        state
            .events
            .persist(AgentEvent::SubAgentSpawned {
                parent: AgentRole::Executor,
                child: AgentRole::Executor,
                item_id: work_item.id.clone(),
            })
            .await?;

        // Spawn child ExecutorActor
        let storage = state.storage.clone();
        let namespace = state.events.namespace.clone();

        let (child_ref, _handle) = Actor::spawn(
            None,
            ExecutorActor::new(storage.clone(), namespace.clone()),
            (storage, namespace),
        )
        .await
        .map_err(|e| {
            crate::error::MnemosyneError::Other(format!("Failed to spawn sub-agent: {:?}", e))
        })?;

        // Register orchestrator reference with child so it can report completion
        if let Some(ref orchestrator) = state.orchestrator {
            let _ = child_ref
                .cast(ExecutorMessage::RegisterOrchestrator(orchestrator.clone()))
                .map_err(|e| {
                    tracing::warn!("Failed to register orchestrator with sub-agent: {:?}", e)
                });
        }

        // Store child reference for tracking
        state.sub_agents.push(child_ref.clone());

        // Send work to child
        child_ref
            .cast(ExecutorMessage::ExecuteWork(work_item))
            .map_err(|e| {
                crate::error::MnemosyneError::Other(format!(
                    "Failed to send work to sub-agent: {:?}",
                    e
                ))
            })?;

        tracing::debug!(
            "Sub-agent spawned successfully, {} active sub-agents",
            state.sub_agents.len()
        );

        Ok(())
    }

    /// Handle sub-agent completion
    async fn handle_sub_agent_completed(
        state: &mut ExecutorState,
        item_id: crate::orchestration::state::WorkItemId,
        result: WorkResult,
    ) -> Result<()> {
        tracing::info!("Sub-agent completed: {:?}", item_id);

        // Notify orchestrator
        if let Some(ref orchestrator) = state.orchestrator {
            let _ = orchestrator
                .cast(OrchestratorMessage::WorkCompleted { item_id, result })
                .map_err(|e| tracing::warn!("Failed to notify orchestrator: {:?}", e));
        }

        Ok(())
    }
}

#[ractor::async_trait]
impl Actor for ExecutorActor {
    type Msg = ExecutorMessage;
    type State = ExecutorState;
    type Arguments = (Arc<dyn StorageBackend>, Namespace);

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> std::result::Result<Self::State, ActorProcessingErr> {
        tracing::debug!("Executor actor starting");
        let (storage, namespace) = args;
        Ok(ExecutorState::new(storage, namespace))
    }

    async fn post_start(
        &self,
        myself: ActorRef<Self::Msg>,
        _state: &mut Self::State,
    ) -> std::result::Result<(), ActorProcessingErr> {
        tracing::debug!("Executor actor started: {:?}", myself.get_id());
        Ok(())
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> std::result::Result<(), ActorProcessingErr> {
        match message {
            ExecutorMessage::Initialize => {
                tracing::debug!("Executor initialized");
            }
            ExecutorMessage::RegisterEventBroadcaster(broadcaster) => {
                tracing::debug!("Registering event broadcaster with Executor");
                state.register_event_broadcaster(broadcaster, self.namespace.clone());
                tracing::info!("Event broadcaster registered with Executor - events will now be broadcast");
            }
            ExecutorMessage::RegisterOrchestrator(orchestrator_ref) => {
                tracing::debug!("Registering orchestrator reference");
                state.register_orchestrator(orchestrator_ref);
            }
            ExecutorMessage::ExecuteWork(item) => {
                Self::execute_work(state, item)
                    .await
                    .map_err(|e| ActorProcessingErr::from(e.to_string()))?;
            }
            ExecutorMessage::SpawnSubAgent { work_item } => {
                Self::spawn_sub_agent(state, work_item)
                    .await
                    .map_err(|e| ActorProcessingErr::from(e.to_string()))?;
            }
            ExecutorMessage::SubAgentCompleted { item_id, result } => {
                Self::handle_sub_agent_completed(state, item_id, result)
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
        tracing::info!("Executor actor stopped");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LibsqlStorage;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_executor_lifecycle() {
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
            ExecutorActor::new(storage.clone(), namespace.clone()),
            (storage, namespace),
        )
        .await
        .unwrap();

        actor_ref.cast(ExecutorMessage::Initialize).unwrap();
        actor_ref.stop(None);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
