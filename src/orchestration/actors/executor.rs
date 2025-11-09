//! Executor Actor
//!
//! Responsibilities:
//! - Primary work execution via Python Claude SDK agent
//! - Sub-agent spawning for parallel work
//! - Deterministic workflow wrapping
//! - Work result reporting
//!
//! Integration with Python:
//! - Spawns Python Claude SDK agent via PyO3 bridge
//! - Delegates actual work execution to intelligent Python agent
//! - Falls back to simple execution if Python feature disabled

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

#[cfg(feature = "python")]
use crate::orchestration::ClaudeAgentBridge;

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

    /// Python Claude SDK agent bridge (if Python feature enabled)
    #[cfg(feature = "python")]
    python_bridge: Option<ClaudeAgentBridge>,
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
            #[cfg(feature = "python")]
            python_bridge: None,
        }
    }

    pub fn register_orchestrator(&mut self, orchestrator: ActorRef<OrchestratorMessage>) {
        self.orchestrator = Some(orchestrator);
    }

    /// Register Python Claude SDK agent bridge
    #[cfg(feature = "python")]
    pub fn register_python_bridge(&mut self, bridge: ClaudeAgentBridge) {
        tracing::info!("Registering Python agent bridge for Executor");
        self.python_bridge = Some(bridge);
    }

    /// Register event broadcaster for real-time observability
    pub fn register_event_broadcaster(
        &mut self,
        broadcaster: crate::api::EventBroadcaster,
        namespace: Namespace,
        agent_id: String,
    ) {
        tracing::info!("Executor: Registering event broadcaster for agent_id: {}", agent_id);
        // Reconstruct EventPersistence with broadcaster
        self.events = EventPersistence::new_with_broadcaster(
            self.storage.clone(),
            namespace.clone(),
            Some(broadcaster.clone()),
        );
        tracing::info!("Executor: EventPersistence recreated with broadcaster");

        // Clone agent_id for the spawn task
        let agent_id_clone = agent_id.clone();

        // Spawn heartbeat task with immediate first beat, then 30s interval
        tokio::spawn(async move {
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
                interval.tick().await;
                let event = crate::api::Event::heartbeat(agent_id_clone.clone());
                if let Err(e) = broadcaster.broadcast(event) {
                    tracing::debug!(
                        "Failed to broadcast heartbeat for {} (no subscribers): {}",
                        agent_id_clone,
                        e
                    );
                }
            }
        });
        tracing::info!("Heartbeat task spawned for {} (immediate first beat + 30s interval)", agent_id);
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
                description: item.description.clone(),
            })
            .await?;

        // Execute work via Python agent bridge (if available) or fallback to simulation
        let result = {
            #[cfg(feature = "python")]
            {
                if let Some(ref bridge) = state.python_bridge {
                    // Delegate to Python Claude SDK agent for intelligent execution
                    tracing::info!("Delegating work to Python Claude SDK agent");
                    match bridge.send_work(item.clone()).await {
                        Ok(mut python_result) => {
                            // Update duration to actual elapsed time
                            python_result.duration = start_time.elapsed();
                            python_result
                        }
                        Err(e) => {
                            tracing::error!("Python agent execution failed: {}", e);
                            // Create error result
                            WorkResult {
                                item_id: item_id.clone(),
                                success: false,
                                data: None,
                                error: Some(format!("Python agent error: {}", e)),
                                duration: start_time.elapsed(),
                                memory_ids: Vec::new(),
                            }
                        }
                    }
                } else {
                    // Python bridge not available - use simple execution
                    tracing::warn!("Python bridge not available, using simple execution");
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    WorkResult::success(item_id.clone(), start_time.elapsed())
                }
            }

            #[cfg(not(feature = "python"))]
            {
                // Python feature disabled - simulate work
                tracing::debug!("Python feature disabled, simulating work execution");
                tokio::time::sleep(Duration::from_millis(100)).await;
                WorkResult::success(item_id.clone(), start_time.elapsed())
            }
        };

        // Remove from active
        state.active_work.remove(&item_id);

        // Save values needed for event persistence before moving result
        let duration_ms = result.duration.as_millis() as u64;
        let memory_ids = result.memory_ids.clone();

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
                duration_ms,
                memory_ids,
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
                let agent_id = format!("{}-executor", self.namespace);
                state.register_event_broadcaster(broadcaster, self.namespace.clone(), agent_id);
                tracing::info!(
                    "Event broadcaster registered with Executor - events will now be broadcast"
                );
            }
            ExecutorMessage::RegisterOrchestrator(orchestrator_ref) => {
                tracing::debug!("Registering orchestrator reference");
                state.register_orchestrator(orchestrator_ref);
            }
            #[cfg(feature = "python")]
            ExecutorMessage::RegisterPythonBridge(bridge) => {
                tracing::info!("Registering Python Claude SDK agent bridge");
                state.register_python_bridge(bridge);
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
