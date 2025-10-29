//! Editor widget for rendering text buffers
//!
//! Provides elegant, minimal UI for text editing with:
//! - Line numbers with subtle styling
//! - Cursor rendering
//! - Smooth scrolling
//! - Change attribution (color-coded by actor)

use super::{Attribution, CrdtBuffer, CursorState, TextBuffer};
use ratatui::{
    buffer::Buffer as RatatuiBuffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
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

/// Editor widget for TextBuffer
pub struct EditorWidget<'a> {
    /// Text buffer to render
    buffer: &'a TextBuffer,

    /// Optional CRDT buffer for attribution
    crdt_buffer: Option<&'a CrdtBuffer>,

    /// Block styling
    block: Option<Block<'a>>,

    /// Whether editor has focus
    focused: bool,
}

impl<'a> EditorWidget<'a> {
    /// Create new editor widget
    pub fn new(buffer: &'a TextBuffer) -> Self {
        Self {
            buffer,
            crdt_buffer: None,
            block: None,
            focused: false,
        }
    }

    /// Set CRDT buffer for attribution display
    pub fn crdt_buffer(mut self, crdt: &'a CrdtBuffer) -> Self {
        self.crdt_buffer = Some(crdt);
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
        let line_count = self.buffer.line_count();
        if line_count == 0 {
            return 3; // Min width
        }
        (line_count.ilog10() as usize + 1).max(3)
    }

    /// Get attribution color for character position
    fn attribution_color(&self, char_pos: usize) -> Option<Color> {
        if let Some(crdt) = self.crdt_buffer {
            if let Some(attr) = crdt.attribution_at(char_pos) {
                let (r, g, b) = attr.actor.color();
                return Some(Color::Rgb(r, g, b));
            }
        }
        None
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
            .constraints([
                Constraint::Length(line_num_width),
                Constraint::Min(10),
            ])
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
        let line_count = self.buffer.line_count();
        let viewport_height = area.height as usize;

        for i in 0..viewport_height {
            let line_num = state.scroll_offset + i;
            if line_num >= line_count {
                break;
            }

            let is_cursor_line = line_num == self.buffer.cursor.position.line;

            let style = if is_cursor_line && self.focused {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let line_num_text = format!("{:>width$}", line_num + 1, width = area.width.saturating_sub(1) as usize);

            buf.set_string(
                area.x,
                area.y + i as u16,
                &line_num_text,
                style,
            );
        }
    }

    /// Render editor content
    fn render_content(&self, area: Rect, buf: &mut RatatuiBuffer, state: &EditorState) {
        let line_count = self.buffer.line_count();
        let viewport_height = area.height as usize;
        let viewport_width = area.width as usize;

        let mut char_pos = 0;

        for i in 0..viewport_height {
            let line_num = state.scroll_offset + i;
            if line_num >= line_count {
                break;
            }

            if let Some(line_text) = self.buffer.line(line_num) {
                let is_cursor_line = line_num == self.buffer.cursor.position.line;

                // Apply horizontal scroll
                let visible_text: String = line_text
                    .chars()
                    .skip(state.h_scroll_offset)
                    .take(viewport_width)
                    .collect();

                // Render line with attribution colors if enabled
                if state.show_attribution && self.crdt_buffer.is_some() {
                    self.render_line_with_attribution(
                        area.x,
                        area.y + i as u16,
                        &visible_text,
                        char_pos,
                        buf,
                    );
                } else {
                    let style = if is_cursor_line && self.focused {
                        Style::default()
                    } else {
                        Style::default()
                    };

                    buf.set_string(
                        area.x,
                        area.y + i as u16,
                        &visible_text,
                        style,
                    );
                }

                // Render cursor if on this line
                if is_cursor_line && self.focused {
                    let cursor_col = self.buffer.cursor.position.column;
                    if cursor_col >= state.h_scroll_offset
                        && cursor_col < state.h_scroll_offset + viewport_width {
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

    /// Render line with attribution colors
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

            buf.set_string(x + i as u16, y, &ch.to_string(), style);
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
        cursor.position = Position { line: 50, column: 0 };
        state.ensure_cursor_visible(&cursor, 20);

        // Should scroll to show cursor
        assert!(state.scroll_offset > 0);
        assert!(state.scroll_offset <= 50);
        assert!(cursor.position.line >= state.scroll_offset);
        assert!(cursor.position.line < state.scroll_offset + 20);
    }

    #[test]
    fn test_line_number_width() {
        let buffer = TextBuffer::new(0, None);
        let widget = EditorWidget::new(&buffer);

        // Empty buffer should have min width
        assert_eq!(widget.line_number_width(), 3);
    }
}
