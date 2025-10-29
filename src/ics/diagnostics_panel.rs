//! Diagnostics panel for ICS
//!
//! Displays validation errors, warnings, and hints from the editor.
//! Provides quick navigation to problem locations.

use crate::ics::editor::{Diagnostic, Severity};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget, Widget},
};

/// Diagnostics panel state
#[derive(Debug, Clone)]
pub struct DiagnosticsPanelState {
    /// List state for selection
    list_state: ListState,
    /// Whether panel is visible
    visible: bool,
    /// Filter by severity
    filter: Option<Severity>,
}

impl Default for DiagnosticsPanelState {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticsPanelState {
    /// Create new diagnostics panel state
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
            visible: false,
            filter: None,
        }
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

    /// Set severity filter
    pub fn set_filter(&mut self, filter: Option<Severity>) {
        self.filter = filter;
    }

    /// Get current filter
    pub fn filter(&self) -> Option<Severity> {
        self.filter
    }

    /// Select next diagnostic
    pub fn select_next(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => (i + 1).min(count - 1),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Select previous diagnostic
    pub fn select_previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Get selected index
    pub fn selected(&self) -> Option<usize> {
        self.list_state.selected()
    }
}

/// Diagnostics panel widget
pub struct DiagnosticsPanel<'a> {
    /// Diagnostics to display
    diagnostics: &'a [Diagnostic],
}

impl<'a> DiagnosticsPanel<'a> {
    /// Create new diagnostics panel
    pub fn new(diagnostics: &'a [Diagnostic]) -> Self {
        Self { diagnostics }
    }
}

impl<'a> StatefulWidget for DiagnosticsPanel<'a> {
    type State = DiagnosticsPanelState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if !state.visible {
            return;
        }

        // Filter diagnostics by severity if filter is set
        let filtered_diagnostics: Vec<&Diagnostic> = if let Some(filter) = state.filter {
            self.diagnostics
                .iter()
                .filter(|d| d.severity == filter)
                .collect()
        } else {
            self.diagnostics.iter().collect()
        };

        // Count by severity
        let error_count = self
            .diagnostics
            .iter()
            .filter(|d| matches!(d.severity, Severity::Error))
            .count();
        let warning_count = self
            .diagnostics
            .iter()
            .filter(|d| matches!(d.severity, Severity::Warning))
            .count();
        let hint_count = self
            .diagnostics
            .iter()
            .filter(|d| matches!(d.severity, Severity::Hint))
            .count();

        let title = format!(
            " Diagnostics ({} errors, {} warnings, {} hints) ",
            error_count, warning_count, hint_count
        );

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().fg(Color::Rgb(180, 180, 200)));

        // Create list items
        let items: Vec<ListItem> = filtered_diagnostics
            .iter()
            .map(|diagnostic| {
                let (icon, color) = match diagnostic.severity {
                    Severity::Error => ("✗", Color::Rgb(200, 140, 140)),
                    Severity::Warning => ("⚠", Color::Rgb(200, 180, 120)),
                    Severity::Hint => ("●", Color::Rgb(160, 180, 180)),
                };

                let location = format!("Ln {}, Col {}", diagnostic.position.line + 1, diagnostic.position.column + 1);

                let line = Line::from(vec![
                    Span::styled(icon, Style::default().fg(color)),
                    Span::raw(" "),
                    Span::styled(
                        location,
                        Style::default().fg(Color::Rgb(160, 160, 160)),
                    ),
                    Span::raw(" "),
                    Span::raw(&diagnostic.message),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(40, 40, 50))
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        StatefulWidget::render(list, area, buf, &mut state.list_state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ics::editor::Position;

    #[test]
    fn test_diagnostics_panel_state() {
        let mut state = DiagnosticsPanelState::new();
        assert!(!state.is_visible());

        state.toggle();
        assert!(state.is_visible());

        state.hide();
        assert!(!state.is_visible());

        state.show();
        assert!(state.is_visible());
    }

    #[test]
    fn test_diagnostics_selection() {
        let mut state = DiagnosticsPanelState::new();
        assert_eq!(state.selected(), None);

        state.select_next(5);
        assert_eq!(state.selected(), Some(0));

        state.select_next(5);
        assert_eq!(state.selected(), Some(1));

        state.select_previous();
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn test_severity_filter() {
        let mut state = DiagnosticsPanelState::new();
        assert_eq!(state.filter(), None);

        state.set_filter(Some(Severity::Error));
        assert_eq!(state.filter(), Some(Severity::Error));

        state.set_filter(None);
        assert_eq!(state.filter(), None);
    }

    #[test]
    fn test_diagnostic_counts() {
        let diagnostics = vec![
            Diagnostic {
                position: Position { line: 0, column: 0 },
                length: 1,
                severity: Severity::Error,
                message: "Error 1".to_string(),
                suggestion: None,
            },
            Diagnostic {
                position: Position { line: 1, column: 0 },
                length: 1,
                severity: Severity::Warning,
                message: "Warning 1".to_string(),
                suggestion: None,
            },
            Diagnostic {
                position: Position { line: 2, column: 0 },
                length: 1,
                severity: Severity::Hint,
                message: "Hint 1".to_string(),
                suggestion: None,
            },
        ];

        let error_count = diagnostics
            .iter()
            .filter(|d| matches!(d.severity, Severity::Error))
            .count();
        let warning_count = diagnostics
            .iter()
            .filter(|d| matches!(d.severity, Severity::Warning))
            .count();
        let hint_count = diagnostics
            .iter()
            .filter(|d| matches!(d.severity, Severity::Hint))
            .count();

        assert_eq!(error_count, 1);
        assert_eq!(warning_count, 1);
        assert_eq!(hint_count, 1);
    }
}
