//! State indicator widget - Color-coded status badges
//!
//! Provides visual indicators for various states with btop-inspired color schemes.

use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

/// State types for visual indication
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateType {
    /// Agent/process is active/running
    Active,
    /// Agent/process is idle/ready
    Idle,
    /// Agent/process is waiting/blocked
    Waiting,
    /// Agent/process completed successfully
    Completed,
    /// Agent/process failed/errored
    Failed,
    /// Health is good (< 5 errors)
    Healthy,
    /// Health is degraded (5+ errors)
    Degraded,
    /// Critical state (urgent attention needed)
    Critical,
    /// Memory operation (store)
    MemoryStore,
    /// Memory operation (recall)
    MemoryRecall,
    /// Memory evolution
    MemoryEvolution,
    /// Skill loaded
    SkillLoaded,
    /// Skill used
    SkillUsed,
    /// Work in progress
    WorkInProgress,
    /// Work completed
    WorkCompleted,
    /// Context safe (< 60%)
    ContextSafe,
    /// Context moderate (60-75%)
    ContextModerate,
    /// Context high (75-90%)
    ContextHigh,
    /// Context critical (> 90%)
    ContextCritical,
}

/// State indicator widget
pub struct StateIndicator {
    state_type: StateType,
    text: String,
    show_icon: bool,
}

impl StateIndicator {
    /// Create new state indicator
    pub fn new(state_type: StateType, text: impl Into<String>) -> Self {
        Self {
            state_type,
            text: text.into(),
            show_icon: true,
        }
    }

    /// Set whether to show icon
    pub fn show_icon(mut self, show: bool) -> Self {
        self.show_icon = show;
        self
    }

    /// Get color for this state type (btop-inspired palette)
    fn color(&self) -> Color {
        match self.state_type {
            StateType::Active => Color::Green,
            StateType::Idle => Color::Gray,
            StateType::Waiting => Color::Yellow,
            StateType::Completed => Color::LightGreen,
            StateType::Failed => Color::Red,
            StateType::Healthy => Color::Green,
            StateType::Degraded => Color::LightRed,
            StateType::Critical => Color::Red,
            StateType::MemoryStore => Color::Blue,
            StateType::MemoryRecall => Color::Cyan,
            StateType::MemoryEvolution => Color::Magenta,
            StateType::SkillLoaded => Color::LightMagenta,
            StateType::SkillUsed => Color::Magenta,
            StateType::WorkInProgress => Color::Yellow,
            StateType::WorkCompleted => Color::Green,
            StateType::ContextSafe => Color::Green,
            StateType::ContextModerate => Color::Yellow,
            StateType::ContextHigh => Color::LightRed,
            StateType::ContextCritical => Color::Red,
        }
    }

    /// Get icon/symbol for this state type
    fn icon(&self) -> &'static str {
        match self.state_type {
            StateType::Active => "●",
            StateType::Idle => "○",
            StateType::Waiting => "◐",
            StateType::Completed => "✓",
            StateType::Failed => "✗",
            StateType::Healthy => "♥",
            StateType::Degraded => "⚠",
            StateType::Critical => "⚠",
            StateType::MemoryStore => "↓",
            StateType::MemoryRecall => "↑",
            StateType::MemoryEvolution => "⟳",
            StateType::SkillLoaded => "⊕",
            StateType::SkillUsed => "⊙",
            StateType::WorkInProgress => "▸",
            StateType::WorkCompleted => "▪",
            StateType::ContextSafe => "▁",
            StateType::ContextModerate => "▃",
            StateType::ContextHigh => "▅",
            StateType::ContextCritical => "▇",
        }
    }

    /// Render as a styled span
    pub fn render(&self) -> Span<'static> {
        let color = self.color();
        let content = if self.show_icon {
            format!("{} {}", self.icon(), self.text)
        } else {
            self.text.clone()
        };

        Span::styled(
            content,
            Style::default()
                .fg(color)
                .add_modifier(Modifier::BOLD),
        )
    }

    /// Render with custom modifier
    pub fn render_with_modifier(&self, modifier: Modifier) -> Span<'static> {
        let color = self.color();
        let content = if self.show_icon {
            format!("{} {}", self.icon(), self.text)
        } else {
            self.text.clone()
        };

        Span::styled(
            content,
            Style::default().fg(color).add_modifier(modifier),
        )
    }

    /// Render just the icon (no text)
    pub fn render_icon_only(&self) -> Span<'static> {
        Span::styled(
            self.icon().to_string(),
            Style::default()
                .fg(self.color())
                .add_modifier(Modifier::BOLD),
        )
    }

    /// Get the underlying text
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get the state type
    pub fn state_type(&self) -> StateType {
        self.state_type
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_indicator_creation() {
        let indicator = StateIndicator::new(StateType::Active, "Running");
        assert_eq!(indicator.text(), "Running");
        assert_eq!(indicator.state_type(), StateType::Active);
    }

    #[test]
    fn test_state_colors() {
        assert_eq!(
            StateIndicator::new(StateType::Active, "").color(),
            Color::Green
        );
        assert_eq!(
            StateIndicator::new(StateType::Failed, "").color(),
            Color::Red
        );
        assert_eq!(
            StateIndicator::new(StateType::Waiting, "").color(),
            Color::Yellow
        );
    }

    #[test]
    fn test_icon_display() {
        let with_icon = StateIndicator::new(StateType::Active, "Test");
        let span = with_icon.render();
        assert!(span.content.contains("●"));

        let without_icon = StateIndicator::new(StateType::Active, "Test").show_icon(false);
        let span = without_icon.render();
        assert!(!span.content.contains("●"));
    }
}
