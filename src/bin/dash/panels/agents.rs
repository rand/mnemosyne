//! Agents panel - Display agent activity with health indicators

use crate::time_series::TimeSeriesBuffer;
use crate::widgets::{Sparkline, StateIndicator, StateType};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};
use serde::Deserialize;

/// Agent info from API
#[derive(Debug, Clone, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub state: AgentState,
    #[serde(default)]
    pub health: Option<AgentHealth>,
}

/// Agent state (matches API state enum)
#[derive(Debug, Clone, Deserialize)]
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

/// Agents panel widget
pub struct AgentsPanel {
    agents: Vec<AgentInfo>,
    title: String,
    active_count_history: TimeSeriesBuffer<f32>,
}

impl AgentsPanel {
    /// Create new agents panel
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            title: "Active Agents".to_string(),
            active_count_history: TimeSeriesBuffer::new(50),
        }
    }

    /// Update agents data
    pub fn update(&mut self, agents: Vec<AgentInfo>) {
        // Count active agents
        let active_count = agents.iter()
            .filter(|agent| matches!(agent.state, AgentState::Active { .. }))
            .count() as f32;

        self.active_count_history.push(active_count);
        self.agents = agents;
    }

    /// Set custom title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
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

        // Activity trend sparkline (show if we have history)
        if !active_count_data.is_empty() && self.agents.len() > 0 {
            let sparkline = Sparkline::new(&active_count_data)
                .width(15)
                .style(Style::default().fg(ratatui::style::Color::Green));

            let mut spans = vec![
                Span::styled(
                    "Activity: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ];
            spans.extend(sparkline.render().spans);

            items.push(ListItem::new(Line::from(spans)));
        }

        // Agent list
        if self.agents.is_empty() {
            items.push(ListItem::new(Line::from(vec![Span::styled(
                "No active agents",
                Style::default()
                    .fg(ratatui::style::Color::Gray)
                    .add_modifier(Modifier::ITALIC),
            )])));
        } else {
            let agent_items: Vec<ListItem> = self.agents
                .iter()
                .map(|agent| {
                    // State indicator
                    let state_type = Self::state_to_indicator_type(&agent.state);
                    let state_indicator = StateIndicator::new(
                        state_type,
                        Self::state_description(&agent.state),
                    );

                    // Health indicator (if available)
                    let health_span = if let Some(health) = &agent.health {
                        let health_type = if health.is_healthy {
                            StateType::Healthy
                        } else {
                            StateType::Degraded
                        };
                        let health_indicator = StateIndicator::new(
                            health_type,
                            format!("{}", health.error_count),
                        );
                        Some(health_indicator.render_icon_only())
                    } else {
                        None
                    };

                    // Build line with agent ID, state, and health
                    let mut spans = vec![
                        Span::styled(
                            format!("{:12}", agent.id),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        state_indicator.render(),
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
                .title(format!("{} ({})", self.title, self.agents.len()))
                .borders(Borders::ALL),
        );

        frame.render_widget(list, area);
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

    #[test]
    fn test_agents_panel_creation() {
        let panel = AgentsPanel::new();
        assert_eq!(panel.agent_count(), 0);
    }

    #[test]
    fn test_agents_panel_update() {
        let mut panel = AgentsPanel::new();

        let agents = vec![AgentInfo {
            id: "executor".to_string(),
            state: AgentState::Active {
                task: "test task".to_string(),
            },
            health: Some(AgentHealth {
                error_count: 0,
                is_healthy: true,
            }),
        }];

        panel.update(agents);
        assert_eq!(panel.agent_count(), 1);
    }

    #[test]
    fn test_state_description_truncation() {
        let long_task = "a".repeat(50);
        let state = AgentState::Active { task: long_task };
        let desc = AgentsPanel::state_description(&state);
        assert!(desc.len() <= 43); // 40 + "..."
    }
}
