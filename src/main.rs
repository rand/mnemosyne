//! Mnemosyne - Project-Aware Agentic Memory System for Claude Code
//!
//! This is the main entry point for the Mnemosyne MCP server, which provides
//! persistent semantic memory capabilities to Claude Code's multi-agent system.

use clap::{Parser, Subcommand};
use mnemosyne::error::Result;
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
        .init();

    info!("Mnemosyne v{} starting...", env!("CARGO_PKG_VERSION"));

    match cli.command {
        Some(Commands::Serve) => {
            info!("Starting MCP server...");
            // TODO: Start MCP server
            eprintln!("MCP server not yet implemented");
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
        None => {
            // Default: start MCP server
            info!("Starting MCP server (default)...");
            eprintln!("MCP server not yet implemented");
            Ok(())
        }
    }
}
