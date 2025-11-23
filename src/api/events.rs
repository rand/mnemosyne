//! Event types and Server-Sent Events (SSE) endpoint

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Event type discriminant
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventType {
    /// Agent started
    AgentStarted {
        agent_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        task: Option<String>,
        timestamp: DateTime<Utc>,
    },
    /// Agent completed task
    AgentCompleted {
        agent_id: String,
        result: String,
        timestamp: DateTime<Utc>,
    },
    /// Agent failed
    AgentFailed {
        agent_id: String,
        error: String,
        timestamp: DateTime<Utc>,
    },
    /// Memory stored
    MemoryStored {
        memory_id: String,
        summary: String,
        timestamp: DateTime<Utc>,
    },
    /// Memory recalled
    MemoryRecalled {
        query: String,
        count: usize,
        timestamp: DateTime<Utc>,
    },
    /// Context file modified
    ContextModified {
        file: String,
        timestamp: DateTime<Utc>,
    },
    /// Context validated
    ContextValidated {
        file: String,
        errors: Vec<String>,
        timestamp: DateTime<Utc>,
    },
    /// System health update
    HealthUpdate {
        memory_mb: f32,
        cpu_percent: f32,
        timestamp: DateTime<Utc>,
    },
    /// Session started
    SessionStarted {
        #[serde(default)]
        instance_id: Option<String>,
        timestamp: DateTime<Utc>,
    },
    /// Session ended
    SessionEnded {
        #[serde(default)]
        instance_id: Option<String>,
        timestamp: DateTime<Utc>,
    },
    /// Heartbeat (published periodically when idle)
    Heartbeat {
        #[serde(default)]
        instance_id: Option<String>,
        timestamp: DateTime<Utc>,
    },
    /// Phase transition (orchestration workflow)
    PhaseChanged {
        from: String,
        to: String,
        timestamp: DateTime<Utc>,
    },
    /// Deadlock detected in work queue
    DeadlockDetected {
        blocked_items: Vec<String>,
        timestamp: DateTime<Utc>,
    },
    /// Context checkpoint created
    ContextCheckpointed {
        agent_id: String,
        usage_percent: f32,
        snapshot_id: String,
        timestamp: DateTime<Utc>,
    },
    /// Review failed for work item
    ReviewFailed {
        item_id: String,
        issues: Vec<String>,
        attempt: u32,
        timestamp: DateTime<Utc>,
    },
    /// Work item retried after failure/review
    WorkItemRetried {
        item_id: String,
        reason: String,
        attempt: u32,
        timestamp: DateTime<Utc>,
    },
    /// Python agent error recorded
    AgentErrorRecorded {
        agent_id: String,
        error_count: usize,
        error_message: String,
        timestamp: DateTime<Utc>,
    },
    /// Python agent restarted
    AgentRestarted {
        agent_id: String,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    /// Python agent health degraded
    AgentHealthDegraded {
        agent_id: String,
        error_count: usize,
        is_healthy: bool,
        timestamp: DateTime<Utc>,
    },
    /// Work item assigned to agent
    WorkItemAssigned {
        agent_id: String,
        item_id: String,
        task: String,
        timestamp: DateTime<Utc>,
    },
    /// Work item completed by agent
    WorkItemCompleted {
        agent_id: String,
        item_id: String,
        timestamp: DateTime<Utc>,
    },
    // Skill-related events
    /// Skill loaded by optimizer
    SkillLoaded {
        skill_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        relevance_score: f32,
        timestamp: DateTime<Utc>,
    },
    /// Skill unloaded to free context
    SkillUnloaded {
        skill_name: String,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    /// Skill used by agent
    SkillUsed {
        skill_name: String,
        agent_id: String,
        timestamp: DateTime<Utc>,
    },
    /// Skill composition detected (multiple skills combined)
    SkillCompositionDetected {
        skills: Vec<String>,
        task_description: String,
        timestamp: DateTime<Utc>,
    },
    // Memory evolution events
    /// Memory evolution process started
    MemoryEvolutionStarted {
        reason: String,
        timestamp: DateTime<Utc>,
    },
    /// Memories consolidated (merged similar memories)
    MemoryConsolidated {
        source_ids: Vec<String>,
        target_id: String,
        timestamp: DateTime<Utc>,
    },
    /// Memory importance decayed
    MemoryDecayed {
        memory_id: String,
        old_importance: f32,
        new_importance: f32,
        timestamp: DateTime<Utc>,
    },
    /// Memory archived (low importance)
    MemoryArchived {
        memory_id: String,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    // Agent interaction events
    /// Agent handoff (work passed between agents)
    AgentHandoff {
        from_agent: String,
        to_agent: String,
        task_description: String,
        timestamp: DateTime<Utc>,
    },
    /// Agent blocked waiting for dependency
    AgentBlocked {
        agent_id: String,
        blocked_on: String,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    /// Agent unblocked, can continue
    AgentUnblocked {
        agent_id: String,
        unblocked_by: String,
        timestamp: DateTime<Utc>,
    },
    /// Sub-agent spawned for parallel work
    SubAgentSpawned {
        parent_agent: String,
        sub_agent: String,
        task_description: String,
        timestamp: DateTime<Utc>,
    },
    // Work orchestration events
    /// Parallel work stream started
    ParallelStreamStarted {
        stream_id: String,
        task_count: usize,
        timestamp: DateTime<Utc>,
    },
    /// Critical path updated (bottleneck identified)
    CriticalPathUpdated {
        path_items: Vec<String>,
        estimated_completion: String,
        timestamp: DateTime<Utc>,
    },
    /// Typed hole filled (interface implemented)
    TypedHoleFilled {
        hole_name: String,
        component_a: String,
        component_b: String,
        timestamp: DateTime<Utc>,
    },
    // CLI operation events
    /// CLI command started
    CliCommandStarted {
        command: String,
        args: Vec<String>,
        timestamp: DateTime<Utc>,
    },
    /// CLI command completed successfully
    CliCommandCompleted {
        command: String,
        duration_ms: u64,
        result_summary: String,
        timestamp: DateTime<Utc>,
    },
    /// CLI command failed
    CliCommandFailed {
        command: String,
        error: String,
        duration_ms: u64,
        timestamp: DateTime<Utc>,
    },
    /// Database search performed (semantic, hybrid, keyword)
    SearchPerformed {
        query: String,
        search_type: String,
        result_count: usize,
        duration_ms: u64,
        timestamp: DateTime<Utc>,
    },
    /// Database operation executed
    DatabaseOperation {
        operation: String,
        table: String,
        affected_rows: usize,
        duration_ms: u64,
        timestamp: DateTime<Utc>,
    },
    /// Network state update
    NetworkStateUpdate {
        connected_peers: usize,
        known_nodes: Vec<String>,
        timestamp: DateTime<Utc>,
    },
}

/// Event wrapper with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Event ID (for deduplication)
    pub id: String,
    /// Instance ID (for multi-instance coordination)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub instance_id: Option<String>,
    /// Event payload
    #[serde(flatten)]
    pub event_type: EventType,
}

impl Event {
    /// Create new event
    pub fn new(event_type: EventType) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            instance_id: None,
            event_type,
        }
    }

    /// Create new event with instance ID
    pub fn new_with_instance(event_type: EventType, instance_id: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            instance_id: Some(instance_id),
            event_type,
        }
    }

    /// Create agent started event
    pub fn agent_started(agent_id: String) -> Self {
        Self::new(EventType::AgentStarted {
            agent_id,
            task: None,
            timestamp: Utc::now(),
        })
    }

    /// Create agent started event with task information
    pub fn agent_started_with_task(agent_id: String, task: String) -> Self {
        Self::new(EventType::AgentStarted {
            agent_id,
            task: Some(task),
            timestamp: Utc::now(),
        })
    }

    /// Create agent completed event
    pub fn agent_completed(agent_id: String, result: String) -> Self {
        Self::new(EventType::AgentCompleted {
            agent_id,
            result,
            timestamp: Utc::now(),
        })
    }

    /// Create agent failed event
    pub fn agent_failed(agent_id: String, error: String) -> Self {
        Self::new(EventType::AgentFailed {
            agent_id,
            error,
            timestamp: Utc::now(),
        })
    }

    /// Create agent error recorded event
    pub fn agent_error_recorded(
        agent_id: String,
        error_count: usize,
        error_message: String,
    ) -> Self {
        Self::new(EventType::AgentErrorRecorded {
            agent_id,
            error_count,
            error_message,
            timestamp: Utc::now(),
        })
    }

    /// Create agent restarted event
    pub fn agent_restarted(agent_id: String, reason: String) -> Self {
        Self::new(EventType::AgentRestarted {
            agent_id,
            reason,
            timestamp: Utc::now(),
        })
    }

    /// Create agent health degraded event
    pub fn agent_health_degraded(agent_id: String, error_count: usize, is_healthy: bool) -> Self {
        Self::new(EventType::AgentHealthDegraded {
            agent_id,
            error_count,
            is_healthy,
            timestamp: Utc::now(),
        })
    }

    /// Create memory stored event
    pub fn memory_stored(memory_id: String, summary: String) -> Self {
        Self::new(EventType::MemoryStored {
            memory_id,
            summary,
            timestamp: Utc::now(),
        })
    }

    /// Create memory recalled event
    pub fn memory_recalled(query: String, count: usize) -> Self {
        Self::new(EventType::MemoryRecalled {
            query,
            count,
            timestamp: Utc::now(),
        })
    }

    /// Create context modified event
    pub fn context_modified(file: String) -> Self {
        Self::new(EventType::ContextModified {
            file,
            timestamp: Utc::now(),
        })
    }

    /// Create context validated event
    pub fn context_validated(file: String, errors: Vec<String>) -> Self {
        Self::new(EventType::ContextValidated {
            file,
            errors,
            timestamp: Utc::now(),
        })
    }

    /// Create health update event
    pub fn health_update(memory_mb: f32, cpu_percent: f32) -> Self {
        Self::new(EventType::HealthUpdate {
            memory_mb,
            cpu_percent,
            timestamp: Utc::now(),
        })
    }

    /// Create session started event
    pub fn session_started(instance_id: String) -> Self {
        Self::new(EventType::SessionStarted {
            instance_id: Some(instance_id),
            timestamp: Utc::now(),
        })
    }

    /// Create heartbeat event
    pub fn heartbeat(instance_id: String) -> Self {
        Self::new(EventType::Heartbeat {
            instance_id: Some(instance_id),
            timestamp: Utc::now(),
        })
    }

    /// Create phase changed event
    pub fn phase_changed(from: String, to: String) -> Self {
        Self::new(EventType::PhaseChanged {
            from,
            to,
            timestamp: Utc::now(),
        })
    }

    /// Create deadlock detected event
    pub fn deadlock_detected(blocked_items: Vec<String>) -> Self {
        Self::new(EventType::DeadlockDetected {
            blocked_items,
            timestamp: Utc::now(),
        })
    }

    /// Create context checkpointed event
    pub fn context_checkpointed(agent_id: String, usage_percent: f32, snapshot_id: String) -> Self {
        Self::new(EventType::ContextCheckpointed {
            agent_id,
            usage_percent,
            snapshot_id,
            timestamp: Utc::now(),
        })
    }

    /// Create review failed event
    pub fn review_failed(item_id: String, issues: Vec<String>, attempt: u32) -> Self {
        Self::new(EventType::ReviewFailed {
            item_id,
            issues,
            attempt,
            timestamp: Utc::now(),
        })
    }

    /// Create work item retried event
    pub fn work_item_retried(item_id: String, reason: String, attempt: u32) -> Self {
        Self::new(EventType::WorkItemRetried {
            item_id,
            reason,
            attempt,
            timestamp: Utc::now(),
        })
    }

    /// Create work item assigned event
    pub fn work_item_assigned(agent_id: String, item_id: String, task: String) -> Self {
        Self::new(EventType::WorkItemAssigned {
            agent_id,
            item_id,
            task,
            timestamp: Utc::now(),
        })
    }

    /// Create work item completed event
    pub fn work_item_completed(agent_id: String, item_id: String) -> Self {
        Self::new(EventType::WorkItemCompleted {
            agent_id,
            item_id,
            timestamp: Utc::now(),
        })
    }

    // Skill event constructors
    /// Create skill loaded event
    pub fn skill_loaded(
        skill_name: String,
        agent_id: Option<String>,
        relevance_score: f32,
    ) -> Self {
        Self::new(EventType::SkillLoaded {
            skill_name,
            agent_id,
            relevance_score,
            timestamp: Utc::now(),
        })
    }

    /// Create skill unloaded event
    pub fn skill_unloaded(skill_name: String, reason: String) -> Self {
        Self::new(EventType::SkillUnloaded {
            skill_name,
            reason,
            timestamp: Utc::now(),
        })
    }

    /// Create skill used event
    pub fn skill_used(skill_name: String, agent_id: String) -> Self {
        Self::new(EventType::SkillUsed {
            skill_name,
            agent_id,
            timestamp: Utc::now(),
        })
    }

    /// Create skill composition detected event
    pub fn skill_composition_detected(skills: Vec<String>, task_description: String) -> Self {
        Self::new(EventType::SkillCompositionDetected {
            skills,
            task_description,
            timestamp: Utc::now(),
        })
    }

    // Memory evolution event constructors
    /// Create memory evolution started event
    pub fn memory_evolution_started(reason: String) -> Self {
        Self::new(EventType::MemoryEvolutionStarted {
            reason,
            timestamp: Utc::now(),
        })
    }

    /// Create memory consolidated event
    pub fn memory_consolidated(source_ids: Vec<String>, target_id: String) -> Self {
        Self::new(EventType::MemoryConsolidated {
            source_ids,
            target_id,
            timestamp: Utc::now(),
        })
    }

    /// Create memory decayed event
    pub fn memory_decayed(memory_id: String, old_importance: f32, new_importance: f32) -> Self {
        Self::new(EventType::MemoryDecayed {
            memory_id,
            old_importance,
            new_importance,
            timestamp: Utc::now(),
        })
    }

    /// Create memory archived event
    pub fn memory_archived(memory_id: String, reason: String) -> Self {
        Self::new(EventType::MemoryArchived {
            memory_id,
            reason,
            timestamp: Utc::now(),
        })
    }

    // Agent interaction event constructors
    /// Create agent handoff event
    pub fn agent_handoff(from_agent: String, to_agent: String, task_description: String) -> Self {
        Self::new(EventType::AgentHandoff {
            from_agent,
            to_agent,
            task_description,
            timestamp: Utc::now(),
        })
    }

    /// Create agent blocked event
    pub fn agent_blocked(agent_id: String, blocked_on: String, reason: String) -> Self {
        Self::new(EventType::AgentBlocked {
            agent_id,
            blocked_on,
            reason,
            timestamp: Utc::now(),
        })
    }

    /// Create agent unblocked event
    pub fn agent_unblocked(agent_id: String, unblocked_by: String) -> Self {
        Self::new(EventType::AgentUnblocked {
            agent_id,
            unblocked_by,
            timestamp: Utc::now(),
        })
    }

    /// Create sub-agent spawned event
    pub fn sub_agent_spawned(
        parent_agent: String,
        sub_agent: String,
        task_description: String,
    ) -> Self {
        Self::new(EventType::SubAgentSpawned {
            parent_agent,
            sub_agent,
            task_description,
            timestamp: Utc::now(),
        })
    }

    // Work orchestration event constructors
    /// Create parallel stream started event
    pub fn parallel_stream_started(stream_id: String, task_count: usize) -> Self {
        Self::new(EventType::ParallelStreamStarted {
            stream_id,
            task_count,
            timestamp: Utc::now(),
        })
    }

    /// Create critical path updated event
    pub fn critical_path_updated(path_items: Vec<String>, estimated_completion: String) -> Self {
        Self::new(EventType::CriticalPathUpdated {
            path_items,
            estimated_completion,
            timestamp: Utc::now(),
        })
    }

    /// Create typed hole filled event
    pub fn typed_hole_filled(hole_name: String, component_a: String, component_b: String) -> Self {
        Self::new(EventType::TypedHoleFilled {
            hole_name,
            component_a,
            component_b,
            timestamp: Utc::now(),
        })
    }

    // CLI operation event constructors
    /// Create CLI command started event
    pub fn cli_command_started(command: String, args: Vec<String>) -> Self {
        Self::new(EventType::CliCommandStarted {
            command,
            args,
            timestamp: Utc::now(),
        })
    }

    /// Create CLI command completed event
    pub fn cli_command_completed(
        command: String,
        duration_ms: u64,
        result_summary: String,
    ) -> Self {
        Self::new(EventType::CliCommandCompleted {
            command,
            duration_ms,
            result_summary,
            timestamp: Utc::now(),
        })
    }

    /// Create CLI command failed event
    pub fn cli_command_failed(command: String, error: String, duration_ms: u64) -> Self {
        Self::new(EventType::CliCommandFailed {
            command,
            error,
            duration_ms,
            timestamp: Utc::now(),
        })
    }

    /// Create search performed event
    pub fn search_performed(
        query: String,
        search_type: String,
        result_count: usize,
        duration_ms: u64,
    ) -> Self {
        Self::new(EventType::SearchPerformed {
            query,
            search_type,
            result_count,
            duration_ms,
            timestamp: Utc::now(),
        })
    }

    /// Create database operation event
    pub fn database_operation(
        operation: String,
        table: String,
        affected_rows: usize,
        duration_ms: u64,
    ) -> Self {
        Self::new(EventType::DatabaseOperation {
            operation,
            table,
            affected_rows,
            duration_ms,
            timestamp: Utc::now(),
        })
    }

    /// Create network state update event
    pub fn network_state_update(connected_peers: usize, known_nodes: Vec<String>) -> Self {
        Self::new(EventType::NetworkStateUpdate {
            connected_peers,
            known_nodes,
            timestamp: Utc::now(),
        })
    }

    /// Convert to SSE data format
    pub fn to_sse(&self) -> String {
        format!(
            "id: {}\ndata: {}\n\n",
            self.id,
            serde_json::to_string(&self).unwrap_or_else(|_| "{}".to_string())
        )
    }
}

/// Event broadcaster using tokio broadcast channel
#[derive(Debug, Clone)]
pub struct EventBroadcaster {
    tx: broadcast::Sender<Event>,
}

impl EventBroadcaster {
    /// Create new broadcaster with channel capacity
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Broadcast event to all subscribers
    pub fn broadcast(
        &self,
        event: Event,
    ) -> Result<usize, Box<broadcast::error::SendError<Event>>> {
        self.tx.send(event).map_err(Box::new)
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }

    /// Get subscriber count
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }

    /// Get a clone of the event sender for creating agent bridges
    pub fn sender(&self) -> broadcast::Sender<Event> {
        self.tx.clone()
    }
}

impl Default for EventBroadcaster {
    fn default() -> Self {
        Self::new(1000) // Default capacity: 1000 events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = Event::agent_started("executor".to_string());
        match event.event_type {
            EventType::AgentStarted { agent_id, .. } => {
                assert_eq!(agent_id, "executor");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_sse_format() {
        let event = Event::memory_stored("mem-123".to_string(), "Test memory".to_string());
        let sse = event.to_sse();
        assert!(sse.contains("id:"));
        assert!(sse.contains("data:"));
        assert!(sse.contains("memory_stored"));
    }

    #[tokio::test]
    async fn test_broadcaster() {
        let broadcaster = EventBroadcaster::new(10);
        let mut rx = broadcaster.subscribe();

        let event = Event::agent_started("test".to_string());
        broadcaster.broadcast(event.clone()).unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received.id, event.id);
    }
}
