//! ICS (Integrated Context Studio) - Standalone Binary
//!
//! AI-assisted context engineering environment for Claude Code.
//!
//! Usage:
//!   mnemosyne-ics [OPTIONS] [FILE]
//!
//! Examples:
//!   mnemosyne-ics context.md              # Edit file
//!   mnemosyne-ics --template api context.md  # Use template
//!   mnemosyne-ics --readonly doc.md       # Read-only mode

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use mnemosyne_core::{
    ics::{IcsApp, IcsConfig},
    storage::StorageBackend,
    ConnectionMode, LibsqlStorage,
};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, Level};
use tracing_subscriber::EnvFilter;

/// ICS command-line arguments
#[derive(Parser)]
#[command(name = "mnemosyne-ics")]
#[command(about = "Integrated Context Studio - AI-assisted context engineering")]
#[command(version)]
struct Args {
    /// File to edit (creates if doesn't exist)
    file: Option<PathBuf>,

    /// Start in read-only mode
    #[arg(long)]
    readonly: bool,

    /// Use template (api, architecture, bugfix, feature, refactor)
    #[arg(long)]
    template: Option<Template>,

    /// Database path (overrides default)
    #[arg(long)]
    db_path: Option<PathBuf>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// Start with specific panel visible
    #[arg(long)]
    panel: Option<Panel>,
}

/// Available templates
#[derive(Debug, Clone, Copy, ValueEnum)]
enum Template {
    /// REST API design context
    Api,
    /// Architecture decision context
    Architecture,
    /// Bug fix context
    Bugfix,
    /// New feature context
    Feature,
    /// Refactoring context
    Refactor,
}

/// Panel options
#[derive(Debug, Clone, Copy, ValueEnum)]
enum Panel {
    /// Memory panel (Ctrl+M)
    Memory,
    /// Diagnostics panel (Ctrl+D)
    Diagnostics,
    /// Proposals panel (Ctrl+P)
    Proposals,
    /// Holes list (Ctrl+H)
    Holes,
}

impl Template {
    /// Get template content
    fn content(&self) -> &'static str {
        match self {
            Template::Api => {
                "# API Design Context\n\n\
                 ## Endpoint\n\
                 ?endpoint - Define the API endpoint\n\n\
                 ## Request/Response\n\
                 ?request_schema - Define request schema\n\
                 ?response_schema - Define response schema\n\n\
                 ## Implementation\n\
                 #api/routes.rs - Route definitions\n\
                 @handle_request - Request handler\n\n\
                 ## Testing\n\
                 ?test_cases - Define test scenarios\n"
            }
            Template::Architecture => {
                "# Architecture Decision\n\n\
                 ## Context\n\
                 Describe the architectural context and problem.\n\n\
                 ## Decision\n\
                 ?decision - What are we deciding?\n\n\
                 ## Consequences\n\
                 ?consequences - What are the implications?\n\n\
                 ## Alternatives\n\
                 ?alternatives - What other options were considered?\n"
            }
            Template::Bugfix => {
                "# Bug Fix Context\n\n\
                 ## Issue\n\
                 Describe the bug and reproduction steps.\n\n\
                 ## Root Cause\n\
                 ?root_cause - What caused the issue?\n\n\
                 ## Fix\n\
                 #src/module.rs:42 - Location of the fix\n\
                 @buggy_function - Function with the bug\n\n\
                 ## Testing\n\
                 ?test_coverage - How do we prevent regression?\n"
            }
            Template::Feature => {
                "# Feature Implementation\n\n\
                 ## Requirements\n\
                 ?requirements - What does this feature need to do?\n\n\
                 ## Design\n\
                 ?architecture - How will it be structured?\n\n\
                 ## Implementation\n\
                 ?components - What components are needed?\n\n\
                 ## Testing\n\
                 ?test_plan - How will we validate it works?\n"
            }
            Template::Refactor => {
                "# Refactoring Context\n\n\
                 ## Current State\n\
                 Describe what exists today.\n\n\
                 ## Target State\n\
                 ?target_design - What should it become?\n\n\
                 ## Migration Strategy\n\
                 ?migration_plan - How do we get there safely?\n\n\
                 ## Risk Mitigation\n\
                 ?risks - What could go wrong?\n"
            }
        }
    }
}

/// Get default database path
fn get_default_db_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("mnemosyne")
        .join("mnemosyne.db")
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let level = match args.log_level.as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    let filter = EnvFilter::new(format!(
        "mnemosyne={},iroh=warn,iroh_net=warn",
        level.as_str().to_lowercase()
    ));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr) // Logs to stderr, not stdout
        .init();

    debug!("ICS v{} starting...", env!("CARGO_PKG_VERSION"));

    // Get database path
    let db_path = args
        .db_path
        .unwrap_or_else(get_default_db_path)
        .to_string_lossy()
        .to_string();

    debug!("Using database: {}", db_path);

    // Ensure parent directory exists
    if let Some(parent) = PathBuf::from(&db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Initialize storage backend
    let storage = LibsqlStorage::new_with_validation(ConnectionMode::Local(db_path), true)
        .await
        .context("Failed to initialize storage backend")?;

    let storage_backend: Arc<dyn StorageBackend> = Arc::new(storage);

    // Create ICS config with readonly setting
    let mut config = IcsConfig::default();
    config.read_only = args.readonly;

    if args.readonly {
        debug!("Read-only mode enabled");
    }

    // Create ICS app (no agent registry or proposal queue in standalone mode)
    let mut app = IcsApp::new(config.clone(), storage_backend, None, None);

    // Load file if provided
    if let Some(file_path) = args.file {
        if file_path.exists() {
            debug!("Loading file: {}", file_path.display());
            app.load_file(file_path.clone())
                .context("Failed to load file")?;
        } else if let Some(template) = args.template {
            // Create new file from template
            debug!(
                "Creating new file with template: {:?}",
                template
            );

            // Use embedded template content
            let content = template.content().to_string();

            // Write template content to file
            std::fs::write(&file_path, &content)
                .context("Failed to create file from template")?;

            app.load_file(file_path.clone())
                .context("Failed to load template file")?;
        } else {
            // Create empty file
            debug!("Creating new empty file: {}", file_path.display());
            std::fs::write(
                &file_path,
                "# Context\n\nEdit your context here...\n",
            )
            .context("Failed to create empty file")?;

            app.load_file(file_path.clone())
                .context("Failed to load new file")?;
        }
    }

    // Open specific panel if requested
    if let Some(panel) = args.panel {
        let panel_type = match panel {
            Panel::Memory => {
                debug!("Opening memory panel");
                mnemosyne_core::ics::PanelType::Memory
            }
            Panel::Diagnostics => {
                debug!("Opening diagnostics panel");
                mnemosyne_core::ics::PanelType::Diagnostics
            }
            Panel::Proposals => {
                debug!("Opening proposals panel");
                mnemosyne_core::ics::PanelType::Proposals
            }
            Panel::Holes => {
                debug!("Opening holes list");
                mnemosyne_core::ics::PanelType::Holes
            }
        };
        app.show_panel(panel_type);
    }

    // Show launch banner
    println!();
    println!("{} ICS - Integrated Context Studio", mnemosyne_core::icons::system::palette());
    println!("   AI-assisted context engineering for Claude Code");
    println!();
    println!("   Shortcuts:");
    println!("   • Ctrl+Q: Quit");
    println!("   • Ctrl+S: Save");
    println!("   • Ctrl+M: Memory panel");
    println!("   • Ctrl+N: Next typed hole");
    println!("   • Ctrl+H: Holes list");
    println!("   • ?: Help");
    println!();

    // Small delay so user can see the banner
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Run the application
    app.run().await.context("ICS runtime error")?;

    debug!("ICS exiting cleanly");
    Ok(())
}
