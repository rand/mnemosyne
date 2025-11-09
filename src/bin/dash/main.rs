//! Mnemosyne Dashboard - Real-time Event Monitoring
//!
//! Redesigned dashboard focused on live data and actionable signals.
//!
//! **4-Panel Layout**:
//! - System Overview: Top panel (6-8 lines) - At-a-glance health
//! - Activity Stream: Left panel (60%) - Intelligent event log
//! - Agent Details: Right-top panel (40%) - Agent activity deep-dive
//! - Operations: Right-bottom panel (40%) - CLI command history
//!
//! **Features**:
//! - Smart event filtering (default: hide heartbeats)
//! - Event correlation (link start→complete with durations)
//! - Color-coded event categories
//! - Real-time updates via Server-Sent Events (SSE)
//!
//! **Keyboard Shortcuts**:
//! - `q` / `Esc`: Quit
//! - `0`: Toggle all panels
//! - `1`: Toggle Activity Stream
//! - `2`: Toggle Agent Details
//! - `3`: Toggle Operations
//! - `h`: Toggle System Overview (header)
//!
//! Usage:
//!   mnemosyne-dash [OPTIONS]
//!
//! Examples:
//!   mnemosyne-dash                    # Connect to localhost:3000
//!   mnemosyne-dash --api http://localhost:3000
//!   mnemosyne-dash --refresh 500     # Faster refresh (ms)

mod colors;
mod correlation;
mod filters;
mod panel_manager;
mod panels;
mod time_series;
mod widgets;

use anyhow::Result;
use clap::Parser;
use mnemosyne_core::api::events::Event;
use crossterm::{
    event::{self, Event as CrosstermEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use panel_manager::{PanelId, PanelManager};
use panels::{
    ActivityStreamPanel, AgentInfo, AgentsPanel, FocusMode, OperationsPanel,
    SystemOverviewPanel,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame, Terminal,
};
use reqwest::Client;
use std::{io, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    sync::mpsc,
    time::interval,
};
use tokio_stream::StreamExt;
use tokio_util::io::StreamReader;
use tracing::{debug, error, Level};
use tracing_subscriber::EnvFilter;

/// Dashboard CLI arguments
#[derive(Parser)]
#[command(name = "mnemosyne-dash")]
#[command(about = "Real-time monitoring dashboard for Mnemosyne (redesigned)")]
#[command(version)]
struct Args {
    /// API server URL
    #[arg(long, default_value = "http://localhost:3000")]
    api: String,

    /// Refresh interval in milliseconds
    #[arg(long, default_value = "1000")]
    refresh: u64,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

/// Application state
struct App {
    /// Panel instances (new 4-panel layout)
    system_overview: SystemOverviewPanel,
    activity_stream: ActivityStreamPanel,
    agents_panel: AgentsPanel,
    operations_panel: OperationsPanel,

    /// Panel manager for visibility/layout
    panel_manager: PanelManager,

    /// Connection status
    connected: bool,
    /// API base URL
    api_url: String,
    /// Event receiver from SSE stream
    event_rx: mpsc::UnboundedReceiver<Event>,
    /// Help overlay visibility
    show_help: bool,
}

impl App {
    fn new(api_url: String, event_rx: mpsc::UnboundedReceiver<Event>) -> Self {
        Self {
            system_overview: SystemOverviewPanel::new(),
            activity_stream: ActivityStreamPanel::new(),
            agents_panel: AgentsPanel::new(),
            operations_panel: OperationsPanel::new(),
            panel_manager: PanelManager::new(),
            connected: false,
            api_url,
            event_rx,
            show_help: false,
        }
    }

    /// Process incoming events from SSE stream
    fn process_events(&mut self) {
        // Drain all available events from channel
        while let Ok(event) = self.event_rx.try_recv() {
            // Route event to all panels
            self.system_overview.add_event(event.clone());
            self.activity_stream.add_event(event.clone());

            // Route specific event types to specialized panels
            use mnemosyne_core::api::events::EventType::*;
            match &event.event_type {
                // Agent events → Agents panel
                AgentStarted { .. }
                | AgentCompleted { .. }
                | AgentFailed { .. }
                | AgentBlocked { .. }
                | AgentUnblocked { .. }
                | AgentRestarted { .. }
                | AgentHealthDegraded { .. }
                | WorkItemAssigned { .. }
                | WorkItemCompleted { .. } => {
                    self.agents_panel.add_event(event.clone());
                }

                // CLI events → Operations panel
                CliCommandStarted { command, args, .. } => {
                    self.operations_panel.add_started(command.clone(), args.clone());
                }
                CliCommandCompleted { command, duration_ms, result_summary, .. } => {
                    self.operations_panel.update_completed(command, *duration_ms, result_summary.clone());
                }
                CliCommandFailed { command, error, duration_ms, .. } => {
                    self.operations_panel.update_failed(command, error.clone(), *duration_ms);
                }

                _ => {
                    // Other events are handled by System Overview and Activity Stream
                }
            }
        }
    }

    /// Update state from API (agents list, metrics snapshot)
    async fn update_state(&mut self, client: &Client) -> Result<()> {
        // Fetch agents
        match client
            .get(format!("{}/state/agents", self.api_url))
            .send()
            .await
        {
            Ok(response) => {
                if let Ok(agents) = response.json::<Vec<AgentInfo>>().await {
                    self.agents_panel.update(agents);
                    self.connected = true;
                }
            }
            Err(e) => {
                debug!("Failed to fetch agents: {}", e);
                self.connected = false;
                return Ok(());
            }
        }

        // TODO: Fetch system metrics for SystemOverview
        // For now, we'll rely on events to populate metrics

        Ok(())
    }

    /// Handle keyboard input
    fn handle_key(&mut self, key: KeyCode) -> bool {
        // If help is showing, any key dismisses it (except '?' to toggle)
        if self.show_help && key != KeyCode::Char('?') {
            self.show_help = false;
            return false;
        }

        match key {
            // Quit
            KeyCode::Char('q') | KeyCode::Esc => return true,

            // Help overlay toggle
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
            }

            // Panel toggles
            KeyCode::Char('0') => {
                // Toggle all panels
                if self.panel_manager.visible_count() == 4 {
                    self.panel_manager.hide_all();
                } else {
                    self.panel_manager.show_all();
                }
            }
            KeyCode::Char('h') => self.panel_manager.toggle_panel(PanelId::SystemOverview),
            KeyCode::Char('1') => self.panel_manager.toggle_panel(PanelId::ActivityStream),
            KeyCode::Char('2') => self.panel_manager.toggle_panel(PanelId::AgentDetails),
            KeyCode::Char('3') => self.panel_manager.toggle_panel(PanelId::Operations),

            // Activity Stream controls
            KeyCode::Char('c') => {
                // Clear activity stream
                self.activity_stream.clear();
            }

            // Operations panel controls
            KeyCode::Char('v') => {
                // Cycle view mode (List → Grouped → Statistics)
                // Will be implemented by sub-agent
            }

            // Focus modes (filter presets)
            KeyCode::Char('e') => {
                // Toggle error focus mode
                self.activity_stream.toggle_error_focus();
            }
            KeyCode::Char('a') => {
                // Toggle agent focus mode (cycle through active agents)
                let available_agents = self.agents_panel.get_active_agent_ids();
                self.activity_stream.toggle_agent_focus(available_agents);
            }

            // Scrolling (Up/Down for active panel)
            KeyCode::Up => {
                // Scroll up in focused panel - will be enhanced by sub-agent
            }
            KeyCode::Down => {
                // Scroll down in focused panel - will be enhanced by sub-agent
            }
            KeyCode::PageUp => {
                // Page up - will be enhanced by sub-agent
            }
            KeyCode::PageDown => {
                // Page down - will be enhanced by sub-agent
            }

            _ => {}
        }
        false // Don't quit
    }

    /// Render the dashboard
    fn render(&mut self, frame: &mut Frame) {
        let size = frame.area();

        // Get focus mode indicator
        let focus_text = match self.activity_stream.get_focus_mode() {
            FocusMode::Normal => String::new(),
            FocusMode::ErrorFocus => " | Focus: ERRORS".to_string(),
            FocusMode::AgentFocus(agent_id) => {
                let truncated = if agent_id.len() > 12 {
                    format!("{}...", &agent_id[..9])
                } else {
                    agent_id.clone()
                };
                format!(" | Focus: {}", truncated)
            }
        };

        // Create header with status
        let status_text = if self.connected {
            format!(" Connected to {}{} | Press '?' for help, 'q' to quit ", self.api_url, focus_text)
        } else {
            format!(" Connecting to {}... ", self.api_url)
        };

        let header = Paragraph::new(status_text)
            .style(if self.connected {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Yellow)
            })
            .block(Block::default().borders(Borders::BOTTOM));

        // Layout: [Header (1 line), Body (rest)]
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(size);

        frame.render_widget(header, main_chunks[0]);

        // Body layout depends on visible panels
        self.render_panels(frame, main_chunks[1]);

        // Render help overlay if active
        if self.show_help {
            self.render_help_overlay(frame, size);
        }
    }

    /// Render panels based on visibility
    fn render_panels(&mut self, frame: &mut Frame, area: Rect) {
        let visibility = self.panel_manager.visibility();

        // Calculate layout based on visible panels
        let mut vertical_constraints = Vec::new();

        // System Overview (top, fixed height)
        if visibility.is_visible(PanelId::SystemOverview) {
            vertical_constraints.push(Constraint::Length(8));
        }

        // Remaining space for Activity Stream and right column
        if visibility.is_visible(PanelId::ActivityStream)
            || visibility.is_visible(PanelId::AgentDetails)
            || visibility.is_visible(PanelId::Operations)
        {
            vertical_constraints.push(Constraint::Min(0));
        }

        // If no panels visible, show message
        if vertical_constraints.is_empty() {
            let msg = Paragraph::new(" All panels hidden. Press '0' to show all. ")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(msg, area);
            return;
        }

        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vertical_constraints)
            .split(area);

        let mut chunk_index = 0;

        // Render System Overview (top panel)
        if visibility.is_visible(PanelId::SystemOverview) {
            self.system_overview.render(frame, vertical_chunks[chunk_index]);
            chunk_index += 1;
        }

        // Render bottom row (Activity Stream + right column)
        if chunk_index < vertical_chunks.len() {
            let bottom_area = vertical_chunks[chunk_index];

            // Horizontal split: Activity Stream (60%) | Right column (40%)
            let horiz_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(bottom_area);

            // Left: Activity Stream
            if visibility.is_visible(PanelId::ActivityStream) {
                self.activity_stream.render(frame, horiz_chunks[0]);
            } else {
                let placeholder = Paragraph::new(" Activity Stream hidden (press '1' to show) ")
                    .style(Style::default().fg(Color::DarkGray))
                    .block(Block::default().borders(Borders::ALL).title("Activity Stream"));
                frame.render_widget(placeholder, horiz_chunks[0]);
            }

            // Right column: Agent Details + Operations
            let right_vert_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(horiz_chunks[1]);

            // Agent Details (right-top)
            if visibility.is_visible(PanelId::AgentDetails) {
                self.agents_panel.render(frame, right_vert_chunks[0]);
            } else {
                let placeholder = Paragraph::new(" Agent Details hidden (press '2' to show) ")
                    .style(Style::default().fg(Color::DarkGray))
                    .block(Block::default().borders(Borders::ALL).title("Agent Details"));
                frame.render_widget(placeholder, right_vert_chunks[0]);
            }

            // Operations (right-bottom)
            if visibility.is_visible(PanelId::Operations) {
                self.operations_panel.render(frame, right_vert_chunks[1]);
            } else {
                let placeholder = Paragraph::new(" Operations hidden (press '3' to show) ")
                    .style(Style::default().fg(Color::DarkGray))
                    .block(Block::default().borders(Borders::ALL).title("Operations"));
                frame.render_widget(placeholder, right_vert_chunks[1]);
            }
        }
    }

    /// Render help overlay with keyboard shortcuts
    fn render_help_overlay(&self, frame: &mut Frame, area: Rect) {
        // Center the help box (60% width, 70% height)
        let popup_width = (area.width as f32 * 0.6) as u16;
        let popup_height = (area.height as f32 * 0.7) as u16;
        let x = (area.width.saturating_sub(popup_width)) / 2;
        let y = (area.height.saturating_sub(popup_height)) / 2;

        let popup_area = Rect {
            x: area.x + x,
            y: area.y + y,
            width: popup_width,
            height: popup_height,
        };

        // Build help text with formatting
        let help_lines = vec![
            Line::from(vec![
                Span::styled(
                    "MNEMOSYNE DASHBOARD - KEYBOARD SHORTCUTS",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("General", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  q, Esc  ", Style::default().fg(Color::Green)),
                Span::raw("Quit dashboard"),
            ]),
            Line::from(vec![
                Span::styled("  ?       ", Style::default().fg(Color::Green)),
                Span::raw("Toggle this help screen"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Panel Visibility", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  0       ", Style::default().fg(Color::Green)),
                Span::raw("Toggle all panels"),
            ]),
            Line::from(vec![
                Span::styled("  h       ", Style::default().fg(Color::Green)),
                Span::raw("Toggle System Overview (header)"),
            ]),
            Line::from(vec![
                Span::styled("  1       ", Style::default().fg(Color::Green)),
                Span::raw("Toggle Activity Stream"),
            ]),
            Line::from(vec![
                Span::styled("  2       ", Style::default().fg(Color::Green)),
                Span::raw("Toggle Agent Details"),
            ]),
            Line::from(vec![
                Span::styled("  3       ", Style::default().fg(Color::Green)),
                Span::raw("Toggle Operations"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Activity Stream", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  c       ", Style::default().fg(Color::Green)),
                Span::raw("Clear activity stream"),
            ]),
            Line::from(vec![
                Span::styled("  e       ", Style::default().fg(Color::Green)),
                Span::raw("Toggle error focus mode (errors only)"),
            ]),
            Line::from(vec![
                Span::styled("  a       ", Style::default().fg(Color::Green)),
                Span::raw("Toggle agent focus mode (cycle through agents)"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Operations Panel", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  v       ", Style::default().fg(Color::Green)),
                Span::raw("Cycle view mode (List → Grouped → Statistics)"),
                Span::styled(" [Coming Soon]", Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Navigation", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  ↑/↓     ", Style::default().fg(Color::Green)),
                Span::raw("Scroll up/down in focused panel"),
                Span::styled(" [Coming Soon]", Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(vec![
                Span::styled("  PgUp/Dn ", Style::default().fg(Color::Green)),
                Span::raw("Page up/down in focused panel"),
                Span::styled(" [Coming Soon]", Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Press any key to dismiss this help",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::ITALIC),
                ),
            ]),
        ];

        let help_text = Paragraph::new(help_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(" Help ")
                    .title_alignment(Alignment::Center),
            )
            .alignment(Alignment::Left)
            .style(Style::default().bg(Color::Black));

        // Clear background and render help
        frame.render_widget(Clear, popup_area);
        frame.render_widget(help_text, popup_area);
    }
}

/// Spawn SSE client to stream events from API server
fn spawn_sse_client(api_url: String, event_tx: mpsc::UnboundedSender<Event>) {
    tokio::spawn(async move {
        loop {
            debug!("Connecting to SSE endpoint: {}/events", api_url);

            let client = Client::new();
            let response = match client.get(format!("{}/events", api_url)).send().await {
                Ok(resp) => resp,
                Err(e) => {
                    debug!("Failed to connect to SSE endpoint: {}", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            if !response.status().is_success() {
                debug!("SSE endpoint returned error: {}", response.status());
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }

            debug!("Connected to SSE stream");

            // Convert response body to async reader
            let stream = response
                .bytes_stream()
                .map(|result| result.map_err(io::Error::other));
            let reader = StreamReader::new(stream);
            let mut lines = BufReader::new(reader).lines();

            let mut current_data = String::new();

            // Read SSE format line by line
            while let Ok(Some(line)) = lines.next_line().await {
                if line.is_empty() {
                    // Empty line marks end of event - process accumulated data
                    if !current_data.is_empty() {
                        // Parse JSON event
                        if let Ok(event) = serde_json::from_str::<Event>(&current_data) {
                            if event_tx.send(event).is_err() {
                                error!("Event channel closed, stopping SSE client");
                                return;
                            }
                        } else {
                            debug!("Failed to parse event: {}", current_data);
                        }
                        current_data.clear();
                    }
                } else if let Some(data) = line.strip_prefix("data: ") {
                    // Accumulate data lines
                    current_data.push_str(data);
                }
                // Ignore other SSE fields (id:, event:, retry:)
            }

            debug!("SSE stream disconnected, reconnecting in 5s...");
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging (to file, not stderr to avoid TUI interference)
    let level = match args.log_level.as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/mnemosyne-dash.log")?;

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(level.into())
                .from_env_lossy(),
        )
        .with_writer(log_file)
        .init();

    debug!("Starting Mnemosyne Dashboard (redesigned 4-panel layout)");
    debug!("API URL: {}", args.api);

    // Create event channel for SSE → App communication
    let (event_tx, event_rx) = mpsc::unbounded_channel();

    // Spawn SSE client
    spawn_sse_client(args.api.clone(), event_tx);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(args.api.clone(), event_rx);

    // HTTP client for API polling
    let client = Client::new();

    // Refresh interval
    let mut refresh_interval = interval(Duration::from_millis(args.refresh));

    // Event loop
    loop {
        // Process SSE events
        app.process_events();

        // Render
        terminal.draw(|frame| app.render(frame))?;

        // Handle input with timeout
        if event::poll(Duration::from_millis(16))? {
            if let CrosstermEvent::Key(key) = event::read()? {
                if app.handle_key(key.code) {
                    break; // Quit requested
                }
            }
        }

        // Refresh state periodically
        tokio::select! {
            _ = refresh_interval.tick() => {
                app.update_state(&client).await?;
            }
            else => {}
        }
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
