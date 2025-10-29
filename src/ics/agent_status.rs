//! Agent status widget for ICS
//!
//! Shows active agents and their current activities in real-time.
//! Uses calm, non-intrusive visual design.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, StatefulWidget, Widget},
};
use std::time::SystemTime;

/// Agent activity status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentActivity {
    /// Agent is idle
    Idle,
    /// Agent is analyzing text
    Analyzing,
    /// Agent is proposing changes
    Proposing,
    /// Agent is waiting for user input
    Waiting,
    /// Agent encountered an error
    Error(String),
}

impl AgentActivity {
    /// Get color for activity status
    pub fn color(&self) -> Color {
        match self {
            AgentActivity::Idle => Color::Rgb(140, 140, 140),
            AgentActivity::Analyzing => Color::Rgb(180, 200, 180),
            AgentActivity::Proposing => Color::Rgb(200, 180, 160),
            AgentActivity::Waiting => Color::Rgb(180, 180, 200),
            AgentActivity::Error(_) => Color::Rgb(200, 140, 140),
        }
    }

    /// Get icon for activity status
    pub fn icon(&self) -> &'static str {
        match self {
            AgentActivity::Idle => "○",
            AgentActivity::Analyzing => "◐",
            AgentActivity::Proposing => "◑",
            AgentActivity::Waiting => "◓",
            AgentActivity::Error(_) => "✗",
        }
    }

    /// Get display name
    pub fn name(&self) -> &str {
        match self {
            AgentActivity::Idle => "Idle",
            AgentActivity::Analyzing => "Analyzing",
            AgentActivity::Proposing => "Proposing",
            AgentActivity::Waiting => "Waiting",
            AgentActivity::Error(_) => "Error",
        }
    }
}

/// Information about an active agent
#[derive(Debug, Clone)]
pub struct AgentInfo {
    /// Agent identifier
    pub id: String,
    /// Agent display name
    pub name: String,
    /// Current activity
    pub activity: AgentActivity,
    /// Last activity timestamp
    pub last_active: SystemTime,
    /// Activity message
    pub message: Option<String>,
}

/// Agent status panel state
#[derive(Debug, Clone, Default)]
pub struct AgentStatusState {
    /// Whether panel is visible
    visible: bool,
}

impl AgentStatusState {
    /// Create new agent status state
    pub fn new() -> Self {
        Self { visible: false }
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Show panel
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide panel
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

/// Agent status widget
pub struct AgentStatusWidget<'a> {
    /// Active agents
    agents: &'a [AgentInfo],
}

impl<'a> AgentStatusWidget<'a> {
    /// Create new agent status widget
    pub fn new(agents: &'a [AgentInfo]) -> Self {
        Self { agents }
    }
}

impl<'a> StatefulWidget for AgentStatusWidget<'a> {
    type State = AgentStatusState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if !state.visible {
            return;
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Agent Status ")
            .style(Style::default().fg(Color::Rgb(180, 180, 200)));

        // Show empty state if no agents
        if self.agents.is_empty() {
            let inner = block.inner(area);
            block.render(area, buf);

            let empty_text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No active agents",
                    Style::default().fg(Color::Rgb(140, 140, 160)),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Active agents will appear here",
                    Style::default().fg(Color::Rgb(120, 120, 140)),
                )),
            ];

            let paragraph = Paragraph::new(empty_text).alignment(ratatui::layout::Alignment::Center);
            paragraph.render(inner, buf);
            return;
        }

        // Create list items
        let items: Vec<ListItem> = self
            .agents
            .iter()
            .map(|agent| {
                let icon = agent.activity.icon();
                let color = agent.activity.color();

                let mut spans = vec![
                    Span::styled(icon, Style::default().fg(color)),
                    Span::raw(" "),
                    Span::styled(&agent.name, Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" - "),
                    Span::styled(agent.activity.name(), Style::default().fg(color)),
                ];

                // Add message if present
                if let Some(ref msg) = agent.message {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(
                        msg,
                        Style::default().fg(Color::Rgb(160, 160, 160)),
                    ));
                }

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items).block(block);
        Widget::render(list, area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_status_state() {
        let mut state = AgentStatusState::new();
        assert!(!state.is_visible());

        state.toggle();
        assert!(state.is_visible());

        state.hide();
        assert!(!state.is_visible());

        state.show();
        assert!(state.is_visible());
    }

    #[test]
    fn test_agent_activity_colors() {
        for activity in [
            AgentActivity::Idle,
            AgentActivity::Analyzing,
            AgentActivity::Proposing,
            AgentActivity::Waiting,
            AgentActivity::Error("test".to_string()),
        ] {
            let _ = activity.color();
            assert!(!activity.icon().is_empty());
            assert!(!activity.name().is_empty());
        }
    }

    #[test]
    fn test_agent_info() {
        let agent = AgentInfo {
            id: "agent-1".to_string(),
            name: "Semantic Analyzer".to_string(),
            activity: AgentActivity::Analyzing,
            last_active: SystemTime::now(),
            message: Some("Processing document".to_string()),
        };

        assert_eq!(agent.id, "agent-1");
        assert_eq!(agent.activity, AgentActivity::Analyzing);
    }
}
