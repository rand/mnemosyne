//! Integrated Context Studio (ICS) command

use clap::ValueEnum;
use mnemosyne_core::{
    error::Result, icons, ics::{IcsApp, IcsConfig, PanelType}, ConnectionMode, LibsqlStorage,
    StorageBackend,
};
use std::{path::PathBuf, sync::Arc};
use tracing::debug;

use super::helpers::get_db_path;

/// ICS template options
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum IcsTemplate {
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

impl IcsTemplate {
    /// Get template content
    pub fn content(&self) -> &'static str {
        match self {
            IcsTemplate::Api => {
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
            IcsTemplate::Architecture => {
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
            IcsTemplate::Bugfix => {
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
            IcsTemplate::Feature => {
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
            IcsTemplate::Refactor => {
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

/// ICS panel options
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum IcsPanel {
    /// Memory panel (Ctrl+M)
    Memory,
    /// Diagnostics panel (Ctrl+D)
    Diagnostics,
    /// Proposals panel (Ctrl+P)
    Proposals,
    /// Holes list (Ctrl+H)
    Holes,
}

/// Handle ICS edit command
pub async fn handle(
    file: Option<PathBuf>,
    readonly: bool,
    template: Option<IcsTemplate>,
    panel: Option<IcsPanel>,
    session_context: Option<PathBuf>,
    global_db_path: Option<String>,
) -> Result<()> {
    debug!("Launching Integrated Context Studio (ICS)...");

    // Initialize storage backend
    let db_path = get_db_path(global_db_path);
    debug!("Using database: {}", db_path);

    // Ensure parent directory exists
    if let Some(parent) = PathBuf::from(&db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let storage =
        LibsqlStorage::new_with_validation(ConnectionMode::Local(db_path), true).await?;
    let storage_backend: Arc<dyn StorageBackend> = Arc::new(storage);

    // Create ICS config with readonly setting
    let config = IcsConfig {
        read_only: readonly,
        ..Default::default()
    };

    if readonly {
        debug!("Read-only mode enabled");
    }

    // Create ICS app with storage (no agent registry or proposal queue in standalone mode)
    let mut app = IcsApp::new(config.clone(), storage_backend, None, None);

    // Load file if provided
    if let Some(file_path) = file {
        if file_path.exists() {
            debug!("Loading file: {}", file_path.display());
            app.load_file(file_path.clone())?;
        } else if let Some(tmpl) = template {
            // Create new file from template
            debug!("Creating new file with template: {:?}", tmpl);

            // Use embedded template content
            let content = tmpl.content().to_string();

            // Write template content to file
            std::fs::write(&file_path, &content)?;

            app.load_file(file_path.clone())?;
        } else {
            // Create empty file
            debug!("Creating new empty file: {}", file_path.display());
            std::fs::write(
                &file_path,
                "# Context\n\nEdit your context here...\n",
            )?;

            app.load_file(file_path.clone())?;
        }
    }

    // Open specific panel if requested
    if let Some(panel_opt) = panel {
        let panel_type = match panel_opt {
            IcsPanel::Memory => {
                debug!("Opening memory panel");
                PanelType::Memory
            }
            IcsPanel::Diagnostics => {
                debug!("Opening diagnostics panel");
                PanelType::Diagnostics
            }
            IcsPanel::Proposals => {
                debug!("Opening proposals panel");
                PanelType::Proposals
            }
            IcsPanel::Holes => {
                debug!("Opening holes list");
                PanelType::Holes
            }
        };
        app.show_panel(panel_type);
    }

    // Show launch banner
    println!();
    println!("{} ICS - Integrated Context Studio", icons::system::palette());
    println!("   AI-assisted context engineering for Claude Code");
    println!();
    println!("   Shortcuts:");
    println!("   - Ctrl+Q: Quit");
    println!("   - Ctrl+S: Save");
    println!("   - Ctrl+M: Memory panel");
    println!("   - Ctrl+N: Next typed hole");
    println!("   - Ctrl+H: Holes list");
    println!("   - ?: Help");
    println!();

    // Small delay so user can see the banner
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // If session_context is provided, write handoff intent
    if let Some(session_path) = session_context {
        debug!("Session handoff enabled: {:?}", session_path);
        // TODO: Implement handoff coordination
        // This will be done in Phase 2 when we create the coordination module
    }

    // Run the ICS application
    app.run().await?;

    debug!("ICS exiting cleanly");

    // TODO: If session_context provided, write result
    // This will be done in Phase 2

    Ok(())
}
