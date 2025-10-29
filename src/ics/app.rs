//! Main ICS application
//!
//! Standalone ICS application that can be run with `mnemosyne --ics`

use super::{IcsConfig, editor::{EditorState, EditorWidget, IcsEditor, Movement}};
use crate::tui::{EventLoop, TerminalConfig, TerminalManager, TuiEvent};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
};
use std::path::PathBuf;

/// Application state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppState {
    Running,
    Quitting,
}

/// Main ICS application
pub struct IcsApp {
    /// Configuration
    config: IcsConfig,
    /// Editor instance
    editor: IcsEditor,
    /// Editor widget state
    editor_state: EditorState,
    /// Application state
    state: AppState,
    /// Status message
    status: String,
}

impl IcsApp {
    /// Create new ICS application
    pub fn new(config: IcsConfig) -> Self {
        Self {
            config,
            editor: IcsEditor::new(),
            editor_state: EditorState::default(),
            state: AppState::Running,
            status: "ICS - Integrated Context Studio | Ctrl+Q: quit | Ctrl+S: save | Ctrl+O: open".to_string(),
        }
    }

    /// Load file into editor
    pub fn load_file(&mut self, path: PathBuf) -> Result<()> {
        let buffer = self.editor.active_buffer_mut();
        buffer.load_file(path.clone())?;
        self.status = format!("Loaded: {}", path.display());
        Ok(())
    }

    /// Save current buffer
    pub fn save_file(&mut self) -> Result<()> {
        let buffer = self.editor.active_buffer_mut();
        buffer.save_file()?;
        self.status = format!("Saved: {}", buffer.path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "untitled".to_string()));
        Ok(())
    }

    /// Run the ICS application
    pub async fn run(&mut self) -> Result<()> {
        // Initialize terminal
        let mut terminal = TerminalManager::new(TerminalConfig::default())?;
        let event_loop = EventLoop::default();

        // Main event loop
        loop {
            // Render UI
            self.render(&mut terminal)?;

            // Poll for events
            if let Some(event) = event_loop.poll_event()? {
                self.handle_event(event).await?;
            }

            // Check if we should quit
            if self.state == AppState::Quitting {
                break;
            }

            // Small delay to avoid busy looping
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        Ok(())
    }

    /// Handle event
    async fn handle_event(&mut self, event: TuiEvent) -> Result<()> {
        match event {
            TuiEvent::Quit => {
                self.state = AppState::Quitting;
            }
            TuiEvent::Key(key) => {
                let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
                let buffer = self.editor.active_buffer_mut();

                match (key.code, ctrl) {
                    // Quit
                    (KeyCode::Char('q'), true) | (KeyCode::Char('c'), true) => {
                        self.state = AppState::Quitting;
                    }

                    // Save
                    (KeyCode::Char('s'), true) => {
                        if let Err(e) = self.save_file() {
                            self.status = format!("Error saving: {}", e);
                        }
                    }

                    // Undo/Redo
                    (KeyCode::Char('z'), true) => {
                        buffer.undo();
                        self.status = "Undo".to_string();
                    }
                    (KeyCode::Char('y'), true) => {
                        buffer.redo();
                        self.status = "Redo".to_string();
                    }

                    // Text input
                    (KeyCode::Char(c), false) => {
                        buffer.insert(&c.to_string());
                    }

                    // Newline
                    (KeyCode::Enter, _) => {
                        buffer.insert("\n");
                    }

                    // Backspace
                    (KeyCode::Backspace, _) => {
                        let pos = buffer.cursor.position.column;
                        if pos > 0 {
                            buffer.move_cursor(Movement::Left);
                            buffer.delete();
                        }
                    }

                    // Delete
                    (KeyCode::Delete, _) => {
                        buffer.delete();
                    }

                    // Cursor movement
                    (KeyCode::Left, _) => {
                        buffer.move_cursor(Movement::Left);
                    }
                    (KeyCode::Right, _) => {
                        buffer.move_cursor(Movement::Right);
                    }
                    (KeyCode::Up, _) => {
                        buffer.move_cursor(Movement::Up);
                    }
                    (KeyCode::Down, _) => {
                        buffer.move_cursor(Movement::Down);
                    }
                    (KeyCode::Home, _) => {
                        buffer.move_cursor(Movement::LineStart);
                    }
                    (KeyCode::End, _) => {
                        buffer.move_cursor(Movement::LineEnd);
                    }

                    _ => {}
                }
            }
            TuiEvent::Resize(_, _) => {
                // Terminal resized
            }
            _ => {}
        }
        Ok(())
    }

    /// Render UI
    fn render(&mut self, terminal: &mut TerminalManager) -> Result<()> {
        let buffer = self.editor.active_buffer();
        let editor_state = &mut self.editor_state;
        let status = &self.status;

        terminal.terminal_mut().draw(|frame| {
            let size = frame.area();

            // Create layout
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),     // Status at top
                    Constraint::Min(10),       // Editor
                    Constraint::Length(1),     // Info bar at bottom
                ])
                .split(size);

            // Render status bar at top (minimal)
            let status_text = format!(" {}", status);
            let status_widget = Paragraph::new(status_text)
                .style(Style::default().fg(Color::White).bg(Color::DarkGray));
            frame.render_widget(status_widget, chunks[0]);

            // Render editor
            let editor_title = if let Some(path) = &buffer.path {
                let dirty_mark = if buffer.dirty { "*" } else { "" };
                format!(" {}{} ", path.display(), dirty_mark)
            } else {
                let dirty_mark = if buffer.dirty { "*" } else { "" };
                format!(" [untitled]{} ", dirty_mark)
            };

            let editor_block = Block::default()
                .borders(Borders::NONE)
                .title(editor_title)
                .style(Style::default());

            let editor_widget = EditorWidget::new(buffer)
                .block(editor_block)
                .focused(true);

            frame.render_stateful_widget(editor_widget, chunks[1], editor_state);

            // Render info bar at bottom (cursor position, language)
            let cursor_pos = format!(
                "Ln {}, Col {} ",
                buffer.cursor.position.line + 1,
                buffer.cursor.position.column + 1
            );
            let lang = format!("{:?} ", buffer.language);
            let info_text = format!("{} | {}", cursor_pos, lang);

            let info_widget = Paragraph::new(info_text)
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(info_widget, chunks[2]);
        })?;

        Ok(())
    }
}

impl Default for IcsApp {
    fn default() -> Self {
        Self::new(IcsConfig::default())
    }
}
