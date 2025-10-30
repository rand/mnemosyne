//! Main TUI application integrating all components

use super::{
    ChatView, CommandPalette, Dashboard, EventLoop, IcsPanel, LayoutManager, TerminalConfig,
    TerminalManager, TuiEvent,
};
use crate::pty::ClaudeCodeWrapper;
use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Direction, Layout};

/// Application state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    /// Running normally
    Running,
    /// Quit requested
    Quitting,
}

/// Main TUI application
pub struct TuiApp {
    /// Terminal manager
    terminal: TerminalManager,
    /// Event loop
    event_loop: EventLoop,
    /// Chat view
    chat_view: ChatView,
    /// Dashboard
    dashboard: Dashboard,
    /// ICS panel
    ics_panel: IcsPanel,
    /// Command palette
    command_palette: CommandPalette,
    /// Layout manager
    layout: LayoutManager,
    /// Claude Code wrapper
    wrapper: Option<ClaudeCodeWrapper>,
    /// Application state
    state: AppState,
}

impl TuiApp {
    /// Create new TUI application
    pub fn new() -> Result<Self> {
        let terminal = TerminalManager::new(TerminalConfig::default())?;
        let event_loop = EventLoop::default();
        let chat_view = ChatView::new();
        let dashboard = Dashboard::new();
        let ics_panel = IcsPanel::new();
        let mut command_palette = CommandPalette::new();

        // Add some default commands
        command_palette.add_command(super::Command {
            id: "quit".to_string(),
            name: "Quit".to_string(),
            description: "Exit application".to_string(),
            category: super::CommandCategory::System,
            shortcut: Some("Ctrl+Q".to_string()),
        });

        command_palette.add_command(super::Command {
            id: "toggle_ics".to_string(),
            name: "Toggle ICS".to_string(),
            description: "Show/hide ICS panel".to_string(),
            category: super::CommandCategory::View,
            shortcut: Some("Ctrl+E".to_string()),
        });

        command_palette.add_command(super::Command {
            id: "clear_chat".to_string(),
            name: "Clear Chat".to_string(),
            description: "Clear chat history".to_string(),
            category: super::CommandCategory::View,
            shortcut: None,
        });

        let layout = LayoutManager::new(ratatui::layout::Rect::default());

        Ok(Self {
            terminal,
            event_loop,
            chat_view,
            dashboard,
            ics_panel,
            command_palette,
            layout,
            wrapper: None,
            state: AppState::Running,
        })
    }

    /// Start with Claude Code wrapper
    pub fn with_wrapper(mut self, wrapper: ClaudeCodeWrapper) -> Self {
        self.wrapper = Some(wrapper);
        self
    }

    /// Run the application
    pub async fn run(mut self) -> Result<()> {
        // Set up output receiver if wrapper exists
        let mut output_rx = self.wrapper.as_mut().and_then(|w| w.take_output_receiver());

        loop {
            // Render UI
            self.render()?;

            // Poll for events
            if let Some(event) = self.event_loop.poll_event()? {
                self.handle_event(event).await?;
            }

            // Poll for PTY output
            if let Some(wrapper) = &mut self.wrapper {
                wrapper.poll_output().await?;
            }

            // Process output from PTY
            if let Some(rx) = &mut output_rx {
                while let Ok(chunk) = rx.try_recv() {
                    self.chat_view.add_message(chunk);
                }
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

    /// Handle TUI event
    async fn handle_event(&mut self, event: TuiEvent) -> Result<()> {
        match event {
            TuiEvent::Quit => {
                self.state = AppState::Quitting;
            }
            TuiEvent::Key(key) => {
                // Handle command palette toggle
                if key.code == KeyCode::Char('p')
                    && key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL)
                {
                    self.command_palette.toggle();
                    return Ok(());
                }

                // Handle ICS toggle
                if key.code == KeyCode::Char('e')
                    && key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL)
                {
                    self.ics_panel.toggle();
                    return Ok(());
                }

                // Route to command palette if visible
                if self.command_palette.is_visible() {
                    match key.code {
                        KeyCode::Esc => {
                            self.command_palette.toggle();
                        }
                        KeyCode::Enter => {
                            if let Some(cmd) = self.command_palette.execute_selected() {
                                self.handle_command(&cmd).await?;
                            }
                        }
                        KeyCode::Up => {
                            self.command_palette.select_previous();
                        }
                        KeyCode::Down => {
                            self.command_palette.select_next();
                        }
                        KeyCode::Char(c) => {
                            self.command_palette.append_query(c);
                        }
                        KeyCode::Backspace => {
                            self.command_palette.backspace_query();
                        }
                        _ => {}
                    }
                } else {
                    // Send to Claude Code wrapper
                    if let Some(wrapper) = &self.wrapper {
                        // Convert key event to bytes
                        if let KeyCode::Char(c) = key.code {
                            wrapper.send_input(c.to_string().as_bytes()).await?;
                        }
                    }
                }
            }
            TuiEvent::Resize(w, h) => {
                if let Some(wrapper) = &self.wrapper {
                    wrapper.resize(w, h).await?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Handle command execution
    async fn handle_command(&mut self, command_id: &str) -> Result<()> {
        match command_id {
            "quit" => {
                self.state = AppState::Quitting;
            }
            "toggle_ics" => {
                self.ics_panel.toggle();
            }
            "clear_chat" => {
                self.chat_view.clear();
            }
            _ => {}
        }
        Ok(())
    }

    /// Render UI
    fn render(&mut self) -> Result<()> {
        self.terminal.terminal_mut().draw(|frame| {
            let size = frame.area();

            // Update layout area
            self.layout.set_area(size);

            // Create main layout
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(10),   // Chat + ICS
                    Constraint::Length(6), // Dashboard
                ])
                .split(size);

            // Split top section if ICS is visible
            let top_chunks = if self.ics_panel.is_visible() {
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(60), // Chat
                        Constraint::Percentage(40), // ICS
                    ])
                    .split(chunks[0])
                    .to_vec()
            } else {
                vec![chunks[0]]
            };

            // Render chat view
            self.chat_view.render(frame, top_chunks[0]);

            // Render ICS panel if visible
            if self.ics_panel.is_visible() && top_chunks.len() > 1 {
                self.ics_panel.render(frame, top_chunks[1]);
            }

            // Render dashboard
            self.dashboard.render(frame, chunks[1]);

            // Render command palette if visible
            if self.command_palette.is_visible() {
                // Center command palette
                let palette_area = ratatui::layout::Rect {
                    x: size.width / 4,
                    y: size.height / 4,
                    width: size.width / 2,
                    height: size.height / 2,
                };
                frame.render_widget(&self.command_palette, palette_area);
            }
        })?;

        Ok(())
    }
}
