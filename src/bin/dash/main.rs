//! Mnemosyne Dashboard - Real-time Monitoring
//!
//! Monitors Mnemosyne event stream and displays:
//! - Agent activity
//! - Memory operations
//! - Context file changes
//! - System health metrics
//!
//! Usage:
//!   mnemosyne-dash [OPTIONS]
//!
//! Examples:
//!   mnemosyne-dash                    # Connect to localhost:3000
//!   mnemosyne-dash --api http://localhost:3000
//!   mnemosyne-dash --refresh 500     # Faster refresh (ms)

mod panel_manager;
mod panels;
mod time_series;
mod widgets;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use panel_manager::{PanelId, PanelManager};
use panels::{
    AgentInfo, AgentsPanel, ContextPanel, EventLogPanel, MemoryOpsMetrics, MemoryPanel,
    SkillsMetrics, SkillsPanel, WorkMetrics, WorkPanel,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use reqwest::Client;
use serde::Deserialize;
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
#[command(about = "Real-time monitoring dashboard for Mnemosyne")]
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

/// Minimal event structure for display (for future SSE integration)
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct EventDisplay {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(flatten)]
    data: serde_json::Value,
}

/// Metrics snapshot from API
#[derive(Debug, Clone, Deserialize, Default)]
struct MetricsSnapshot {
    #[allow(dead_code)]
    pub agent_states: AgentStateCounts,
    pub memory_ops: MemoryOpsMetrics,
    pub skills: SkillsMetrics,
    pub work: WorkMetrics,
}

/// Agent state counts from metrics
#[derive(Debug, Clone, Deserialize, Default)]
struct AgentStateCounts {
    pub active: usize,
    pub idle: usize,
    pub waiting: usize,
    pub completed: usize,
    pub failed: usize,
    pub total: usize,
}

/// Application state
struct App {
    /// Panel instances
    agents_panel: AgentsPanel,
    memory_panel: MemoryPanel,
    skills_panel: SkillsPanel,
    work_panel: WorkPanel,
    context_panel: ContextPanel,
    event_log_panel: EventLogPanel,

    /// Panel manager for visibility/layout
    panel_manager: PanelManager,

    /// Connection status
    connected: bool,
    /// API base URL
    api_url: String,
    /// Event receiver from SSE stream
    event_rx: mpsc::UnboundedReceiver<String>,
}

impl App {
    fn new(api_url: String, event_rx: mpsc::UnboundedReceiver<String>) -> Self {
        Self {
            agents_panel: AgentsPanel::new(),
            memory_panel: MemoryPanel::new(),
            skills_panel: SkillsPanel::new(),
            work_panel: WorkPanel::new(),
            context_panel: ContextPanel::new(),
            event_log_panel: EventLogPanel::new().max_events(1000),
            panel_manager: PanelManager::new(),
            connected: false,
            api_url,
            event_rx,
        }
    }

    fn process_events(&mut self) {
        // Drain all available events from channel
        while let Ok(event) = self.event_rx.try_recv() {
            self.event_log_panel.add_event(event);
        }
    }

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

        // Fetch metrics snapshot
        match client
            .get(format!("{}/state/metrics", self.api_url))
            .send()
            .await
        {
            Ok(response) => {
                if let Ok(metrics) = response.json::<MetricsSnapshot>().await {
                    // Update panels with metrics
                    self.memory_panel.update(metrics.memory_ops);
                    self.skills_panel.update(metrics.skills);
                    self.work_panel.update(metrics.work);
                    self.connected = true;
                }
            }
            Err(e) => {
                debug!("Failed to fetch metrics: {}", e);
            }
        }

        // TODO: Fetch context utilization (for now use mock data)
        // In a real implementation, this would come from /state/stats or a dedicated endpoint
        self.context_panel.update(45.0, 2); // Mock: 45% utilization, 2 checkpoints

        Ok(())
    }

    /// Handle keyboard input for panel toggles
    fn handle_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => return true, // Quit
            KeyCode::Char('0') => {
                // Toggle all panels
                if self.panel_manager.visible_count() == 6 {
                    self.panel_manager.hide_all();
                } else {
                    self.panel_manager.show_all();
                }
            }
            KeyCode::Char('1') => self.panel_manager.toggle_panel(PanelId::Agents),
            KeyCode::Char('2') => self.panel_manager.toggle_panel(PanelId::Memory),
            KeyCode::Char('3') => self.panel_manager.toggle_panel(PanelId::Skills),
            KeyCode::Char('4') => self.panel_manager.toggle_panel(PanelId::Work),
            KeyCode::Char('5') => self.panel_manager.toggle_panel(PanelId::Context),
            KeyCode::Char('6') => self.panel_manager.toggle_panel(PanelId::Events),
            _ => {}
        }
        false // Don't quit
    }
}

/// Spawn SSE client to stream events from API server
fn spawn_sse_client(api_url: String, event_tx: mpsc::UnboundedSender<String>) {
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
                        if let Ok(event) = serde_json::from_str::<EventDisplay>(&current_data) {
                            let formatted = format!(
                                "[{}] {}",
                                event.event_type,
                                serde_json::to_string(&event.data).unwrap_or_default()
                            );
                            if event_tx.send(formatted).is_err() {
                                error!("Event channel closed, stopping SSE client");
                                return;
                            }
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

    // Initialize logging (to file, not stderr)
    let level = match args.log_level.as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    let filter = EnvFilter::new(format!("mnemosyne_dash={}", level.as_str().to_lowercase()));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(|| {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/mnemosyne-dash.log")
                .unwrap()
        })
        .init();

    debug!("Dashboard v{} starting...", env!("CARGO_PKG_VERSION"));
    debug!("API URL: {}", args.api);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create event channel for SSE stream
    let (event_tx, event_rx) = mpsc::unbounded_channel();

    // Spawn SSE client
    spawn_sse_client(args.api.clone(), event_tx);

    // Create app state
    let mut app = App::new(args.api.clone(), event_rx);
    let client = Client::new();

    // Refresh interval
    let mut tick = interval(Duration::from_millis(args.refresh));

    // Run app
    let result = run_app(&mut terminal, &mut app, &client, &mut tick).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        error!("Error: {:?}", err);
        return Err(err);
    }

    debug!("Dashboard exiting cleanly");
    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    client: &Client,
    tick: &mut tokio::time::Interval,
) -> Result<()> {
    // Force initial state refresh immediately on connection
    // This ensures agents are visible right away, even if they spawned before SSE connected
    app.update_state(client).await?;
    debug!("Initial state refresh complete");

    loop {
        terminal.draw(|f| {
            let total_height = f.area().height;

            // Calculate dynamic layout: Header (3) + Panels (dynamic) + Footer (1)
            let header_height = 3;
            let footer_height = 1;
            let panels_height = total_height.saturating_sub(header_height + footer_height);

            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(header_height),
                    Constraint::Length(panels_height),
                    Constraint::Length(footer_height),
                ])
                .split(f.area());

            // Header
            let title = if app.connected {
                format!(
                    "{} Mnemosyne Dashboard [Connected]",
                    mnemosyne_core::icons::system::palette()
                )
            } else {
                format!(
                    "{} Mnemosyne Dashboard [Disconnected]",
                    mnemosyne_core::icons::system::palette()
                )
            };
            let header = Paragraph::new(title.as_str())
                .style(Style::default().fg(if app.connected {
                    Color::Green
                } else {
                    Color::Red
                }))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(header, main_chunks[0]);

            // Dynamic panel layout
            let panel_constraints = app.panel_manager.layout_constraints(panels_height);
            let panel_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(panel_constraints)
                .split(main_chunks[1]);

            // Render visible panels in order
            let mut chunk_index = 0;
            if app.panel_manager.is_panel_visible(PanelId::Agents) {
                app.agents_panel.render(f, panel_chunks[chunk_index]);
                chunk_index += 1;
            }
            if app.panel_manager.is_panel_visible(PanelId::Memory) {
                app.memory_panel.render(f, panel_chunks[chunk_index]);
                chunk_index += 1;
            }
            if app.panel_manager.is_panel_visible(PanelId::Skills) {
                app.skills_panel.render(f, panel_chunks[chunk_index]);
                chunk_index += 1;
            }
            if app.panel_manager.is_panel_visible(PanelId::Work) {
                app.work_panel.render(f, panel_chunks[chunk_index]);
                chunk_index += 1;
            }
            if app.panel_manager.is_panel_visible(PanelId::Context) {
                app.context_panel.render(f, panel_chunks[chunk_index]);
                chunk_index += 1;
            }
            if app.panel_manager.is_panel_visible(PanelId::Events) {
                app.event_log_panel.render(f, panel_chunks[chunk_index]);
            }

            // Footer with keyboard shortcuts
            let visible_count = app.panel_manager.visible_count();
            let footer_text = if app.connected {
                format!(
                    "Press '0' to toggle all | '1-6' for panels | 'q' to quit | {} panels visible",
                    visible_count
                )
            } else {
                "Disconnected - check API server | Press 'q' to quit".to_string()
            };
            let footer = Paragraph::new(footer_text).style(Style::default().fg(Color::Gray));
            f.render_widget(footer, main_chunks[2]);
        })?;

        // Handle input with keyboard shortcuts
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if app.handle_key(key.code) {
                    return Ok(());
                }
            }
        }

        // Process incoming events and update state on tick
        app.process_events();

        tokio::select! {
            _ = tick.tick() => {
                app.update_state(client).await?;
            }
        }
    }
}
