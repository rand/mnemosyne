//! Event filtering engine for intelligent noise reduction
//!
//! Provides smart filtering to transform noisy event streams into actionable signals.
//! Default behavior: hide heartbeats, show everything else.

use mnemosyne_core::api::events::{Event, EventType};
use regex::Regex;
use std::time::Duration;

/// Event category for high-level filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventCategory {
    /// CLI command events (cli_command_*)
    Cli,
    /// Memory operations (memory_stored, memory_recalled, memory_evolution_*)
    Memory,
    /// Agent lifecycle (agent_started, agent_completed, agent_failed, etc.)
    Agent,
    /// Skill loading/usage (skill_loaded, skill_used, skill_unloaded)
    Skill,
    /// Work orchestration (work_item_*, phase_changed, deadlock_*)
    Work,
    /// Context management (context_modified, context_validated, context_checkpointed)
    Context,
    /// System health (health_update, heartbeat, session_started)
    System,
    /// Errors and failures (all *_failed, *_error_*, review_failed)
    Error,
}

impl EventCategory {
    /// Determine category from event type
    pub fn from_event(event: &Event) -> Self {
        match &event.event_type {
            // CLI events
            EventType::CliCommandStarted { .. }
            | EventType::CliCommandCompleted { .. }
            | EventType::CliCommandFailed { .. } => Self::Cli,

            // Memory events
            EventType::MemoryStored { .. }
            | EventType::MemoryRecalled { .. }
            | EventType::MemoryEvolutionStarted { .. }
            | EventType::MemoryConsolidated { .. }
            | EventType::MemoryDecayed { .. }
            | EventType::MemoryArchived { .. }
            | EventType::SearchPerformed { .. } => Self::Memory,

            // Agent events
            EventType::AgentStarted { .. }
            | EventType::AgentCompleted { .. }
            | EventType::AgentHandoff { .. }
            | EventType::AgentBlocked { .. }
            | EventType::AgentUnblocked { .. }
            | EventType::AgentRestarted { .. }
            | EventType::SubAgentSpawned { .. } => Self::Agent,

            // Error events (highest priority)
            EventType::AgentFailed { .. }
            | EventType::AgentErrorRecorded { .. }
            | EventType::AgentHealthDegraded { .. }
            | EventType::ReviewFailed { .. }
            | EventType::CliCommandFailed { .. }
            | EventType::DeadlockDetected { .. } => Self::Error,

            // Skill events
            EventType::SkillLoaded { .. }
            | EventType::SkillUnloaded { .. }
            | EventType::SkillUsed { .. }
            | EventType::SkillCompositionDetected { .. } => Self::Skill,

            // Work events
            EventType::WorkItemAssigned { .. }
            | EventType::WorkItemCompleted { .. }
            | EventType::WorkItemRetried { .. }
            | EventType::PhaseChanged { .. }
            | EventType::ParallelStreamStarted { .. }
            | EventType::CriticalPathUpdated { .. }
            | EventType::TypedHoleFilled { .. } => Self::Work,

            // Context events
            EventType::ContextModified { .. }
            | EventType::ContextValidated { .. }
            | EventType::ContextCheckpointed { .. } => Self::Context,

            // System events
            EventType::HealthUpdate { .. }
            | EventType::SessionStarted { .. }
            | EventType::Heartbeat { .. }
            | EventType::DatabaseOperation { .. } 
            | EventType::NetworkStateUpdate { .. } => Self::System,
        }
    }

    /// Get category display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Cli => "CLI",
            Self::Memory => "Memory",
            Self::Agent => "Agent",
            Self::Skill => "Skill",
            Self::Work => "Work",
            Self::Context => "Context",
            Self::System => "System",
            Self::Error => "Error",
        }
    }
}

/// Event filter specification
#[derive(Debug, Clone)]
pub enum EventFilter {
    /// Hide all heartbeat events
    HideHeartbeats,
    /// Show only error/failure events
    ErrorsOnly,
    /// Show events from specific category
    Category(EventCategory),
    /// Show events from specific agent
    AgentId(String),
    /// Search event content with regex
    Search(Regex),
    /// Time range filter (events within last N duration)
    TimeRange(Duration),
    /// Compound filter (all must match - AND logic)
    All(Vec<EventFilter>),
    /// Compound filter (any must match - OR logic)
    Any(Vec<EventFilter>),
    /// Negation filter
    Not(Box<EventFilter>),
}

impl EventFilter {
    /// Check if event passes this filter
    pub fn matches(&self, event: &Event) -> bool {
        match self {
            Self::HideHeartbeats => !matches!(event.event_type, EventType::Heartbeat { .. }),

            Self::ErrorsOnly => EventCategory::from_event(event) == EventCategory::Error,

            Self::Category(cat) => EventCategory::from_event(event) == *cat,

            Self::AgentId(agent_id) => Self::extract_agent_id(event)
                .map(|id| &id == agent_id)
                .unwrap_or(false),

            Self::Search(regex) => {
                // Search across serialized event content
                let event_json = serde_json::to_string(&event.event_type).unwrap_or_default();
                regex.is_match(&event_json)
            }

            Self::TimeRange(duration) => {
                if let Some(event_time) = Self::extract_timestamp(event) {
                    let now = chrono::Utc::now();
                    let age = now.signed_duration_since(event_time);
                    age.to_std().map(|d| d <= *duration).unwrap_or(false)
                } else {
                    true // No timestamp, let it through
                }
            }

            Self::All(filters) => filters.iter().all(|f| f.matches(event)),

            Self::Any(filters) => filters.iter().any(|f| f.matches(event)),

            Self::Not(filter) => !filter.matches(event),
        }
    }

    /// Extract agent ID from event if present
    fn extract_agent_id(event: &Event) -> Option<String> {
        match &event.event_type {
            EventType::AgentStarted { agent_id, .. }
            | EventType::AgentCompleted { agent_id, .. }
            | EventType::AgentFailed { agent_id, .. }
            | EventType::AgentErrorRecorded { agent_id, .. }
            | EventType::AgentRestarted { agent_id, .. }
            | EventType::AgentHealthDegraded { agent_id, .. }
            | EventType::WorkItemAssigned { agent_id, .. }
            | EventType::WorkItemCompleted { agent_id, .. }
            | EventType::AgentBlocked { agent_id, .. }
            | EventType::AgentUnblocked { agent_id, .. }
            | EventType::ContextCheckpointed { agent_id, .. }
            | EventType::SkillUsed { agent_id, .. } => Some(agent_id.clone()),

            EventType::SkillLoaded { agent_id, .. } => agent_id.clone(),

            EventType::AgentHandoff { to_agent, .. } => Some(to_agent.clone()),

            EventType::SubAgentSpawned {
                sub_agent, ..
            } => Some(sub_agent.clone()),

            _ => None,
        }
    }

    /// Extract timestamp from event
    fn extract_timestamp(event: &Event) -> Option<chrono::DateTime<chrono::Utc>> {
        use EventType::*;
        match &event.event_type {
            AgentStarted { timestamp, .. }
            | AgentCompleted { timestamp, .. }
            | AgentFailed { timestamp, .. }
            | MemoryStored { timestamp, .. }
            | MemoryRecalled { timestamp, .. }
            | ContextModified { timestamp, .. }
            | ContextValidated { timestamp, .. }
            | HealthUpdate { timestamp, .. }
            | SessionStarted { timestamp, .. }
            | Heartbeat { timestamp, .. }
            | PhaseChanged { timestamp, .. }
            | DeadlockDetected { timestamp, .. }
            | ContextCheckpointed { timestamp, .. }
            | ReviewFailed { timestamp, .. }
            | WorkItemRetried { timestamp, .. }
            | AgentErrorRecorded { timestamp, .. }
            | AgentRestarted { timestamp, .. }
            | AgentHealthDegraded { timestamp, .. }
            | WorkItemAssigned { timestamp, .. }
            | WorkItemCompleted { timestamp, .. }
            | SkillLoaded { timestamp, .. }
            | SkillUnloaded { timestamp, .. }
            | SkillUsed { timestamp, .. }
            | SkillCompositionDetected { timestamp, .. }
            | MemoryEvolutionStarted { timestamp, .. }
            | MemoryConsolidated { timestamp, .. }
            | MemoryDecayed { timestamp, .. }
            | MemoryArchived { timestamp, .. }
            | AgentHandoff { timestamp, .. }
            | AgentBlocked { timestamp, .. }
            | AgentUnblocked { timestamp, .. }
            | SubAgentSpawned { timestamp, .. }
            | ParallelStreamStarted { timestamp, .. }
            | CriticalPathUpdated { timestamp, .. }
            | TypedHoleFilled { timestamp, .. }
            | CliCommandStarted { timestamp, .. }
            | CliCommandCompleted { timestamp, .. }
            | CliCommandFailed { timestamp, .. }
            | SearchPerformed { timestamp, .. }
            | DatabaseOperation { timestamp, .. } 
            | NetworkStateUpdate { timestamp, .. } => Some(*timestamp),
        }
    }
}

/// Filter presets for common use cases
pub struct FilterPresets;

impl FilterPresets {
    /// Default filter: hide heartbeats only
    pub fn default() -> EventFilter {
        EventFilter::HideHeartbeats
    }

    /// Errors only: show failures, errors, degraded health
    pub fn errors_only() -> EventFilter {
        EventFilter::ErrorsOnly
    }

    /// CLI focus: show CLI commands only
    pub fn cli_focus() -> EventFilter {
        EventFilter::Category(EventCategory::Cli)
    }

    /// Memory focus: show memory operations only
    pub fn memory_focus() -> EventFilter {
        EventFilter::Category(EventCategory::Memory)
    }

    /// Agent focus: show specific agent's events
    pub fn agent_focus(agent_id: impl Into<String>) -> EventFilter {
        EventFilter::All(vec![
            EventFilter::HideHeartbeats,
            EventFilter::AgentId(agent_id.into()),
        ])
    }

    /// Activity focus: hide heartbeats and system events
    pub fn activity_focus() -> EventFilter {
        EventFilter::All(vec![
            EventFilter::HideHeartbeats,
            EventFilter::Not(Box::new(EventFilter::Category(EventCategory::System))),
        ])
    }

    /// Recent only: events from last N minutes
    pub fn recent(minutes: u64) -> EventFilter {
        EventFilter::All(vec![
            EventFilter::HideHeartbeats,
            EventFilter::TimeRange(Duration::from_secs(minutes * 60)),
        ])
    }

    /// High priority: errors + phase changes + deadlocks
    pub fn high_priority() -> EventFilter {
        EventFilter::Any(vec![
            EventFilter::ErrorsOnly,
            EventFilter::Category(EventCategory::Work),
        ])
    }

    /// Error focus: show only Error category events + failures, hide heartbeats
    pub fn error_focus() -> EventFilter {
        EventFilter::All(vec![
            EventFilter::HideHeartbeats,
            EventFilter::ErrorsOnly,
        ])
    }
}

/// Filter statistics
#[derive(Debug, Default, Clone)]
pub struct FilterStats {
    /// Total events received
    pub total: usize,
    /// Events that passed filter
    pub passed: usize,
    /// Events filtered out
    pub filtered: usize,
}

impl FilterStats {
    /// Record an event
    pub fn record(&mut self, passed: bool) {
        self.total += 1;
        if passed {
            self.passed += 1;
        } else {
            self.filtered += 1;
        }
    }

    /// Get pass rate as percentage
    pub fn pass_rate(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            (self.passed as f32 / self.total as f32) * 100.0
        }
    }

    /// Reset statistics
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mnemosyne_core::api::events::Event;

    #[test]
    fn test_hide_heartbeats() {
        let filter = EventFilter::HideHeartbeats;

        let heartbeat = Event::new(EventType::Heartbeat {
            instance_id: "test".to_string(),
            timestamp: chrono::Utc::now(),
        });

        let memory = Event::new(EventType::MemoryStored {
            memory_id: "mem-123".to_string(),
            summary: "Test".to_string(),
            timestamp: chrono::Utc::now(),
        });

        assert!(!filter.matches(&heartbeat));
        assert!(filter.matches(&memory));
    }

    #[test]
    fn test_errors_only() {
        let filter = EventFilter::ErrorsOnly;

        let error = Event::new(EventType::AgentFailed {
            agent_id: "agent-1".to_string(),
            error: "Test error".to_string(),
            timestamp: chrono::Utc::now(),
        });

        let success = Event::new(EventType::AgentCompleted {
            agent_id: "agent-1".to_string(),
            result: "Success".to_string(),
            timestamp: chrono::Utc::now(),
        });

        assert!(filter.matches(&error));
        assert!(!filter.matches(&success));
    }

    #[test]
    fn test_category_filter() {
        let filter = EventFilter::Category(EventCategory::Memory);

        let memory_event = Event::new(EventType::MemoryRecalled {
            query: "test".to_string(),
            count: 5,
            timestamp: chrono::Utc::now(),
        });

        let agent_event = Event::new(EventType::AgentStarted {
            agent_id: "agent-1".to_string(),
            task: None,
            timestamp: chrono::Utc::now(),
        });

        assert!(filter.matches(&memory_event));
        assert!(!filter.matches(&agent_event));
    }

    #[test]
    fn test_agent_id_filter() {
        let filter = EventFilter::AgentId("executor".to_string());

        let matching = Event::new(EventType::AgentStarted {
            agent_id: "executor".to_string(),
            task: None,
            timestamp: chrono::Utc::now(),
        });

        let non_matching = Event::new(EventType::AgentStarted {
            agent_id: "optimizer".to_string(),
            task: None,
            timestamp: chrono::Utc::now(),
        });

        assert!(filter.matches(&matching));
        assert!(!filter.matches(&non_matching));
    }

    #[test]
    fn test_compound_filter_all() {
        let filter = EventFilter::All(vec![
            EventFilter::HideHeartbeats,
            EventFilter::Category(EventCategory::Memory),
        ]);

        let memory_event = Event::new(EventType::MemoryStored {
            memory_id: "mem-123".to_string(),
            summary: "Test".to_string(),
            timestamp: chrono::Utc::now(),
        });

        let heartbeat = Event::new(EventType::Heartbeat {
            instance_id: "test".to_string(),
            timestamp: chrono::Utc::now(),
        });

        assert!(filter.matches(&memory_event));
        assert!(!filter.matches(&heartbeat));
    }

    #[test]
    fn test_filter_stats() {
        let mut stats = FilterStats::default();

        stats.record(true);
        stats.record(true);
        stats.record(false);

        assert_eq!(stats.total, 3);
        assert_eq!(stats.passed, 2);
        assert_eq!(stats.filtered, 1);
        assert!((stats.pass_rate() - 66.66).abs() < 0.1);
    }

    #[test]
    fn test_filter_presets() {
        // Just ensure presets compile and return filters
        let _ = FilterPresets::default();
        let _ = FilterPresets::errors_only();
        let _ = FilterPresets::cli_focus();
        let _ = FilterPresets::memory_focus();
        let _ = FilterPresets::agent_focus("test");
        let _ = FilterPresets::activity_focus();
        let _ = FilterPresets::recent(5);
        let _ = FilterPresets::high_priority();
        let _ = FilterPresets::error_focus();
    }

    #[test]
    fn test_error_focus_preset() {
        let filter = FilterPresets::error_focus();

        // Should hide heartbeats
        let heartbeat = Event::new(EventType::Heartbeat {
            instance_id: "test".to_string(),
            timestamp: chrono::Utc::now(),
        });
        assert!(!filter.matches(&heartbeat));

        // Should show errors
        let error = Event::new(EventType::AgentFailed {
            agent_id: "agent-1".to_string(),
            error: "Test error".to_string(),
            timestamp: chrono::Utc::now(),
        });
        assert!(filter.matches(&error));

        // Should not show normal events
        let normal = Event::new(EventType::MemoryStored {
            memory_id: "mem-123".to_string(),
            summary: "Test".to_string(),
            timestamp: chrono::Utc::now(),
        });
        assert!(!filter.matches(&normal));
    }

    #[test]
    fn test_agent_focus_preset_hides_heartbeats() {
        let filter = FilterPresets::agent_focus("executor");

        // Should hide heartbeats even from the focused agent
        let heartbeat = Event::new(EventType::Heartbeat {
            instance_id: "test".to_string(),
            timestamp: chrono::Utc::now(),
        });
        assert!(!filter.matches(&heartbeat));
    }
}
