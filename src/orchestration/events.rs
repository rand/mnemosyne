//! Agent Event Sourcing
//!
//! All agent state changes are persisted as events in Mnemosyne, enabling:
//! - Deterministic replay after crashes
//! - Complete audit trail
//! - Cross-session state recovery
//! - Time-travel debugging
//!
//! Events are stored as Mnemosyne memories with type `AgentEvent`.

use crate::error::Result;
use crate::launcher::agents::AgentRole;
use crate::orchestration::state::{AgentState, Phase, WorkItemId};
use crate::storage::StorageBackend;
use crate::types::{MemoryId, MemoryNote, MemoryType, Namespace};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Agent events for event sourcing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// Work item assigned to an agent
    WorkItemAssigned {
        agent: AgentRole,
        item_id: WorkItemId,
        description: String,
        phase: Phase,
    },

    /// Work item started by agent
    WorkItemStarted {
        agent: AgentRole,
        item_id: WorkItemId,
        description: String,
    },

    /// Work item completed successfully
    WorkItemCompleted {
        agent: AgentRole,
        item_id: WorkItemId,
        duration_ms: u64,
        memory_ids: Vec<MemoryId>,
    },

    /// Work item failed
    WorkItemFailed {
        agent: AgentRole,
        item_id: WorkItemId,
        error: String,
    },

    /// Phase transition occurred
    PhaseTransition {
        from: Phase,
        to: Phase,
        approved_by: AgentRole,
    },

    /// Context checkpoint created
    ContextCheckpoint {
        agent: AgentRole,
        usage_pct: f32,
        snapshot_id: MemoryId,
        reason: String,
    },

    /// Deadlock detected
    DeadlockDetected {
        blocked_items: Vec<WorkItemId>,
        detected_at: chrono::DateTime<Utc>,
    },

    /// Deadlock resolved
    DeadlockResolved {
        blocked_items: Vec<WorkItemId>,
        resolution: String,
    },

    /// Agent state changed
    AgentStateChanged {
        agent: AgentRole,
        from: AgentState,
        to: AgentState,
        reason: Option<String>,
    },

    /// Sub-agent spawned
    SubAgentSpawned {
        parent: AgentRole,
        child: AgentRole,
        item_id: WorkItemId,
    },

    /// Inter-agent message sent
    MessageSent {
        from: AgentRole,
        to: AgentRole,
        message_type: String,
    },

    /// Review failed for work item
    ReviewFailed {
        item_id: WorkItemId,
        issues: Vec<String>,
        attempt: u32,
    },

    /// Work item re-queued after review failure
    WorkItemRequeued {
        item_id: WorkItemId,
        reason: String,
        review_attempt: u32,
    },

    /// Context consolidated for work item
    ContextConsolidated {
        item_id: WorkItemId,
        consolidated_memory_id: MemoryId,
        estimated_tokens: usize,
        consolidation_level: String,
    },

    // CLI Operation Events
    /// CLI command started
    CliCommandStarted {
        command: String,
        args: Vec<String>,
        timestamp: chrono::DateTime<Utc>,
    },

    /// CLI command completed successfully
    CliCommandCompleted {
        command: String,
        duration_ms: u64,
        result_summary: String,
    },

    /// CLI command failed
    CliCommandFailed {
        command: String,
        error: String,
        duration_ms: u64,
    },

    /// Memory recall executed
    RecallExecuted {
        query: String,
        result_count: usize,
        duration_ms: u64,
    },

    /// Memory remember executed
    RememberExecuted {
        content_preview: String,
        memory_id: MemoryId,
        importance: u8,
    },

    /// Evolution process started
    EvolveStarted {
        timestamp: chrono::DateTime<Utc>,
    },

    /// Evolution process completed
    EvolveCompleted {
        consolidations: usize,
        decayed: usize,
        archived: usize,
        duration_ms: u64,
    },

    /// Database search performed
    SearchPerformed {
        query: String,
        search_type: String, // "semantic", "hybrid", "keyword"
        result_count: usize,
        duration_ms: u64,
    },

    /// Database operation executed
    DatabaseOperation {
        operation: String, // "insert", "update", "delete", "query"
        table: String,
        affected_rows: usize,
        duration_ms: u64,
    },
}

impl AgentEvent {
    /// Get the agent role involved in this event
    pub fn agent(&self) -> Option<AgentRole> {
        match self {
            AgentEvent::WorkItemAssigned { agent, .. }
            | AgentEvent::WorkItemStarted { agent, .. }
            | AgentEvent::WorkItemCompleted { agent, .. }
            | AgentEvent::WorkItemFailed { agent, .. }
            | AgentEvent::ContextCheckpoint { agent, .. }
            | AgentEvent::AgentStateChanged { agent, .. }
            | AgentEvent::SubAgentSpawned { parent: agent, .. } => Some(*agent),
            AgentEvent::PhaseTransition { approved_by, .. } => Some(*approved_by),
            _ => None,
        }
    }

    /// Get event importance for Mnemosyne storage
    pub fn importance(&self) -> u8 {
        match self {
            AgentEvent::PhaseTransition { .. } => 9,
            AgentEvent::DeadlockDetected { .. } => 8,
            AgentEvent::ContextCheckpoint { .. } => 8,
            AgentEvent::ContextConsolidated { .. } => 8,
            AgentEvent::EvolveCompleted { .. } => 7,
            AgentEvent::ReviewFailed { .. } => 7,
            AgentEvent::WorkItemCompleted { .. } => 7,
            AgentEvent::WorkItemFailed { .. } => 7,
            AgentEvent::CliCommandFailed { .. } => 6,
            AgentEvent::WorkItemRequeued { .. } => 6,
            AgentEvent::WorkItemAssigned { .. } => 6,
            AgentEvent::DeadlockResolved { .. } => 6,
            AgentEvent::RememberExecuted { .. } => 5,
            AgentEvent::AgentStateChanged { .. } => 5,
            AgentEvent::SubAgentSpawned { .. } => 5,
            AgentEvent::EvolveStarted { .. } => 5,
            AgentEvent::CliCommandCompleted { .. } => 4,
            AgentEvent::WorkItemStarted { .. } => 4,
            AgentEvent::RecallExecuted { .. } => 4,
            AgentEvent::SearchPerformed { .. } => 3,
            AgentEvent::MessageSent { .. } => 3,
            AgentEvent::CliCommandStarted { .. } => 3,
            AgentEvent::DatabaseOperation { .. } => 2,
        }
    }

    /// Convert event to a summary string
    pub fn summary(&self) -> String {
        match self {
            AgentEvent::WorkItemAssigned {
                agent, description, ..
            } => {
                format!("{:?} assigned: {}", agent, description)
            }
            AgentEvent::WorkItemStarted { agent, description, .. } => {
                format!("{:?} started: {}", agent, description)
            }
            AgentEvent::WorkItemCompleted {
                agent, duration_ms, ..
            } => {
                format!("{:?} completed work in {}ms", agent, duration_ms)
            }
            AgentEvent::WorkItemFailed { agent, error, .. } => {
                format!("{:?} failed: {}", agent, error)
            }
            AgentEvent::PhaseTransition { from, to, .. } => {
                format!("Phase transition: {:?} → {:?}", from, to)
            }
            AgentEvent::ContextCheckpoint {
                usage_pct, reason, ..
            } => {
                format!(
                    "Context checkpoint at {:.1}%: {}",
                    usage_pct * 100.0,
                    reason
                )
            }
            AgentEvent::DeadlockDetected { blocked_items, .. } => {
                format!("Deadlock detected: {} items blocked", blocked_items.len())
            }
            AgentEvent::DeadlockResolved { resolution, .. } => {
                format!("Deadlock resolved: {}", resolution)
            }
            AgentEvent::AgentStateChanged {
                agent, from, to, ..
            } => {
                format!("{:?} state: {:?} → {:?}", agent, from, to)
            }
            AgentEvent::SubAgentSpawned { parent, child, .. } => {
                format!("{:?} spawned {:?}", parent, child)
            }
            AgentEvent::MessageSent {
                from,
                to,
                message_type,
            } => {
                format!("{:?} → {:?}: {}", from, to, message_type)
            }
            AgentEvent::ReviewFailed {
                item_id,
                issues,
                attempt,
            } => {
                format!(
                    "Review failed for {:?} (attempt {}): {} issues",
                    item_id,
                    attempt,
                    issues.len()
                )
            }
            AgentEvent::WorkItemRequeued {
                item_id,
                reason,
                review_attempt,
            } => {
                format!(
                    "Work item {:?} re-queued (attempt {}): {}",
                    item_id, review_attempt, reason
                )
            }
            AgentEvent::ContextConsolidated {
                item_id,
                consolidated_memory_id,
                estimated_tokens,
                consolidation_level,
            } => {
                format!(
                    "Context consolidated for {:?}: {} (memory: {}, {} tokens)",
                    item_id, consolidation_level, consolidated_memory_id, estimated_tokens
                )
            }
            AgentEvent::CliCommandStarted { command, args, .. } => {
                if args.is_empty() {
                    format!("CLI: {} started", command)
                } else {
                    format!("CLI: {} {} started", command, args.join(" "))
                }
            }
            AgentEvent::CliCommandCompleted {
                command,
                duration_ms,
                result_summary,
            } => {
                format!(
                    "CLI: {} completed in {}ms - {}",
                    command, duration_ms, result_summary
                )
            }
            AgentEvent::CliCommandFailed {
                command,
                error,
                duration_ms,
            } => {
                format!("CLI: {} failed after {}ms: {}", command, duration_ms, error)
            }
            AgentEvent::RecallExecuted {
                query,
                result_count,
                duration_ms,
            } => {
                format!(
                    "Recalled '{}': {} results in {}ms",
                    query, result_count, duration_ms
                )
            }
            AgentEvent::RememberExecuted {
                content_preview,
                memory_id,
                importance,
            } => {
                format!(
                    "Remembered (importance {}): {} (id: {})",
                    importance, content_preview, memory_id
                )
            }
            AgentEvent::EvolveStarted { .. } => {
                "Evolution process started".to_string()
            }
            AgentEvent::EvolveCompleted {
                consolidations,
                decayed,
                archived,
                duration_ms,
            } => {
                format!(
                    "Evolution completed in {}ms: {} consolidated, {} decayed, {} archived",
                    duration_ms, consolidations, decayed, archived
                )
            }
            AgentEvent::SearchPerformed {
                query,
                search_type,
                result_count,
                duration_ms,
            } => {
                format!(
                    "{} search '{}': {} results in {}ms",
                    search_type, query, result_count, duration_ms
                )
            }
            AgentEvent::DatabaseOperation {
                operation,
                table,
                affected_rows,
                duration_ms,
            } => {
                format!(
                    "DB {}: {} row(s) in {} ({}ms)",
                    operation, affected_rows, table, duration_ms
                )
            }
        }
    }
}

/// Event persistence layer - stores events to Mnemosyne
pub struct EventPersistence {
    storage: Arc<dyn StorageBackend>,
    pub(crate) namespace: Namespace,
    /// Optional event broadcaster for real-time API updates
    event_broadcaster: Option<crate::api::EventBroadcaster>,
}

impl EventPersistence {
    /// Create a new event persistence layer
    pub fn new(storage: Arc<dyn StorageBackend>, namespace: Namespace) -> Self {
        Self::new_with_broadcaster(storage, namespace, None)
    }

    /// Create a new event persistence layer with event broadcasting
    pub fn new_with_broadcaster(
        storage: Arc<dyn StorageBackend>,
        namespace: Namespace,
        event_broadcaster: Option<crate::api::EventBroadcaster>,
    ) -> Self {
        if event_broadcaster.is_some() {
            tracing::info!("Creating EventPersistence WITH broadcaster for namespace: {}", namespace);
        } else {
            tracing::warn!("Creating EventPersistence WITHOUT broadcaster for namespace: {}", namespace);
        }
        Self {
            storage,
            namespace,
            event_broadcaster,
        }
    }

    /// Convert AgentRole to agent ID string
    fn agent_role_to_id(&self, agent: &AgentRole) -> String {
        match agent {
            AgentRole::Orchestrator => "orchestrator",
            AgentRole::Optimizer => "optimizer",
            AgentRole::Reviewer => "reviewer",
            AgentRole::Executor => "executor",
        }
        .to_string()
    }

    /// Convert orchestration event to API event for real-time broadcasting
    fn to_api_event(&self, event: &AgentEvent) -> Option<crate::api::Event> {
        use crate::api::Event;

        match event {
            AgentEvent::WorkItemAssigned {
                agent,
                item_id,
                description,
                ..
            } => Some(Event::work_item_assigned(
                self.agent_role_to_id(agent),
                format!("{:?}", item_id),
                description.clone(),
            )),
            AgentEvent::WorkItemStarted { agent, description, .. } => {
                Some(Event::agent_started_with_task(
                    self.agent_role_to_id(agent),
                    description.clone(),
                ))
            }
            AgentEvent::WorkItemCompleted { agent, item_id, .. } => {
                Some(Event::work_item_completed(
                    self.agent_role_to_id(agent),
                    format!("{:?}", item_id),
                ))
            }
            AgentEvent::WorkItemFailed { agent, error, .. } => {
                Some(Event::agent_failed(self.agent_role_to_id(agent), error.clone()))
            }
            AgentEvent::PhaseTransition { from, to, .. } => Some(Event::phase_changed(
                format!("{:?}", from),
                format!("{:?}", to),
            )),
            AgentEvent::DeadlockDetected { blocked_items, .. } => Some(Event::deadlock_detected(
                blocked_items.iter().map(|id| format!("{:?}", id)).collect(),
            )),
            AgentEvent::ContextCheckpoint {
                agent,
                usage_pct,
                snapshot_id,
                ..
            } => Some(Event::context_checkpointed(
                self.agent_role_to_id(agent),
                *usage_pct,
                snapshot_id.to_string(),
            )),
            AgentEvent::ReviewFailed {
                item_id,
                issues,
                attempt,
            } => Some(Event::review_failed(
                format!("{:?}", item_id),
                issues.clone(),
                *attempt,
            )),
            AgentEvent::WorkItemRequeued {
                item_id,
                reason,
                review_attempt,
            } => Some(Event::work_item_retried(
                format!("{:?}", item_id),
                reason.clone(),
                *review_attempt,
            )),
            // CLI operation events
            AgentEvent::CliCommandStarted { command, args, .. } => {
                Some(Event::cli_command_started(command.clone(), args.clone()))
            }
            AgentEvent::CliCommandCompleted {
                command,
                duration_ms,
                result_summary,
            } => Some(Event::cli_command_completed(
                command.clone(),
                *duration_ms,
                result_summary.clone(),
            )),
            AgentEvent::CliCommandFailed {
                command,
                error,
                duration_ms,
            } => Some(Event::cli_command_failed(
                command.clone(),
                error.clone(),
                *duration_ms,
            )),
            AgentEvent::RecallExecuted {
                query,
                result_count,
                duration_ms,
            } => Some(Event::memory_recalled(query.clone(), *result_count)),
            AgentEvent::RememberExecuted {
                content_preview,
                memory_id,
                ..
            } => Some(Event::memory_stored(
                memory_id.to_string(),
                content_preview.clone(),
            )),
            AgentEvent::EvolveStarted { .. } => Some(Event::memory_evolution_started(
                "Manual evolution triggered".to_string(),
            )),
            AgentEvent::EvolveCompleted {
                consolidations,
                decayed,
                archived,
                ..
            } => {
                // Use memory_evolution_started as a completion indicator
                // In a real implementation, we might want a dedicated completion event
                Some(Event::memory_evolution_started(format!(
                    "Evolution complete: {} consolidated, {} decayed, {} archived",
                    consolidations, decayed, archived
                )))
            }
            AgentEvent::SearchPerformed {
                query,
                search_type,
                result_count,
                duration_ms,
            } => Some(Event::search_performed(
                query.clone(),
                search_type.clone(),
                *result_count,
                *duration_ms,
            )),
            AgentEvent::DatabaseOperation {
                operation,
                table,
                affected_rows,
                duration_ms,
            } => Some(Event::database_operation(
                operation.clone(),
                table.clone(),
                *affected_rows,
                *duration_ms,
            )),
            // Other events are persisted but not broadcast
            _ => None,
        }
    }

    /// Persist an event to Mnemosyne
    pub async fn persist(&self, event: AgentEvent) -> Result<MemoryId> {
        let now = Utc::now();

        // Serialize event
        let content = serde_json::to_string_pretty(&event)?;

        // Create memory
        let memory = MemoryNote {
            id: crate::types::MemoryId::new(),
            namespace: self.namespace.clone(),
            created_at: now,
            updated_at: now,
            content: content.clone(),
            summary: event.summary(),
            keywords: vec!["agent_event".to_string()],
            tags: vec!["orchestration".to_string(), "event_sourcing".to_string()],
            context: "Agent orchestration event".to_string(),
            memory_type: MemoryType::AgentEvent,
            importance: event.importance(),
            confidence: 1.0,
            links: vec![],
            related_files: vec![],
            related_entities: vec![],
            access_count: 0,
            last_accessed_at: now,
            expires_at: None,
            is_archived: false,
            superseded_by: None,
            embedding: None,
            embedding_model: String::new(),
        };

        // Store to Mnemosyne
        self.storage.store_memory(&memory).await?;

        tracing::debug!("Persisted event: {}", event.summary());

        // Broadcast to API if broadcaster is available
        if let Some(broadcaster) = &self.event_broadcaster {
            tracing::debug!("EventPersistence has broadcaster, checking if event can be converted to API event");
            if let Some(api_event) = self.to_api_event(&event) {
                tracing::info!("Broadcasting event to API: {:?}", api_event.event_type);
                if let Err(e) = broadcaster.broadcast(api_event) {
                    tracing::debug!("Failed to broadcast event to API: {}", e);
                    // Don't fail persistence if broadcasting fails
                } else {
                    tracing::debug!("Successfully broadcast event to API");
                }
            } else {
                tracing::debug!("Event type not mapped to API event: {}", event.summary());
            }
        } else {
            tracing::debug!("No broadcaster available for EventPersistence");
        }

        Ok(memory.id)
    }
}

/// Event replay - reconstruct state from event log
pub struct EventReplay {
    storage: Arc<dyn StorageBackend>,
    namespace: Namespace,
}

impl EventReplay {
    /// Create a new event replay instance
    pub fn new(storage: Arc<dyn StorageBackend>, namespace: Namespace) -> Self {
        Self { storage, namespace }
    }

    /// Load all events from storage
    pub async fn load_events(&self) -> Result<Vec<AgentEvent>> {
        // Query all memories in the namespace and filter by type
        // Use semantic search with empty query to get all memories
        let memories = self
            .storage
            .hybrid_search("", Some(self.namespace.clone()), 10000, false)
            .await?;

        // Parse events, filtering by memory_type, and collect with timestamps
        let mut events_with_time: Vec<(chrono::DateTime<chrono::Utc>, AgentEvent)> = Vec::new();
        for result in memories {
            // Filter by memory_type
            if result.memory.memory_type != MemoryType::AgentEvent {
                continue;
            }

            if let Ok(event) = serde_json::from_str::<AgentEvent>(&result.memory.content) {
                events_with_time.push((result.memory.created_at, event));
            } else {
                tracing::warn!("Failed to parse event: {}", result.memory.id);
            }
        }

        // Sort by timestamp (chronological order: oldest first)
        events_with_time.sort_by_key(|(created_at, _)| *created_at);

        // Extract just the events
        let events: Vec<AgentEvent> = events_with_time
            .into_iter()
            .map(|(_, event)| event)
            .collect();

        tracing::info!("Loaded {} events from storage", events.len());

        Ok(events)
    }

    /// Replay events to reconstruct state
    pub async fn replay(&self) -> Result<ReplayedState> {
        let events = self.load_events().await?;

        let mut state = ReplayedState::default();

        for event in events {
            state.apply(event);
        }

        Ok(state)
    }
}

/// Replayed state from event log
#[derive(Debug, Default)]
pub struct ReplayedState {
    /// Completed work items
    pub completed_items: Vec<WorkItemId>,

    /// Failed work items
    pub failed_items: Vec<(WorkItemId, String)>,

    /// Current phase
    pub current_phase: Phase,

    /// Context checkpoints
    pub checkpoints: Vec<MemoryId>,

    /// Detected deadlocks
    pub deadlocks: Vec<Vec<WorkItemId>>,
}

impl ReplayedState {
    /// Apply an event to the state
    pub fn apply(&mut self, event: AgentEvent) {
        match event {
            AgentEvent::WorkItemCompleted { item_id, .. } => {
                self.completed_items.push(item_id);
            }
            AgentEvent::WorkItemFailed { item_id, error, .. } => {
                self.failed_items.push((item_id, error));
            }
            AgentEvent::PhaseTransition { to, .. } => {
                self.current_phase = to;
            }
            AgentEvent::ContextCheckpoint { snapshot_id, .. } => {
                self.checkpoints.push(snapshot_id);
            }
            AgentEvent::DeadlockDetected { blocked_items, .. } => {
                self.deadlocks.push(blocked_items);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LibsqlStorage;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_event_persistence() {
        // Use temp directory and create database if missing
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

        let persistence = EventPersistence::new(
            storage.clone(),
            Namespace::Session {
                project: "test".to_string(),
                session_id: "test-session".to_string(),
            },
        );

        let event = AgentEvent::WorkItemStarted {
            agent: AgentRole::Executor,
            item_id: WorkItemId::new(),
            description: "Test work".to_string(),
        };

        let memory_id = persistence.persist(event).await.unwrap();
        assert!(!memory_id.to_string().is_empty());
    }

    #[tokio::test]
    async fn test_event_replay() {
        // Use temp directory and create database if missing
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

        let persistence = EventPersistence::new(storage.clone(), namespace.clone());

        // Persist some events
        let item_id = WorkItemId::new();

        persistence
            .persist(AgentEvent::WorkItemStarted {
                agent: AgentRole::Executor,
                item_id: item_id.clone(),
                description: "Test work".to_string(),
            })
            .await
            .unwrap();

        persistence
            .persist(AgentEvent::WorkItemCompleted {
                agent: AgentRole::Executor,
                item_id: item_id.clone(),
                duration_ms: 100,
                memory_ids: vec![],
            })
            .await
            .unwrap();

        // Replay events
        let replay = EventReplay::new(storage, namespace);
        let state = replay.replay().await.unwrap();

        assert_eq!(state.completed_items.len(), 1);
        assert_eq!(state.completed_items[0], item_id);
    }

    #[tokio::test]
    async fn test_event_to_api_event_mapping() {
        // Create a broadcaster for testing
        let broadcaster = crate::api::EventBroadcaster::new(10);

        let storage = Arc::new(
            LibsqlStorage::new(crate::ConnectionMode::InMemory)
                .await
                .expect("Failed to create in-memory storage"),
        );

        let persistence = EventPersistence::new_with_broadcaster(
            storage,
            Namespace::Session {
                project: "test".to_string(),
                session_id: "test-mapping".to_string(),
            },
            Some(broadcaster.clone()),
        );

        // Test WorkItemStarted mapping
        let event = AgentEvent::WorkItemStarted {
            agent: AgentRole::Executor,
            item_id: WorkItemId::new(),
            description: "Test task".to_string(),
        };
        let api_event = persistence.to_api_event(&event);
        assert!(api_event.is_some());
        if let Some(api_event) = api_event {
            match &api_event.event_type {
                crate::api::EventType::AgentStarted { agent_id, task, .. } => {
                    assert_eq!(agent_id, "executor");
                    assert_eq!(task.as_ref().unwrap(), "Test task");
                }
                _ => panic!("Wrong event type"),
            }
        }

        // Test WorkItemCompleted mapping
        let event = AgentEvent::WorkItemCompleted {
            agent: AgentRole::Reviewer,
            item_id: WorkItemId::new(),
            duration_ms: 1000,
            memory_ids: vec![],
        };
        let api_event = persistence.to_api_event(&event);
        assert!(api_event.is_some());
        if let Some(api_event) = api_event {
            assert!(matches!(
                api_event.event_type,
                crate::api::EventType::WorkItemCompleted { .. }
            ));
        }

        // Test WorkItemFailed mapping
        let event = AgentEvent::WorkItemFailed {
            agent: AgentRole::Optimizer,
            item_id: WorkItemId::new(),
            error: "Test error".to_string(),
        };
        let api_event = persistence.to_api_event(&event);
        assert!(api_event.is_some());
        if let Some(api_event) = api_event {
            assert!(matches!(
                api_event.event_type,
                crate::api::EventType::AgentFailed { .. }
            ));
        }

        // Test PhaseTransition mapping
        let event = AgentEvent::PhaseTransition {
            from: Phase::PromptToSpec,
            to: Phase::SpecToFullSpec,
            approved_by: AgentRole::Orchestrator,
        };
        let api_event = persistence.to_api_event(&event);
        assert!(api_event.is_some());
        if let Some(api_event) = api_event {
            assert!(matches!(
                api_event.event_type,
                crate::api::EventType::PhaseChanged { .. }
            ));
        }
    }

    #[tokio::test]
    async fn test_event_broadcasting() {
        // Create a broadcaster
        let broadcaster = crate::api::EventBroadcaster::new(10);
        let mut subscriber = broadcaster.subscribe();

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let storage = Arc::new(
            LibsqlStorage::new_with_validation(
                crate::ConnectionMode::Local(db_path.to_str().unwrap().to_string()),
                true,
            )
            .await
            .expect("Failed to create test storage"),
        );

        let persistence = EventPersistence::new_with_broadcaster(
            storage.clone(),
            Namespace::Session {
                project: "test".to_string(),
                session_id: "test-broadcast".to_string(),
            },
            Some(broadcaster.clone()),
        );

        // Persist an event that should be broadcast
        let event = AgentEvent::WorkItemStarted {
            agent: AgentRole::Executor,
            item_id: WorkItemId::new(),
            description: "Test work".to_string(),
        };

        persistence.persist(event).await.unwrap();

        // Check that the event was broadcast
        let api_event =
            tokio::time::timeout(tokio::time::Duration::from_millis(100), subscriber.recv()).await;

        assert!(api_event.is_ok(), "Event should have been broadcast");
        let api_event = api_event.unwrap().unwrap();
        assert!(matches!(
            api_event.event_type,
            crate::api::EventType::AgentStarted { .. }
        ));
    }
}
