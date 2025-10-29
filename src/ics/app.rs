//! Main ICS application
//!
//! Standalone ICS application that can be run with `mnemosyne --ics`

use super::{IcsConfig, editor::IcsEditor};
use crate::tui::{EventLoop, TerminalConfig, TerminalManager, TuiEvent};
use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
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
            state: AppState::Running,
            status: "ICS - Integrated Context Studio | Press Ctrl+Q to quit".to_string(),
        }
    }

    /// Load file into editor
    pub fn load_file(&mut self, path: PathBuf) -> Result<()> {
        // TODO: Implement file loading
        self.status = format!("Loaded: {}", path.display());
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
            TuiEvent::Key(key) => match key.code {
                KeyCode::Char('q')
                    if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
                {
                    self.state = AppState::Quitting;
                }
                KeyCode::Char('s')
                    if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
                {
                    self.status = "Saved (stub)".to_string();
                }
                KeyCode::Char(c) => {
                    self.status = format!("Key pressed: {}", c);
                }
                _ => {}
            },
            TuiEvent::Resize(_, _) => {
                // Terminal resized
            }
            _ => {}
        }
        Ok(())
    }

    /// Render UI
    fn render(&mut self, terminal: &mut TerminalManager) -> Result<()> {
        terminal.terminal_mut().draw(|frame| {
            let size = frame.area();

            // Create layout
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),     // Header
                    Constraint::Min(10),       // Editor
                    Constraint::Length(3),     // Status
                ])
                .split(size);

            // Render header
            let header = Paragraph::new("ICS - Integrated Context Studio")
                .style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(header, chunks[0]);

            // Render editor area
            let editor_text = vec![
                Line::from("Context engineering workspace"),
                Line::from(""),
                Line::from(Span::styled(
                    "This is a placeholder for the editor.",
                    Style::default().fg(Color::Gray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Features:",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from("  - Semantic analysis"),
                Line::from("  - Typed hole tracking"),
                Line::from("  - Symbol resolution"),
                Line::from("  - AI-powered suggestions"),
                Line::from(""),
                Line::from(Span::styled(
                    "Keyboard shortcuts:",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from("  Ctrl+S: Save (stub)"),
                Line::from("  Ctrl+Q: Quit"),
            ];

            let editor_widget = Paragraph::new(editor_text)
                .block(Block::default().borders(Borders::ALL).title("Editor"));
            frame.render_widget(editor_widget, chunks[1]);

            // Render status bar
            let status_widget = Paragraph::new(self.status.as_str())
                .style(Style::default().fg(Color::White).bg(Color::DarkGray))
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(status_widget, chunks[2]);
        })?;

        Ok(())
    }
}

impl Default for IcsApp {
    fn default() -> Self {
        Self::new(IcsConfig::default())
    }
}
