//! Real-time Completion Popup Widget
//!
//! Displays completion suggestions in a popup overlay with:
//! - Triggered by @ or # characters
//! - Real-time filtering as user types
//! - Keyboard navigation (up/down arrows)
//! - Visual feedback for selection
//! - Integration with CompletionEngine

use crate::ics::editor::Position;
use crate::ics::symbols::CompletionCandidate;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget},
};

/// Completion popup state
pub struct CompletionPopup {
    /// Available completion candidates
    candidates: Vec<CompletionCandidate>,

    /// Currently selected index
    selected: usize,

    /// Whether popup is visible
    visible: bool,

    /// Position where completion was triggered
    trigger_position: Position,

    /// Prefix being completed
    prefix: String,

    /// List widget state for rendering
    list_state: ListState,
}

impl Default for CompletionPopup {
    fn default() -> Self {
        Self::new()
    }
}

impl CompletionPopup {
    /// Create new completion popup
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            candidates: Vec::new(),
            selected: 0,
            visible: false,
            trigger_position: Position { line: 0, column: 0 },
            prefix: String::new(),
            list_state,
        }
    }

    /// Show popup with candidates
    pub fn show(
        &mut self,
        candidates: Vec<CompletionCandidate>,
        trigger_pos: Position,
        prefix: String,
    ) {
        self.candidates = candidates;
        self.trigger_position = trigger_pos;
        self.prefix = prefix;
        self.selected = 0;
        self.list_state.select(Some(0));
        self.visible = !self.candidates.is_empty();
    }

    /// Hide popup
    pub fn hide(&mut self) {
        self.visible = false;
        self.candidates.clear();
        self.prefix.clear();
    }

    /// Check if popup is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Get current prefix
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Get selected completion
    pub fn selected_completion(&self) -> Option<&CompletionCandidate> {
        self.candidates.get(self.selected)
    }

    /// Move selection up
    pub fn select_previous(&mut self) {
        if self.candidates.is_empty() {
            return;
        }

        if self.selected > 0 {
            self.selected -= 1;
        } else {
            self.selected = self.candidates.len() - 1;
        }
        self.list_state.select(Some(self.selected));
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if self.candidates.is_empty() {
            return;
        }

        if self.selected < self.candidates.len() - 1 {
            self.selected += 1;
        } else {
            self.selected = 0;
        }
        self.list_state.select(Some(self.selected));
    }

    /// Get number of candidates
    pub fn candidate_count(&self) -> usize {
        self.candidates.len()
    }

    /// Calculate popup area based on cursor position
    ///
    /// Places popup near cursor, ensuring it fits within terminal bounds
    pub fn popup_area(&self, terminal_area: Rect, cursor_line: u16, cursor_col: u16) -> Rect {
        // Popup dimensions
        let width = 40;
        let height = (self.candidates.len().min(10) + 2) as u16; // +2 for borders

        // Calculate position (below cursor, slightly offset)
        let x = cursor_col.min(terminal_area.width.saturating_sub(width));
        let y = (cursor_line + 1).min(terminal_area.height.saturating_sub(height));

        Rect {
            x,
            y,
            width,
            height,
        }
    }

    /// Render the completion popup
    pub fn render(&mut self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        if !self.visible || self.candidates.is_empty() {
            return;
        }

        // Create list items
        let items: Vec<ListItem> = self
            .candidates
            .iter()
            .map(|candidate| {
                let kind_icon = match candidate.kind {
                    crate::ics::symbols::SymbolKind::Variable => "V",
                    crate::ics::symbols::SymbolKind::Function => "F",
                    crate::ics::symbols::SymbolKind::Type => "T",
                    crate::ics::symbols::SymbolKind::Concept => "C",
                    crate::ics::symbols::SymbolKind::File => "f",
                    crate::ics::symbols::SymbolKind::Entity => "E",
                };

                let kind_color = match candidate.kind {
                    crate::ics::symbols::SymbolKind::Variable => Color::Rgb(180, 200, 180),
                    crate::ics::symbols::SymbolKind::Function => Color::Rgb(200, 180, 160),
                    crate::ics::symbols::SymbolKind::Type => Color::Rgb(160, 180, 200),
                    crate::ics::symbols::SymbolKind::Concept => Color::Rgb(200, 180, 200),
                    crate::ics::symbols::SymbolKind::File => Color::Rgb(200, 200, 160),
                    crate::ics::symbols::SymbolKind::Entity => Color::Rgb(180, 180, 180),
                };

                // Build display line
                let spans = vec![
                    Span::styled(
                        format!("[{}] ", kind_icon),
                        Style::default().fg(kind_color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(&candidate.text, Style::default().fg(Color::White)),
                    Span::styled(
                        candidate
                            .detail
                            .as_ref()
                            .map(|d| format!(" - {}", d))
                            .unwrap_or_default(),
                        Style::default().fg(Color::Gray),
                    ),
                ];

                ListItem::new(Line::from(spans))
            })
            .collect();

        // Create list widget
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Completions ({})", self.candidates.len()))
                    .style(Style::default().bg(Color::Rgb(30, 30, 30))),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(60, 60, 80))
                    .add_modifier(Modifier::BOLD),
            );

        // Render with state
        StatefulWidget::render(list, area, buf, &mut self.list_state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ics::symbols::SymbolKind;

    #[test]
    fn test_completion_popup_lifecycle() {
        let mut popup = CompletionPopup::new();

        // Initially hidden
        assert!(!popup.is_visible());

        // Show with candidates
        let candidates = vec![
            CompletionCandidate {
                text: "@test".to_string(),
                kind: SymbolKind::Variable,
                detail: Some("Test variable".to_string()),
                score: 100.0,
            },
            CompletionCandidate {
                text: "@test_func".to_string(),
                kind: SymbolKind::Function,
                detail: None,
                score: 90.0,
            },
        ];

        popup.show(
            candidates,
            Position { line: 5, column: 10 },
            "te".to_string(),
        );

        // Now visible
        assert!(popup.is_visible());
        assert_eq!(popup.candidate_count(), 2);
        assert_eq!(popup.prefix(), "te");

        // Hide
        popup.hide();
        assert!(!popup.is_visible());
        assert_eq!(popup.candidate_count(), 0);
    }

    #[test]
    fn test_completion_selection() {
        let mut popup = CompletionPopup::new();

        let candidates = vec![
            CompletionCandidate {
                text: "@first".to_string(),
                kind: SymbolKind::Variable,
                detail: None,
                score: 100.0,
            },
            CompletionCandidate {
                text: "@second".to_string(),
                kind: SymbolKind::Variable,
                detail: None,
                score: 90.0,
            },
            CompletionCandidate {
                text: "@third".to_string(),
                kind: SymbolKind::Variable,
                detail: None,
                score: 80.0,
            },
        ];

        popup.show(candidates, Position { line: 0, column: 0 }, "".to_string());

        // Initially first is selected
        assert_eq!(popup.selected_completion().unwrap().text, "@first");

        // Move down
        popup.select_next();
        assert_eq!(popup.selected_completion().unwrap().text, "@second");

        popup.select_next();
        assert_eq!(popup.selected_completion().unwrap().text, "@third");

        // Wrap around
        popup.select_next();
        assert_eq!(popup.selected_completion().unwrap().text, "@first");

        // Move up
        popup.select_previous();
        assert_eq!(popup.selected_completion().unwrap().text, "@third");
    }

    #[test]
    fn test_popup_area_calculation() {
        let popup = CompletionPopup::new();

        let terminal_area = Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 24,
        };

        // Popup near top-left
        let area = popup.popup_area(terminal_area, 2, 10);
        assert_eq!(area.x, 10);
        assert_eq!(area.y, 3); // Below cursor

        // Popup near right edge (should clamp)
        let area = popup.popup_area(terminal_area, 10, 70);
        assert!(area.x <= 80 - 40); // Doesn't overflow

        // Popup near bottom (should clamp)
        let area = popup.popup_area(terminal_area, 22, 10);
        assert!(area.y + area.height <= 24); // Doesn't overflow
    }
}
