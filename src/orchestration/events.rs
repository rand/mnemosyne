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
            AgentEvent::WorkItemCompleted { .. } => 7,
            AgentEvent::WorkItemFailed { .. } => 7,
            AgentEvent::WorkItemAssigned { .. } => 6,
            AgentEvent::DeadlockResolved { .. } => 6,
            AgentEvent::AgentStateChanged { .. } => 5,
            AgentEvent::SubAgentSpawned { .. } => 5,
            AgentEvent::WorkItemStarted { .. } => 4,
            AgentEvent::MessageSent { .. } => 3,
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
            AgentEvent::WorkItemStarted { agent, .. } => {
                format!("{:?} started work", agent)
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
        }
    }
}

/// Event persistence layer - stores events to Mnemosyne
pub struct EventPersistence {
    storage: Arc<dyn StorageBackend>,
    pub(crate) namespace: Namespace,
}

impl EventPersistence {
    /// Create a new event persistence layer
    pub fn new(storage: Arc<dyn StorageBackend>, namespace: Namespace) -> Self {
        Self { storage, namespace }
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

impl Default for Phase {
    fn default() -> Self {
        Phase::PromptToSpec
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
}
