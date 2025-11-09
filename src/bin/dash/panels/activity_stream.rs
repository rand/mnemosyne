//! Activity Stream panel - Intelligent event log with filtering
//!
//! The centerpiece panel that transforms noisy event streams into actionable signals.
//! Default behavior: hide heartbeats, show everything else.
//!
//! Features:
//! - Smart event filtering (default: hide heartbeats)
//! - Event correlation (link start→complete with durations)
//! - Color-coded event categories
//! - Relative timestamps ("2s ago", "1m ago")
//! - Auto-scroll with recent events at bottom
//! - Ring buffer to prevent unlimited growth

use crate::correlation::{CorrelatedEvent, CorrelationTracker, OperationStatus};
use crate::filters::{EventCategory, EventFilter, FilterStats};
use mnemosyne_core::api::events::Event;
use chrono::{DateTime, Utc};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};
use std::collections::VecDeque;

/// Maximum events to retain in ring buffer
const MAX_EVENTS: usize = 1000;

/// Activity stream entry (either raw event or correlated operation)
#[derive(Debug, Clone)]
enum ActivityEntry {
    /// Raw event (not part of a correlation)
    RawEvent { event: Event, timestamp: DateTime<Utc> },
    /// Correlated operation (start→complete)
    CorrelatedOperation(CorrelatedEvent),
}

impl ActivityEntry {
    /// Get timestamp for sorting
    fn timestamp(&self) -> DateTime<Utc> {
        match self {
            ActivityEntry::RawEvent { timestamp, .. } => *timestamp,
            ActivityEntry::CorrelatedOperation(corr) => corr.started_at,
        }
    }

    /// Get event for filtering
    fn event(&self) -> Option<&Event> {
        match self {
            ActivityEntry::RawEvent { event, .. } => Some(event),
            ActivityEntry::CorrelatedOperation(_) => None,
        }
    }
}

/// Activity Stream panel widget
pub struct ActivityStreamPanel {
    /// Ring buffer of activity entries
    entries: VecDeque<ActivityEntry>,
    /// Event filter (default: hide heartbeats)
    filter: EventFilter,
    /// Correlation tracker
    correlation_tracker: CorrelationTracker,
    /// Filter statistics
    filter_stats: FilterStats,
    /// Maximum entries to retain
    max_entries: usize,
}

impl ActivityStreamPanel {
    /// Create new activity stream panel
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            filter: EventFilter::HideHeartbeats, // Default: hide heartbeats only
            correlation_tracker: CorrelationTracker::new(100),
            filter_stats: FilterStats::default(),
            max_entries: MAX_EVENTS,
        }
    }

    /// Update event filter
    pub fn set_filter(&mut self, filter: EventFilter) {
        self.filter = filter;
        self.filter_stats.reset();
    }

    /// Get current filter
    pub fn filter(&self) -> &EventFilter {
        &self.filter
    }

    /// Get filter statistics
    pub fn filter_stats(&self) -> &FilterStats {
        &self.filter_stats
    }

    /// Add an event to the activity stream
    pub fn add_event(&mut self, event: Event) {
        // Try to correlate event
        if let Some(correlated) = self.correlation_tracker.process(event.clone()) {
            // Successfully correlated - add as operation
            self.entries.push_back(ActivityEntry::CorrelatedOperation(correlated));
        } else if !Self::is_start_event(&event) {
            // Not a start event and not correlated - add as raw event
            // (start events are tracked internally by correlation tracker, don't display until complete)
            let timestamp = Self::extract_timestamp(&event).unwrap_or_else(Utc::now);

            // Apply filter
            let passes = self.filter.matches(&event);
            self.filter_stats.record(passes);

            if passes {
                self.entries.push_back(ActivityEntry::RawEvent { event, timestamp });
            }
        }
        // If it's a start event, it's now tracked by correlation tracker, don't display yet

        // Trim to max entries
        while self.entries.len() > self.max_entries {
            self.entries.pop_front();
        }
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
        self.correlation_tracker.clear_pending();
        self.filter_stats.reset();
    }

    /// Get entry count
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Check if event is a start event (tracked for correlation)
    fn is_start_event(event: &Event) -> bool {
        use mnemosyne_core::api::events::EventType::*;
        matches!(
            &event.event_type,
            CliCommandStarted { .. }
                | AgentStarted { .. }
                | MemoryEvolutionStarted { .. }
                | WorkItemAssigned { .. }
        )
    }

    /// Extract timestamp from event
    fn extract_timestamp(event: &Event) -> Option<DateTime<Utc>> {
        use mnemosyne_core::api::events::EventType::*;
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
            | DatabaseOperation { timestamp, .. } => Some(*timestamp),
        }
    }

    /// Render the activity stream panel
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = if self.entries.is_empty() {
            // Empty state
            vec![
                ListItem::new(Line::from(vec![
                    Span::styled("No events yet", Style::default().fg(Color::DarkGray)),
                ])),
                ListItem::new(Line::from(vec![
                    Span::styled("Waiting for activity...", Style::default().fg(Color::DarkGray)),
                ])),
            ]
        } else {
            // Render entries (most recent at bottom for auto-scroll)
            self.entries
                .iter()
                .map(|entry| self.render_entry(entry))
                .collect()
        };

        // Calculate filter stats for title
        let title = if self.filter_stats.total > 0 {
            format!(
                "Activity Stream ({}/{} events, {:.0}% pass rate)",
                self.filter_stats.passed,
                self.filter_stats.total,
                self.filter_stats.pass_rate()
            )
        } else {
            "Activity Stream".to_string()
        };

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::Cyan)),
        );

        frame.render_widget(list, area);
    }

    /// Render a single activity entry
    fn render_entry(&self, entry: &ActivityEntry) -> ListItem {
        match entry {
            ActivityEntry::RawEvent { event, timestamp } => self.render_raw_event(event, *timestamp),
            ActivityEntry::CorrelatedOperation(corr) => self.render_correlated_operation(corr),
        }
    }

    /// Render a raw event
    fn render_raw_event(&self, event: &Event, timestamp: DateTime<Utc>) -> ListItem {
        let category = EventCategory::from_event(event);
        let category_color = Self::category_color(&category);
        let category_prefix = Self::category_prefix(&category);

        let relative_time = Self::format_relative_time(timestamp);
        let description = Self::event_description(event);

        ListItem::new(Line::from(vec![
            Span::styled(
                format!("{:>6} ", relative_time),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("[{}] ", category_prefix),
                Style::default()
                    .fg(category_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(description, Style::default().fg(Color::White)),
        ]))
    }

    /// Render a correlated operation
    fn render_correlated_operation(&self, corr: &CorrelatedEvent) -> ListItem {
        let relative_time = Self::format_relative_time(corr.started_at);
        let description = corr.description();

        // Status color and symbol
        let (status_color, status_symbol) = match corr.status {
            OperationStatus::InProgress => (Color::Yellow, "⟳"),
            OperationStatus::Completed => (Color::Green, "✓"),
            OperationStatus::Failed => (Color::Red, "✗"),
        };

        // Duration string (if completed)
        let duration_str = if let Some(duration_ms) = corr.duration_ms() {
            format!(" ({}ms)", duration_ms)
        } else {
            String::new()
        };

        ListItem::new(Line::from(vec![
            Span::styled(
                format!("{:>6} ", relative_time),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("[{}] ", status_symbol),
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(description, Style::default().fg(Color::White)),
            Span::styled(
                duration_str,
                Style::default().fg(if corr.is_slow() {
                    Color::Yellow
                } else {
                    Color::DarkGray
                }),
            ),
        ]))
    }

    /// Get color for event category
    fn category_color(category: &EventCategory) -> Color {
        match category {
            EventCategory::Cli => Color::Blue,
            EventCategory::Memory => Color::Magenta,
            EventCategory::Agent => Color::Yellow,
            EventCategory::Skill => Color::Cyan,
            EventCategory::Work => Color::Green,
            EventCategory::Context => Color::LightBlue,
            EventCategory::System => Color::DarkGray,
            EventCategory::Error => Color::Red,
        }
    }

    /// Get prefix symbol for event category
    fn category_prefix(category: &EventCategory) -> &'static str {
        match category {
            EventCategory::Cli => "CLI",
            EventCategory::Memory => "MEM",
            EventCategory::Agent => "AGT",
            EventCategory::Skill => "SKL",
            EventCategory::Work => "WRK",
            EventCategory::Context => "CTX",
            EventCategory::System => "SYS",
            EventCategory::Error => "ERR",
        }
    }

    /// Format timestamp as relative time
    fn format_relative_time(timestamp: DateTime<Utc>) -> String {
        let now = Utc::now();
        let age = now.signed_duration_since(timestamp);

        if age.num_seconds() < 1 {
            "now".to_string()
        } else if age.num_seconds() < 60 {
            format!("{}s", age.num_seconds())
        } else if age.num_minutes() < 60 {
            format!("{}m", age.num_minutes())
        } else if age.num_hours() < 24 {
            format!("{}h", age.num_hours())
        } else {
            format!("{}d", age.num_days())
        }
    }

    /// Get human-readable event description
    fn event_description(event: &Event) -> String {
        use mnemosyne_core::api::events::EventType::*;
        match &event.event_type {
            // CLI events
            CliCommandStarted { command, .. } => format!("CLI started: {}", command),
            CliCommandCompleted { command, duration_ms, .. } => {
                format!("CLI completed: {} ({}ms)", command, duration_ms)
            }
            CliCommandFailed { command, error, .. } => {
                format!("CLI failed: {} - {}", command, Self::truncate(error, 50))
            }

            // Memory events
            MemoryStored { memory_id, summary, .. } => {
                format!("Memory stored: {} - {}", Self::truncate(memory_id, 10), Self::truncate(summary, 40))
            }
            MemoryRecalled { query, count, .. } => {
                format!("Memory recalled: {} ({} results)", Self::truncate(query, 30), count)
            }
            MemoryEvolutionStarted { reason, .. } => {
                format!("Evolution started: {}", Self::truncate(reason, 40))
            }

            // Agent events
            AgentStarted { agent_id, task, .. } => {
                if let Some(task) = task {
                    format!("Agent started: {} - {}", Self::truncate(agent_id, 10), Self::truncate(task, 40))
                } else {
                    format!("Agent started: {}", Self::truncate(agent_id, 10))
                }
            }
            AgentCompleted { agent_id, result, .. } => {
                format!("Agent completed: {} - {}", Self::truncate(agent_id, 10), Self::truncate(result, 40))
            }
            AgentFailed { agent_id, error, .. } => {
                format!("Agent failed: {} - {}", Self::truncate(agent_id, 10), Self::truncate(error, 40))
            }

            // Work events
            WorkItemAssigned { item_id, agent_id, .. } => {
                format!("Work assigned: {} → {}", Self::truncate(item_id, 10), Self::truncate(agent_id, 10))
            }
            WorkItemCompleted { item_id, .. } => {
                format!("Work completed: {}", Self::truncate(item_id, 10))
            }
            PhaseChanged { from, to, .. } => {
                format!("Phase: {} → {}", Self::truncate(from, 15), Self::truncate(to, 15))
            }

            // Skill events
            SkillLoaded { skill_name, .. } => {
                format!("Skill loaded: {}", Self::truncate(skill_name, 30))
            }
            SkillUsed { skill_name, .. } => {
                format!("Skill used: {}", Self::truncate(skill_name, 30))
            }

            // Error events
            DeadlockDetected { blocked_items, .. } => {
                format!("Deadlock: {} items blocked", blocked_items.len())
            }
            AgentHealthDegraded { agent_id, error_count, .. } => {
                format!("Health degraded: {} ({} errors)", Self::truncate(agent_id, 10), error_count)
            }
            ReviewFailed { item_id, issues, .. } => {
                format!("Review failed: {} ({} issues)", Self::truncate(item_id, 10), issues.len())
            }

            // System events
            SessionStarted { instance_id, .. } => {
                format!("Session started: {}", Self::truncate(instance_id, 10))
            }
            HealthUpdate { .. } => "Health update".to_string(),
            Heartbeat { .. } => "Heartbeat".to_string(),

            // Fallback for other events
            _ => format!("{:?}", event.event_type).chars().take(60).collect(),
        }
    }

    /// Truncate string to max length
    fn truncate(s: &str, max_len: usize) -> String {
        if s.len() > max_len {
            format!("{}...", &s[..(max_len - 3)])
        } else {
            s.to_string()
        }
    }
}

impl Default for ActivityStreamPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mnemosyne_core::api::events::{Event, EventType};

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

    fn create_memory_stored() -> Event {
        Event::new(EventType::MemoryStored {
            memory_id: "mem-123".to_string(),
            summary: "Test memory".to_string(),
            timestamp: Utc::now(),
        })
    }

    fn create_heartbeat() -> Event {
        Event::new(EventType::Heartbeat {
            instance_id: "test".to_string(),
            timestamp: Utc::now(),
        })
    }

    #[test]
    fn test_activity_stream_creation() {
        let panel = ActivityStreamPanel::new();
        assert_eq!(panel.entry_count(), 0);
        assert_eq!(panel.filter_stats().total, 0);
    }

    #[test]
    fn test_add_raw_event() {
        let mut panel = ActivityStreamPanel::new();

        let event = create_memory_stored();
        panel.add_event(event);

        assert_eq!(panel.entry_count(), 1);
        assert_eq!(panel.filter_stats().total, 1);
        assert_eq!(panel.filter_stats().passed, 1);
    }

    #[test]
    fn test_filter_heartbeats_by_default() {
        let mut panel = ActivityStreamPanel::new();

        // Heartbeat should be filtered out by default
        let heartbeat = create_heartbeat();
        panel.add_event(heartbeat);

        assert_eq!(panel.entry_count(), 0); // Not added
        assert_eq!(panel.filter_stats().total, 1); // But counted in stats
        assert_eq!(panel.filter_stats().passed, 0); // Didn't pass filter
    }

    #[test]
    fn test_event_correlation() {
        let mut panel = ActivityStreamPanel::new();

        // Start event
        let start = create_cli_started("remember");
        panel.add_event(start);

        // Should not create entry yet (waiting for completion)
        assert_eq!(panel.entry_count(), 0);

        // Complete event
        let complete = create_cli_completed("remember");
        panel.add_event(complete);

        // Should create correlated entry
        assert_eq!(panel.entry_count(), 1);

        // Verify it's a correlated operation
        if let Some(ActivityEntry::CorrelatedOperation(corr)) = panel.entries.back() {
            assert_eq!(corr.status, OperationStatus::Completed);
            assert!(corr.duration.is_some());
        } else {
            panic!("Expected correlated operation");
        }
    }

    #[test]
    fn test_ring_buffer_trimming() {
        let mut panel = ActivityStreamPanel::new();
        panel.max_entries = 3; // Small limit for testing

        // Add 5 events
        for _ in 0..5 {
            panel.add_event(create_memory_stored());
        }

        // Should only keep last 3
        assert_eq!(panel.entry_count(), 3);
    }

    #[test]
    fn test_clear() {
        let mut panel = ActivityStreamPanel::new();

        panel.add_event(create_memory_stored());
        panel.add_event(create_memory_stored());

        assert_eq!(panel.entry_count(), 2);

        panel.clear();

        assert_eq!(panel.entry_count(), 0);
        assert_eq!(panel.filter_stats().total, 0);
    }

    #[test]
    fn test_custom_filter() {
        let mut panel = ActivityStreamPanel::new();

        // Set filter to errors only
        panel.set_filter(EventFilter::ErrorsOnly);

        // Add normal event - should be filtered
        panel.add_event(create_memory_stored());
        assert_eq!(panel.entry_count(), 0);

        // Add error event - should pass
        let error = Event::new(EventType::AgentFailed {
            agent_id: "test".to_string(),
            error: "Error".to_string(),
            timestamp: Utc::now(),
        });
        panel.add_event(error);
        assert_eq!(panel.entry_count(), 1);
    }

    #[test]
    fn test_format_relative_time() {
        let now = Utc::now();

        // Recent
        assert_eq!(ActivityStreamPanel::format_relative_time(now), "now");

        // Seconds ago
        let secs_ago = now - chrono::Duration::seconds(30);
        assert_eq!(ActivityStreamPanel::format_relative_time(secs_ago), "30s");

        // Minutes ago
        let mins_ago = now - chrono::Duration::minutes(5);
        assert_eq!(ActivityStreamPanel::format_relative_time(mins_ago), "5m");

        // Hours ago
        let hours_ago = now - chrono::Duration::hours(3);
        assert_eq!(ActivityStreamPanel::format_relative_time(hours_ago), "3h");
    }

    #[test]
    fn test_category_colors_and_prefixes() {
        assert_eq!(ActivityStreamPanel::category_color(&EventCategory::Cli), Color::Blue);
        assert_eq!(ActivityStreamPanel::category_color(&EventCategory::Error), Color::Red);

        assert_eq!(ActivityStreamPanel::category_prefix(&EventCategory::Cli), "CLI");
        assert_eq!(ActivityStreamPanel::category_prefix(&EventCategory::Error), "ERR");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(ActivityStreamPanel::truncate("short", 10), "short");
        assert_eq!(
            ActivityStreamPanel::truncate("very long string here", 10),
            "very lo..."
        );
    }
}
