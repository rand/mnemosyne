//! Editor widget for rendering text buffers
#![allow(dead_code)]
//!
//! Provides elegant, minimal UI for text editing with:
//! - Line numbers with subtle styling
//! - Cursor rendering
//! - Smooth scrolling
//! - Change attribution (color-coded by actor)
//! - Inline diagnostic indicators (gutter icons and text underlines)

use super::{CrdtBuffer, CursorState, Diagnostic, Severity};
use ratatui::{
    buffer::Buffer as RatatuiBuffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, StatefulWidget, Widget},
};

/// Editor widget state
pub struct EditorState {
    /// Vertical scroll offset (line number)
    pub scroll_offset: usize,

    /// Horizontal scroll offset (column)
    pub h_scroll_offset: usize,

    /// Whether to show line numbers
    pub show_line_numbers: bool,

    /// Whether to show change attribution colors
    pub show_attribution: bool,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            scroll_offset: 0,
            h_scroll_offset: 0,
            show_line_numbers: true,
            show_attribution: true,
        }
    }
}

impl EditorState {
    /// Scroll to ensure cursor is visible
    pub fn ensure_cursor_visible(&mut self, cursor: &CursorState, viewport_height: usize) {
        let cursor_line = cursor.position.line;

        // Scroll down if cursor is below viewport
        if cursor_line >= self.scroll_offset + viewport_height {
            self.scroll_offset = cursor_line - viewport_height + 1;
        }

        // Scroll up if cursor is above viewport
        if cursor_line < self.scroll_offset {
            self.scroll_offset = cursor_line;
        }
    }
}

/// Editor widget for CrdtBuffer
pub struct EditorWidget<'a> {
    /// CRDT buffer to render (with built-in attribution)
    buffer: &'a CrdtBuffer,

    /// Optional diagnostics for inline indicators
    diagnostics: Option<&'a [Diagnostic]>,

    /// Block styling
    block: Option<Block<'a>>,

    /// Whether editor has focus
    focused: bool,
}

impl<'a> EditorWidget<'a> {
    /// Create new editor widget
    pub fn new(buffer: &'a CrdtBuffer) -> Self {
        Self {
            buffer,
            diagnostics: None,
            block: None,
            focused: false,
        }
    }

    /// Set diagnostics for inline indicators
    pub fn diagnostics(mut self, diagnostics: &'a [Diagnostic]) -> Self {
        self.diagnostics = Some(diagnostics);
        self
    }

    /// Set block styling
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Set focus state
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Get line number width (for gutter)
    fn line_number_width(&self) -> usize {
        let line_count = self.buffer.line_count().unwrap_or(0);
        if line_count == 0 {
            return 3; // Min width
        }
        (line_count.ilog10() as usize + 1).max(3)
    }

    /// Get attribution color for character position
    fn attribution_color(&self, char_pos: usize) -> Option<Color> {
        if let Some(attr) = self.buffer.attribution_at(char_pos) {
            let (r, g, b) = attr.actor.color();
            return Some(Color::Rgb(r, g, b));
        }
        None
    }

    /// Get the most severe diagnostic for a line (for gutter display)
    fn line_diagnostic(&self, line_num: usize) -> Option<&Diagnostic> {
        let diagnostics = self.diagnostics?;

        // Find all diagnostics on this line
        let mut line_diagnostics: Vec<&Diagnostic> = diagnostics
            .iter()
            .filter(|d| d.position.line == line_num)
            .collect();

        if line_diagnostics.is_empty() {
            return None;
        }

        // Sort by severity (Error > Warning > Hint)
        line_diagnostics.sort_by_key(|d| match d.severity {
            Severity::Error => 0,
            Severity::Warning => 1,
            Severity::Hint => 2,
        });

        Some(line_diagnostics[0])
    }

    /// Get diagnostic at a specific position (for underlining)
    fn diagnostic_at(&self, line: usize, column: usize) -> Option<&Diagnostic> {
        let diagnostics = self.diagnostics?;

        diagnostics.iter().find(|d| {
            d.position.line == line
                && column >= d.position.column
                && column < d.position.column + d.length
        })
    }
}

impl<'a> StatefulWidget for EditorWidget<'a> {
    type State = EditorState;

    fn render(self, area: Rect, buf: &mut RatatuiBuffer, state: &mut Self::State) {
        // Apply block if present
        let inner_area = if let Some(ref block) = self.block {
            let inner = block.inner(area);
            block.clone().render(area, buf);
            inner
        } else {
            area
        };

        // Ensure cursor is visible
        state.ensure_cursor_visible(&self.buffer.cursor, inner_area.height as usize);

        // Calculate layout (line numbers + content)
        let line_num_width = if state.show_line_numbers {
            self.line_number_width() as u16 + 2 // +2 for padding
        } else {
            0
        };

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(line_num_width), Constraint::Min(10)])
            .split(inner_area);

        let line_num_area = chunks[0];
        let content_area = chunks[1];

        // Render line numbers
        if state.show_line_numbers && line_num_width > 0 {
            self.render_line_numbers(line_num_area, buf, state);
        }

        // Render content
        self.render_content(content_area, buf, state);
    }
}

impl<'a> EditorWidget<'a> {
    /// Render line numbers in the gutter
    fn render_line_numbers(&self, area: Rect, buf: &mut RatatuiBuffer, state: &EditorState) {
        let line_count = self.buffer.line_count().unwrap_or(0);
        let viewport_height = area.height as usize;

        for i in 0..viewport_height {
            let line_num = state.scroll_offset + i;
            if line_num >= line_count {
                break;
            }

            let is_cursor_line = line_num == self.buffer.cursor.position.line;

            // Check for diagnostic on this line
            let diagnostic = self.line_diagnostic(line_num);

            // Line number style
            let line_num_style = if is_cursor_line && self.focused {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            // Render line number (reserve space for diagnostic icon)
            let available_width = area.width.saturating_sub(2) as usize; // Reserve 2 chars for icon
            let line_num_text = format!("{:>width$}", line_num + 1, width = available_width);

            buf.set_string(area.x, area.y + i as u16, &line_num_text, line_num_style);

            // Render diagnostic icon if present
            if let Some(diag) = diagnostic {
                let (icon, color) = match diag.severity {
                    Severity::Error => ("✗", Color::Rgb(200, 140, 140)),
                    Severity::Warning => ("⚠", Color::Rgb(200, 180, 120)),
                    Severity::Hint => ("●", Color::Rgb(140, 140, 160)),
                };

                let icon_style = Style::default().fg(color);
                buf.set_string(
                    area.x + available_width as u16 + 1,
                    area.y + i as u16,
                    icon,
                    icon_style,
                );
            }
        }
    }

    /// Render editor content
    fn render_content(&self, area: Rect, buf: &mut RatatuiBuffer, state: &EditorState) {
        let line_count = self.buffer.line_count().unwrap_or(0);
        let viewport_height = area.height as usize;
        let viewport_width = area.width as usize;

        let mut char_pos = 0;

        for i in 0..viewport_height {
            let line_num = state.scroll_offset + i;
            if line_num >= line_count {
                break;
            }

            if let Ok(Some(line_text)) = self.buffer.line(line_num) {
                let is_cursor_line = line_num == self.buffer.cursor.position.line;

                // Apply horizontal scroll
                let visible_text: String = line_text
                    .chars()
                    .skip(state.h_scroll_offset)
                    .take(viewport_width)
                    .collect();

                // Render line with both attribution and diagnostics
                self.render_line_with_diagnostics(
                    area.x,
                    area.y + i as u16,
                    &visible_text,
                    line_num,
                    state.h_scroll_offset,
                    char_pos,
                    state.show_attribution,
                    buf,
                );

                // Render cursor if on this line
                if is_cursor_line && self.focused {
                    let cursor_col = self.buffer.cursor.position.column;
                    if cursor_col >= state.h_scroll_offset
                        && cursor_col < state.h_scroll_offset + viewport_width
                    {
                        let cursor_x = area.x + (cursor_col - state.h_scroll_offset) as u16;
                        let cursor_y = area.y + i as u16;

                        // Render cursor as inverted color
                        if let Some(cell) = buf.cell_mut((cursor_x, cursor_y)) {
                            let fg = cell.fg;
                            let bg = cell.bg;
                            cell.set_fg(bg);
                            cell.set_bg(fg);
                            cell.set_style(cell.style().add_modifier(Modifier::REVERSED));
                        }
                    }
                }

                char_pos += line_text.len() + 1; // +1 for newline
            }
        }
    }

    /// Render line with diagnostics (and optionally attribution)
    fn render_line_with_diagnostics(
        &self,
        x: u16,
        y: u16,
        text: &str,
        line_num: usize,
        h_scroll: usize,
        start_char_pos: usize,
        show_attribution: bool,
        buf: &mut RatatuiBuffer,
    ) {
        for (i, ch) in text.chars().enumerate() {
            let column = h_scroll + i;
            let char_pos = start_char_pos + i;

            // Get base color (attribution or default)
            let base_color = if show_attribution {
                self.attribution_color(char_pos).unwrap_or(Color::White)
            } else {
                Color::White
            };

            // Check for diagnostic at this position
            let has_diagnostic = self.diagnostic_at(line_num, column).is_some();

            // Build style with underline if diagnostic present
            let mut style = Style::default().fg(base_color);
            if has_diagnostic {
                style = style.add_modifier(Modifier::UNDERLINED);
            }

            buf.set_string(x + i as u16, y, ch.to_string(), style);
        }
    }

    /// Render line with attribution colors (legacy method for compatibility)
    fn render_line_with_attribution(
        &self,
        x: u16,
        y: u16,
        text: &str,
        start_char_pos: usize,
        buf: &mut RatatuiBuffer,
    ) {
        for (i, ch) in text.chars().enumerate() {
            let char_pos = start_char_pos + i;
            let color = self.attribution_color(char_pos).unwrap_or(Color::White);
            let style = Style::default().fg(color);

            buf.set_string(x + i as u16, y, ch.to_string(), style);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ics::editor::Position;

    #[test]
    fn test_editor_state_scroll() {
        let mut state = EditorState::default();
        let mut cursor = CursorState::default();

        // Cursor at line 50, viewport height 20
        cursor.position = Position {
            line: 50,
            column: 0,
        };
        state.ensure_cursor_visible(&cursor, 20);

        // Should scroll to show cursor
        assert!(state.scroll_offset > 0);
        assert!(state.scroll_offset <= 50);
        assert!(cursor.position.line >= state.scroll_offset);
        assert!(cursor.position.line < state.scroll_offset + 20);
    }

    #[test]
    fn test_line_number_width() {
        use crate::ics::editor::Actor;
        let buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();
        let widget = EditorWidget::new(&buffer);

        // Empty buffer should have min width
        assert_eq!(widget.line_number_width(), 3);
    }

    #[test]
    fn test_line_diagnostic() {
        use crate::ics::editor::Actor;
        let buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();
        let diagnostics = vec![
            Diagnostic {
                position: Position {
                    line: 5,
                    column: 10,
                },
                length: 5,
                severity: Severity::Error,
                message: "Error on line 5".to_string(),
                suggestion: None,
            },
            Diagnostic {
                position: Position {
                    line: 5,
                    column: 20,
                },
                length: 3,
                severity: Severity::Warning,
                message: "Warning on line 5".to_string(),
                suggestion: None,
            },
            Diagnostic {
                position: Position {
                    line: 10,
                    column: 0,
                },
                length: 1,
                severity: Severity::Hint,
                message: "Hint on line 10".to_string(),
                suggestion: None,
            },
        ];

        let widget = EditorWidget::new(&buffer).diagnostics(&diagnostics);

        // Line 5 should return the most severe diagnostic (Error)
        let diag = widget.line_diagnostic(5);
        assert!(diag.is_some());
        assert_eq!(diag.unwrap().severity, Severity::Error);

        // Line 10 should return the hint
        let diag = widget.line_diagnostic(10);
        assert!(diag.is_some());
        assert_eq!(diag.unwrap().severity, Severity::Hint);

        // Line 0 should have no diagnostic
        assert!(widget.line_diagnostic(0).is_none());
    }

    #[test]
    fn test_diagnostic_at() {
        use crate::ics::editor::Actor;
        let buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();
        let diagnostics = vec![Diagnostic {
            position: Position {
                line: 5,
                column: 10,
            },
            length: 5, // covers columns 10-14
            severity: Severity::Error,
            message: "Error".to_string(),
            suggestion: None,
        }];

        let widget = EditorWidget::new(&buffer).diagnostics(&diagnostics);

        // Column 10 is the start of diagnostic
        assert!(widget.diagnostic_at(5, 10).is_some());

        // Column 12 is within diagnostic
        assert!(widget.diagnostic_at(5, 12).is_some());

        // Column 14 is within diagnostic
        assert!(widget.diagnostic_at(5, 14).is_some());

        // Column 15 is past the diagnostic
        assert!(widget.diagnostic_at(5, 15).is_none());

        // Column 9 is before the diagnostic
        assert!(widget.diagnostic_at(5, 9).is_none());

        // Different line
        assert!(widget.diagnostic_at(4, 10).is_none());
    }

    #[test]
    fn test_severity_ordering() {
        use crate::ics::editor::Actor;
        let buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();
        let diagnostics = vec![
            Diagnostic {
                position: Position { line: 0, column: 0 },
                length: 1,
                severity: Severity::Hint,
                message: "Hint".to_string(),
                suggestion: None,
            },
            Diagnostic {
                position: Position { line: 0, column: 5 },
                length: 1,
                severity: Severity::Warning,
                message: "Warning".to_string(),
                suggestion: None,
            },
            Diagnostic {
                position: Position {
                    line: 0,
                    column: 10,
                },
                length: 1,
                severity: Severity::Error,
                message: "Error".to_string(),
                suggestion: None,
            },
        ];

        let widget = EditorWidget::new(&buffer).diagnostics(&diagnostics);

        // Should return Error as it's most severe
        let diag = widget.line_diagnostic(0);
        assert!(diag.is_some());
        assert_eq!(diag.unwrap().severity, Severity::Error);
    }
}
