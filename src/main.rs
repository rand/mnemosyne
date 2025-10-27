//! Mnemosyne - Project-Aware Agentic Memory System for Claude Code
//!
//! This is the main entry point for the Mnemosyne MCP server, which provides
//! persistent semantic memory capabilities to Claude Code's multi-agent system.

use clap::{Parser, Subcommand};
use mnemosyne_core::{
    error::Result, ConfigManager, ConnectionMode, EmbeddingService, LibsqlStorage, LlmConfig,
    LlmService, McpServer, StorageBackend, ToolHandler,
};
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber;

#[derive(Parser)]
#[command(name = "mnemosyne")]
#[command(about = "Project-aware agentic memory system for Claude Code", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Set log level
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Start MCP server (stdio mode)
    Serve,

    /// Initialize database
    Init {
        /// Database path
        #[arg(short, long)]
        database: Option<String>,
    },

    /// Export memories to Markdown
    Export {
        /// Output path
        #[arg(short, long)]
        output: String,

        /// Namespace filter
        #[arg(short, long)]
        namespace: Option<String>,
    },

    /// Show system status
    Status,

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

    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .with_writer(std::io::stderr) // Write logs to stderr, not stdout
        .init();

    info!("Mnemosyne v{} starting...", env!("CARGO_PKG_VERSION"));

    match cli.command {
        Some(Commands::Serve) => {
            info!("Starting MCP server...");

            // Initialize configuration
            let _config_manager = ConfigManager::new()?;

            // Initialize storage
            // TODO: Make database path configurable
            let db_path = "mnemosyne.db";
            let storage =
                LibsqlStorage::new(ConnectionMode::Local(db_path.to_string())).await?;

            // Initialize LLM service (will error on first use if no API key)
            let llm = match LlmService::with_default() {
                Ok(service) => {
                    info!("LLM service initialized successfully");
                    Arc::new(service)
                }
                Err(_) => {
                    info!("LLM service not configured (ANTHROPIC_API_KEY not set)");
                    info!("Tools requiring LLM will return errors until configured");
                    info!("Configure with: mnemosyne config set-key");

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
            server.run().await?;

            Ok(())
        }
        Some(Commands::Init { database: _ }) => {
            info!("Initializing database...");
            eprintln!("Database initialization not yet implemented");
            Ok(())
        }
        Some(Commands::Export { output, namespace: _ }) => {
            info!("Exporting memories to {}...", output);
            eprintln!("Export not yet implemented");
            Ok(())
        }
        Some(Commands::Status) => {
            println!("Mnemosyne v{}", env!("CARGO_PKG_VERSION"));
            println!("Status: Operational (Phase 1 - Core Types)");
            Ok(())
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
                            config_manager.secrets().set_secret("ANTHROPIC_API_KEY", &key_value)?;
                        } else {
                            print!("Enter API key: ");
                            std::io::Write::flush(&mut std::io::stdout())?;
                            let mut input = String::new();
                            std::io::stdin().read_line(&mut input)?;
                            config_manager.secrets().set_secret("ANTHROPIC_API_KEY", input.trim())?;
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
                                    format!("{}...{}", &key[..8], &key[key.len()-4..])
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
                        eprintln!("To reset, delete: {}", config_manager.secrets().secrets_file().display());
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
                        println!("⚠️  Secrets already initialized at: {}", secrets.secrets_file().display());
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
                    println!("Initialized:    {}", if secrets.is_initialized() { "yes" } else { "no" });
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
            polling_interval,
            max_concurrent,
        }) => {
            info!("Launching multi-agent orchestration system...");

            // Build Python command
            let db_path = database.unwrap_or_else(|| "mnemosyne.db".to_string());

            // Create Python script invocation
            let python_script = format!(
                r#"
import asyncio
import sys
import json
from pathlib import Path

# Add src directory to Python path
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

try:
    from orchestration import create_engine
except ImportError as e:
    print(f"Error: PyO3 bindings not available: {{e}}", file=sys.stderr)
    print("Install with: maturin develop --features python", file=sys.stderr)
    sys.exit(1)

async def main():
    # Parse work plan
    plan_str = {}
    try:
        # Try to parse as JSON
        work_plan = json.loads(plan_str)
    except json.JSONDecodeError:
        # Treat as plain prompt
        work_plan = {{"prompt": plan_str}}

    # Create engine
    engine = await create_engine(db_path={})

    # Execute work plan
    result = await engine.execute_work_plan(work_plan)

    # Print results
    print(json.dumps(result, indent=2))

    # Cleanup
    await engine.stop()

if __name__ == "__main__":
    asyncio.run(main())
"#,
                serde_json::to_string(&plan).unwrap(),
                serde_json::to_string(&db_path).unwrap()
            );

            // Write script to temp file
            let script_path = std::env::temp_dir().join("mnemosyne_orchestrate.py");
            std::fs::write(&script_path, python_script)?;

            println!("Configuration:");
            println!("  Database: {}", db_path);
            println!("  Polling interval: {}ms", polling_interval);
            println!("  Max concurrent agents: {}", max_concurrent);
            println!("  Dashboard: {}", if dashboard { "enabled" } else { "disabled" });
            println!();

            // Execute Python script
            let output = std::process::Command::new("python3")
                .arg(&script_path)
                .output()?;

            // Clean up temp file
            let _ = std::fs::remove_file(&script_path);

            if output.status.success() {
                println!("{}", String::from_utf8_lossy(&output.stdout));
                Ok(())
            } else {
                eprintln!("Orchestration failed:");
                eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                std::process::exit(1);
            }
        }
        Some(Commands::Remember {
            content,
            namespace,
            importance,
            context,
            tags,
            format,
        }) => {
            // Initialize storage and services
            let db_path = "mnemosyne.db";
            let storage =
                LibsqlStorage::new(ConnectionMode::Local(db_path.to_string())).await?;

            let llm = LlmService::with_default()?;
            let embedding_service = {
                let config = LlmConfig::default();
                EmbeddingService::new(config.api_key.clone(), config)
            };

            // Parse namespace
            let ns = if namespace.starts_with("project:") {
                let project = namespace.strip_prefix("project:").unwrap();
                mnemosyne_core::Namespace::Project { name: project.to_string() }
            } else if namespace.starts_with("session:") {
                let parts: Vec<&str> = namespace.strip_prefix("session:").unwrap().split('/').collect();
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

            // Enrich memory with LLM
            let ctx = context.unwrap_or_else(|| "CLI input".to_string());
            let mut memory = llm.enrich_memory(&content, &ctx).await?;

            // Override with CLI parameters
            memory.namespace = ns;
            memory.importance = importance.clamp(1, 10);

            // Add custom tags if provided
            if let Some(tag_str) = tags {
                let custom_tags: Vec<String> = tag_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                memory.tags.extend(custom_tags);
            }

            // Generate embedding
            let embedding = embedding_service.generate_embedding(&memory.content).await?;
            memory.embedding = Some(embedding);

            // Store memory
            storage.store_memory(&memory).await?;

            // Output result
            if format == "json" {
                println!("{}", serde_json::json!({
                    "id": memory.id.to_string(),
                    "summary": memory.summary,
                    "importance": memory.importance,
                    "tags": memory.tags,
                    "namespace": serde_json::to_string(&memory.namespace).unwrap_or_default()
                }));
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
            let db_path = "mnemosyne.db";
            let storage =
                LibsqlStorage::new(ConnectionMode::Local(db_path.to_string())).await?;

            let embedding_service = {
                let config = LlmConfig::default();
                EmbeddingService::new(config.api_key.clone(), config)
            };

            // Parse namespace
            let ns = namespace.as_ref().map(|ns_str| {
                if ns_str.starts_with("project:") {
                    let project = ns_str.strip_prefix("project:").unwrap();
                    mnemosyne_core::Namespace::Project { name: project.to_string() }
                } else if ns_str.starts_with("session:") {
                    let parts: Vec<&str> = ns_str.strip_prefix("session:").unwrap().split('/').collect();
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
            let keyword_results = storage.hybrid_search(&query, ns.clone(), limit * 2, true).await?;

            // Vector search (optional - gracefully handle failures)
            let vector_results = match embedding_service.generate_embedding(&query).await {
                Ok(query_embedding) => {
                    storage.vector_search(&query_embedding, limit * 2, ns.clone())
                        .await
                        .unwrap_or_default()
                }
                Err(_) => Vec::new(),
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

            for result in vector_results {
                memory_scores
                    .entry(result.memory.id)
                    .or_insert((result.memory.clone(), vec![]))
                    .1
                    .push(result.score * 0.3);
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

                println!("{}", serde_json::json!({
                    "results": json_results,
                    "count": json_results.len()
                }));
            } else {
                if results.is_empty() {
                    println!("No memories found matching '{}'", query);
                } else {
                    println!("Found {} memories:\n", results.len());
                    for (i, (memory, score)) in results.iter().enumerate() {
                        println!("{}. {} (score: {:.2}, importance: {}/10)",
                            i + 1, memory.summary, score, memory.importance);
                        println!("   ID: {}", memory.id);
                        println!("   Tags: {}", memory.tags.join(", "));
                        println!("   Content: {}\n",
                            if memory.content.len() > 100 {
                                format!("{}...", &memory.content[..100])
                            } else {
                                memory.content.clone()
                            });
                    }
                }
            }

            Ok(())
        }
        None => {
            // Default: start MCP server
            info!("Starting MCP server (default)...");

            // Initialize configuration
            let _config_manager = ConfigManager::new()?;

            // Initialize storage
            let db_path = "mnemosyne.db";
            let storage =
                LibsqlStorage::new(ConnectionMode::Local(db_path.to_string())).await?;

            // Initialize LLM service (will error on first use if no API key)
            let llm = match LlmService::with_default() {
                Ok(service) => {
                    info!("LLM service initialized successfully");
                    Arc::new(service)
                }
                Err(_) => {
                    info!("LLM service not configured (ANTHROPIC_API_KEY not set)");
                    info!("Tools requiring LLM will return errors until configured");
                    info!("Configure with: mnemosyne config set-key");

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
            server.run().await?;

            Ok(())
        }
    }
}
