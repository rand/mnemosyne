//! Mnemosyne - Project-Aware Agentic Memory System for Claude Code
//!
//! This is the main entry point for the Mnemosyne MCP server, which provides
//! persistent semantic memory capabilities to Claude Code's multi-agent system.

use clap::{Parser, Subcommand};
use mnemosyne_core::{
    error::{MnemosyneError, Result},
    launcher,
    storage::MemorySortOrder,
    ConfigManager, ConnectionMode, LibsqlStorage, LlmConfig, LlmService, McpServer, StorageBackend,
    ToolHandler,
};
// Use the v1.0 embedding service for backward compatibility
use mnemosyne_core::services::embeddings::EmbeddingService;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info, warn, Level};
use tracing_subscriber::{self, EnvFilter};

/// Get the default database path using XDG_DATA_HOME standard
fn get_default_db_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("mnemosyne")
        .join("mnemosyne.db")
}

/// Get the database path from CLI arg, env var, project dir, or default
fn get_db_path(cli_path: Option<String>) -> String {
    cli_path
        .or_else(|| std::env::var("MNEMOSYNE_DB_PATH").ok())
        .or_else(|| {
            // Check DATABASE_URL for test compatibility
            std::env::var("DATABASE_URL").ok().and_then(|url| {
                // Strip sqlite:// prefix if present
                if url.starts_with("sqlite://") {
                    let path = url.strip_prefix("sqlite://").unwrap().to_string();
                    if !path.is_empty() {
                        Some(path)
                    } else {
                        None
                    }
                } else if !url.is_empty() && url != ":memory:" && !url.starts_with("libsql://") {
                    Some(url)
                } else {
                    None
                }
            })
        })
        .or_else(|| {
            // Check for project-specific database in .mnemosyne/
            let project_db = PathBuf::from(".mnemosyne").join("project.db");
            if project_db.exists() {
                Some(project_db.to_string_lossy().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| get_default_db_path().to_string_lossy().to_string())
}

/// Process structured JSON work plan
///
/// Parses and displays a structured work plan in JSON format.
/// Supports common schemas with tasks, phases, or steps.
fn process_structured_plan(plan: &serde_json::Value) {
    // Try to extract tasks from various common JSON structures
    let tasks = extract_tasks_from_plan(plan);

    if tasks.is_empty() {
        println!("  ℹ  No tasks found in plan structure");
        println!("  Expected JSON with 'tasks', 'phases', or 'steps' field");
        return;
    }

    println!("  Found {} task(s):", tasks.len());
    println!();

    for (i, task) in tasks.iter().enumerate() {
        println!("  {}. {}", i + 1, task);
    }

    println!();
    println!("  ℹ  Structured execution not yet fully implemented");
    println!("  Falling back to prompt-based orchestration");
}

/// Extract tasks from various JSON plan formats
fn extract_tasks_from_plan(plan: &serde_json::Value) -> Vec<String> {
    use serde_json::Value;

    let mut tasks = Vec::new();

    // Try direct "tasks" array
    if let Some(Value::Array(task_array)) = plan.get("tasks") {
        for task in task_array {
            if let Some(desc) = extract_task_description(task) {
                tasks.push(desc);
            }
        }
    }

    // Try "phases" with tasks
    if let Some(Value::Array(phases)) = plan.get("phases") {
        for phase in phases {
            if let Some(Value::Array(phase_tasks)) = phase.get("tasks") {
                for task in phase_tasks {
                    if let Some(desc) = extract_task_description(task) {
                        tasks.push(desc);
                    }
                }
            }
        }
    }

    // Try "steps" array
    if let Some(Value::Array(steps)) = plan.get("steps") {
        for step in steps {
            if let Some(desc) = extract_task_description(step) {
                tasks.push(desc);
            }
        }
    }

    tasks
}

/// Extract task description from various formats
fn extract_task_description(task: &serde_json::Value) -> Option<String> {
    use serde_json::Value;

    // Try string directly
    if let Value::String(s) = task {
        return Some(s.clone());
    }

    // Try object with common fields
    if let Value::Object(obj) = task {
        // Try "description", "title", "name", "task", "content"
        for field in &["description", "title", "name", "task", "content"] {
            if let Some(Value::String(s)) = obj.get(*field) {
                return Some(s.clone());
            }
        }
    }

    None
}

/// Start MCP server in stdio mode
async fn start_mcp_server(db_path_arg: Option<String>) -> Result<()> {
    debug!("Starting MCP server...");

    // Initialize configuration
    let _config_manager = ConfigManager::new()?;

    // Initialize storage with configured database path
    let db_path = get_db_path(db_path_arg);
    debug!("Using database: {}", db_path);

    // Ensure parent directory exists
    if let Some(parent) = PathBuf::from(&db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // MCP server should create database if it doesn't exist (for first-time setup)
    let storage = LibsqlStorage::new_with_validation(ConnectionMode::Local(db_path), true).await?;

    // Initialize LLM service (will error on first use if no API key)
    let llm = match LlmService::with_default() {
        Ok(service) => {
            debug!("LLM service initialized successfully");
            Arc::new(service)
        }
        Err(_) => {
            debug!("LLM service not configured (ANTHROPIC_API_KEY not set)");
            debug!("Tools requiring LLM will return errors until configured");
            debug!("Configure with: mnemosyne config set-key");

            // Create a dummy service - it will error on use
            // This allows the server to start for basic operations
            Arc::new(LlmService::new(LlmConfig {
                api_key: String::new(),
                model: "claude-3-5-haiku-20241022".to_string(),
                max_tokens: 1024,
                temperature: 0.7,
            })?)
        }
    };

    // Initialize embedding service (shares API key with LLM)
    let embeddings = {
        let config = LlmConfig::default();
        Arc::new(EmbeddingService::new(config.api_key.clone(), config))
    };

    // Initialize tool handler
    let tool_handler = ToolHandler::new(Arc::new(storage), llm, embeddings);

    // Create and run server
    let server = McpServer::new(tool_handler);

    // Run server with graceful shutdown on signals
    tokio::select! {
        result = server.run() => {
            result?;
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal, stopping MCP server gracefully...");
        }
    }

    info!("MCP server shut down complete");
    Ok(())
}

async fn start_mcp_server_with_api(
    db_path_arg: Option<String>,
    api_addr: String,
    api_capacity: usize,
) -> Result<()> {
    use mnemosyne_core::api::{ApiServer, ApiServerConfig};
    use std::net::SocketAddr;

    debug!("Starting MCP server with API monitoring...");

    // Initialize configuration
    let _config_manager = ConfigManager::new()?;

    // Initialize storage
    let db_path = get_db_path(db_path_arg);
    debug!("Using database: {}", db_path);

    if let Some(parent) = PathBuf::from(&db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let storage = LibsqlStorage::new_with_validation(ConnectionMode::Local(db_path), true).await?;

    // Initialize LLM service
    let llm = match LlmService::with_default() {
        Ok(service) => Arc::new(service),
        Err(_) => Arc::new(LlmService::new(LlmConfig {
            api_key: String::new(),
            model: "claude-3-5-haiku-20241022".to_string(),
            max_tokens: 1024,
            temperature: 0.7,
        })?),
    };

    // Initialize embedding service
    let embeddings = {
        let config = LlmConfig::default();
        Arc::new(EmbeddingService::new(config.api_key.clone(), config))
    };

    // Parse API server address
    let socket_addr: SocketAddr = api_addr
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid API address '{}': {}", api_addr, e))?;

    // Create API server
    let api_config = ApiServerConfig {
        addr: socket_addr,
        event_capacity: api_capacity,
    };
    let api_server = ApiServer::new(api_config);
    let event_broadcaster = api_server.broadcaster().clone();

    info!("API server will be available at http://{}", socket_addr);
    info!("Dashboard: mnemosyne-dash --api http://{}", socket_addr);

    // Initialize tool handler with event broadcasting
    let tool_handler = ToolHandler::new_with_events(
        Arc::new(storage),
        llm,
        embeddings,
        Some(event_broadcaster),
    );

    // Create MCP server
    let mcp_server = McpServer::new(tool_handler);

    // Run both servers concurrently with graceful shutdown on signals
    tokio::select! {
        result = mcp_server.run() => {
            result?;
        }
        result = api_server.serve() => {
            result?;
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal, stopping MCP server and API server gracefully...");
        }
    }

    info!("MCP server and API server shut down complete");
    Ok(())
}

/// Parse memory type from string with support for aliases
fn parse_memory_type(type_str: &str) -> mnemosyne_core::MemoryType {
    match type_str.to_lowercase().as_str() {
        // Canonical names and aliases
        "architecture_decision" | "architecture" | "decision" => mnemosyne_core::MemoryType::ArchitectureDecision,
        "code_pattern" | "pattern" => mnemosyne_core::MemoryType::CodePattern,
        "bug_fix" | "bug" | "bugfix" => mnemosyne_core::MemoryType::BugFix,
        "configuration" | "config" => mnemosyne_core::MemoryType::Configuration,
        "constraint" => mnemosyne_core::MemoryType::Constraint,
        "entity" => mnemosyne_core::MemoryType::Entity,
        "insight" => mnemosyne_core::MemoryType::Insight,
        "reference" | "ref" => mnemosyne_core::MemoryType::Reference,
        "preference" | "pref" => mnemosyne_core::MemoryType::Preference,
        "task" | "todo" => mnemosyne_core::MemoryType::Task,
        "agent_event" | "event" => mnemosyne_core::MemoryType::AgentEvent,
        // Specification workflow types
        "constitution" => mnemosyne_core::MemoryType::Constitution,
        "feature_spec" | "spec" | "feature" => mnemosyne_core::MemoryType::FeatureSpec,
        "implementation_plan" | "plan" | "impl_plan" => mnemosyne_core::MemoryType::ImplementationPlan,
        "task_breakdown" | "tasks" | "breakdown" => mnemosyne_core::MemoryType::TaskBreakdown,
        "quality_checklist" | "checklist" | "qa" => mnemosyne_core::MemoryType::QualityChecklist,
        "clarification" | "clarify" => mnemosyne_core::MemoryType::Clarification,
        // Default to Insight for unknown types
        _ => mnemosyne_core::MemoryType::Insight,
    }
}

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

    /// Set log level
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// Database path (overrides MNEMOSYNE_DB_PATH env var and default)
    #[arg(long)]
    db_path: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start MCP server (stdio mode)
    Serve {
        /// Also start HTTP API server for monitoring
        #[arg(long)]
        with_api: bool,

        /// API server address (when --with-api is enabled)
        #[arg(long, default_value = "127.0.0.1:3000")]
        api_addr: String,

        /// API event channel capacity
        #[arg(long, default_value = "1000")]
        api_capacity: usize,
    },

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

    /// Launch Integrated Context Studio (ICS)
    Ics {
        /// File to open in ICS
        file: Option<String>,
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
        action: ConfigAction,
    },

    /// Manage encrypted secrets
    Secrets {
        #[command(subcommand)]
        command: SecretsCommand,
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
        action: ModelsAction,
    },

    /// Run evolution jobs (importance recalibration, link decay, archival)
    Evolve {
        #[command(subcommand)]
        job: EvolveJob,
    },

    /// Manage specification workflow artifacts
    Artifact {
        #[command(subcommand)]
        command: ArtifactCommands,
    },
}

#[derive(Subcommand)]
enum ModelsAction {
    /// List available embedding models
    List,

    /// Show model cache information
    Info,

    /// Clear model cache
    Clear {
        /// Confirm deletion without prompting
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum EvolveJob {
    /// Run importance recalibration job
    Importance {
        /// Batch size (max memories to process)
        #[arg(short, long, default_value = "100")]
        batch_size: usize,

        /// Database path
        #[arg(short, long)]
        database: Option<String>,
    },

    /// Run link decay job
    Links {
        /// Batch size (max links to process)
        #[arg(short, long, default_value = "100")]
        batch_size: usize,

        /// Database path
        #[arg(short, long)]
        database: Option<String>,
    },

    /// Run archival job
    Archival {
        /// Batch size (max memories to archive)
        #[arg(short, long, default_value = "50")]
        batch_size: usize,

        /// Database path
        #[arg(short, long)]
        database: Option<String>,
    },

    /// Run consolidation job (detect and merge duplicates)
    Consolidation {
        /// Batch size (max memories to check)
        #[arg(short, long, default_value = "100")]
        batch_size: usize,

        /// Database path
        #[arg(short, long)]
        database: Option<String>,
    },

    /// Run all evolution jobs
    All {
        /// Batch size for each job
        #[arg(short, long, default_value = "100")]
        batch_size: usize,

        /// Database path
        #[arg(short, long)]
        database: Option<String>,
    },
}

#[derive(Subcommand)]
enum ArtifactCommands {
    /// Initialize artifact directory structure
    Init,

    /// List all artifacts
    List {
        /// Filter by artifact type (constitution|spec|plan|tasks|checklist|clarification)
        #[arg(short, long)]
        artifact_type: Option<String>,
    },

    /// Show artifact details
    Show {
        /// Artifact ID or file path
        artifact: String,
    },

    /// Validate artifact structure and frontmatter
    Validate {
        /// Artifact file path
        path: String,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Set Anthropic API key (stores securely in OS keychain)
    SetKey {
        /// API key (if not provided, will prompt interactively)
        key: Option<String>,
    },

    /// Show API key configuration status
    ShowKey,

    /// Delete stored API key
    DeleteKey,
}

#[derive(Subcommand)]
enum SecretsCommand {
    /// Initialize secrets (first-time setup)
    Init,

    /// Set a secret value
    Set {
        /// Secret name (e.g., ANTHROPIC_API_KEY)
        name: String,

        /// Value (optional, will prompt if not provided)
        #[arg(short, long)]
        value: Option<String>,
    },

    /// Get a secret value (for testing)
    Get {
        /// Secret name
        name: String,
    },

    /// List configured secrets (names only)
    List,

    /// Show where secrets are stored
    Info,
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
    let filter = EnvFilter::new(format!(
        "mnemosyne={},iroh=warn,iroh_net=warn,tokio::sync::broadcast=error,tokio_stream=error",
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
        Some(Commands::Serve {
            with_api,
            api_addr,
            api_capacity,
        }) => {
            if with_api {
                // Start both MCP server and API server
                start_mcp_server_with_api(cli.db_path, api_addr, api_capacity).await
            } else {
                // Start MCP server only
                start_mcp_server(cli.db_path).await
            }
        }
        Some(Commands::ApiServer { addr, capacity }) => {
            use mnemosyne_core::api::{ApiServer, ApiServerConfig};
            use std::net::SocketAddr;

            debug!("Starting HTTP API server...");

            let socket_addr: SocketAddr = addr
                .parse()
                .map_err(|e| anyhow::anyhow!("Invalid address '{}': {}", addr, e))?;
            let config = ApiServerConfig {
                addr: socket_addr,
                event_capacity: capacity,
            };

            println!();
            println!("🌐 Mnemosyne API Server");
            println!("   Real-time event streaming and state coordination");
            println!();
            println!("   Address: http://{}", socket_addr);
            println!("   Event capacity: {}", capacity);
            println!();
            println!("   Endpoints:");
            println!("   • GET  /events - Server-Sent Events stream");
            println!("   • GET  /state/agents - List active agents");
            println!("   • POST /state/agents - Update agent state");
            println!("   • GET  /state/context-files - List context files");
            println!("   • POST /state/context-files - Update context file");
            println!("   • GET  /state/stats - System statistics");
            println!("   • GET  /health - Health check");
            println!();
            println!("   Dashboard:");
            println!("   mnemosyne-dash --api http://{}", socket_addr);
            println!();

            let server = ApiServer::new(config);
            server.serve().await?;

            Ok(())
        }
        Some(Commands::Init { database }) => {
            debug!("Initializing database...");

            // Use provided database path or fall back to global/default
            let db_path = database
                .or_else(|| cli.db_path.clone())
                .unwrap_or_else(|| get_default_db_path().to_string_lossy().to_string());

            debug!("Database path: {}", db_path);

            // Create parent directory if it doesn't exist
            if let Some(parent) = PathBuf::from(&db_path).parent() {
                std::fs::create_dir_all(parent)?;
                debug!("Created directory: {}", parent.display());
            }

            // Initialize storage (this will create the database and run migrations)
            // Init command explicitly creates database if missing
            let _storage =
                LibsqlStorage::new_with_validation(ConnectionMode::Local(db_path.clone()), true)
                    .await?;

            println!("✓ Database initialized: {}", db_path);
            Ok(())
        }
        Some(Commands::Export { output, namespace }) => {
            if let Some(ref out_path) = output {
                debug!("Exporting memories to {}...", out_path);
            } else {
                debug!("Exporting memories to stdout...");
            }

            // Initialize storage (read-only)
            let db_path = get_db_path(cli.db_path.clone());
            let storage =
                LibsqlStorage::new_with_validation(ConnectionMode::Local(db_path), false).await?;

            // Parse namespace if provided
            let ns = namespace.map(|ns_str| {
                if ns_str.starts_with("project:") {
                    let project = ns_str.strip_prefix("project:").unwrap();
                    mnemosyne_core::Namespace::Project {
                        name: project.to_string(),
                    }
                } else if ns_str.starts_with("session:") {
                    let parts: Vec<&str> = ns_str
                        .strip_prefix("session:")
                        .unwrap()
                        .split(':')
                        .collect();
                    if parts.len() == 2 {
                        mnemosyne_core::Namespace::Session {
                            project: parts[0].to_string(),
                            session_id: parts[1].to_string(),
                        }
                    } else {
                        mnemosyne_core::Namespace::Global
                    }
                } else {
                    mnemosyne_core::Namespace::Global
                }
            });

            // Query all memories (or filtered by namespace)
            let memories = storage
                .list_memories(ns, 10000, MemorySortOrder::Recent)
                .await?;

            // Determine output format and destination
            let (format, use_stdout) = if let Some(ref path) = output {
                let fmt = if path.ends_with(".jsonl") {
                    "jsonl"
                } else if path.ends_with(".md") || path.ends_with(".markdown") {
                    "markdown"
                } else {
                    "json" // default
                };
                (fmt, false)
            } else {
                // Default to JSON for stdout
                ("json", true)
            };

            use std::io::Write;

            // Helper closure to write formatted output
            let write_output = |writer: &mut dyn Write| -> Result<()> {
                match format {
                    "json" => {
                        // Pretty-printed JSON
                        let json = serde_json::to_string_pretty(&memories)?;
                        writer.write_all(json.as_bytes())?;
                        writer.write_all(b"\n")?;
                    }
                    "jsonl" => {
                        // Newline-delimited JSON (one object per line)
                        for memory in &memories {
                            let json = serde_json::to_string(memory)?;
                            writeln!(writer, "{}", json)?;
                        }
                    }
                    "markdown" => {
                        // Human-readable Markdown
                        writeln!(writer, "# Memory Export\n")?;
                        writeln!(writer, "Exported {} memories\n", memories.len())?;
                        writeln!(writer, "---\n")?;

                        for (i, memory) in memories.iter().enumerate() {
                            writeln!(writer, "## {}. {}\n", i + 1, memory.summary)?;
                            writeln!(writer, "**ID**: {}", memory.id)?;
                            writeln!(
                                writer,
                                "**Namespace**: {}",
                                serde_json::to_string(&memory.namespace)?
                            )?;
                            writeln!(writer, "**Importance**: {}/10", memory.importance)?;
                            writeln!(writer, "**Type**: {:?}", memory.memory_type)?;
                            writeln!(
                                writer,
                                "**Created**: {}",
                                memory.created_at.format("%Y-%m-%d %H:%M:%S")
                            )?;
                            if !memory.tags.is_empty() {
                                writeln!(writer, "**Tags**: {}", memory.tags.join(", "))?;
                            }
                            if !memory.keywords.is_empty() {
                                writeln!(writer, "**Keywords**: {}", memory.keywords.join(", "))?;
                            }
                            writeln!(writer, "\n### Content\n")?;
                            writeln!(writer, "{}\n", memory.content)?;
                            writeln!(writer, "---\n")?;
                        }
                    }
                    _ => {
                        return Err(MnemosyneError::ValidationError(format!(
                            "Unsupported export format: {}",
                            format
                        ))
                        .into());
                    }
                }
                Ok(())
            };

            // Write to stdout or file
            if use_stdout {
                let stdout = std::io::stdout();
                let mut handle = stdout.lock();
                write_output(&mut handle)?;
            } else {
                use std::fs::File;
                let output_path = PathBuf::from(output.as_ref().unwrap());
                let mut file = File::create(&output_path)?;
                write_output(&mut file)?;
                eprintln!(
                    "✓ Exported {} memories to {}",
                    memories.len(),
                    output_path.display()
                );
            }

            Ok(())
        }
        Some(Commands::Status) => {
            // Print header
            println!("╭─────────────────────────────────────────╮");
            println!("│  Mnemosyne v{}                    │", env!("CARGO_PKG_VERSION"));
            println!("│  Project-Aware Agentic Memory          │");
            println!("╰─────────────────────────────────────────╯");
            println!();

            // Get database path
            let db_path = get_db_path(cli.db_path.clone());
            let db_path_obj = PathBuf::from(&db_path);

            // Check database status
            let db_exists = db_path_obj.exists();
            let db_size = if db_exists {
                std::fs::metadata(&db_path)
                    .ok()
                    .map(|m| {
                        let bytes = m.len();
                        if bytes < 1024 {
                            format!("{} B", bytes)
                        } else if bytes < 1024 * 1024 {
                            format!("{:.1} KB", bytes as f64 / 1024.0)
                        } else if bytes < 1024 * 1024 * 1024 {
                            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
                        } else {
                            format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
                        }
                    })
                    .unwrap_or_else(|| "unknown".to_string())
            } else {
                "N/A".to_string()
            };

            println!("📊 Database");
            println!("   Path:   {}", db_path);
            println!("   Status: {}", if db_exists { "✓ exists" } else { "✗ not initialized" });
            if db_exists {
                println!("   Size:   {}", db_size);

                // Try to count memories (only if database exists)
                match LibsqlStorage::new_with_validation(ConnectionMode::Local(db_path.clone()), false).await {
                    Ok(storage) => {
                        match storage.count_memories(None).await {
                            Ok(count) => {
                                println!("   Memories: {}", count);
                            }
                            Err(_) => {
                                println!("   Memories: Unable to query");
                            }
                        }
                    }
                    Err(e) => {
                        println!("   Health:  ✗ {}", e);
                    }
                }
            }
            println!();

            // Check API key
            println!("🔑 Configuration");
            let config = ConfigManager::new()?;
            match config.get_api_key() {
                Ok(_) => println!("   API Key: ✓ configured"),
                Err(_) => println!("   API Key: ✗ not configured (set with: mnemosyne config set-key)"),
            }

            // Check if env var is set
            if std::env::var("ANTHROPIC_API_KEY").is_ok() {
                println!("   Env Var: ✓ ANTHROPIC_API_KEY set");
            }
            println!();

            // System info
            println!("⚙️  System");
            println!("   Rust:    {}", rustc_version_runtime::version());
            println!("   OS:      {}", std::env::consts::OS);
            println!("   Arch:    {}", std::env::consts::ARCH);
            println!();

            if !db_exists {
                println!("💡 Next steps:");
                println!("   Initialize database: mnemosyne init");
                println!();
            }

            Ok(())
        }
        Some(Commands::Ics { file }) => {
            use mnemosyne_core::ics::{IcsApp, IcsConfig};

            debug!("Launching Integrated Context Studio (ICS)...");

            // Initialize storage backend
            let db_path = get_db_path(None);
            debug!("Using database: {}", db_path);

            // Ensure parent directory exists
            if let Some(parent) = PathBuf::from(&db_path).parent() {
                std::fs::create_dir_all(parent)?;
            }

            let storage =
                LibsqlStorage::new_with_validation(ConnectionMode::Local(db_path), true).await?;
            let storage_backend: Arc<dyn StorageBackend> = Arc::new(storage);

            // Create ICS app with storage (no agent registry or proposal queue in standalone mode)
            let config = IcsConfig::default();
            let mut app = IcsApp::new(config, storage_backend, None, None);

            // Load file if provided
            if let Some(file_path) = file {
                let path = std::path::PathBuf::from(file_path);
                app.load_file(path)?;
            }

            // Run the ICS application
            app.run().await?;

            Ok(())
        }
        Some(Commands::Tui { with_ics: _, no_dashboard: _ }) => {
            // TUI wrapper mode is deprecated due to TUI-in-TUI conflicts
            eprintln!();
            eprintln!("⚠️  DEPRECATED: 'mnemosyne tui' is no longer supported");
            eprintln!();
            eprintln!("   The PTY wrapper mode has been removed due to terminal conflicts");
            eprintln!("   when wrapping Claude Code's TUI interface.");
            eprintln!();
            eprintln!("   📚 New Architecture: Composable Tools");
            eprintln!();
            eprintln!("   Instead of wrapping Claude Code, Mnemosyne now provides");
            eprintln!("   standalone tools that work alongside it:");
            eprintln!();
            eprintln!("   1️⃣  Edit Context:");
            eprintln!("      mnemosyne-ics context.md");
            eprintln!("      (Full-featured context editor with semantic highlighting)");
            eprintln!();
            eprintln!("   2️⃣  Chat with Claude:");
            eprintln!("      claude");
            eprintln!("      (Memory integration happens automatically via MCP)");
            eprintln!();
            eprintln!("   3️⃣  Monitor Activity:");
            eprintln!("      mnemosyne dash");
            eprintln!("      (Real-time dashboard - coming soon)");
            eprintln!();
            eprintln!("   💡 Tip: Use tmux/screen to see all tools at once:");
            eprintln!("      tmux split-window -h 'mnemosyne-ics context.md'");
            eprintln!("      tmux split-window -v 'mnemosyne dash'");
            eprintln!("      claude");
            eprintln!();
            eprintln!("   📖 Migration Guide:");
            eprintln!("      https://github.com/rand/mnemosyne/blob/main/docs/MIGRATION.md");
            eprintln!();

            std::process::exit(1);
        }
        Some(Commands::Config { action }) => {
            let config_manager = ConfigManager::new()?;

            match action {
                ConfigAction::SetKey { key } => {
                    #[cfg(feature = "keyring-fallback")]
                    {
                        if let Some(key_value) = key {
                            // API key provided as argument
                            config_manager.set_api_key(&key_value)?;
                            println!("✓ API key securely saved to OS keychain");
                        } else {
                            // Interactive prompt
                            config_manager.prompt_and_set_api_key()?;
                        }
                    }
                    #[cfg(not(feature = "keyring-fallback"))]
                    {
                        eprintln!("Config set-key is deprecated. Use: mnemosyne secrets set ANTHROPIC_API_KEY");
                        if let Some(key_value) = key {
                            config_manager
                                .secrets()
                                .set_secret("ANTHROPIC_API_KEY", &key_value)?;
                        } else {
                            print!("Enter API key: ");
                            std::io::Write::flush(&mut std::io::stdout())?;
                            let mut input = String::new();
                            std::io::stdin().read_line(&mut input)?;
                            config_manager
                                .secrets()
                                .set_secret("ANTHROPIC_API_KEY", input.trim())?;
                        }
                    }
                    Ok(())
                }
                ConfigAction::ShowKey => {
                    if config_manager.has_api_key() {
                        match config_manager.get_api_key() {
                            Ok(key) => {
                                // Show only first and last 4 characters
                                let masked = if key.len() > 12 {
                                    format!("{}...{}", &key[..8], &key[key.len() - 4..])
                                } else {
                                    "***".to_string()
                                };
                                println!("✓ API key configured: {}", masked);

                                // Show source
                                if std::env::var("ANTHROPIC_API_KEY").is_ok() {
                                    println!("  Source: ANTHROPIC_API_KEY environment variable");
                                } else {
                                    println!("  Source: OS keychain");
                                }
                            }
                            Err(e) => {
                                eprintln!("✗ Error retrieving API key: {}", e);
                            }
                        }
                    } else {
                        println!("✗ No API key configured");
                        println!("\nTo set your API key:");
                        println!("  mnemosyne config set-key");
                        println!("or");
                        println!("  export ANTHROPIC_API_KEY=sk-ant-...");
                    }
                    Ok(())
                }
                ConfigAction::DeleteKey => {
                    #[cfg(feature = "keyring-fallback")]
                    {
                        config_manager.delete_api_key()?;
                        println!("✓ API key deleted from OS keychain");
                        println!("\nNote: If ANTHROPIC_API_KEY environment variable is set,");
                        println!("      it will still be used. Unset it with:");
                        println!("      unset ANTHROPIC_API_KEY");
                    }
                    #[cfg(not(feature = "keyring-fallback"))]
                    {
                        eprintln!("Config delete-key is deprecated. Secrets are managed in encrypted config.");
                        eprintln!(
                            "To reset, delete: {}",
                            config_manager.secrets().secrets_file().display()
                        );
                    }
                    Ok(())
                }
            }
        }
        Some(Commands::Secrets { command }) => {
            use mnemosyne_core::secrets::SecretsManager;
            use secrecy::ExposeSecret;

            let secrets = SecretsManager::new()?;

            match command {
                SecretsCommand::Init => {
                    if secrets.is_initialized() {
                        println!(
                            "⚠️  Secrets already initialized at: {}",
                            secrets.secrets_file().display()
                        );
                        print!("Reinitialize? This will overwrite existing secrets. [y/N]: ");
                        std::io::Write::flush(&mut std::io::stdout())?;

                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input)?;

                        if !input.trim().eq_ignore_ascii_case("y") {
                            println!("Cancelled.");
                            return Ok(());
                        }
                    }
                    secrets.initialize_interactive()?;
                    Ok(())
                }
                SecretsCommand::Set { name, value } => {
                    let val = if let Some(v) = value {
                        v
                    } else {
                        print!("Enter value for {}: ", name);
                        std::io::Write::flush(&mut std::io::stdout())?;
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input)?;
                        input.trim().to_string()
                    };
                    secrets.set_secret(&name, &val)?;
                    Ok(())
                }
                SecretsCommand::Get { name } => {
                    match secrets.get_secret(&name) {
                        Ok(secret) => {
                            println!("{}", secret.expose_secret());
                        }
                        Err(e) => {
                            eprintln!("✗ {}", e);
                        }
                    }
                    Ok(())
                }
                SecretsCommand::List => {
                    if !secrets.is_initialized() {
                        println!("No secrets configured. Run: mnemosyne secrets init");
                        return Ok(());
                    }

                    let names = secrets.list_secrets()?;
                    if names.is_empty() {
                        println!("No secrets configured. Run: mnemosyne secrets init");
                    } else {
                        println!("Configured secrets:");
                        for name in names {
                            // Check if available via environment variable
                            let source = if std::env::var(&name).is_ok() {
                                " (from environment)"
                            } else {
                                ""
                            };
                            println!("  - {}{}", name, source);
                        }
                    }
                    Ok(())
                }
                SecretsCommand::Info => {
                    println!("Secrets Configuration");
                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                    println!("Config dir:     {}", secrets.config_dir().display());
                    println!("Secrets file:   {}", secrets.secrets_file().display());
                    println!("Identity key:   {}", secrets.identity_file().display());
                    println!(
                        "Initialized:    {}",
                        if secrets.is_initialized() {
                            "yes"
                        } else {
                            "no"
                        }
                    );
                    println!();

                    if secrets.is_initialized() {
                        let names = secrets.list_secrets()?;
                        println!("Configured secrets: {}", names.len());
                        for name in names {
                            let available = secrets.get_secret(&name).is_ok();
                            let status = if available { "✓" } else { "✗" };
                            let source = if std::env::var(&name).is_ok() {
                                " (env)"
                            } else {
                                " (file)"
                            };
                            println!("  {} {}{}", status, name, source);
                        }
                    } else {
                        println!("Run 'mnemosyne secrets init' to set up encrypted secrets.");
                    }
                    Ok(())
                }
            }
        }
        Some(Commands::Orchestrate {
            plan,
            database,
            dashboard,
            polling_interval: _,
            max_concurrent,
        }) => {
            debug!("Launching multi-agent orchestration system...");

            let db_path = get_db_path(database);

            println!("🤖 Mnemosyne Multi-Agent Orchestration");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("Configuration:");
            println!("  Database: {}", db_path);
            println!("  Max concurrent agents: {}", max_concurrent);
            println!(
                "  Dashboard: {}",
                if dashboard {
                    "enabled (future)"
                } else {
                    "disabled"
                }
            );
            println!("  Work plan: {}", plan);
            println!();

            // Create launcher configuration
            let mut config = launcher::LauncherConfig::default();
            config.mnemosyne_db_path = Some(db_path.clone());
            config.max_concurrent_agents = max_concurrent as u8;

            // Parse plan as JSON or treat as prompt
            if let Ok(plan_json) = serde_json::from_str::<serde_json::Value>(&plan) {
                debug!("Parsed work plan as JSON");
                debug!("Plan: {:?}", plan_json);

                // Process structured work plan
                println!("📋 Structured work plan detected:");
                println!();
                process_structured_plan(&plan_json);
                println!();
            } else {
                debug!("Treating plan as plain text prompt");
                println!("📝 Prompt-based orchestration:");
                println!("   {}", plan);
                println!();
            }

            // Launch orchestrated session
            println!("🚀 Starting orchestration engine...");
            println!();

            launcher::launch_orchestrated_session(Some(db_path), Some(plan), None).await?;

            println!();
            println!("✨ Orchestration session complete");
            Ok(())
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
            // Initialize storage and services
            let db_path = get_db_path(cli.db_path.clone());
            // Remember command creates database if it doesn't exist (write implies initialize)
            let storage =
                LibsqlStorage::new_with_validation(ConnectionMode::Local(db_path.clone()), true)
                    .await?;

            // Check if API key is available for LLM enrichment
            let llm_config = LlmConfig::default();
            let has_api_key = !llm_config.api_key.is_empty();

            // Parse namespace
            let ns = if namespace.starts_with("project:") {
                let project = namespace.strip_prefix("project:").unwrap();
                mnemosyne_core::Namespace::Project {
                    name: project.to_string(),
                }
            } else if namespace.starts_with("session:") {
                let parts: Vec<&str> = namespace
                    .strip_prefix("session:")
                    .unwrap()
                    .split(':')
                    .collect();
                if parts.len() == 2 {
                    mnemosyne_core::Namespace::Session {
                        project: parts[0].to_string(),
                        session_id: parts[1].to_string(),
                    }
                } else {
                    mnemosyne_core::Namespace::Global
                }
            } else {
                mnemosyne_core::Namespace::Global
            };

            // Create or enrich memory
            let mut memory = if has_api_key {
                // Try to enrich memory with LLM, but fall back if it fails
                let llm = LlmService::new(llm_config.clone())?;
                let ctx = context.unwrap_or_else(|| "CLI input".to_string());

                match llm.enrich_memory(&content, &ctx).await {
                    Ok(enriched_memory) => {
                        debug!("Memory enriched successfully with LLM");
                        enriched_memory
                    }
                    Err(e) => {
                        // LLM enrichment failed - fall back to basic memory
                        // Log specific error type for better debugging
                        match &e {
                            mnemosyne_core::MnemosyneError::AuthenticationError(_) => {
                                warn!("LLM enrichment failed (invalid API key): {}, storing memory without enrichment", e);
                            }
                            mnemosyne_core::MnemosyneError::RateLimitExceeded(_) => {
                                warn!("LLM enrichment failed (rate limit): {}, storing memory without enrichment", e);
                            }
                            mnemosyne_core::MnemosyneError::NetworkError(_) => {
                                warn!("LLM enrichment failed (network error): {}, storing memory without enrichment", e);
                            }
                            _ => {
                                warn!(
                                    "LLM enrichment failed: {}, storing memory without enrichment",
                                    e
                                );
                            }
                        }

                        use mnemosyne_core::types::MemoryId;
                        use mnemosyne_core::MemoryNote;

                        let now = chrono::Utc::now();

                        MemoryNote {
                            id: MemoryId::new(),
                            namespace: ns.clone(),
                            created_at: now,
                            updated_at: now,
                            content: content.clone(),
                            summary: content.chars().take(100).collect::<String>(),
                            keywords: Vec::new(),
                            tags: Vec::new(),
                            context: ctx.clone(),
                            memory_type: memory_type
                                .as_deref()
                                .map(|t| parse_memory_type(t))
                                .unwrap_or(mnemosyne_core::MemoryType::Insight),
                            importance: importance.clamp(1, 10),
                            confidence: 0.5,
                            links: Vec::new(),
                            related_files: Vec::new(),
                            related_entities: Vec::new(),
                            access_count: 0,
                            last_accessed_at: now,
                            expires_at: None,
                            is_archived: false,
                            superseded_by: None,
                            embedding: None,
                            embedding_model: String::new(),
                        }
                    }
                }
            } else {
                // Create basic memory without LLM enrichment
                debug!("Creating basic memory without LLM enrichment - no API key");
                use mnemosyne_core::types::MemoryId;
                use mnemosyne_core::MemoryNote;

                let now = chrono::Utc::now();
                let ctx = context.unwrap_or_else(|| "CLI input".to_string());

                MemoryNote {
                    id: MemoryId::new(),
                    namespace: ns.clone(),
                    created_at: now,
                    updated_at: now,
                    content: content.clone(),
                    summary: content.chars().take(100).collect::<String>(),
                    keywords: Vec::new(),
                    tags: Vec::new(),
                    context: ctx,
                    memory_type: memory_type
                        .as_deref()
                        .map(|t| parse_memory_type(t))
                        .unwrap_or(mnemosyne_core::MemoryType::Insight),
                    importance: importance.clamp(1, 10),
                    confidence: 0.5,
                    links: Vec::new(),
                    related_files: Vec::new(),
                    related_entities: Vec::new(),
                    access_count: 0,
                    last_accessed_at: now,
                    expires_at: None,
                    is_archived: false,
                    superseded_by: None,
                    embedding: None,
                    embedding_model: String::new(),
                }
            };

            // Override with CLI parameters (in case LLM set different values)
            memory.namespace = ns;
            memory.importance = importance.clamp(1, 10);
            if let Some(ref type_str) = memory_type {
                memory.memory_type = parse_memory_type(type_str);
            }

            // Add custom tags if provided
            if let Some(tag_str) = tags {
                let custom_tags: Vec<String> = tag_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                memory.tags.extend(custom_tags);
            }

            // Generate embedding if API key available
            if has_api_key {
                let embedding_service =
                    EmbeddingService::new(llm_config.api_key.clone(), llm_config);
                match embedding_service.generate_embedding(&memory.content).await {
                    Ok(embedding) => memory.embedding = Some(embedding),
                    Err(_) => {
                        debug!("Failed to generate embedding, storing without it");
                    }
                }
            }

            // Store memory
            storage.store_memory(&memory).await?;

            // Output result
            if format == "json" {
                println!(
                    "{}",
                    serde_json::json!({
                        "id": memory.id.to_string(),
                        "summary": memory.summary,
                        "importance": memory.importance,
                        "tags": memory.tags,
                        "namespace": serde_json::to_string(&memory.namespace).unwrap_or_default()
                    })
                );
            } else {
                println!("✅ Memory saved");
                println!("ID: {}", memory.id);
                println!("Summary: {}", memory.summary);
                println!("Importance: {}/10", memory.importance);
                println!("Tags: {}", memory.tags.join(", "));
            }

            Ok(())
        }
        Some(Commands::Recall {
            query,
            namespace,
            limit,
            min_importance,
            format,
        }) => {
            // Initialize storage and services
            let db_path = get_db_path(cli.db_path.clone());
            let storage = LibsqlStorage::new(ConnectionMode::Local(db_path.clone())).await?;

            // Check if API key is available for vector search
            let embedding_service_config = LlmConfig::default();
            let has_api_key = !embedding_service_config.api_key.is_empty();

            // Parse namespace
            let ns = namespace.as_ref().map(|ns_str| {
                if ns_str.starts_with("project:") {
                    let project = ns_str.strip_prefix("project:").unwrap();
                    mnemosyne_core::Namespace::Project {
                        name: project.to_string(),
                    }
                } else if ns_str.starts_with("session:") {
                    let parts: Vec<&str> = ns_str
                        .strip_prefix("session:")
                        .unwrap()
                        .split(':')
                        .collect();
                    if parts.len() == 2 {
                        mnemosyne_core::Namespace::Session {
                            project: parts[0].to_string(),
                            session_id: parts[1].to_string(),
                        }
                    } else {
                        mnemosyne_core::Namespace::Global
                    }
                } else {
                    mnemosyne_core::Namespace::Global
                }
            });

            // Perform hybrid search (keyword + vector + graph)
            let keyword_results = storage
                .hybrid_search(&query, ns.clone(), limit * 2, true)
                .await?;

            // Vector search (optional - only if API key available)
            let vector_results = if has_api_key {
                let embedding_service = EmbeddingService::new(
                    embedding_service_config.api_key.clone(),
                    embedding_service_config.clone(),
                );
                match embedding_service.generate_embedding(&query).await {
                    Ok(query_embedding) => storage
                        .vector_search(&query_embedding, limit * 2, ns.clone())
                        .await
                        .unwrap_or_default(),
                    Err(_) => Vec::new(),
                }
            } else {
                debug!("Skipping vector search - no API key configured");
                Vec::new()
            };

            // Merge results
            let mut memory_scores = std::collections::HashMap::new();

            for result in keyword_results {
                memory_scores
                    .entry(result.memory.id)
                    .or_insert((result.memory.clone(), vec![]))
                    .1
                    .push(result.score * 0.4);
            }

            for (memory_id, similarity) in vector_results {
                // Fetch the memory for this ID
                if let Ok(memory) = storage.get_memory(memory_id).await {
                    memory_scores
                        .entry(memory_id)
                        .or_insert((memory, vec![]))
                        .1
                        .push(similarity * 0.3);
                }
            }

            let mut results: Vec<_> = memory_scores
                .into_iter()
                .map(|(_, (memory, scores))| {
                    let total_score: f32 = scores.iter().sum();
                    (memory, total_score)
                })
                .collect();

            results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            results.truncate(limit);

            // Filter by importance if specified
            if let Some(min_imp) = min_importance {
                results.retain(|(m, _)| m.importance >= min_imp);
            }

            // Output results
            if format == "json" {
                let json_results: Vec<_> = results
                    .iter()
                    .map(|(m, score)| {
                        serde_json::json!({
                            "id": m.id.to_string(),
                            "summary": m.summary,
                            "content": m.content,
                            "importance": m.importance,
                            "tags": m.tags,
                            "memory_type": format!("{:?}", m.memory_type),
                            "score": score,
                            "namespace": serde_json::to_string(&m.namespace).unwrap_or_default()
                        })
                    })
                    .collect();

                println!(
                    "{}",
                    serde_json::json!({
                        "results": json_results,
                        "count": json_results.len()
                    })
                );
            } else {
                if results.is_empty() {
                    println!("No memories found matching '{}'", query);
                } else {
                    println!("Found {} memories:\n", results.len());
                    for (i, (memory, score)) in results.iter().enumerate() {
                        println!(
                            "{}. {} (score: {:.2}, importance: {}/10)",
                            i + 1,
                            memory.summary,
                            score,
                            memory.importance
                        );
                        println!("   ID: {}", memory.id);
                        println!("   Tags: {}", memory.tags.join(", "));
                        println!(
                            "   Content: {}\n",
                            if memory.content.len() > 100 {
                                format!("{}...", &memory.content[..100])
                            } else {
                                memory.content.clone()
                            }
                        );
                    }
                }
            }

            Ok(())
        }
        Some(Commands::Embed {
            all,
            memory_id,
            namespace,
            batch_size,
            progress,
        }) => {
            use mnemosyne_core::{ConnectionMode, EmbeddingConfig, LocalEmbeddingService};
            use std::sync::Arc;

            // Initialize embedding service
            println!("Initializing local embedding service...");
            let embedding_config = EmbeddingConfig::default();
            let embedding_service = Arc::new(LocalEmbeddingService::new(embedding_config).await?);

            // Initialize storage
            let db_path = get_db_path(cli.db_path.clone());
            let mut storage = LibsqlStorage::new(ConnectionMode::Local(db_path.clone())).await?;

            // Set embedding service on storage
            storage.set_embedding_service(embedding_service.clone());

            // Determine which memories to embed
            let memories = if let Some(id_str) = memory_id {
                // Single memory
                use mnemosyne_core::MemoryId;
                use uuid::Uuid;
                let uuid = Uuid::parse_str(&id_str)
                    .map_err(|e| anyhow::anyhow!("Invalid memory ID: {}", e))?;
                let id = MemoryId(uuid);
                vec![storage.get_memory(id).await?]
            } else {
                // Fetch all memories using search with empty query
                let ns = if let Some(ns_str) = namespace {
                    println!("Fetching memories in namespace '{}'...", ns_str);
                    Some(if ns_str.starts_with("project:") {
                        let project = ns_str.strip_prefix("project:").unwrap();
                        mnemosyne_core::Namespace::Project {
                            name: project.to_string(),
                        }
                    } else if ns_str.starts_with("session:") {
                        let parts: Vec<&str> = ns_str
                            .strip_prefix("session:")
                            .unwrap()
                            .split(':')
                            .collect();
                        if parts.len() == 2 {
                            mnemosyne_core::Namespace::Session {
                                project: parts[0].to_string(),
                                session_id: parts[1].to_string(),
                            }
                        } else {
                            mnemosyne_core::Namespace::Global
                        }
                    } else {
                        mnemosyne_core::Namespace::Global
                    })
                } else if all {
                    println!("Fetching all memories...");
                    None
                } else {
                    eprintln!("Error: Must specify --all, --memory-id, or --namespace");
                    std::process::exit(1);
                };

                // Use hybrid_search with empty query to get all memories
                let results = storage.hybrid_search("", ns, 10000, false).await?;
                results.into_iter().map(|r| r.memory).collect()
            };

            let total = memories.len();
            println!("Generating embeddings for {} memories...", total);

            // Process memories in batches
            let mut processed = 0;
            let mut succeeded = 0;
            let mut failed = 0;

            for chunk in memories.chunks(batch_size) {
                for memory in chunk {
                    processed += 1;

                    if progress {
                        print!("\rProgress: {}/{} ", processed, total);
                        use std::io::Write;
                        std::io::stdout().flush().unwrap();
                    }

                    match storage
                        .generate_and_store_embedding(&memory.id, &memory.content)
                        .await
                    {
                        Ok(_) => succeeded += 1,
                        Err(e) => {
                            if progress {
                                eprintln!("\nFailed to embed memory {}: {}", memory.id, e);
                            }
                            failed += 1;
                        }
                    }
                }
            }

            if progress {
                println!();
            }

            println!("Embedding generation complete!");
            println!("  Total: {}", total);
            println!("  Succeeded: {}", succeeded);
            println!("  Failed: {}", failed);

            Ok(())
        }
        Some(Commands::Models { action }) => {
            use mnemosyne_core::EmbeddingConfig;

            let config = EmbeddingConfig::default();
            let cache_dir = &config.cache_dir;

            match action {
                ModelsAction::List => {
                    println!("Available embedding models:");
                    println!();
                    println!("  nomic-embed-text-v1.5  (768 dims, recommended)");
                    println!("  nomic-embed-text-v1    (768 dims)");
                    println!("  all-MiniLM-L6-v2       (384 dims)");
                    println!("  all-MiniLM-L12-v2      (384 dims)");
                    println!("  bge-small-en-v1.5      (384 dims)");
                    println!("  bge-base-en-v1.5       (768 dims)");
                    println!("  bge-large-en-v1.5      (1024 dims)");
                    println!();
                    println!("Set model in config or use EmbeddingConfig::default()");
                    Ok(())
                }
                ModelsAction::Info => {
                    println!("Model cache directory: {}", cache_dir.display());
                    println!();

                    if cache_dir.exists() {
                        if let Ok(entries) = std::fs::read_dir(cache_dir) {
                            let mut found_models = Vec::new();
                            for entry in entries.flatten() {
                                if entry.file_type().ok().map(|t| t.is_dir()).unwrap_or(false) {
                                    found_models.push(entry.file_name());
                                }
                            }

                            if found_models.is_empty() {
                                println!("No models cached yet.");
                                println!("Models will be downloaded automatically on first use.");
                            } else {
                                println!("Cached models:");
                                for model in found_models {
                                    println!("  - {}", model.to_string_lossy());
                                }

                                // Calculate total size
                                if let Ok(metadata) = std::fs::metadata(cache_dir) {
                                    println!();
                                    println!("Total cache size: {} bytes", metadata.len());
                                }
                            }
                        }
                    } else {
                        println!("Cache directory does not exist yet.");
                        println!("It will be created on first model download.");
                    }

                    Ok(())
                }
                ModelsAction::Clear { yes } => {
                    if !cache_dir.exists() {
                        println!("Cache directory does not exist.");
                        return Ok(());
                    }

                    let confirm = if yes {
                        true
                    } else {
                        use std::io::{self, Write};
                        print!("Clear model cache at {}? (y/N): ", cache_dir.display());
                        io::stdout().flush()?;

                        let mut response = String::new();
                        io::stdin().read_line(&mut response)?;
                        response.trim().to_lowercase() == "y"
                    };

                    if confirm {
                        std::fs::remove_dir_all(cache_dir)?;
                        println!("Model cache cleared successfully.");
                        println!("Models will be re-downloaded on next use.");
                    } else {
                        println!("Cancelled.");
                    }

                    Ok(())
                }
            }
        }
        Some(Commands::Evolve { job }) => {
            use anyhow::Context;
            use mnemosyne_core::evolution::{
                ArchivalJob, ConsolidationJob, EvolutionJob, ImportanceRecalibrator, JobConfig,
                LinkDecayJob,
            };
            use mnemosyne_core::{ConnectionMode, LibsqlStorage};
            use std::sync::Arc;
            use std::time::Duration;

            // Determine database path
            let db_path = match &job {
                EvolveJob::Importance { database, .. }
                | EvolveJob::Links { database, .. }
                | EvolveJob::Archival { database, .. }
                | EvolveJob::Consolidation { database, .. }
                | EvolveJob::All { database, .. } => database
                    .clone()
                    .or_else(|| cli.db_path.clone())
                    .unwrap_or_else(|| get_default_db_path().to_string_lossy().to_string()),
            };

            // Initialize storage
            let storage = Arc::new(
                LibsqlStorage::new(ConnectionMode::Local(db_path.into()))
                    .await
                    .context("Failed to initialize storage")?,
            );

            match job {
                EvolveJob::Importance { batch_size, .. } => {
                    println!("Running importance recalibration job...");
                    let job = ImportanceRecalibrator::new(storage.clone());
                    let config = JobConfig {
                        enabled: true,
                        interval: Duration::from_secs(0),
                        batch_size,
                        max_duration: Duration::from_secs(300), // 5 minutes
                    };

                    match job.run(&config).await {
                        Ok(report) => {
                            println!("✓ Importance recalibration complete:");
                            println!("  Memories processed: {}", report.memories_processed);
                            println!("  Changes made: {}", report.changes_made);
                            println!("  Errors: {}", report.errors);
                            println!("  Duration: {:?}", report.duration);
                            Ok(())
                        }
                        Err(e) => {
                            eprintln!("✗ Importance recalibration failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                EvolveJob::Links { batch_size, .. } => {
                    println!("Running link decay job...");
                    let job = LinkDecayJob::new(storage.clone());
                    let config = JobConfig {
                        enabled: true,
                        interval: Duration::from_secs(0),
                        batch_size,
                        max_duration: Duration::from_secs(300), // 5 minutes
                    };

                    match job.run(&config).await {
                        Ok(report) => {
                            println!("✓ Link decay complete:");
                            println!("  Links processed: {}", report.memories_processed);
                            println!("  Changes made: {}", report.changes_made);
                            println!("  Errors: {}", report.errors);
                            println!("  Duration: {:?}", report.duration);
                            Ok(())
                        }
                        Err(e) => {
                            eprintln!("✗ Link decay failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                EvolveJob::Archival { batch_size, .. } => {
                    println!("Running archival job...");
                    let job = ArchivalJob::new(storage.clone());
                    let config = JobConfig {
                        enabled: true,
                        interval: Duration::from_secs(0),
                        batch_size,
                        max_duration: Duration::from_secs(300), // 5 minutes
                    };

                    match job.run(&config).await {
                        Ok(report) => {
                            println!("✓ Archival complete:");
                            println!("  Memories processed: {}", report.memories_processed);
                            println!("  Changes made: {}", report.changes_made);
                            println!("  Errors: {}", report.errors);
                            println!("  Duration: {:?}", report.duration);
                            Ok(())
                        }
                        Err(e) => {
                            eprintln!("✗ Archival failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                EvolveJob::Consolidation { batch_size, .. } => {
                    println!("Running consolidation job...");
                    let job = ConsolidationJob::new(storage.clone());
                    let config = JobConfig {
                        enabled: true,
                        interval: Duration::from_secs(0),
                        batch_size,
                        max_duration: Duration::from_secs(300), // 5 minutes
                    };

                    match job.run(&config).await {
                        Ok(report) => {
                            println!("✓ Consolidation complete:");
                            println!("  Memories processed: {}", report.memories_processed);
                            println!("  Changes made: {}", report.changes_made);
                            println!("  Errors: {}", report.errors);
                            println!("  Duration: {:?}", report.duration);
                            Ok(())
                        }
                        Err(e) => {
                            eprintln!("✗ Consolidation failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                EvolveJob::All { batch_size, .. } => {
                    println!("Running all evolution jobs...");
                    println!();

                    let config = JobConfig {
                        enabled: true,
                        interval: Duration::from_secs(0),
                        batch_size,
                        max_duration: Duration::from_secs(300), // 5 minutes per job
                    };

                    // 1. Importance recalibration
                    println!("1/3: Importance recalibration...");
                    let importance_job = ImportanceRecalibrator::new(storage.clone());
                    match importance_job.run(&config).await {
                        Ok(report) => {
                            println!(
                                "  ✓ {} memories processed, {} updated",
                                report.memories_processed, report.changes_made
                            );
                        }
                        Err(e) => {
                            eprintln!("  ✗ Failed: {}", e);
                        }
                    }
                    println!();

                    // 2. Link decay
                    println!("2/3: Link decay...");
                    let link_job = LinkDecayJob::new(storage.clone());
                    match link_job.run(&config).await {
                        Ok(report) => {
                            println!(
                                "  ✓ {} links processed, {} updated",
                                report.memories_processed, report.changes_made
                            );
                        }
                        Err(e) => {
                            eprintln!("  ✗ Failed: {}", e);
                        }
                    }
                    println!();

                    // 3. Archival
                    println!("3/3: Archival...");
                    let archival_job = ArchivalJob::new(storage.clone());
                    match archival_job.run(&config).await {
                        Ok(report) => {
                            println!(
                                "  ✓ {} memories processed, {} archived",
                                report.memories_processed, report.changes_made
                            );
                        }
                        Err(e) => {
                            eprintln!("  ✗ Failed: {}", e);
                        }
                    }
                    println!();

                    println!("All evolution jobs complete!");
                    Ok(())
                }
            }
        }
        Some(Commands::Artifact { command }) => {
            use mnemosyne_core::artifacts::{
                Constitution, FeatureSpec, Artifact as ArtifactTrait,
                parse_frontmatter,
            };
            use std::fs;

            match command {
                ArtifactCommands::Init => {
                    println!("Initializing artifact directory structure...");

                    // Create artifact directories
                    let base = PathBuf::from(".mnemosyne/artifacts");
                    let subdirs = [
                        "constitution",
                        "specs",
                        "plans",
                        "tasks",
                        "checklists",
                        "clarifications",
                    ];

                    for subdir in &subdirs {
                        let path = base.join(subdir);
                        fs::create_dir_all(&path)?;
                        println!("  ✓ Created {}", path.display());
                    }

                    // Create README
                    let readme_path = base.join("README.md");
                    let readme_content = r#"# Mnemosyne Artifacts

This directory contains specification workflow artifacts for structured specification-driven development.

## Structure

- `constitution/` - Project constitution defining principles and quality gates
- `specs/` - Feature specifications with user scenarios
- `plans/` - Implementation plans with technical architecture
- `tasks/` - Task breakdowns with dependencies
- `checklists/` - Quality checklists for validation
- `clarifications/` - Clarifications resolving ambiguities

## Usage

Use slash commands in Claude Code:
- `/project-constitution` - Create/update constitution
- `/feature-specify <description>` - Create feature spec
- `/feature-plan <feature-id>` - Create implementation plan
- `/feature-tasks <feature-id>` - Create task breakdown
- `/feature-checklist <feature-id>` - Create quality checklist

Or use CLI:
```bash
mnemosyne artifact list
mnemosyne artifact show <artifact-id>
mnemosyne artifact validate <path>
```

For more information, see: docs/specs/specification-artifacts.md
"#;
                    fs::write(&readme_path, readme_content)?;
                    println!("  ✓ Created {}", readme_path.display());

                    println!();
                    println!("✓ Artifact structure initialized successfully!");
                    println!();
                    println!("Next steps:");
                    println!("  1. Create constitution: /project-constitution");
                    println!("  2. Create feature spec: /feature-specify <description>");
                    println!("  3. View artifacts: mnemosyne artifact list");
                    Ok(())
                }
                ArtifactCommands::List { artifact_type } => {
                    println!("Listing artifacts...");

                    let base = PathBuf::from(".mnemosyne/artifacts");
                    if !base.exists() {
                        eprintln!("✗ Artifact directory not found. Run 'mnemosyne artifact init' first.");
                        std::process::exit(1);
                    }

                    let search_dirs = if let Some(ref atype) = artifact_type {
                        // Map type to directory
                        let dir = match atype.as_str() {
                            "constitution" => "constitution",
                            "spec" | "feature_spec" => "specs",
                            "plan" | "implementation_plan" => "plans",
                            "tasks" | "task_breakdown" => "tasks",
                            "checklist" | "quality_checklist" => "checklists",
                            "clarification" => "clarifications",
                            _ => {
                                eprintln!("✗ Unknown artifact type: {}", atype);
                                eprintln!("Valid types: constitution, spec, plan, tasks, checklist, clarification");
                                std::process::exit(1);
                            }
                        };
                        vec![base.join(dir)]
                    } else {
                        // All directories
                        vec![
                            base.join("constitution"),
                            base.join("specs"),
                            base.join("plans"),
                            base.join("tasks"),
                            base.join("checklists"),
                            base.join("clarifications"),
                        ]
                    };

                    let mut found_any = false;
                    for dir in search_dirs {
                        if !dir.exists() {
                            continue;
                        }

                        let dir_name = dir.file_name().unwrap().to_string_lossy();
                        let entries: Vec<_> = fs::read_dir(&dir)?
                            .filter_map(|e| e.ok())
                            .filter(|e| {
                                e.path().extension().map_or(false, |ext| ext == "md")
                            })
                            .collect();

                        if !entries.is_empty() {
                            println!("\n{}:", dir_name);
                            found_any = true;

                            for entry in entries {
                                let path = entry.path();
                                let name = path.file_name().unwrap().to_string_lossy();

                                // Try to parse frontmatter to get metadata
                                if let Ok(content) = fs::read_to_string(&path) {
                                    if let Ok((frontmatter, _)) = parse_frontmatter(&content) {
                                        let version = frontmatter.get("version")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("unknown");
                                        let status = frontmatter.get("status")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("unknown");

                                        println!("  • {} (v{}, {})", name, version, status);
                                    } else {
                                        println!("  • {}", name);
                                    }
                                } else {
                                    println!("  • {}", name);
                                }
                            }
                        }
                    }

                    if !found_any {
                        println!("No artifacts found.");
                        println!("Create your first artifact with: /project-constitution");
                    }

                    Ok(())
                }
                ArtifactCommands::Show { artifact } => {
                    // Try to find artifact by ID or path
                    let path = if artifact.ends_with(".md") {
                        PathBuf::from(artifact)
                    } else {
                        // Search for artifact by ID in all directories
                        let base = PathBuf::from(".mnemosyne/artifacts");
                        let search_dirs = [
                            base.join("constitution"),
                            base.join("specs"),
                            base.join("plans"),
                            base.join("tasks"),
                            base.join("checklists"),
                            base.join("clarifications"),
                        ];

                        let mut found_path: Option<PathBuf> = None;
                        for dir in &search_dirs {
                            if !dir.exists() {
                                continue;
                            }

                            let artifact_file = format!("{}.md", artifact);
                            let candidate = dir.join(&artifact_file);
                            if candidate.exists() {
                                found_path = Some(candidate);
                                break;
                            }
                        }

                        found_path.unwrap_or_else(|| {
                            eprintln!("✗ Artifact not found: {}", artifact);
                            eprintln!("Try: mnemosyne artifact list");
                            std::process::exit(1);
                        })
                    };

                    if !path.exists() {
                        eprintln!("✗ File not found: {}", path.display());
                        std::process::exit(1);
                    }

                    let content = fs::read_to_string(&path)?;
                    println!("{}", content);
                    Ok(())
                }
                ArtifactCommands::Validate { path } => {
                    println!("Validating artifact: {}", path);

                    let path_buf = PathBuf::from(&path);
                    if !path_buf.exists() {
                        eprintln!("✗ File not found: {}", path);
                        std::process::exit(1);
                    }

                    let content = fs::read_to_string(&path_buf)?;

                    // Parse frontmatter
                    match parse_frontmatter(&content) {
                        Ok((frontmatter, markdown)) => {
                            println!("✓ Valid YAML frontmatter");

                            // Check required fields
                            let required_fields = ["type", "id", "name", "version"];
                            let mut missing_fields = Vec::new();

                            for field in &required_fields {
                                if frontmatter.get(*field).is_none() {
                                    missing_fields.push(*field);
                                }
                            }

                            if !missing_fields.is_empty() {
                                eprintln!("✗ Missing required fields: {}", missing_fields.join(", "));
                                std::process::exit(1);
                            }

                            println!("✓ All required fields present");

                            // Validate version format
                            if let Some(version) = frontmatter.get("version").and_then(|v| v.as_str()) {
                                if version.split('.').count() == 3 {
                                    println!("✓ Valid semantic version: {}", version);
                                } else {
                                    eprintln!("✗ Invalid version format: {} (expected X.Y.Z)", version);
                                    std::process::exit(1);
                                }
                            }

                            // Check markdown content
                            if markdown.trim().is_empty() {
                                eprintln!("✗ Empty content (no markdown after frontmatter)");
                                std::process::exit(1);
                            }

                            println!("✓ Non-empty content ({} chars)", markdown.len());

                            println!();
                            println!("✓ Artifact is valid!");
                        }
                        Err(e) => {
                            eprintln!("✗ Invalid artifact: {}", e);
                            std::process::exit(1);
                        }
                    }

                    Ok(())
                }
            }
        }
        None => {
            // Default: launch orchestrated Claude Code session
            debug!("Launching orchestrated Claude Code session...");

            // Get database path
            let db_path = get_db_path(cli.db_path);

            // Define agent names
            let agent_names = ["Orchestrator", "Optimizer", "Reviewer", "Executor"];

            // Show clean launch UI with banner
            launcher::ui::show_launch_header(
                env!("CARGO_PKG_VERSION"),
                &db_path,
                &agent_names,
            );

            // Show playful loading messages cycling
            let progress = launcher::ui::LaunchProgress::new();
            progress.cycle_loading_messages(4);

            // Launch orchestrated session
            let result = launcher::launch_orchestrated_session(Some(db_path), None, None).await;

            // Show completion or error
            if result.is_ok() {
                progress.show_step_complete("Orchestration ready");
                println!(); // Extra spacing after startup
            } else if let Err(ref e) = result {
                progress.show_error(&format!("{}", e));
            }

            result
        }
    }
}
