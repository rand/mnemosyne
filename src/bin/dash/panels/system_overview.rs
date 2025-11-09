//! System Overview panel - At-a-glance health summary
//!
//! Compact top panel (6-8 lines) providing system-wide visibility:
//! - Instance info, uptime, agent counts, graph size, activity rate
//! - Agent state breakdown with color-coded badges
//! - Memory activity sparklines (stores/min, recalls/min)
//! - Recent critical events (errors/warnings only)
//! - System health metrics (CPU, memory, subscribers, last event age)

use crate::colors::DashboardColors;
use crate::filters::EventCategory;
use mnemosyne_core::api::events::{Event, EventType};
use crate::time_series::TimeSeriesBuffer;
use crate::widgets::{Sparkline, StateIndicator, StateType};
use chrono::{DateTime, Utc};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};
use std::time::Duration;

/// System metrics snapshot
#[derive(Debug, Clone, Default)]
pub struct SystemMetrics {
    /// Instance ID
    pub instance_id: Option<String>,
    /// System uptime
    pub uptime: Option<Duration>,
    /// Active agent count
    pub agents_active: usize,
    /// Idle agent count
    pub agents_idle: usize,
    /// Failed agent count
    pub agents_failed: usize,
    /// Total agent count
    pub agents_total: usize,
    /// Memory graph size (number of memories)
    pub graph_size: usize,
    /// Memory stores per minute
    pub stores_per_min: f32,
    /// Memory recalls per minute
    pub recalls_per_min: f32,
    /// CLI operations per minute
    pub ops_per_min: f32,
    /// CPU usage percentage
    pub cpu_percent: f32,
    /// Memory usage in MB
    pub memory_mb: f32,
    /// API subscriber count
    pub subscribers: usize,
    /// Last event timestamp
    pub last_event_at: Option<DateTime<Utc>>,
}

/// System Overview panel widget
pub struct SystemOverviewPanel {
    /// Current metrics
    metrics: SystemMetrics,
    /// Memory stores per minute history
    stores_history: TimeSeriesBuffer<f32>,
    /// Memory recalls per minute history
    recalls_history: TimeSeriesBuffer<f32>,
    /// Recent critical events (errors/warnings only)
    critical_events: Vec<CriticalEvent>,
    /// Maximum critical events to display
    max_critical: usize,
}

/// A critical event (error or warning)
#[derive(Debug, Clone)]
struct CriticalEvent {
    /// Event category
    category: EventCategory,
    /// Event description
    description: String,
    /// When it occurred
    timestamp: DateTime<Utc>,
}

impl SystemOverviewPanel {
    /// Create new system overview panel
    pub fn new() -> Self {
        Self {
            metrics: SystemMetrics::default(),
            stores_history: TimeSeriesBuffer::new(60), // Last 60 data points
            recalls_history: TimeSeriesBuffer::new(60),
            critical_events: Vec::new(),
            max_critical: 3, // Show last 3 critical events
        }
    }

    /// Update system metrics
    pub fn update_metrics(&mut self, metrics: SystemMetrics) {
        self.stores_history.push(metrics.stores_per_min);
        self.recalls_history.push(metrics.recalls_per_min);
        self.metrics = metrics;
    }

    /// Add an event (filters for critical events only)
    pub fn add_event(&mut self, event: Event) {
        let category = EventCategory::from_event(&event);

        // Only track critical events (errors and important state changes)
        if matches!(category, EventCategory::Error) || Self::is_critical(&event) {
            let description = Self::event_description(&event);
            let timestamp = Self::extract_timestamp(&event).unwrap_or_else(Utc::now);

            self.critical_events.push(CriticalEvent {
                category,
                description,
                timestamp,
            });

            // Trim to max
            if self.critical_events.len() > self.max_critical {
                self.critical_events.remove(0);
            }
        }
    }

    /// Check if event is critical (even if not an error)
    fn is_critical(event: &Event) -> bool {
        matches!(
            &event.event_type,
            EventType::DeadlockDetected { .. }
                | EventType::AgentHealthDegraded { .. }
                | EventType::ReviewFailed { .. }
                | EventType::PhaseChanged { .. } // Phase changes are important
        )
    }

    /// Extract human-readable event description
    fn event_description(event: &Event) -> String {
        match &event.event_type {
            EventType::AgentFailed { agent_id, error, .. } => {
                format!("Agent {} failed: {}", Self::truncate(agent_id, 10), Self::truncate(error, 40))
            }
            EventType::CliCommandFailed { command, error, .. } => {
                format!("CLI {} failed: {}", command, Self::truncate(error, 35))
            }
            EventType::DeadlockDetected { blocked_items, .. } => {
                format!("Deadlock: {} items blocked", blocked_items.len())
            }
            EventType::AgentHealthDegraded { agent_id, error_count, .. } => {
                format!("Agent {} degraded ({} errors)", Self::truncate(agent_id, 10), error_count)
            }
            EventType::ReviewFailed { item_id, issues, attempt, .. } => {
                format!("Review failed: {} (attempt {}, {} issues)", Self::truncate(item_id, 10), attempt, issues.len())
            }
            EventType::PhaseChanged { from, to, .. } => {
                format!("Phase: {} → {}", Self::truncate(from, 15), Self::truncate(to, 15))
            }
            EventType::AgentErrorRecorded { agent_id, error_message, .. } => {
                format!("Error in {}: {}", Self::truncate(agent_id, 10), Self::truncate(error_message, 35))
            }
            _ => "Critical event".to_string(),
        }
    }

    /// Extract timestamp from event
    fn extract_timestamp(event: &Event) -> Option<DateTime<Utc>> {
        use EventType::*;
        match &event.event_type {
            AgentFailed { timestamp, .. }
            | CliCommandFailed { timestamp, .. }
            | DeadlockDetected { timestamp, .. }
            | AgentHealthDegraded { timestamp, .. }
            | ReviewFailed { timestamp, .. }
            | PhaseChanged { timestamp, .. }
            | AgentErrorRecorded { timestamp, .. } => Some(*timestamp),
            _ => None,
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

    /// Render the system overview panel
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let mut items: Vec<ListItem> = Vec::new();

        // Row 1: Instance info and counts
        let instance_id = self.metrics.instance_id.as_deref().unwrap_or("unknown");
        let uptime_str = if let Some(uptime) = self.metrics.uptime {
            let hours = uptime.as_secs() / 3600;
            let mins = (uptime.as_secs() % 3600) / 60;
            format!("{}h {}m", hours, mins)
        } else {
            "N/A".to_string()
        };

        items.push(ListItem::new(Line::from(vec![
            Span::styled("[", Style::default().fg(DashboardColors::SECONDARY)),
            Span::styled(
                &instance_id[..instance_id.len().min(8)],
                Style::default().fg(DashboardColors::HIGHLIGHT).add_modifier(Modifier::BOLD),
            ),
            Span::styled("]", Style::default().fg(DashboardColors::SECONDARY)),
            Span::raw(" Uptime: "),
            Span::styled(uptime_str, Style::default().fg(DashboardColors::SUCCESS)),
            Span::raw(" | Agents: "),
            Span::styled(
                format!("{}", self.metrics.agents_total),
                Style::default().fg(DashboardColors::AGENT).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" | Graph: "),
            Span::styled(
                format!("{}", self.metrics.graph_size),
                Style::default().fg(DashboardColors::MEMORY),
            ),
            Span::raw(" | Activity: "),
            Span::styled(
                format!("{:.1} ops/min", self.metrics.ops_per_min),
                Style::default().fg(DashboardColors::IN_PROGRESS),
            ),
        ])));

        // Row 2: Agent state breakdown
        let active_indicator = StateIndicator::new(StateType::Active, format!("{}", self.metrics.agents_active));
        let idle_indicator = StateIndicator::new(StateType::Idle, format!("{}", self.metrics.agents_idle));
        let failed_indicator = if self.metrics.agents_failed > 0 {
            StateIndicator::new(StateType::Failed, format!("{}", self.metrics.agents_failed))
        } else {
            StateIndicator::new(StateType::Idle, "0".to_string()) // Use idle style for zero
        };

        let mut agent_spans = vec![Span::raw("Agent States: ")];
        agent_spans.push(active_indicator.render());
        agent_spans.push(Span::raw(" "));
        agent_spans.push(idle_indicator.render());
        agent_spans.push(Span::raw(" "));
        agent_spans.push(failed_indicator.render());

        items.push(ListItem::new(Line::from(agent_spans)));

        // Row 3-4: Memory activity sparklines
        let stores_data = self.stores_history.to_vec();
        let recalls_data = self.recalls_history.to_vec();

        if !stores_data.is_empty() {
            let stores_sparkline = Sparkline::new(&stores_data)
                .width(20)
                .style(Style::default().fg(DashboardColors::SUCCESS));

            let mut stores_spans = vec![
                Span::styled("Memory Stores: ", Style::default().add_modifier(Modifier::DIM)),
            ];
            stores_spans.extend(stores_sparkline.render().spans);
            stores_spans.push(Span::styled(
                format!(" ({:.1}/min)", self.metrics.stores_per_min),
                Style::default().fg(DashboardColors::SECONDARY),
            ));

            items.push(ListItem::new(Line::from(stores_spans)));
        }

        if !recalls_data.is_empty() {
            let recalls_sparkline = Sparkline::new(&recalls_data)
                .width(20)
                .style(Style::default().fg(DashboardColors::SKILL));

            let mut recalls_spans = vec![
                Span::styled("Memory Recalls: ", Style::default().add_modifier(Modifier::DIM)),
            ];
            recalls_spans.extend(recalls_sparkline.render().spans);
            recalls_spans.push(Span::styled(
                format!(" ({:.1}/min)", self.metrics.recalls_per_min),
                Style::default().fg(DashboardColors::SECONDARY),
            ));

            items.push(ListItem::new(Line::from(recalls_spans)));
        }

        // Row 5: Recent critical events
        if !self.critical_events.is_empty() {
            let events_line = Line::from(vec![
                Span::styled("Critical: ", Style::default().fg(DashboardColors::ERROR).add_modifier(Modifier::BOLD)),
                Span::styled(
                    self.critical_events.last().unwrap().description.clone(),
                    Style::default().fg(DashboardColors::ERROR),
                ),
            ]);
            items.push(ListItem::new(events_line));
        }

        // Row 6: System health
        let cpu_color = if self.metrics.cpu_percent > 80.0 {
            DashboardColors::USAGE_HIGH
        } else if self.metrics.cpu_percent > 50.0 {
            DashboardColors::USAGE_MEDIUM
        } else {
            DashboardColors::USAGE_LOW
        };

        let mem_color = if self.metrics.memory_mb > 1000.0 {
            DashboardColors::USAGE_HIGH
        } else if self.metrics.memory_mb > 500.0 {
            DashboardColors::USAGE_MEDIUM
        } else {
            DashboardColors::USAGE_LOW
        };

        let last_event_str = if let Some(last_event) = self.metrics.last_event_at {
            let now = Utc::now();
            let age = now.signed_duration_since(last_event);
            if age.num_seconds() < 60 {
                format!("{}s ago", age.num_seconds())
            } else if age.num_minutes() < 60 {
                format!("{}m ago", age.num_minutes())
            } else {
                format!("{}h ago", age.num_hours())
            }
        } else {
            "N/A".to_string()
        };

        items.push(ListItem::new(Line::from(vec![
            Span::styled("Health: ", Style::default().add_modifier(Modifier::DIM)),
            Span::raw("CPU: "),
            Span::styled(format!("{:.0}%", self.metrics.cpu_percent), Style::default().fg(cpu_color).add_modifier(Modifier::BOLD)),
            Span::raw(" | RAM: "),
            Span::styled(format!("{:.0}MB", self.metrics.memory_mb), Style::default().fg(mem_color).add_modifier(Modifier::BOLD)),
            Span::raw(" | API: "),
            Span::styled(format!("{} sub", self.metrics.subscribers), Style::default().fg(DashboardColors::IN_PROGRESS)),
            Span::raw(" | Last event: "),
            Span::styled(last_event_str, Style::default().fg(DashboardColors::SECONDARY)),
        ])));

        // Render with border
        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("System Overview")
                .border_style(Style::default().fg(DashboardColors::BORDER)),
        );

        frame.render_widget(list, area);
    }
}

impl Default for SystemOverviewPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_overview_creation() {
        let panel = SystemOverviewPanel::new();
        assert_eq!(panel.metrics.agents_total, 0);
        assert_eq!(panel.critical_events.len(), 0);
    }

    #[test]
    fn test_metrics_update() {
        let mut panel = SystemOverviewPanel::new();

        let metrics = SystemMetrics {
            instance_id: Some("test-instance".to_string()),
            uptime: Some(Duration::from_secs(3600)),
            agents_active: 2,
            agents_idle: 1,
            agents_failed: 0,
            agents_total: 3,
            graph_size: 1234,
            stores_per_min: 5.2,
            recalls_per_min: 2.1,
            ops_per_min: 7.3,
            cpu_percent: 12.5,
            memory_mb: 245.0,
            subscribers: 2,
            last_event_at: Some(Utc::now()),
        };

        panel.update_metrics(metrics.clone());

        assert_eq!(panel.metrics.agents_active, 2);
        assert_eq!(panel.metrics.graph_size, 1234);
        assert!((panel.metrics.stores_per_min - 5.2).abs() < 0.1);
    }

    #[test]
    fn test_critical_event_tracking() {
        let mut panel = SystemOverviewPanel::new();

        // Add error event
        let error_event = Event::new(EventType::AgentFailed {
            agent_id: "test-agent".to_string(),
            error: "Test error".to_string(),
            timestamp: Utc::now(),
        });

        panel.add_event(error_event);
        assert_eq!(panel.critical_events.len(), 1);

        // Non-critical event should not be added
        let normal_event = Event::new(EventType::MemoryStored {
            memory_id: "mem-123".to_string(),
            summary: "Test".to_string(),
            timestamp: Utc::now(),
        });

        panel.add_event(normal_event);
        assert_eq!(panel.critical_events.len(), 1); // Still 1
    }

    #[test]
    fn test_critical_event_trimming() {
        let mut panel = SystemOverviewPanel::new();

        // Add more than max_critical events
        for i in 0..5 {
            let event = Event::new(EventType::AgentFailed {
                agent_id: format!("agent-{}", i),
                error: "Error".to_string(),
                timestamp: Utc::now(),
            });
            panel.add_event(event);
        }

        // Should only keep last 3
        assert_eq!(panel.critical_events.len(), 3);
    }

    #[test]
    fn test_truncate() {
        assert_eq!(SystemOverviewPanel::truncate("short", 10), "short");
        assert_eq!(
            SystemOverviewPanel::truncate("very long string here", 10),
            "very lo..."
        );
    }
}
