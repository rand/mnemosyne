//! Event correlation engine for linking related events
//!
//! Transforms raw event streams into meaningful operation timelines by correlating
//! start/complete events, calculating durations, and tracking outcomes.

use mnemosyne_core::api::events::{Event, EventType};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::time::Duration;

/// Correlation key for matching related events
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CorrelationKey {
    /// CLI command (by command name)
    CliCommand(String),
    /// Agent operation (by agent ID)
    Agent(String),
    /// Memory evolution (instance-wide)
    MemoryEvolution,
    /// Work item (by item ID)
    WorkItem(String),
}

/// Status of a correlated operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationStatus {
    /// Operation started, waiting for completion
    InProgress,
    /// Operation completed successfully
    Completed,
    /// Operation failed
    Failed,
}

/// A correlated event pair (or ongoing operation)
#[derive(Debug, Clone)]
pub struct CorrelatedEvent {
    /// Correlation key
    pub key: CorrelationKey,
    /// Start event
    pub start: Event,
    /// End event (if completed/failed)
    pub end: Option<Event>,
    /// Operation status
    pub status: OperationStatus,
    /// Duration (if completed)
    pub duration: Option<Duration>,
    /// Start timestamp
    pub started_at: DateTime<Utc>,
    /// End timestamp (if completed)
    pub ended_at: Option<DateTime<Utc>>,
}

impl CorrelatedEvent {
    /// Create new in-progress correlated event
    fn new(key: CorrelationKey, start: Event, started_at: DateTime<Utc>) -> Self {
        Self {
            key,
            start,
            end: None,
            status: OperationStatus::InProgress,
            duration: None,
            started_at,
            ended_at: None,
        }
    }

    /// Complete this correlated event
    fn complete(&mut self, end: Event, ended_at: DateTime<Utc>, failed: bool) {
        self.end = Some(end);
        self.status = if failed {
            OperationStatus::Failed
        } else {
            OperationStatus::Completed
        };
        self.ended_at = Some(ended_at);

        // Calculate duration
        let duration = ended_at.signed_duration_since(self.started_at);
        self.duration = duration.to_std().ok();
    }

    /// Get human-readable description
    pub fn description(&self) -> String {
        match &self.key {
            CorrelationKey::CliCommand(cmd) => format!("CLI: {}", cmd),
            CorrelationKey::Agent(agent_id) => format!("Agent: {}", agent_id),
            CorrelationKey::MemoryEvolution => "Memory Evolution".to_string(),
            CorrelationKey::WorkItem(item_id) => format!("Work: {}", item_id),
        }
    }

    /// Get duration as milliseconds
    pub fn duration_ms(&self) -> Option<u64> {
        self.duration.map(|d| d.as_millis() as u64)
    }

    /// Check if operation is slow (>1s for CLI, >5s for agent, >10s for evolution)
    pub fn is_slow(&self) -> bool {
        if let Some(duration) = &self.duration {
            match &self.key {
                CorrelationKey::CliCommand(_) => duration.as_secs() > 1,
                CorrelationKey::Agent(_) => duration.as_secs() > 5,
                CorrelationKey::MemoryEvolution => duration.as_secs() > 10,
                CorrelationKey::WorkItem(_) => duration.as_secs() > 2,
            }
        } else {
            false
        }
    }
}

/// Event correlation tracker
pub struct CorrelationTracker {
    /// Pending operations (started but not completed)
    pending: HashMap<CorrelationKey, CorrelatedEvent>,
    /// Completed operations (for history)
    completed: Vec<CorrelatedEvent>,
    /// Maximum history to keep
    max_history: usize,
}

impl CorrelationTracker {
    /// Create new correlation tracker
    pub fn new(max_history: usize) -> Self {
        Self {
            pending: HashMap::new(),
            completed: Vec::new(),
            max_history,
        }
    }

    /// Process an event and return correlated event if this completes an operation
    pub fn process(&mut self, event: Event) -> Option<CorrelatedEvent> {
        // Try to extract correlation info
        if let Some((key, _started, timestamp)) = Self::extract_start(&event) {
            // This is a start event - track it
            let correlated = CorrelatedEvent::new(key.clone(), event, timestamp);
            self.pending.insert(key, correlated);
            None
        } else if let Some((key, timestamp, failed)) = Self::extract_end(&event) {
            // This is an end event - try to match with pending
            if let Some(mut correlated) = self.pending.remove(&key) {
                correlated.complete(event, timestamp, failed);

                // Add to completed history
                self.completed.push(correlated.clone());

                // Trim history if needed
                if self.completed.len() > self.max_history {
                    self.completed.remove(0);
                }

                Some(correlated)
            } else {
                // No matching start event - orphaned end event
                None
            }
        } else {
            // Not a correlatable event
            None
        }
    }

    /// Extract start event information
    fn extract_start(event: &Event) -> Option<(CorrelationKey, bool, DateTime<Utc>)> {
        match &event.event_type {
            EventType::CliCommandStarted {
                command, timestamp, ..
            } => Some((
                CorrelationKey::CliCommand(command.clone()),
                true,
                *timestamp,
            )),

            EventType::AgentStarted {
                agent_id,
                timestamp,
                ..
            } => Some((
                CorrelationKey::Agent(agent_id.clone()),
                true,
                *timestamp,
            )),

            EventType::MemoryEvolutionStarted { timestamp, .. } => {
                Some((CorrelationKey::MemoryEvolution, true, *timestamp))
            }

            EventType::WorkItemAssigned {
                item_id, timestamp, ..
            } => Some((
                CorrelationKey::WorkItem(item_id.clone()),
                true,
                *timestamp,
            )),

            _ => None,
        }
    }

    /// Extract end event information (returns key, timestamp, failed)
    fn extract_end(event: &Event) -> Option<(CorrelationKey, DateTime<Utc>, bool)> {
        match &event.event_type {
            EventType::CliCommandCompleted {
                command, timestamp, ..
            } => Some((
                CorrelationKey::CliCommand(command.clone()),
                *timestamp,
                false,
            )),

            EventType::CliCommandFailed {
                command, timestamp, ..
            } => Some((
                CorrelationKey::CliCommand(command.clone()),
                *timestamp,
                true,
            )),

            EventType::AgentCompleted {
                agent_id,
                timestamp,
                ..
            } => Some((
                CorrelationKey::Agent(agent_id.clone()),
                *timestamp,
                false,
            )),

            EventType::AgentFailed {
                agent_id,
                timestamp,
                ..
            } => Some((
                CorrelationKey::Agent(agent_id.clone()),
                *timestamp,
                true,
            )),

            EventType::WorkItemCompleted {
                item_id, timestamp, ..
            } => Some((
                CorrelationKey::WorkItem(item_id.clone()),
                *timestamp,
                false,
            )),

            _ => None,
        }
    }

    /// Get all pending operations
    pub fn pending_operations(&self) -> Vec<&CorrelatedEvent> {
        self.pending.values().collect()
    }

    /// Get completed operation history
    pub fn completed_operations(&self) -> &[CorrelatedEvent] {
        &self.completed
    }

    /// Get recent completed operations (last N)
    pub fn recent_completed(&self, n: usize) -> &[CorrelatedEvent] {
        let start = self.completed.len().saturating_sub(n);
        &self.completed[start..]
    }

    /// Clear all pending operations (useful on reconnect)
    pub fn clear_pending(&mut self) {
        self.pending.clear();
    }

    /// Get statistics
    pub fn stats(&self) -> CorrelationStats {
        let mut stats = CorrelationStats::default();

        stats.pending_count = self.pending.len();
        stats.completed_count = self.completed.len();

        for correlated in &self.completed {
            if correlated.status == OperationStatus::Failed {
                stats.failed_count += 1;
            }
            if correlated.is_slow() {
                stats.slow_count += 1;
            }

            // Track by type
            match &correlated.key {
                CorrelationKey::CliCommand(_) => stats.cli_operations += 1,
                CorrelationKey::Agent(_) => stats.agent_operations += 1,
                CorrelationKey::MemoryEvolution => stats.evolution_operations += 1,
                CorrelationKey::WorkItem(_) => stats.work_operations += 1,
            }
        }

        stats
    }
}

/// Correlation statistics
#[derive(Debug, Default, Clone)]
pub struct CorrelationStats {
    /// Currently pending operations
    pub pending_count: usize,
    /// Total completed operations
    pub completed_count: usize,
    /// Failed operations
    pub failed_count: usize,
    /// Slow operations
    pub slow_count: usize,
    /// CLI operations
    pub cli_operations: usize,
    /// Agent operations
    pub agent_operations: usize,
    /// Evolution operations
    pub evolution_operations: usize,
    /// Work operations
    pub work_operations: usize,
}

impl CorrelationStats {
    /// Get success rate as percentage
    pub fn success_rate(&self) -> f32 {
        if self.completed_count == 0 {
            0.0
        } else {
            let success = self.completed_count - self.failed_count;
            (success as f32 / self.completed_count as f32) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_cli_started(cmd: &str) -> Event {
        Event::new(EventType::CliCommandStarted {
            command: cmd.to_string(),
            args: vec![],
            timestamp: Utc::now(),
        })
    }

    fn create_cli_completed(cmd: &str) -> Event {
        Event::new(EventType::CliCommandCompleted {
            command: cmd.to_string(),
            duration_ms: 123,
            result_summary: "Success".to_string(),
            timestamp: Utc::now(),
        })
    }

    fn create_cli_failed(cmd: &str) -> Event {
        Event::new(EventType::CliCommandFailed {
            command: cmd.to_string(),
            error: "Test error".to_string(),
            duration_ms: 100,
            timestamp: Utc::now(),
        })
    }

    #[test]
    fn test_correlation_success() {
        let mut tracker = CorrelationTracker::new(100);

        // Start event - should not return correlated event yet
        let start = create_cli_started("remember");
        assert!(tracker.process(start).is_none());
        assert_eq!(tracker.pending.len(), 1);

        // Complete event - should return correlated event
        let complete = create_cli_completed("remember");
        let correlated = tracker.process(complete);

        assert!(correlated.is_some());
        let correlated = correlated.unwrap();
        assert_eq!(correlated.status, OperationStatus::Completed);
        assert!(correlated.duration.is_some());
        assert_eq!(tracker.pending.len(), 0);
        assert_eq!(tracker.completed.len(), 1);
    }

    #[test]
    fn test_correlation_failure() {
        let mut tracker = CorrelationTracker::new(100);

        let start = create_cli_started("recall");
        tracker.process(start);

        let failed = create_cli_failed("recall");
        let correlated = tracker.process(failed);

        assert!(correlated.is_some());
        let correlated = correlated.unwrap();
        assert_eq!(correlated.status, OperationStatus::Failed);
    }

    #[test]
    fn test_orphaned_end_event() {
        let mut tracker = CorrelationTracker::new(100);

        // Complete event without start - should not correlate
        let complete = create_cli_completed("remember");
        assert!(tracker.process(complete).is_none());
        assert_eq!(tracker.completed.len(), 0);
    }

    #[test]
    fn test_multiple_operations() {
        let mut tracker = CorrelationTracker::new(100);

        // Start two different operations
        tracker.process(create_cli_started("remember"));
        tracker.process(create_cli_started("recall"));
        assert_eq!(tracker.pending.len(), 2);

        // Complete one
        tracker.process(create_cli_completed("remember"));
        assert_eq!(tracker.pending.len(), 1);
        assert_eq!(tracker.completed.len(), 1);

        // Complete the other
        tracker.process(create_cli_completed("recall"));
        assert_eq!(tracker.pending.len(), 0);
        assert_eq!(tracker.completed.len(), 2);
    }

    #[test]
    fn test_history_trimming() {
        let mut tracker = CorrelationTracker::new(3); // Max 3

        // Add 5 operations
        for i in 0..5 {
            tracker.process(create_cli_started("remember"));
            tracker.process(create_cli_completed("remember"));
        }

        // Should only keep last 3
        assert_eq!(tracker.completed.len(), 3);
    }

    #[test]
    fn test_correlation_stats() {
        let mut tracker = CorrelationTracker::new(100);

        // Add success
        tracker.process(create_cli_started("remember"));
        tracker.process(create_cli_completed("remember"));

        // Add failure
        tracker.process(create_cli_started("recall"));
        tracker.process(create_cli_failed("recall"));

        let stats = tracker.stats();
        assert_eq!(stats.completed_count, 2);
        assert_eq!(stats.failed_count, 1);
        assert_eq!(stats.cli_operations, 2);
        assert!((stats.success_rate() - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_clear_pending() {
        let mut tracker = CorrelationTracker::new(100);

        tracker.process(create_cli_started("remember"));
        tracker.process(create_cli_started("recall"));
        assert_eq!(tracker.pending.len(), 2);

        tracker.clear_pending();
        assert_eq!(tracker.pending.len(), 0);
    }

    #[test]
    fn test_recent_completed() {
        let mut tracker = CorrelationTracker::new(100);

        for _ in 0..5 {
            tracker.process(create_cli_started("remember"));
            tracker.process(create_cli_completed("remember"));
        }

        let recent = tracker.recent_completed(3);
        assert_eq!(recent.len(), 3);
    }
}
