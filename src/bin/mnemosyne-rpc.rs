//! mnemosyne-rpc - gRPC Server Binary
//!
//! Standalone gRPC server for remote access to mnemosyne's memory system.
//!
//! Usage:
//!   mnemosyne-rpc [OPTIONS]
//!
//! Examples:
//!   mnemosyne-rpc                           # Start on default port 50051
//!   mnemosyne-rpc --port 8080               # Custom port
//!   mnemosyne-rpc --host 0.0.0.0 --port 9090  # Listen on all interfaces

#![cfg(feature = "rpc")]

use anyhow::{Context, Result};
use clap::Parser;
use mnemosyne_core::{
    rpc::RpcServer,
    services::{LlmConfig, LlmService},
    storage::{libsql::LibsqlStorage, StorageBackend},
};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

/// mnemosyne-rpc command-line arguments
#[derive(Parser)]
#[command(name = "mnemosyne-rpc")]
#[command(about = "gRPC server for mnemosyne memory system")]
#[command(version)]
struct Args {
    /// Host to bind to
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Port to listen on
    #[arg(short, long, default_value = "50051")]
    port: u16,

    /// Database path (overrides default)
    #[arg(long)]
    db_path: Option<PathBuf>,

    /// Enable LLM enrichment
    #[arg(long)]
    enable_llm: bool,

    /// Anthropic API key (for LLM enrichment)
    #[arg(long, env = "ANTHROPIC_API_KEY")]
    anthropic_api_key: Option<String>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = args.log_level.parse::<Level>().unwrap_or(Level::INFO);

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(log_level.into()))
        .init();

    info!("Starting mnemosyne RPC server");

    // Initialize storage backend
    let db_path = args.db_path.unwrap_or_else(|| {
        let mut path = dirs::data_local_dir().expect("Failed to get data directory");
        path.push("mnemosyne");
        path.push("mnemosyne.db");
        path
    });

    info!("Using local database: {}", db_path.display());

    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).context("Failed to create database directory")?;
    }

    // Convert PathBuf to &str for LibsqlStorage
    let db_path_str = db_path
        .to_str()
        .context("Database path contains invalid UTF-8")?;

    let storage: Arc<dyn StorageBackend> = Arc::new(
        LibsqlStorage::new_local(db_path_str)
            .await
            .context("Failed to initialize local database")?,
    );

    // Initialize LLM service if enabled
    let llm: Option<Arc<LlmService>> = if args.enable_llm {
        if let Some(api_key) = args.anthropic_api_key {
            info!("LLM enrichment enabled");
            let llm_config = LlmConfig {
                api_key,
                model: "claude-haiku-4-5-20251001".to_string(),
                max_tokens: 1024,
                temperature: 0.7,
            };
            Some(Arc::new(
                LlmService::new(llm_config).context("Failed to initialize LLM service")?,
            ))
        } else {
            info!("LLM enrichment requested but no API key provided");
            None
        }
    } else {
        None
    };

    // Create and start RPC server
    let server = RpcServer::new(storage, llm);
    let addr = format!("{}:{}", args.host, args.port);

    info!("RPC server listening on {}", addr);

    server.serve(addr).await.context("RPC server failed")?;

    Ok(())
}
