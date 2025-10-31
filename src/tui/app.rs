//! Main TUI application integrating all components

use super::{
    ChatView, CommandPalette, Dashboard, Dialog, EventLoop, HelpOverlay, IcsPanel, LayoutManager,
    TerminalConfig, TerminalManager, TuiEvent,
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

/// Pending action after dialog closes
#[derive(Debug, Clone)]
pub enum PendingDialogAction {
    /// No pending action
    None,
    /// Save file (filename comes from dialog result)
    SaveFile,
    /// Submit content to Claude Code
    SubmitToClaude,
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
    /// Help overlay
    help_overlay: HelpOverlay,
    /// Layout manager
    layout: LayoutManager,
    /// Active dialog (modal)
    active_dialog: Option<Box<dyn Dialog>>,
    /// Pending action after dialog closes
    pending_dialog_action: PendingDialogAction,
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

        // ICS Commands
        command_palette.add_command(super::Command {
            id: "ics:submit-to-claude".to_string(),
            name: "Submit to Claude".to_string(),
            description: "Send refined context as prompt to Claude Code".to_string(),
            category: super::CommandCategory::Ics,
            shortcut: Some("Ctrl+Enter".to_string()),
        });

        command_palette.add_command(super::Command {
            id: "ics:save-file".to_string(),
            name: "Save File".to_string(),
            description: "Save edited document to disk".to_string(),
            category: super::CommandCategory::Ics,
            shortcut: Some("Ctrl+S".to_string()),
        });

        command_palette.add_command(super::Command {
            id: "ics:export-context".to_string(),
            name: "Export Context".to_string(),
            description: "Export ICS content to markdown".to_string(),
            category: super::CommandCategory::Ics,
            shortcut: None,
        });

        command_palette.add_command(super::Command {
            id: "ics:toggle-highlighting".to_string(),
            name: "Toggle Highlighting".to_string(),
            description: "Toggle syntax/semantic highlighting".to_string(),
            category: super::CommandCategory::Ics,
            shortcut: None,
        });

        command_palette.add_command(super::Command {
            id: "ics:focus-editor".to_string(),
            name: "Focus ICS Editor".to_string(),
            description: "Focus the ICS editor panel".to_string(),
            category: super::CommandCategory::Ics,
            shortcut: Some("Ctrl+E".to_string()),
        });

        let layout = LayoutManager::new(ratatui::layout::Rect::default());
        let help_overlay = HelpOverlay::new();

        Ok(Self {
            terminal,
            event_loop,
            chat_view,
            dashboard,
            ics_panel,
            command_palette,
            help_overlay,
            layout,
            active_dialog: None,
            pending_dialog_action: PendingDialogAction::None,
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
                // Handle active dialog first (highest priority)
                if let Some(dialog) = &mut self.active_dialog {
                    let should_close = dialog.handle_key(key);
                    if should_close {
                        // Get dialog result before dropping it
                        let result = dialog.result();
                        self.active_dialog = None;

                        // Process pending action based on dialog result
                        self.process_dialog_result(result).await?;
                    }
                    return Ok(());
                }

                // Handle help overlay toggle (? key)
                if key.code == KeyCode::Char('?') {
                    let ics_visible = self.ics_panel.is_visible();
                    self.help_overlay.toggle(ics_visible);
                    return Ok(());
                }

                // Handle help overlay dismiss (Esc when help is visible)
                if self.help_overlay.is_visible() && key.code == KeyCode::Esc {
                    self.help_overlay.hide();
                    return Ok(());
                }

                // Don't process other keys if help is visible
                if self.help_overlay.is_visible() {
                    return Ok(());
                }

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
            // ICS Commands
            "ics:submit-to-claude" => {
                // Get ICS content for preview
                let content = self.ics_panel.get_content();

                // Show confirm dialog with preview
                let dialog = super::ConfirmDialog::new(
                    "Submit to Claude Code",
                    "Send this content as a prompt to Claude Code?",
                )
                .with_preview(content);

                self.active_dialog = Some(Box::new(dialog));
                self.pending_dialog_action = PendingDialogAction::SubmitToClaude;
                tracing::debug!("ICS: Submit dialog shown");
            }
            "ics:save-file" => {
                // Show input dialog for filename
                use chrono::Local;
                let timestamp = Local::now().format("%Y%m%d_%H%M%S");
                let default_filename = format!("ics-{}.md", timestamp);

                let dialog = super::InputDialog::new(
                    "Save File",
                    "Enter filename (will be saved in current directory):",
                )
                .with_default(default_filename)
                .with_validator(|s| {
                    if s.is_empty() {
                        Err("Filename cannot be empty".to_string())
                    } else if s.contains('/') || s.contains('\\') {
                        Err("Filename cannot contain path separators".to_string())
                    } else {
                        Ok(())
                    }
                });

                self.active_dialog = Some(Box::new(dialog));
                self.pending_dialog_action = PendingDialogAction::SaveFile;
                tracing::debug!("ICS: Save file dialog shown");
            }
            "ics:export-context" => {
                // Get ICS content
                let content = self.ics_panel.get_content();

                // Generate timestamped filename
                use chrono::Local;
                let timestamp = Local::now().format("%Y%m%d_%H%M%S");
                let filename = format!("ics-export-{}.md", timestamp);

                // Ensure exports directory exists
                std::fs::create_dir_all("./exports").ok();
                let filepath = format!("./exports/{}", filename);

                // Write to file
                match std::fs::write(&filepath, content) {
                    Ok(_) => {
                        tracing::info!("ICS: Exported to {}", filepath);
                        // TODO: Show success notification when notification system is implemented
                    }
                    Err(e) => {
                        tracing::error!("ICS: Export failed: {}", e);
                        // TODO: Show error dialog when error notification system is implemented
                    }
                }
            }
            "ics:toggle-highlighting" => {
                self.ics_panel.toggle_highlighting();
                tracing::debug!("ICS: Highlighting toggled");
            }
            "ics:focus-editor" => {
                // Ensure ICS panel is visible and focused
                if !self.ics_panel.is_visible() {
                    self.ics_panel.toggle();
                }
                self.ics_panel.set_focused(true);
                // TODO: Unfocus other panels when multi-panel focus is implemented
                tracing::debug!("ICS: Editor focused");
            }
            _ => {}
        }
        Ok(())
    }

    /// Process dialog result and execute pending action
    async fn process_dialog_result(&mut self, result: super::DialogResult) -> Result<()> {
        use super::DialogResult;

        match result {
            DialogResult::Confirmed => {
                // Handle confirm-only dialogs (submit)
                if let PendingDialogAction::SubmitToClaude = &self.pending_dialog_action {
                    let content = self.ics_panel.get_content();
                    if let Some(wrapper) = &self.wrapper {
                        wrapper.send_input(content.as_bytes()).await?;
                        tracing::info!("ICS: Content submitted to Claude Code");
                    } else {
                        tracing::warn!("ICS: No PTY wrapper available for submit");
                    }
                }
            }
            DialogResult::ConfirmedWithInput(input) => {
                // Handle input dialogs (save file)
                if let PendingDialogAction::SaveFile = &self.pending_dialog_action {
                    let content = self.ics_panel.get_content();
                    match std::fs::write(&input, &content) {
                        Ok(_) => {
                            tracing::info!("ICS: File saved to {}", input);
                            // TODO: Show success notification
                        }
                        Err(e) => {
                            tracing::error!("ICS: Failed to save file: {}", e);
                            // TODO: Show error dialog
                        }
                    }
                }
            }
            DialogResult::Cancelled => {
                tracing::debug!("Dialog cancelled");
            }
            DialogResult::Pending => {
                // Should not happen as dialog is already closed
            }
        }

        // Reset pending action
        self.pending_dialog_action = PendingDialogAction::None;
        Ok(())
    }

    /// Render UI
    fn render(&mut self) -> Result<()> {
        // Capture ICS visibility state before entering draw closure
        let ics_visible = self.ics_panel.is_visible();

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
                    Constraint::Length(1), // Status bar
                ])
                .split(size);

            // Split top section if ICS is visible
            let top_chunks = if ics_visible {
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
            if ics_visible && top_chunks.len() > 1 {
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

            // Render help overlay if visible (renders on top of everything)
            if self.help_overlay.is_visible() {
                frame.render_widget(&self.help_overlay, size);
            }

            // Render active dialog if present (highest priority, on top of everything)
            if let Some(dialog) = &self.active_dialog {
                dialog.render(frame, size);
            }

            // Build and render status bar with context-aware hints
            use super::StatusBar;
            let status_bar = if ics_visible {
                // ICS mode hints
                StatusBar::new()
                    .left_item("Mode", "ICS")
                    .right_item("Ctrl+Enter", "Submit")
                    .right_item("Ctrl+S", "Save")
                    .right_item("?", "Help")
                    .right_item("Ctrl+P", "Commands")
            } else {
                // Normal mode hints
                StatusBar::new()
                    .left_item("Mode", "Chat")
                    .right_item("Ctrl+P", "Commands")
                    .right_item("Ctrl+E", "ICS")
                    .right_item("Ctrl+D", "Dashboard")
                    .right_item("Ctrl+Q", "Quit")
            };
            frame.render_widget(status_bar, chunks[2]);
        })?;

        Ok(())
    }
}
