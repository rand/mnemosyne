//! Agents panel - Display agent activity with health indicators
//!
//! Features:
//! - Real-time agent state tracking (Active/Idle/Failed/Blocked)
//! - Event-driven state transitions with duration tracking
//! - Color-coded state indicators
//! - Agent statistics (total, active, idle, failed, blocked)
//! - Scrolling support for many agents
//! - Health indicators from events

use crate::time_series::TimeSeriesBuffer;
use crate::widgets::{Sparkline, StateIndicator, StateType};
use chrono::{DateTime, Utc};
use mnemosyne_core::api::events::{Event, EventType};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};
use serde::Deserialize;
use std::collections::HashMap;

/// Agent statistics
#[derive(Debug, Clone, Default)]
struct AgentStatistics {
    /// Total agents
    total: usize,
    /// Active agents
    active: usize,
    /// Idle agents
    idle: usize,
    /// Failed agents
    failed: usize,
    /// Blocked agents
    blocked: usize,
    /// Average operation duration in milliseconds
    avg_operation_duration_ms: i64,
}

/// Agent info from API
#[derive(Debug, Clone, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub state: AgentState,
    #[serde(default)]
    pub health: Option<AgentHealth>,
}

/// Agent state (matches API state enum)
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentState {
    Idle,
    Active { task: String },
    Waiting { reason: String },
    Completed { result: String },
    Failed { error: String },
}

/// Agent health information
#[derive(Debug, Clone, Deserialize)]
pub struct AgentHealth {
    pub error_count: usize,
    pub is_healthy: bool,
}

/// Tracked agent state (enhanced with duration tracking)
#[derive(Debug, Clone)]
struct TrackedAgent {
    /// Agent ID
    id: String,
    /// Current state
    state: TrackedAgentState,
    /// When current state started
    state_started_at: DateTime<Utc>,
    /// Current task/operation (if any)
    current_task: Option<String>,
    /// Error count
    error_count: usize,
    /// Is healthy
    is_healthy: bool,
}

/// Tracked agent state (simplified for event tracking)
#[derive(Debug, Clone, PartialEq)]
enum TrackedAgentState {
    Idle,
    Active,
    Failed,
    Blocked,
    Completed,
}

impl TrackedAgent {
    /// Create new tracked agent
    fn new(id: String) -> Self {
        Self {
            id,
            state: TrackedAgentState::Idle,
            state_started_at: Utc::now(),
            current_task: None,
            error_count: 0,
            is_healthy: true,
        }
    }

    /// Get state duration in milliseconds
    fn state_duration_ms(&self) -> i64 {
        let now = Utc::now();
        now.signed_duration_since(self.state_started_at).num_milliseconds()
    }

    /// Update state
    fn set_state(&mut self, new_state: TrackedAgentState) {
        if self.state != new_state {
            self.state = new_state;
            self.state_started_at = Utc::now();
        }
    }
}

/// Agents panel widget
pub struct AgentsPanel {
    /// Agents fetched from API (fallback data)
    agents: Vec<AgentInfo>,
    /// Event-tracked agents (primary data source)
    tracked_agents: HashMap<String, TrackedAgent>,
    /// Title
    title: String,
    /// Active count history
    active_count_history: TimeSeriesBuffer<f32>,
    /// Total operation durations (for average calculation)
    total_operation_duration_ms: i64,
    /// Total completed operations count
    completed_operations_count: usize,
}

impl AgentsPanel {
    /// Create new agents panel
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            tracked_agents: HashMap::new(),
            title: "Agent Activity".to_string(),
            active_count_history: TimeSeriesBuffer::new(50),
            total_operation_duration_ms: 0,
            completed_operations_count: 0,
        }
    }

    /// Update agents data from API (fallback)
    pub fn update(&mut self, agents: Vec<AgentInfo>) {
        // Count active agents
        let active_count = agents.iter()
            .filter(|agent| matches!(agent.state, AgentState::Active { .. }))
            .count() as f32;

        self.active_count_history.push(active_count);
        self.agents = agents;
    }

    /// Add event to update agent tracking
    pub fn add_event(&mut self, event: Event) {
        match &event.event_type {
            EventType::AgentStarted { agent_id, task, .. } => {
                let agent = self.tracked_agents
                    .entry(agent_id.clone())
                    .or_insert_with(|| TrackedAgent::new(agent_id.clone()));

                agent.set_state(TrackedAgentState::Active);
                agent.current_task = task.as_ref().map(|t| Self::truncate(t, 40));
            }

            EventType::AgentCompleted { agent_id, result, .. } => {
                if let Some(agent) = self.tracked_agents.get_mut(agent_id) {
                    let duration = agent.state_duration_ms();

                    // Track for average calculation
                    self.total_operation_duration_ms += duration;
                    self.completed_operations_count += 1;

                    agent.set_state(TrackedAgentState::Completed);
                    agent.current_task = Some(Self::truncate(result, 40));
                }
            }

            EventType::AgentFailed { agent_id, error, .. } => {
                let agent = self.tracked_agents
                    .entry(agent_id.clone())
                    .or_insert_with(|| TrackedAgent::new(agent_id.clone()));

                agent.set_state(TrackedAgentState::Failed);
                agent.current_task = Some(Self::truncate(error, 40));
                agent.error_count += 1;
                agent.is_healthy = false;
            }

            EventType::AgentBlocked { agent_id, reason, .. } => {
                let agent = self.tracked_agents
                    .entry(agent_id.clone())
                    .or_insert_with(|| TrackedAgent::new(agent_id.clone()));

                agent.set_state(TrackedAgentState::Blocked);
                agent.current_task = Some(Self::truncate(reason, 40));
            }

            EventType::AgentUnblocked { agent_id, .. } => {
                if let Some(agent) = self.tracked_agents.get_mut(agent_id) {
                    // Return to active state (task should still be set)
                    agent.set_state(TrackedAgentState::Active);
                }
            }

            EventType::AgentRestarted { agent_id, reason, .. } => {
                if let Some(agent) = self.tracked_agents.get_mut(agent_id) {
                    agent.set_state(TrackedAgentState::Idle);
                    agent.current_task = Some(format!("Restarted: {}", Self::truncate(reason, 30)));
                    agent.error_count = 0; // Reset on restart
                    agent.is_healthy = true;
                }
            }

            EventType::AgentHealthDegraded { agent_id, error_count, is_healthy, .. } => {
                if let Some(agent) = self.tracked_agents.get_mut(agent_id) {
                    agent.error_count = *error_count;
                    agent.is_healthy = *is_healthy;
                }
            }

            EventType::WorkItemAssigned { agent_id, task, .. } => {
                let agent = self.tracked_agents
                    .entry(agent_id.clone())
                    .or_insert_with(|| TrackedAgent::new(agent_id.clone()));

                agent.set_state(TrackedAgentState::Active);
                agent.current_task = Some(Self::truncate(task, 40));
            }

            EventType::WorkItemCompleted { agent_id, .. } => {
                if let Some(agent) = self.tracked_agents.get_mut(agent_id) {
                    let duration = agent.state_duration_ms();

                    // Track for average calculation
                    self.total_operation_duration_ms += duration;
                    self.completed_operations_count += 1;

                    agent.set_state(TrackedAgentState::Idle);
                    agent.current_task = None;
                }
            }

            _ => {
                // Ignore other event types
            }
        }

        // Update active count history
        let active_count = self.tracked_agents.values()
            .filter(|a| a.state == TrackedAgentState::Active)
            .count() as f32;
        self.active_count_history.push(active_count);
    }

    /// Set custom title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Get statistics
    fn get_statistics(&self) -> AgentStatistics {
        let mut stats = AgentStatistics::default();

        for agent in self.tracked_agents.values() {
            stats.total += 1;
            match agent.state {
                TrackedAgentState::Active => stats.active += 1,
                TrackedAgentState::Idle => stats.idle += 1,
                TrackedAgentState::Failed => stats.failed += 1,
                TrackedAgentState::Blocked => stats.blocked += 1,
                TrackedAgentState::Completed => stats.idle += 1, // Count completed as idle
            }
        }

        stats.avg_operation_duration_ms = if self.completed_operations_count > 0 {
            self.total_operation_duration_ms / (self.completed_operations_count as i64)
        } else {
            0
        };

        stats
    }

    /// Truncate string to max length
    fn truncate(s: &str, max_len: usize) -> String {
        if s.len() > max_len {
            format!("{}...", &s[..(max_len - 3)])
        } else {
            s.to_string()
        }
    }

    /// Convert agent state to state indicator type
    fn state_to_indicator_type(state: &AgentState) -> StateType {
        match state {
            AgentState::Idle => StateType::Idle,
            AgentState::Active { .. } => StateType::Active,
            AgentState::Waiting { .. } => StateType::Waiting,
            AgentState::Completed { .. } => StateType::Completed,
            AgentState::Failed { .. } => StateType::Failed,
        }
    }

    /// Get state description text
    fn state_description(state: &AgentState) -> String {
        match state {
            AgentState::Idle => "Idle".to_string(),
            AgentState::Active { task } => {
                // Truncate long task descriptions
                if task.len() > 40 {
                    format!("{}...", &task[..37])
                } else {
                    task.clone()
                }
            }
            AgentState::Waiting { reason } => {
                format!("Waiting: {}", if reason.len() > 30 {
                    format!("{}...", &reason[..27])
                } else {
                    reason.clone()
                })
            }
            AgentState::Completed { result } => {
                format!("Completed: {}", if result.len() > 30 {
                    format!("{}...", &result[..27])
                } else {
                    result.clone()
                })
            }
            AgentState::Failed { error } => {
                format!("Failed: {}", if error.len() > 30 {
                    format!("{}...", &error[..27])
                } else {
                    error.clone()
                })
            }
        }
    }

    /// Render the agents panel
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Prepare data for sparkline (must live for entire function)
        let active_count_data = self.active_count_history.to_vec();

        let mut items: Vec<ListItem> = Vec::new();

        // Get statistics
        let stats = self.get_statistics();

        // Statistics section
        if stats.total > 0 {
            // Row 1: Totals and state breakdown
            let stats_line = Line::from(vec![
                Span::styled("Stats: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("Total: "),
                Span::styled(format!("{}", stats.total), Style::default().fg(Color::Cyan)),
                Span::raw(" | "),
                StateIndicator::new(StateType::Active, format!("{}", stats.active)).render(),
                Span::raw(" "),
                StateIndicator::new(StateType::Idle, format!("{}", stats.idle)).render(),
                Span::raw(" "),
                StateIndicator::new(StateType::Failed, format!("{}", stats.failed)).render(),
                Span::raw(" "),
                StateIndicator::new(StateType::Waiting, format!("{}", stats.blocked)).render(),
            ]);
            items.push(ListItem::new(stats_line));

            // Row 2: Average operation duration
            if stats.avg_operation_duration_ms > 0 {
                let avg_duration_line = Line::from(vec![
                    Span::styled("Avg Duration: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(
                        Self::format_duration_ms(stats.avg_operation_duration_ms),
                        Style::default().fg(if stats.avg_operation_duration_ms > 10000 {
                            Color::Yellow
                        } else {
                            Color::Green
                        }),
                    ),
                ]);
                items.push(ListItem::new(avg_duration_line));
            }

            // Row 3: Activity trend sparkline (show if we have history)
            if !active_count_data.is_empty() {
                let sparkline = Sparkline::new(&active_count_data)
                    .width(20)
                    .style(Style::default().fg(Color::Green));

                let mut spans = vec![
                    Span::styled(
                        "Activity: ",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                ];
                spans.extend(sparkline.render().spans);

                items.push(ListItem::new(Line::from(spans)));
            }
        }

        // Agent list (from tracked agents)
        if self.tracked_agents.is_empty() {
            items.push(ListItem::new(Line::from(vec![Span::styled(
                "No active agents",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )])));
        } else {
            // Sort agents by state (Active > Blocked > Failed > Idle) then by ID
            let mut agents: Vec<&TrackedAgent> = self.tracked_agents.values().collect();
            agents.sort_by(|a, b| {
                let a_priority = Self::state_priority(&a.state);
                let b_priority = Self::state_priority(&b.state);
                a_priority.cmp(&b_priority).then_with(|| a.id.cmp(&b.id))
            });

            let agent_items: Vec<ListItem> = agents
                .iter()
                .map(|agent| {
                    // State indicator
                    let state_type = Self::tracked_state_to_indicator_type(&agent.state);
                    let state_text = Self::tracked_state_description(&agent.state);

                    // Duration if active
                    let duration_str = if agent.state == TrackedAgentState::Active || agent.state == TrackedAgentState::Blocked {
                        let duration = agent.state_duration_ms();
                        format!(" ({})", Self::format_duration_ms(duration))
                    } else {
                        String::new()
                    };

                    // Task description
                    let task_str = if let Some(task) = &agent.current_task {
                        format!(": {}", task)
                    } else {
                        String::new()
                    };

                    // Health indicator
                    let health_span = if agent.error_count > 0 {
                        let health_type = if agent.is_healthy {
                            StateType::Degraded
                        } else {
                            StateType::Failed
                        };
                        let health_indicator = StateIndicator::new(
                            health_type,
                            format!("{}", agent.error_count),
                        );
                        Some(health_indicator.render_icon_only())
                    } else {
                        None
                    };

                    // Build line with agent ID, state, duration, task, and health
                    let mut spans = vec![
                        Span::styled(
                            format!("{:12}", agent.id),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        StateIndicator::new(state_type, state_text).render(),
                        Span::styled(duration_str, Style::default().fg(Color::DarkGray)),
                        Span::styled(task_str, Style::default().fg(Color::Gray)),
                    ];

                    if let Some(health) = health_span {
                        spans.push(Span::raw(" "));
                        spans.push(health);
                    }

                    ListItem::new(Line::from(spans))
                })
                .collect();

            items.extend(agent_items);
        }

        let list = List::new(items).block(
            Block::default()
                .title(format!("{} ({})", self.title, stats.total))
                .borders(Borders::ALL),
        );

        frame.render_widget(list, area);
    }

    /// Format duration in milliseconds to human-readable string
    fn format_duration_ms(duration_ms: i64) -> String {
        if duration_ms < 1000 {
            format!("{}ms", duration_ms)
        } else if duration_ms < 60000 {
            format!("{:.1}s", duration_ms as f64 / 1000.0)
        } else if duration_ms < 3600000 {
            format!("{:.1}m", duration_ms as f64 / 60000.0)
        } else {
            format!("{:.1}h", duration_ms as f64 / 3600000.0)
        }
    }

    /// Get state priority for sorting (lower = higher priority)
    fn state_priority(state: &TrackedAgentState) -> u8 {
        match state {
            TrackedAgentState::Failed => 0,
            TrackedAgentState::Blocked => 1,
            TrackedAgentState::Active => 2,
            TrackedAgentState::Completed => 3,
            TrackedAgentState::Idle => 4,
        }
    }

    /// Convert tracked agent state to state indicator type
    fn tracked_state_to_indicator_type(state: &TrackedAgentState) -> StateType {
        match state {
            TrackedAgentState::Idle => StateType::Idle,
            TrackedAgentState::Active => StateType::Active,
            TrackedAgentState::Failed => StateType::Failed,
            TrackedAgentState::Blocked => StateType::Waiting,
            TrackedAgentState::Completed => StateType::Completed,
        }
    }

    /// Get state description text for tracked agent
    fn tracked_state_description(state: &TrackedAgentState) -> String {
        match state {
            TrackedAgentState::Idle => "Idle".to_string(),
            TrackedAgentState::Active => "Active".to_string(),
            TrackedAgentState::Failed => "Failed".to_string(),
            TrackedAgentState::Blocked => "Blocked".to_string(),
            TrackedAgentState::Completed => "Completed".to_string(),
        }
    }

    /// Get number of agents
    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }
}

impl Default for AgentsPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_agent_started(agent_id: &str, task: &str) -> Event {
        Event::new(EventType::AgentStarted {
            agent_id: agent_id.to_string(),
            task: Some(task.to_string()),
            timestamp: Utc::now(),
        })
    }

    fn create_agent_completed(agent_id: &str, result: &str) -> Event {
        Event::new(EventType::AgentCompleted {
            agent_id: agent_id.to_string(),
            result: result.to_string(),
            timestamp: Utc::now(),
        })
    }

    fn create_agent_failed(agent_id: &str, error: &str) -> Event {
        Event::new(EventType::AgentFailed {
            agent_id: agent_id.to_string(),
            error: error.to_string(),
            timestamp: Utc::now(),
        })
    }

    fn create_agent_blocked(agent_id: &str, reason: &str) -> Event {
        Event::new(EventType::AgentBlocked {
            agent_id: agent_id.to_string(),
            blocked_on: "dependency".to_string(),
            reason: reason.to_string(),
            timestamp: Utc::now(),
        })
    }

    fn create_agent_unblocked(agent_id: &str) -> Event {
        Event::new(EventType::AgentUnblocked {
            agent_id: agent_id.to_string(),
            unblocked_by: "resolver".to_string(),
            timestamp: Utc::now(),
        })
    }

    fn create_agent_restarted(agent_id: &str, reason: &str) -> Event {
        Event::new(EventType::AgentRestarted {
            agent_id: agent_id.to_string(),
            reason: reason.to_string(),
            timestamp: Utc::now(),
        })
    }

    #[test]
    fn test_agents_panel_creation() {
        let panel = AgentsPanel::new();
        assert_eq!(panel.agent_count(), 0);
        assert_eq!(panel.tracked_agents.len(), 0);
    }

    #[test]
    fn test_agent_started_event() {
        let mut panel = AgentsPanel::new();

        let event = create_agent_started("executor", "test task");
        panel.add_event(event);

        assert_eq!(panel.tracked_agents.len(), 1);
        let agent = panel.tracked_agents.get("executor").unwrap();
        assert_eq!(agent.state, TrackedAgentState::Active);
        assert_eq!(agent.current_task, Some("test task".to_string()));
    }

    #[test]
    fn test_agent_completed_event() {
        let mut panel = AgentsPanel::new();

        // Start agent
        panel.add_event(create_agent_started("executor", "task"));

        // Wait a bit for duration tracking
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Complete agent
        panel.add_event(create_agent_completed("executor", "success"));

        let agent = panel.tracked_agents.get("executor").unwrap();
        assert_eq!(agent.state, TrackedAgentState::Completed);
        assert_eq!(panel.completed_operations_count, 1);
        assert!(panel.total_operation_duration_ms > 0);
    }

    #[test]
    fn test_agent_failed_event() {
        let mut panel = AgentsPanel::new();

        // Start agent
        panel.add_event(create_agent_started("executor", "task"));

        // Fail agent
        panel.add_event(create_agent_failed("executor", "error message"));

        let agent = panel.tracked_agents.get("executor").unwrap();
        assert_eq!(agent.state, TrackedAgentState::Failed);
        assert_eq!(agent.error_count, 1);
        assert!(!agent.is_healthy);
    }

    #[test]
    fn test_agent_blocked_unblocked_transition() {
        let mut panel = AgentsPanel::new();

        // Start agent
        panel.add_event(create_agent_started("executor", "task"));

        // Block agent
        panel.add_event(create_agent_blocked("executor", "waiting for dependency"));

        let agent = panel.tracked_agents.get("executor").unwrap();
        assert_eq!(agent.state, TrackedAgentState::Blocked);

        // Unblock agent
        panel.add_event(create_agent_unblocked("executor"));

        let agent = panel.tracked_agents.get("executor").unwrap();
        assert_eq!(agent.state, TrackedAgentState::Active);
    }

    #[test]
    fn test_agent_restarted_event() {
        let mut panel = AgentsPanel::new();

        // Start and fail agent
        panel.add_event(create_agent_started("executor", "task"));
        panel.add_event(create_agent_failed("executor", "error"));

        let agent = panel.tracked_agents.get("executor").unwrap();
        assert_eq!(agent.error_count, 1);

        // Restart agent
        panel.add_event(create_agent_restarted("executor", "auto-restart"));

        let agent = panel.tracked_agents.get("executor").unwrap();
        assert_eq!(agent.state, TrackedAgentState::Idle);
        assert_eq!(agent.error_count, 0); // Reset on restart
        assert!(agent.is_healthy);
    }

    #[test]
    fn test_multiple_agents_tracking() {
        let mut panel = AgentsPanel::new();

        // Start three agents
        panel.add_event(create_agent_started("executor", "task1"));
        panel.add_event(create_agent_started("optimizer", "task2"));
        panel.add_event(create_agent_started("reviewer", "task3"));

        assert_eq!(panel.tracked_agents.len(), 3);

        let stats = panel.get_statistics();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.active, 3);
        assert_eq!(stats.idle, 0);
    }

    #[test]
    fn test_statistics_calculation() {
        let mut panel = AgentsPanel::new();

        // Create agents in different states
        panel.add_event(create_agent_started("executor", "task"));
        panel.add_event(create_agent_started("optimizer", "task"));
        panel.add_event(create_agent_failed("reviewer", "error"));
        panel.add_event(create_agent_blocked("orchestrator", "waiting"));

        let stats = panel.get_statistics();
        assert_eq!(stats.total, 4);
        assert_eq!(stats.active, 2); // executor, optimizer
        assert_eq!(stats.failed, 1); // reviewer
        assert_eq!(stats.blocked, 1); // orchestrator
        assert_eq!(stats.idle, 0);
    }

    #[test]
    fn test_average_duration_tracking() {
        let mut panel = AgentsPanel::new();

        // Start and complete two agents
        panel.add_event(create_agent_started("agent1", "task1"));
        std::thread::sleep(std::time::Duration::from_millis(10));
        panel.add_event(create_agent_completed("agent1", "done"));

        panel.add_event(create_agent_started("agent2", "task2"));
        std::thread::sleep(std::time::Duration::from_millis(10));
        panel.add_event(create_agent_completed("agent2", "done"));

        assert_eq!(panel.completed_operations_count, 2);
        assert!(panel.total_operation_duration_ms > 0);

        let stats = panel.get_statistics();
        assert!(stats.avg_operation_duration_ms > 0);
    }

    #[test]
    fn test_work_item_events() {
        let mut panel = AgentsPanel::new();

        // Work item assigned
        let event = Event::new(EventType::WorkItemAssigned {
            agent_id: "executor".to_string(),
            item_id: "item-123".to_string(),
            task: "Implement feature".to_string(),
            timestamp: Utc::now(),
        });
        panel.add_event(event);

        let agent = panel.tracked_agents.get("executor").unwrap();
        assert_eq!(agent.state, TrackedAgentState::Active);
        assert_eq!(agent.current_task, Some("Implement feature".to_string()));

        // Work item completed
        std::thread::sleep(std::time::Duration::from_millis(10));
        let event = Event::new(EventType::WorkItemCompleted {
            agent_id: "executor".to_string(),
            item_id: "item-123".to_string(),
            timestamp: Utc::now(),
        });
        panel.add_event(event);

        let agent = panel.tracked_agents.get("executor").unwrap();
        assert_eq!(agent.state, TrackedAgentState::Idle);
        assert_eq!(panel.completed_operations_count, 1);
    }

    #[test]
    fn test_health_degradation_event() {
        let mut panel = AgentsPanel::new();

        panel.add_event(create_agent_started("executor", "task"));

        // Health degraded
        let event = Event::new(EventType::AgentHealthDegraded {
            agent_id: "executor".to_string(),
            error_count: 3,
            is_healthy: false,
            timestamp: Utc::now(),
        });
        panel.add_event(event);

        let agent = panel.tracked_agents.get("executor").unwrap();
        assert_eq!(agent.error_count, 3);
        assert!(!agent.is_healthy);
    }

    #[test]
    fn test_task_truncation() {
        let mut panel = AgentsPanel::new();

        let long_task = "a".repeat(50);
        panel.add_event(create_agent_started("executor", &long_task));

        let agent = panel.tracked_agents.get("executor").unwrap();
        assert!(agent.current_task.as_ref().unwrap().len() <= 43); // 40 + "..."
    }

    #[test]
    fn test_state_priority_ordering() {
        // Failed should be highest priority (lowest value)
        assert!(AgentsPanel::state_priority(&TrackedAgentState::Failed)
            < AgentsPanel::state_priority(&TrackedAgentState::Blocked));
        assert!(AgentsPanel::state_priority(&TrackedAgentState::Blocked)
            < AgentsPanel::state_priority(&TrackedAgentState::Active));
        assert!(AgentsPanel::state_priority(&TrackedAgentState::Active)
            < AgentsPanel::state_priority(&TrackedAgentState::Idle));
    }

    #[test]
    fn test_duration_formatting() {
        assert_eq!(AgentsPanel::format_duration_ms(500), "500ms");
        assert_eq!(AgentsPanel::format_duration_ms(1500), "1.5s");
        assert_eq!(AgentsPanel::format_duration_ms(90000), "1.5m");
        assert_eq!(AgentsPanel::format_duration_ms(7200000), "2.0h");
    }

    #[test]
    fn test_active_count_history() {
        let mut panel = AgentsPanel::new();

        panel.add_event(create_agent_started("agent1", "task"));
        panel.add_event(create_agent_started("agent2", "task"));

        let data = panel.active_count_history.to_vec();
        assert!(!data.is_empty());
        assert_eq!(*data.last().unwrap(), 2.0);
    }

    #[test]
    fn test_state_description_truncation() {
        let long_task = "a".repeat(50);
        let state = AgentState::Active { task: long_task };
        let desc = AgentsPanel::state_description(&state);
        assert!(desc.len() <= 43); // 40 + "..."
    }

    #[test]
    fn test_tracked_agent_duration_tracking() {
        let agent = TrackedAgent::new("test".to_string());

        std::thread::sleep(std::time::Duration::from_millis(10));

        let duration = agent.state_duration_ms();
        assert!(duration >= 10);
    }

    #[test]
    fn test_state_transition_resets_timer() {
        let mut agent = TrackedAgent::new("test".to_string());

        std::thread::sleep(std::time::Duration::from_millis(10));

        let first_duration = agent.state_duration_ms();
        agent.set_state(TrackedAgentState::Active);

        let second_duration = agent.state_duration_ms();
        assert!(second_duration < first_duration);
    }
}
