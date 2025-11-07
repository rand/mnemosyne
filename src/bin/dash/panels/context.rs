//! Context panel - Display context budget utilization

use crate::widgets::{ProgressBar, StateType};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

/// Context utilization state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextState {
    Safe,      // < 60%
    Moderate,  // 60-75%
    High,      // 75-90%
    Critical,  // > 90%
}

impl ContextState {
    /// Determine state from percentage
    pub fn from_percentage(pct: f64) -> Self {
        if pct < 60.0 {
            Self::Safe
        } else if pct < 75.0 {
            Self::Moderate
        } else if pct < 90.0 {
            Self::High
        } else {
            Self::Critical
        }
    }

    /// Get state indicator type
    pub fn indicator_type(&self) -> StateType {
        match self {
            Self::Safe => StateType::ContextSafe,
            Self::Moderate => StateType::ContextModerate,
            Self::High => StateType::ContextHigh,
            Self::Critical => StateType::ContextCritical,
        }
    }

    /// Get state name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Safe => "Safe",
            Self::Moderate => "Moderate",
            Self::High => "High",
            Self::Critical => "Critical",
        }
    }

    /// Get state color
    pub fn color(&self) -> ratatui::style::Color {
        match self {
            Self::Safe => ratatui::style::Color::Green,
            Self::Moderate => ratatui::style::Color::Yellow,
            Self::High => ratatui::style::Color::LightRed,
            Self::Critical => ratatui::style::Color::Red,
        }
    }
}

/// Context panel widget
pub struct ContextPanel {
    utilization_pct: f64,
    checkpoint_count: usize,
    title: String,
}

impl ContextPanel {
    /// Create new context panel
    pub fn new() -> Self {
        Self {
            utilization_pct: 0.0,
            checkpoint_count: 0,
            title: "Context Budget".to_string(),
        }
    }

    /// Update context metrics
    pub fn update(&mut self, utilization_pct: f64, checkpoint_count: usize) {
        self.utilization_pct = utilization_pct.clamp(0.0, 100.0);
        self.checkpoint_count = checkpoint_count;
    }

    /// Set custom title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Get current context state
    pub fn state(&self) -> ContextState {
        ContextState::from_percentage(self.utilization_pct)
    }

    /// Render the context panel
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Split area: progress bar on top, details below
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(3), // Progress bar
                ratatui::layout::Constraint::Min(0),     // Details
            ])
            .split(area);

        // Progress bar with context state
        let state = self.state();
        let progress_bar = ProgressBar::new(self.utilization_pct)
            .label(format!("Context: {:.1}% ({})", self.utilization_pct, state.name()));

        progress_bar.render(
            frame,
            chunks[0],
            Block::default().title(&self.title).borders(Borders::ALL),
        );

        // Details
        let items = vec![
            ListItem::new(Line::from(vec![
                Span::styled(
                    "State: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(state.name(), Style::default().fg(state.color())),
            ])),
            ListItem::new(Line::from(vec![Span::styled(
                format!("Checkpoints: {}", self.checkpoint_count),
                Style::default().fg(ratatui::style::Color::Cyan),
            )])),
            ListItem::new(Line::from(vec![Span::styled(
                match state {
                    ContextState::Safe => "✓ Healthy context usage",
                    ContextState::Moderate => "⚠ Approaching optimization threshold",
                    ContextState::High => "⚠ Context optimization recommended",
                    ContextState::Critical => "⚠ Critical - checkpoint imminent",
                },
                Style::default()
                    .fg(state.color())
                    .add_modifier(Modifier::ITALIC),
            )])),
        ];

        let list = List::new(items).block(Block::default().borders(Borders::ALL));

        frame.render_widget(list, chunks[1]);
    }

    /// Get current utilization percentage
    pub fn utilization(&self) -> f64 {
        self.utilization_pct
    }

    /// Get checkpoint count
    pub fn checkpoints(&self) -> usize {
        self.checkpoint_count
    }
}

impl Default for ContextPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_panel_creation() {
        let panel = ContextPanel::new();
        assert_eq!(panel.utilization(), 0.0);
        assert_eq!(panel.checkpoints(), 0);
    }

    #[test]
    fn test_context_panel_update() {
        let mut panel = ContextPanel::new();
        panel.update(75.5, 3);
        assert_eq!(panel.utilization(), 75.5);
        assert_eq!(panel.checkpoints(), 3);
    }

    #[test]
    fn test_utilization_clamping() {
        let mut panel = ContextPanel::new();
        panel.update(150.0, 0);
        assert_eq!(panel.utilization(), 100.0);

        panel.update(-10.0, 0);
        assert_eq!(panel.utilization(), 0.0);
    }

    #[test]
    fn test_context_states() {
        assert_eq!(ContextState::from_percentage(30.0), ContextState::Safe);
        assert_eq!(ContextState::from_percentage(65.0), ContextState::Moderate);
        assert_eq!(ContextState::from_percentage(80.0), ContextState::High);
        assert_eq!(ContextState::from_percentage(95.0), ContextState::Critical);
    }

    #[test]
    fn test_state_boundaries() {
        assert_eq!(ContextState::from_percentage(59.9), ContextState::Safe);
        assert_eq!(ContextState::from_percentage(60.0), ContextState::Moderate);
        assert_eq!(ContextState::from_percentage(74.9), ContextState::Moderate);
        assert_eq!(ContextState::from_percentage(75.0), ContextState::High);
        assert_eq!(ContextState::from_percentage(89.9), ContextState::High);
        assert_eq!(ContextState::from_percentage(90.0), ContextState::Critical);
    }

    #[test]
    fn test_state_names() {
        assert_eq!(ContextState::Safe.name(), "Safe");
        assert_eq!(ContextState::Moderate.name(), "Moderate");
        assert_eq!(ContextState::High.name(), "High");
        assert_eq!(ContextState::Critical.name(), "Critical");
    }
}
