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

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
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

/// Agent info from API
#[derive(Debug, Clone, Deserialize)]
struct AgentInfo {
    id: String,
    state: serde_json::Value,
}

/// System stats from API
#[derive(Debug, Clone, Deserialize)]
struct SystemStats {
    total_agents: usize,
    active_agents: usize,
    idle_agents: usize,
    waiting_agents: usize,
    context_files: usize,
}

/// Application state
struct App {
    /// Recent events from SSE stream
    events: Vec<String>,
    /// Max events to keep
    max_events: usize,
    /// Agents list
    agents: Vec<AgentInfo>,
    /// System stats
    stats: Option<SystemStats>,
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
            events: Vec::new(),
            max_events: 100,
            agents: Vec::new(),
            stats: None,
            connected: false,
            api_url,
            event_rx,
        }
    }

    fn add_event(&mut self, event: String) {
        self.events.push(event);
        if self.events.len() > self.max_events {
            self.events.remove(0);
        }
    }

    fn process_events(&mut self) {
        // Drain all available events from channel
        while let Ok(event) = self.event_rx.try_recv() {
            self.add_event(event);
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
                    self.agents = agents;
                }
            }
            Err(e) => {
                debug!("Failed to fetch agents: {}", e);
                self.connected = false;
                return Ok(());
            }
        }

        // Fetch stats
        match client
            .get(format!("{}/state/stats", self.api_url))
            .send()
            .await
        {
            Ok(response) => {
                if let Ok(stats) = response.json::<SystemStats>().await {
                    self.stats = Some(stats);
                    self.connected = true;
                }
            }
            Err(e) => {
                debug!("Failed to fetch stats: {}", e);
                self.connected = false;
            }
        }

        Ok(())
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
                .map(|result| result.map_err(|e| io::Error::new(io::ErrorKind::Other, e)));
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
    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Header
                    Constraint::Min(10),   // Events
                    Constraint::Length(7), // Agents
                    Constraint::Length(5), // Stats
                    Constraint::Length(1), // Footer
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
            f.render_widget(header, chunks[0]);

            // Events
            let events: Vec<ListItem> = if app.events.is_empty() && app.connected {
                vec![ListItem::new(Line::from(vec![Span::styled(
                    "System idle - no recent activity",
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::ITALIC),
                )]))]
            } else {
                app.events
                    .iter()
                    .rev()
                    .take(chunks[1].height as usize - 2)
                    .map(|e| {
                        // Color code events by priority and type
                        let color = if e.contains("session_started") {
                            Color::Green
                        } else if e.contains("heartbeat") {
                            Color::Blue
                        } else if e.contains("deadlock_detected") {
                            Color::Red // Critical: deadlock detected
                        } else if e.contains("review_failed") {
                            Color::LightRed // Warning: quality gate failure
                        } else if e.contains("error")
                            || e.contains("failed")
                            || e.contains("agent_failed")
                        {
                            Color::Red // Error conditions
                        } else if e.contains("phase_changed") {
                            Color::Magenta // Important: workflow phase transition
                        } else if e.contains("work_item_retried") {
                            Color::Yellow // Notice: retry attempt
                        } else if e.contains("context_checkpointed") {
                            Color::Cyan // Info: context optimization
                        } else if e.contains("agent_started") || e.contains("agent_completed") {
                            Color::LightGreen // Success: agent lifecycle
                        } else {
                            Color::White // Default
                        };
                        ListItem::new(Line::from(vec![Span::styled(
                            e,
                            Style::default().fg(color),
                        )]))
                    })
                    .collect()
            };
            let events_list = List::new(events).block(
                Block::default()
                    .title("Recent Events")
                    .borders(Borders::ALL),
            );
            f.render_widget(events_list, chunks[1]);

            // Agents
            let agent_items: Vec<ListItem> = app
                .agents
                .iter()
                .map(|a| {
                    let state_str = a.state.to_string();
                    ListItem::new(Line::from(vec![
                        Span::styled(&a.id, Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(": "),
                        Span::raw(state_str),
                    ]))
                })
                .collect();
            let agents_list = List::new(agent_items).block(
                Block::default()
                    .title("Active Agents")
                    .borders(Borders::ALL),
            );
            f.render_widget(agents_list, chunks[2]);

            // Stats
            let stats_text = if let Some(stats) = &app.stats {
                vec![
                    Line::from(format!("Total Agents: {}", stats.total_agents)),
                    Line::from(format!(
                        "Active: {} | Idle: {} | Waiting: {}",
                        stats.active_agents, stats.idle_agents, stats.waiting_agents
                    )),
                    Line::from(format!("Context Files: {}", stats.context_files)),
                ]
            } else {
                vec![Line::from("Loading...")]
            };
            let stats_widget = Paragraph::new(stats_text).block(
                Block::default()
                    .title("System Statistics")
                    .borders(Borders::ALL),
            );
            f.render_widget(stats_widget, chunks[3]);

            // Footer
            let footer_text = if app.connected {
                if app.events.is_empty() {
                    "Press 'q' to quit | Connected to API | Waiting for activity..."
                } else {
                    "Press 'q' to quit | Connected to API | Receiving events"
                }
            } else {
                "Press 'q' to quit | Disconnected - check API server"
            };
            let footer = Paragraph::new(footer_text).style(Style::default().fg(Color::Gray));
            f.render_widget(footer, chunks[4]);
        })?;

        // Handle input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
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
