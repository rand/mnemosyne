//! Mnemosyne - Project-Aware Agentic Memory System for Claude Code
//!
//! This is the main entry point for the Mnemosyne MCP server, which provides
//! persistent semantic memory capabilities to Claude Code's multi-agent system.

use clap::{Parser, Subcommand};
use mnemosyne_core::{
    error::Result, ConfigManager, LlmConfig, LlmService, McpServer, SqliteStorage, ToolHandler,
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
            let storage = SqliteStorage::new(db_path).await?;

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

            // Initialize tool handler
            let tool_handler = ToolHandler::new(Arc::new(storage), llm);

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
                    if let Some(key_value) = key {
                        // API key provided as argument
                        config_manager.set_api_key(&key_value)?;
                        println!("✓ API key securely saved to OS keychain");
                    } else {
                        // Interactive prompt
                        config_manager.prompt_and_set_api_key()?;
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
                    config_manager.delete_api_key()?;
                    println!("✓ API key deleted from OS keychain");
                    println!("\nNote: If ANTHROPIC_API_KEY environment variable is set,");
                    println!("      it will still be used. Unset it with:");
                    println!("      unset ANTHROPIC_API_KEY");
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
        None => {
            // Default: start MCP server
            info!("Starting MCP server (default)...");

            // Initialize configuration
            let _config_manager = ConfigManager::new()?;

            // Initialize storage
            let db_path = "mnemosyne.db";
            let storage = SqliteStorage::new(db_path).await?;

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

            // Initialize tool handler
            let tool_handler = ToolHandler::new(Arc::new(storage), llm);

            // Create and run server
            let server = McpServer::new(tool_handler);
            server.run().await?;

            Ok(())
        }
    }
}
