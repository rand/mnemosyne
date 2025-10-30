//! TUI view components

use crate::pty::ParsedChunk;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use std::collections::VecDeque;

/// Chat view showing Claude Code conversation
pub struct ChatView {
    /// Message history
    messages: VecDeque<ParsedChunk>,
    /// Maximum messages to keep
    max_messages: usize,
    /// Scroll offset
    scroll_offset: usize,
}

impl ChatView {
    /// Create new chat view
    pub fn new() -> Self {
        Self {
            messages: VecDeque::new(),
            max_messages: 1000,
            scroll_offset: 0,
        }
    }

    /// Add message to chat
    pub fn add_message(&mut self, chunk: ParsedChunk) {
        self.messages.push_back(chunk);
        if self.messages.len() > self.max_messages {
            self.messages.pop_front();
        }
    }

    /// Clear messages
    pub fn clear(&mut self) {
        self.messages.clear();
        self.scroll_offset = 0;
    }

    /// Scroll up
    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    /// Scroll down
    pub fn scroll_down(&mut self, amount: usize) {
        let max_offset = self.messages.len().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + amount).min(max_offset);
    }

    /// Render chat view
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .messages
            .iter()
            .skip(self.scroll_offset)
            .map(|chunk| {
                let style = if chunk.is_error {
                    Style::default().fg(Color::Red)
                } else if chunk.is_tool_use {
                    Style::default().fg(Color::Cyan)
                } else if let Some(agent) = chunk.agent {
                    let (r, g, b) = agent.color();
                    Style::default().fg(Color::Rgb(r, g, b))
                } else {
                    Style::default()
                };

                let prefix = if let Some(agent) = chunk.agent {
                    format!("[{}] ", agent.display_name())
                } else {
                    String::new()
                };

                ListItem::new(Line::from(vec![
                    Span::styled(prefix, style.add_modifier(Modifier::BOLD)),
                    Span::styled(&chunk.text, style),
                ]))
            })
            .collect();

        let block = Block::default()
            .title("Claude Code Chat")
            .borders(Borders::ALL);

        let list = List::new(items).block(block);
        frame.render_widget(list, area);
    }
}

/// Status dashboard showing system metrics
pub struct Dashboard {
    /// Active agents count
    active_agents: usize,
    /// Total messages
    total_messages: usize,
    /// Memory usage (MB)
    memory_mb: f32,
    /// CPU usage percentage
    cpu_percent: f32,
}

impl Dashboard {
    /// Create new dashboard
    pub fn new() -> Self {
        Self {
            active_agents: 0,
            total_messages: 0,
            memory_mb: 0.0,
            cpu_percent: 0.0,
        }
    }

    /// Update metrics
    pub fn update(&mut self, active_agents: usize, total_messages: usize) {
        self.active_agents = active_agents;
        self.total_messages = total_messages;
        // TODO: Gather real metrics
    }

    /// Render dashboard
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let text = vec![
            Line::from(vec![
                Span::styled("Active Agents: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}", self.active_agents)),
            ]),
            Line::from(vec![
                Span::styled("Messages: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}", self.total_messages)),
            ]),
            Line::from(vec![
                Span::styled("Memory: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{:.1} MB", self.memory_mb)),
            ]),
            Line::from(vec![
                Span::styled("CPU: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{:.1}%", self.cpu_percent)),
            ]),
        ];

        let block = Block::default()
            .title("System Dashboard")
            .borders(Borders::ALL);

        let paragraph = Paragraph::new(text).block(block);
        frame.render_widget(paragraph, area);
    }
}

/// ICS panel (embedded)
pub struct IcsPanel {
    /// Content to display
    content: String,
    /// Whether panel is visible
    visible: bool,
}

impl IcsPanel {
    /// Create new ICS panel
    pub fn new() -> Self {
        Self {
            content: String::from("ICS - Integrated Context Studio\n\nPress Ctrl+E to toggle"),
            visible: false,
        }
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Set content
    pub fn set_content(&mut self, content: String) {
        self.content = content;
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Render ICS panel
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let block = Block::default()
            .title("ICS - Context Studio")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));

        let paragraph = Paragraph::new(self.content.as_str()).block(block);
        frame.render_widget(paragraph, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_view_creation() {
        let view = ChatView::new();
        assert_eq!(view.messages.len(), 0);
    }

    #[test]
    fn test_dashboard_creation() {
        let dashboard = Dashboard::new();
        assert_eq!(dashboard.active_agents, 0);
    }

    #[test]
    fn test_ics_panel_toggle() {
        let mut panel = IcsPanel::new();
        assert!(!panel.is_visible());
        panel.toggle();
        assert!(panel.is_visible());
        panel.toggle();
        assert!(!panel.is_visible());
    }
}
