//! Dialog widgets for user interactions

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Dialog trait for modal interactions
pub trait Dialog {
    /// Render the dialog
    fn render(&self, frame: &mut Frame, area: Rect);

    /// Handle keyboard input
    /// Returns true if dialog should close
    fn handle_key(&mut self, key: KeyEvent) -> bool;

    /// Check if dialog is visible
    fn is_visible(&self) -> bool;

    /// Get dialog result (if any)
    fn result(&self) -> DialogResult;
}

/// Dialog result types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogResult {
    /// User confirmed
    Confirmed,
    /// User confirmed with input
    ConfirmedWithInput(String),
    /// User cancelled
    Cancelled,
    /// No result yet
    Pending,
}

/// Confirm dialog for Yes/No decisions
pub struct ConfirmDialog {
    title: String,
    message: String,
    preview: Option<String>,
    visible: bool,
    result: DialogResult,
    selected: usize, // 0 = Yes, 1 = No
}

impl ConfirmDialog {
    /// Create new confirm dialog
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            preview: None,
            visible: true,
            result: DialogResult::Pending,
            selected: 0,
        }
    }

    /// Set preview text
    pub fn with_preview(mut self, preview: impl Into<String>) -> Self {
        self.preview = Some(preview.into());
        self
    }

    /// Show the dialog
    pub fn show(&mut self) {
        self.visible = true;
        self.result = DialogResult::Pending;
        self.selected = 0;
    }

    /// Hide the dialog
    pub fn hide(&mut self) {
        self.visible = false;
    }
}

impl Dialog for ConfirmDialog {
    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Calculate dialog size
        let width = area.width.min(80);
        let height = if self.preview.is_some() {
            area.height.min(30)
        } else {
            area.height.min(12)
        };

        // Center dialog
        let dialog_area = Rect {
            x: (area.width.saturating_sub(width)) / 2,
            y: (area.height.saturating_sub(height)) / 2,
            width,
            height,
        };

        // Clear background
        frame.render_widget(Clear, dialog_area);

        // Create block
        let block = Block::default()
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        // Layout: message + preview (if any) + buttons
        let chunks = if self.preview.is_some() {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Message
                    Constraint::Min(5),    // Preview
                    Constraint::Length(3), // Buttons
                ])
                .split(inner)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(3),    // Message
                    Constraint::Length(3), // Buttons
                ])
                .split(inner)
        };

        // Render message
        let message = Paragraph::new(self.message.as_str())
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center);
        frame.render_widget(message, chunks[0]);

        // Render preview if present
        if let Some(preview_text) = &self.preview {
            let preview_idx = 1;
            let preview = Paragraph::new(preview_text.as_str())
                .block(
                    Block::default()
                        .title("Preview")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray)),
                )
                .wrap(Wrap { trim: false })
                .scroll((0, 0));
            frame.render_widget(preview, chunks[preview_idx]);
        }

        // Render buttons
        let button_idx = if self.preview.is_some() { 2 } else { 1 };
        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[button_idx]);

        // Yes button
        let yes_style = if self.selected == 0 {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        let yes_button = Paragraph::new("[ Yes ]")
            .style(yes_style)
            .alignment(Alignment::Center);
        frame.render_widget(yes_button, button_chunks[0]);

        // No button
        let no_style = if self.selected == 1 {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Red)
        };
        let no_button = Paragraph::new("[ No ]")
            .style(no_style)
            .alignment(Alignment::Center);
        frame.render_widget(no_button, button_chunks[1]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                self.selected = 0;
                false
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.selected = 1;
                false
            }
            KeyCode::Tab => {
                self.selected = (self.selected + 1) % 2;
                false
            }
            KeyCode::Enter => {
                self.result = if self.selected == 0 {
                    DialogResult::Confirmed
                } else {
                    DialogResult::Cancelled
                };
                self.visible = false;
                true
            }
            KeyCode::Esc => {
                self.result = DialogResult::Cancelled;
                self.visible = false;
                true
            }
            _ => false,
        }
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn result(&self) -> DialogResult {
        self.result.clone()
    }
}

/// Input dialog for text entry
pub struct InputDialog {
    title: String,
    prompt: String,
    input: String,
    cursor_pos: usize,
    visible: bool,
    result: DialogResult,
    validator: Option<Box<dyn Fn(&str) -> Result<(), String>>>,
    error: Option<String>,
}

impl InputDialog {
    /// Create new input dialog
    pub fn new(title: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            prompt: prompt.into(),
            input: String::new(),
            cursor_pos: 0,
            visible: true,
            result: DialogResult::Pending,
            validator: None,
            error: None,
        }
    }

    /// Set default input value
    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.input = default.into();
        self.cursor_pos = self.input.len();
        self
    }

    /// Set validator function
    pub fn with_validator<F>(mut self, validator: F) -> Self
    where
        F: Fn(&str) -> Result<(), String> + 'static,
    {
        self.validator = Some(Box::new(validator));
        self
    }

    /// Show the dialog
    pub fn show(&mut self) {
        self.visible = true;
        self.result = DialogResult::Pending;
        self.error = None;
    }

    /// Hide the dialog
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Validate current input
    fn validate(&mut self) -> bool {
        if let Some(validator) = &self.validator {
            match validator(&self.input) {
                Ok(()) => {
                    self.error = None;
                    true
                }
                Err(err) => {
                    self.error = Some(err);
                    false
                }
            }
        } else {
            true
        }
    }
}

impl Dialog for InputDialog {
    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Calculate dialog size
        let width = area.width.min(60);
        let height = area.height.min(10);

        // Center dialog
        let dialog_area = Rect {
            x: (area.width.saturating_sub(width)) / 2,
            y: (area.height.saturating_sub(height)) / 2,
            width,
            height,
        };

        // Clear background
        frame.render_widget(Clear, dialog_area);

        // Create block
        let block = Block::default()
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        // Layout: prompt + input + error (if any) + hint
        let constraints = if self.error.is_some() {
            vec![
                Constraint::Length(2), // Prompt
                Constraint::Length(3), // Input
                Constraint::Length(2), // Error
                Constraint::Length(1), // Hint
            ]
        } else {
            vec![
                Constraint::Length(2), // Prompt
                Constraint::Length(3), // Input
                Constraint::Length(1), // Hint
            ]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        // Render prompt
        let prompt = Paragraph::new(self.prompt.as_str()).alignment(Alignment::Left);
        frame.render_widget(prompt, chunks[0]);

        // Render input box
        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green));
        let input_inner = input_block.inner(chunks[1]);
        frame.render_widget(input_block, chunks[1]);

        let input_text =
            Paragraph::new(self.input.as_str()).style(Style::default().fg(Color::White));
        frame.render_widget(input_text, input_inner);

        // Render cursor
        if self.cursor_pos <= self.input.len() {
            let cursor_x = input_inner.x + self.cursor_pos as u16;
            if cursor_x < input_inner.x + input_inner.width {
                frame.set_cursor_position((cursor_x, input_inner.y));
            }
        }

        // Render error if present
        let mut hint_idx = 2;
        if let Some(error) = &self.error {
            let error_text = Paragraph::new(error.as_str()).style(Style::default().fg(Color::Red));
            frame.render_widget(error_text, chunks[2]);
            hint_idx = 3;
        }

        // Render hint
        let hint = Paragraph::new("Enter: Confirm | Esc: Cancel")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(hint, chunks[hint_idx]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(c) => {
                self.input.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
                self.error = None; // Clear error on input
                false
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.input.remove(self.cursor_pos);
                    self.error = None;
                }
                false
            }
            KeyCode::Delete => {
                if self.cursor_pos < self.input.len() {
                    self.input.remove(self.cursor_pos);
                    self.error = None;
                }
                false
            }
            KeyCode::Left => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
                false
            }
            KeyCode::Right => {
                if self.cursor_pos < self.input.len() {
                    self.cursor_pos += 1;
                }
                false
            }
            KeyCode::Home => {
                self.cursor_pos = 0;
                false
            }
            KeyCode::End => {
                self.cursor_pos = self.input.len();
                false
            }
            KeyCode::Enter => {
                if self.validate() {
                    self.result = DialogResult::ConfirmedWithInput(self.input.clone());
                    self.visible = false;
                    true
                } else {
                    false
                }
            }
            KeyCode::Esc => {
                self.result = DialogResult::Cancelled;
                self.visible = false;
                true
            }
            _ => false,
        }
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn result(&self) -> DialogResult {
        self.result.clone()
    }
}

/// Preview dialog for large text display
pub struct PreviewDialog {
    title: String,
    content: String,
    visible: bool,
    scroll_offset: u16,
    result: DialogResult,
}

impl PreviewDialog {
    /// Create new preview dialog
    pub fn new(title: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            content: content.into(),
            visible: true,
            scroll_offset: 0,
            result: DialogResult::Pending,
        }
    }

    /// Show the dialog
    pub fn show(&mut self) {
        self.visible = true;
        self.scroll_offset = 0;
        self.result = DialogResult::Pending;
    }

    /// Hide the dialog
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Get line count
    fn line_count(&self) -> usize {
        self.content.lines().count()
    }
}

impl Dialog for PreviewDialog {
    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Calculate dialog size (larger for preview)
        let width = area.width.min(100);
        let height = area.height.min(40);

        // Center dialog
        let dialog_area = Rect {
            x: (area.width.saturating_sub(width)) / 2,
            y: (area.height.saturating_sub(height)) / 2,
            width,
            height,
        };

        // Clear background
        frame.render_widget(Clear, dialog_area);

        // Create block
        let block = Block::default()
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        // Layout: content + hint
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),    // Content
                Constraint::Length(1), // Hint
            ])
            .split(inner);

        // Render content with scroll
        let content = Paragraph::new(self.content.as_str())
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset, 0));
        frame.render_widget(content, chunks[0]);

        // Render hint
        let hint = Paragraph::new("↑↓: Scroll | Esc: Close")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(hint, chunks[1]);
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
                false
            }
            KeyCode::Down | KeyCode::Char('j') => {
                // Allow scrolling if there's content below
                let max_scroll = self.line_count().saturating_sub(1) as u16;
                self.scroll_offset = (self.scroll_offset + 1).min(max_scroll);
                false
            }
            KeyCode::PageUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(10);
                false
            }
            KeyCode::PageDown => {
                let max_scroll = self.line_count().saturating_sub(1) as u16;
                self.scroll_offset = (self.scroll_offset + 10).min(max_scroll);
                false
            }
            KeyCode::Home => {
                self.scroll_offset = 0;
                false
            }
            KeyCode::End => {
                let max_scroll = self.line_count().saturating_sub(1) as u16;
                self.scroll_offset = max_scroll;
                false
            }
            KeyCode::Esc | KeyCode::Enter => {
                self.result = DialogResult::Cancelled;
                self.visible = false;
                true
            }
            _ => false,
        }
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn result(&self) -> DialogResult {
        self.result.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confirm_dialog_creation() {
        let dialog = ConfirmDialog::new("Test", "Are you sure?");
        assert_eq!(dialog.is_visible(), true);
        assert_eq!(dialog.result(), DialogResult::Pending);
    }

    #[test]
    fn test_confirm_dialog_navigation() {
        let mut dialog = ConfirmDialog::new("Test", "Confirm?");
        assert_eq!(dialog.selected, 0);

        // Navigate right
        dialog.handle_key(KeyEvent::from(KeyCode::Right));
        assert_eq!(dialog.selected, 1);

        // Navigate left
        dialog.handle_key(KeyEvent::from(KeyCode::Left));
        assert_eq!(dialog.selected, 0);

        // Tab navigation
        dialog.handle_key(KeyEvent::from(KeyCode::Tab));
        assert_eq!(dialog.selected, 1);
    }

    #[test]
    fn test_confirm_dialog_confirm() {
        let mut dialog = ConfirmDialog::new("Test", "Confirm?");
        let should_close = dialog.handle_key(KeyEvent::from(KeyCode::Enter));
        assert!(should_close);
        assert_eq!(dialog.result(), DialogResult::Confirmed);
        assert_eq!(dialog.is_visible(), false);
    }

    #[test]
    fn test_confirm_dialog_cancel() {
        let mut dialog = ConfirmDialog::new("Test", "Confirm?");
        let should_close = dialog.handle_key(KeyEvent::from(KeyCode::Esc));
        assert!(should_close);
        assert_eq!(dialog.result(), DialogResult::Cancelled);
        assert_eq!(dialog.is_visible(), false);
    }

    #[test]
    fn test_input_dialog_creation() {
        let dialog = InputDialog::new("Test", "Enter filename:");
        assert_eq!(dialog.is_visible(), true);
        assert_eq!(dialog.result(), DialogResult::Pending);
        assert_eq!(dialog.input, "");
    }

    #[test]
    fn test_input_dialog_typing() {
        let mut dialog = InputDialog::new("Test", "Enter:");
        dialog.handle_key(KeyEvent::from(KeyCode::Char('t')));
        dialog.handle_key(KeyEvent::from(KeyCode::Char('e')));
        dialog.handle_key(KeyEvent::from(KeyCode::Char('s')));
        dialog.handle_key(KeyEvent::from(KeyCode::Char('t')));
        assert_eq!(dialog.input, "test");
        assert_eq!(dialog.cursor_pos, 4);
    }

    #[test]
    fn test_input_dialog_backspace() {
        let mut dialog = InputDialog::new("Test", "Enter:").with_default("test");
        assert_eq!(dialog.cursor_pos, 4);

        dialog.handle_key(KeyEvent::from(KeyCode::Backspace));
        assert_eq!(dialog.input, "tes");
        assert_eq!(dialog.cursor_pos, 3);
    }

    #[test]
    fn test_input_dialog_with_validator() {
        let mut dialog = InputDialog::new("Test", "Enter:").with_validator(|s| {
            if s.is_empty() {
                Err("Cannot be empty".to_string())
            } else {
                Ok(())
            }
        });

        // Try to confirm empty input
        let should_close = dialog.handle_key(KeyEvent::from(KeyCode::Enter));
        assert!(!should_close);
        assert!(dialog.error.is_some());

        // Add text and confirm
        dialog.handle_key(KeyEvent::from(KeyCode::Char('a')));
        let should_close = dialog.handle_key(KeyEvent::from(KeyCode::Enter));
        assert!(should_close);
        assert_eq!(
            dialog.result(),
            DialogResult::ConfirmedWithInput("a".to_string())
        );
    }

    #[test]
    fn test_preview_dialog_creation() {
        let dialog = PreviewDialog::new("Preview", "Line 1\nLine 2\nLine 3");
        assert_eq!(dialog.is_visible(), true);
        assert_eq!(dialog.scroll_offset, 0);
    }

    #[test]
    fn test_preview_dialog_scroll() {
        let mut dialog = PreviewDialog::new("Preview", "Line 1\nLine 2\nLine 3");

        dialog.handle_key(KeyEvent::from(KeyCode::Down));
        assert_eq!(dialog.scroll_offset, 1);

        dialog.handle_key(KeyEvent::from(KeyCode::Up));
        assert_eq!(dialog.scroll_offset, 0);
    }
}
