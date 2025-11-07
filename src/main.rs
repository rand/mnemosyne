//! Mnemosyne - Project-Aware Agentic Memory System for Claude Code
//!
//! This is the main entry point for the Mnemosyne MCP server, which provides
//! persistent semantic memory capabilities to Claude Code's multi-agent system.

mod cli;

use clap::{Parser, Subcommand};
use mnemosyne_core::{
    error::Result,
    launcher,
};
use std::path::PathBuf;
use tracing::{debug, info, warn, Level};
use tracing_subscriber::{self, EnvFilter};

// Import helper functions from cli module
use cli::helpers::{
    get_db_path,
    start_mcp_server,
};

/// Mnemosyne CLI arguments
#[derive(Parser)]
#[command(name = "mnemosyne")]
#[command(about = "Project-aware agentic memory system for Claude Code", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Start MCP server only (don't launch Claude Code session)
    #[arg(long)]
    serve: bool,

    /// Set log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "warn")]
    log_level: String,

    /// Database path (overrides MNEMOSYNE_DB_PATH env var and default)
    #[arg(long)]
    db_path: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start MCP server (stdio mode with automatic API server)
    ///
    /// Automatically starts API server on port 3000 (owner mode) or connects
    /// to existing API server (client mode) for dashboard observability.
    Serve,

    /// Start HTTP API server for event streaming and state coordination
    ApiServer {
        /// Server address
        #[arg(long, default_value = "127.0.0.1:3000")]
        addr: String,

        /// Event channel capacity
        #[arg(long, default_value = "1000")]
        capacity: usize,
    },

    /// Initialize database
    Init {
        /// Database path
        #[arg(short, long)]
        database: Option<String>,
    },

    /// Export memories to Markdown
    Export {
        /// Output path (prints to stdout if not specified)
        #[arg(short, long)]
        output: Option<String>,

        /// Namespace filter
        #[arg(short, long)]
        namespace: Option<String>,
    },

    /// Show system status
    Status,

    /// Launch Integrated Context Studio (ICS) - Full-featured context editor
    ///
    /// Edit context files with syntax highlighting, semantic analysis,
    /// memory integration, and AI-powered suggestions.
    #[command(visible_alias = "ics")]
    Edit {
        /// File to edit (creates if doesn't exist)
        file: Option<PathBuf>,

        /// Start in read-only mode
        #[arg(long)]
        readonly: bool,

        /// Use template (api, architecture, bugfix, feature, refactor)
        #[arg(long)]
        template: Option<cli::edit::IcsTemplate>,

        /// Start with specific panel visible (memory, diagnostics, proposals, holes)
        #[arg(long)]
        panel: Option<cli::edit::IcsPanel>,

        /// Session context file for handoff coordination (hidden, for integration)
        #[arg(long, hide = true)]
        session_context: Option<PathBuf>,
    },

    /// Launch TUI wrapper mode (enhanced interface with command palette and ICS)
    Tui {
        /// Start with ICS panel visible
        #[arg(long)]
        with_ics: bool,

        /// Disable dashboard
        #[arg(long)]
        no_dashboard: bool,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        action: cli::config::ConfigAction,
    },

    /// Manage encrypted secrets
    Secrets {
        #[command(subcommand)]
        command: cli::secrets::SecretsCommand,
    },

    /// Launch multi-agent orchestration system
    Orchestrate {
        /// Work plan file (JSON) or prompt string
        #[arg(short, long)]
        plan: String,

        /// Database path
        #[arg(short, long)]
        database: Option<String>,

        /// Enable dashboard monitoring
        #[arg(long)]
        dashboard: bool,

        /// Polling interval in milliseconds (default: 10ms)
        #[arg(long, default_value = "10")]
        polling_interval: u64,

        /// Max concurrent agents (default: 4)
        #[arg(long, default_value = "4")]
        max_concurrent: u8,
    },

    /// Remember new information (store a memory)
    Remember {
        /// Content to remember
        #[arg(short, long)]
        content: String,

        /// Namespace (e.g., "project:myapp" or "global")
        #[arg(short, long, default_value = "global")]
        namespace: String,

        /// Importance (1-10)
        #[arg(short, long, default_value = "5")]
        importance: u8,

        /// Additional context
        #[arg(long)]
        context: Option<String>,

        /// Tags (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,

        /// Memory type (architecture|code_pattern|bug_fix|configuration|constraint|entity|insight|reference|preference|task|decision)
        #[arg(short = 'y', long, alias = "type")]
        memory_type: Option<String>,

        /// Output format
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Recall memories (search and retrieve)
    Recall {
        /// Search query
        #[arg(short, long)]
        query: String,

        /// Namespace filter
        #[arg(short, long)]
        namespace: Option<String>,

        /// Maximum results
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Minimum importance (1-10)
        #[arg(long)]
        min_importance: Option<u8>,

        /// Output format (text/json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Generate embeddings for memories
    Embed {
        /// Embed all memories (regenerate all embeddings)
        #[arg(long)]
        all: bool,

        /// Memory ID to embed (specific memory)
        #[arg(long)]
        memory_id: Option<String>,

        /// Namespace filter (embed all memories in namespace)
        #[arg(short, long)]
        namespace: Option<String>,

        /// Batch size for processing (default: 32)
        #[arg(long, default_value = "32")]
        batch_size: usize,

        /// Show progress bar
        #[arg(long)]
        progress: bool,
    },

    /// Manage embedding models
    Models {
        #[command(subcommand)]
        action: cli::models::ModelsAction,
    },

    /// Run evolution jobs (importance recalibration, link decay, archival)
    Evolve {
        #[command(subcommand)]
        job: cli::evolve::EvolveJob,
    },

    /// Manage specification workflow artifacts
    Artifact {
        #[command(subcommand)]
        command: cli::artifact::ArtifactCommands,
    },

    /// Run health checks on the mnemosyne system
    Doctor {
        /// Show detailed diagnostics
        #[arg(short, long)]
        verbose: bool,

        /// Attempt to fix issues automatically
        #[arg(short, long)]
        fix: bool,

        /// Output in JSON format
        #[arg(short, long)]
        json: bool,
    },

    /// Check for and install tool updates
    Update {
        /// Specific tools to update (e.g., "mnemosyne", "claude", "beads")
        /// If not specified, all tools with available updates will be updated
        tools: Vec<String>,

        /// Show installation instructions instead of updating
        #[arg(long)]
        install: bool,

        /// Only check for updates without installing
        #[arg(long)]
        check: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let level = match cli.log_level.as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    // Build filter: use specified level for mnemosyne, but WARN for noisy external crates
    // Suppress tokio broadcast channel "recv error" spam from SSE disconnections
    // Suppress iroh netcheck probe warnings during shutdown (expected when cancelling background tasks)
    let filter = EnvFilter::new(format!(
        "mnemosyne={},iroh=warn,iroh_net=warn,iroh::net::magicsock=error,iroh::netcheck=error,iroh_net::netcheck=error,tokio::sync::broadcast=error,tokio_stream=error",
        level.as_str().to_lowercase()
    ));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr) // Write logs to stderr, not stdout
        .init();

    debug!("Mnemosyne v{} starting...", env!("CARGO_PKG_VERSION"));

    // Handle --serve flag (start MCP server without Claude Code)
    if cli.serve && cli.command.is_none() {
        debug!("Starting MCP server (--serve mode)...");
        return start_mcp_server(cli.db_path).await;
    }

    match cli.command {
        Some(Commands::Serve) => {
            cli::serve::handle(cli.db_path).await
        }
        Some(Commands::ApiServer { addr, capacity }) => {
            cli::api_server::handle(addr, capacity).await
        }
        Some(Commands::Init { database }) => {
            cli::init::handle(database, cli.db_path.clone()).await
        }
        Some(Commands::Export { output, namespace }) => {
            cli::export::handle(output, namespace, cli.db_path.clone()).await
        }
        Some(Commands::Status) => {
            cli::status::handle(cli.db_path.clone()).await
        }
        Some(Commands::Edit {
            file,
            readonly,
            template,
            panel,
            session_context,
        }) => {
            cli::edit::handle(file, readonly, template, panel, session_context, cli.db_path.clone())
                .await
        }
        Some(Commands::Tui { with_ics: _, no_dashboard: _ }) => {
            cli::tui::handle().await
        }
        Some(Commands::Config { action }) => {
            cli::config::handle(action).await
        }
        Some(Commands::Secrets { command }) => cli::secrets::handle(command).await,
        Some(Commands::Orchestrate {
            plan,
            database,
            dashboard,
            polling_interval: _,
            max_concurrent,
        }) => {
            cli::orchestrate::handle(plan, database, dashboard, max_concurrent).await
        }
        Some(Commands::Remember {
            content,
            namespace,
            importance,
            context,
            tags,
            memory_type,
            format,
        }) => {
            cli::remember::handle(
                content,
                namespace,
                importance,
                context,
                tags,
                memory_type,
                format,
                cli.db_path.clone(),
            )
            .await
        }
        Some(Commands::Recall {
            query,
            namespace,
            limit,
            min_importance,
            format,
        }) => {
            cli::recall::handle(query, namespace, limit, min_importance, format, cli.db_path.clone())
                .await
        }
        Some(Commands::Embed {
            all,
            memory_id,
            namespace,
            batch_size,
            progress,
        }) => {
            cli::embed::handle(all, memory_id, namespace, batch_size, progress, cli.db_path.clone())
                .await
        }
        Some(Commands::Models { action }) => {
            cli::models::handle(action).await
        }
        Some(Commands::Evolve { job }) => cli::evolve::handle(job, cli.db_path.clone()).await,
        Some(Commands::Artifact { command }) => {
            cli::artifact::handle(command, cli.db_path.clone()).await
        }
        Some(Commands::Doctor { verbose, fix, json }) => {
            cli::doctor::handle(verbose, fix, json, cli.db_path.clone()).await
        }
        Some(Commands::Update { tools, install, check }) => {
            cli::update::handle(tools, install, check).await
        }
        None => {
            use mnemosyne_core::api::{ApiServer, ApiServerConfig};
            use std::net::SocketAddr;

            // Default: launch orchestrated Claude Code session with API server
            debug!("Launching orchestrated Claude Code session with API monitoring...");

            // Get database path
            let db_path = get_db_path(cli.db_path);

            // Define agent names
            let agent_names = ["Orchestrator", "Optimizer", "Reviewer", "Executor"];

            // Create API server for dashboard connectivity
            let socket_addr: SocketAddr = "127.0.0.1:3000"
                .parse()
                .expect("Invalid default API address");
            let api_config = ApiServerConfig {
                addr: socket_addr,
                event_capacity: 1000,
            };
            let api_server = ApiServer::new(api_config);
            let event_broadcaster = api_server.broadcaster().clone();
            let state_manager = api_server.state_manager().clone();

            // Spawn API server in background
            let api_handle = tokio::spawn(async move {
                if let Err(e) = api_server.serve().await {
                    warn!("API server error: {}", e);
                }
            });

            // Show clean launch UI with banner
            launcher::ui::show_launch_header(
                env!("CARGO_PKG_VERSION"),
                &db_path,
                &agent_names,
            );

            // Check for updates (non-blocking, 3s timeout)
            launcher::ui::check_and_show_updates().await;

            // Show dashboard availability
            info!("Dashboard available at: http://{}", socket_addr);
            info!("Run 'mnemosyne-dash' in another terminal to monitor activity");

            // Show playful loading messages with 3-line animation
            let progress = launcher::ui::LaunchProgress::new();
            progress.show_multiline_loading();
            progress.show_transition();

            // Launch orchestrated session with event broadcasting and state management
            let result = launcher::launch_orchestrated_session(
                Some(db_path),
                None,
                Some(event_broadcaster),
                Some(state_manager),
            )
            .await;

            // Show completion or error
            if result.is_ok() {
                progress.show_step_complete("Orchestration ready");
                println!(); // Extra spacing after startup
            } else if let Err(ref e) = result {
                progress.show_error(&format!("{}", e));
            }

            // Clean up API server
            api_handle.abort();
            debug!("API server shut down");

            result
        }
    }
}
