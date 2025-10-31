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

impl Default for ChatView {
    fn default() -> Self {
        Self::new()
    }
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

impl Default for Dashboard {
    fn default() -> Self {
        Self::new()
    }
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

        // Gather real system metrics
        let (memory_mb, cpu_percent) = Self::gather_system_metrics();
        self.memory_mb = memory_mb;
        self.cpu_percent = cpu_percent;
    }

    /// Gather system metrics using ps command
    ///
    /// Returns (memory_mb, cpu_percent)
    fn gather_system_metrics() -> (f32, f32) {
        use std::process::Command;

        // Get current process ID
        let pid = std::process::id();

        // Use ps to get memory (RSS in KB) and CPU percentage
        // Format: ps -p <pid> -o rss=,pcpu=
        // Output: "12345 1.5" (RSS in KB, CPU percentage)
        let output = Command::new("ps")
            .args([
                "-p",
                &pid.to_string(),
                "-o",
                "rss=,pcpu=", // RSS (resident set size in KB), CPU%
            ])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let output_str = String::from_utf8_lossy(&output.stdout);
                let parts: Vec<&str> = output_str.split_whitespace().collect();

                if parts.len() >= 2 {
                    // Parse RSS (in KB) and convert to MB
                    let memory_mb = parts[0].parse::<f32>().unwrap_or(0.0) / 1024.0;

                    // Parse CPU percentage
                    let cpu_percent = parts[1].parse::<f32>().unwrap_or(0.0);

                    return (memory_mb, cpu_percent);
                }
            }
            _ => {
                // If ps command fails, return zeros
                // This can happen on systems without ps or with different ps implementations
            }
        }

        // Fallback: return zeros if metrics gathering fails
        (0.0, 0.0)
    }

    /// Render dashboard
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let text = vec![
            Line::from(vec![
                Span::styled(
                    "Active Agents: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
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
    /// Lines of content
    lines: Vec<String>,
    /// Cursor position (line, column)
    cursor: (usize, usize),
    /// Scroll offset (vertical)
    scroll_offset: usize,
    /// Whether panel is visible
    visible: bool,
    /// Whether panel is focused
    focused: bool,
    /// Markdown highlighter
    highlighter: crate::ics::markdown_highlight::MarkdownHighlighter,
}

impl Default for IcsPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl IcsPanel {
    /// Create new ICS panel
    pub fn new() -> Self {
        let initial_lines = vec![
            "# ICS - Integrated Context Studio".to_string(),
            String::new(),
            "Press Ctrl+E to toggle | Ctrl+Enter to submit".to_string(),
            String::new(),
            "## Pattern Syntax".to_string(),
            "- #file.rs - File reference (blue)".to_string(),
            "- @symbol - Symbol reference (green)".to_string(),
            "- ?interface - Typed hole (yellow)".to_string(),
        ];

        Self {
            lines: initial_lines,
            cursor: (0, 0),
            scroll_offset: 0,
            visible: false,
            focused: false,
            highlighter: crate::ics::markdown_highlight::MarkdownHighlighter::new()
                .expect("Failed to initialize markdown highlighter"),
        }
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Set focused state
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Check if focused
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Get content as string
    pub fn get_content(&self) -> String {
        self.lines.join("\n")
    }

    /// Set content from string
    pub fn set_content(&mut self, content: String) {
        self.lines = content.lines().map(|s| s.to_string()).collect();
        // Reset cursor if out of bounds
        if self.cursor.0 >= self.lines.len() {
            self.cursor.0 = self.lines.len().saturating_sub(1);
        }
        if let Some(line) = self.lines.get(self.cursor.0) {
            if self.cursor.1 > line.len() {
                self.cursor.1 = line.len();
            }
        }
    }

    /// Toggle semantic highlighting
    pub fn toggle_highlighting(&mut self) {
        let currently_enabled = self.highlighter.is_semantic_enabled();
        self.highlighter.set_semantic_enabled(!currently_enabled);
    }

    /// Check if highlighting is enabled
    pub fn is_highlighting_enabled(&self) -> bool {
        self.highlighter.is_semantic_enabled()
    }

    /// Insert character at cursor
    pub fn insert_char(&mut self, c: char) {
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }

        let (line_idx, col_idx) = self.cursor;
        if line_idx < self.lines.len() {
            self.lines[line_idx].insert(col_idx, c);
            self.cursor.1 += 1;
        }
    }

    /// Insert newline at cursor
    pub fn insert_newline(&mut self) {
        if self.lines.is_empty() {
            self.lines.push(String::new());
            self.lines.push(String::new());
            self.cursor = (1, 0);
            return;
        }

        let (line_idx, col_idx) = self.cursor;
        if line_idx < self.lines.len() {
            let current_line = self.lines[line_idx].clone();
            let (before, after) = current_line.split_at(col_idx);
            self.lines[line_idx] = before.to_string();
            self.lines.insert(line_idx + 1, after.to_string());
            self.cursor = (line_idx + 1, 0);
        }
    }

    /// Delete character before cursor (backspace)
    pub fn backspace(&mut self) {
        let (line_idx, col_idx) = self.cursor;

        if col_idx > 0 {
            // Delete char in current line
            if line_idx < self.lines.len() {
                self.lines[line_idx].remove(col_idx - 1);
                self.cursor.1 -= 1;
            }
        } else if line_idx > 0 {
            // Merge with previous line
            let current_line = self.lines.remove(line_idx);
            let prev_len = self.lines[line_idx - 1].len();
            self.lines[line_idx - 1].push_str(&current_line);
            self.cursor = (line_idx - 1, prev_len);
        }
    }

    /// Delete character at cursor (delete key)
    pub fn delete(&mut self) {
        let (line_idx, col_idx) = self.cursor;

        if line_idx < self.lines.len() {
            if col_idx < self.lines[line_idx].len() {
                // Delete char at cursor
                self.lines[line_idx].remove(col_idx);
            } else if line_idx + 1 < self.lines.len() {
                // Merge with next line
                let next_line = self.lines.remove(line_idx + 1);
                self.lines[line_idx].push_str(&next_line);
            }
        }
    }

    /// Move cursor left
    pub fn move_cursor_left(&mut self) {
        if self.cursor.1 > 0 {
            self.cursor.1 -= 1;
        } else if self.cursor.0 > 0 {
            self.cursor.0 -= 1;
            if let Some(line) = self.lines.get(self.cursor.0) {
                self.cursor.1 = line.len();
            }
        }
    }

    /// Move cursor right
    pub fn move_cursor_right(&mut self) {
        if let Some(line) = self.lines.get(self.cursor.0) {
            if self.cursor.1 < line.len() {
                self.cursor.1 += 1;
            } else if self.cursor.0 + 1 < self.lines.len() {
                self.cursor.0 += 1;
                self.cursor.1 = 0;
            }
        }
    }

    /// Move cursor up
    pub fn move_cursor_up(&mut self) {
        if self.cursor.0 > 0 {
            self.cursor.0 -= 1;
            // Clamp column to line length
            if let Some(line) = self.lines.get(self.cursor.0) {
                self.cursor.1 = self.cursor.1.min(line.len());
            }
        }
    }

    /// Move cursor down
    pub fn move_cursor_down(&mut self) {
        if self.cursor.0 + 1 < self.lines.len() {
            self.cursor.0 += 1;
            // Clamp column to line length
            if let Some(line) = self.lines.get(self.cursor.0) {
                self.cursor.1 = self.cursor.1.min(line.len());
            }
        }
    }

    /// Render ICS panel
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Border style changes based on focus
        let border_style = if self.focused {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Cyan)
        };

        let block = Block::default()
            .title(if self.focused { "ICS - Context Studio [FOCUSED]" } else { "ICS - Context Studio" })
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Calculate visible range
        let visible_height = inner.height as usize;
        let visible_lines = self.lines
            .iter()
            .skip(self.scroll_offset)
            .take(visible_height)
            .enumerate();

        // Render lines with highlighting
        let mut y_offset = 0;
        for (_idx, line) in visible_lines {
            if y_offset >= inner.height {
                break;
            }

            let highlighted_line = self.highlighter.highlight_line(line);
            let line_area = Rect {
                x: inner.x,
                y: inner.y + y_offset,
                width: inner.width,
                height: 1,
            };
            frame.render_widget(highlighted_line, line_area);
            y_offset += 1;
        }

        // Render cursor if focused
        if self.focused && self.cursor.0 >= self.scroll_offset {
            let cursor_y = self.cursor.0 - self.scroll_offset;
            if cursor_y < visible_height {
                let cursor_x = inner.x + self.cursor.1.min(inner.width as usize - 1) as u16;
                let cursor_y = inner.y + cursor_y as u16;
                frame.set_cursor_position((cursor_x, cursor_y));
            }
        }
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
