//! Mnemosyne - Project-Aware Agentic Memory System for Claude Code
//!
//! This is the main entry point for the Mnemosyne MCP server, which provides
//! persistent semantic memory capabilities to Claude Code's multi-agent system.

mod cli;

use clap::{Parser, Subcommand};
use mnemosyne_core::{
    error::{MnemosyneError, Result},
    icons,
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

// Import helper functions from cli module
use cli::helpers::{
    get_db_path, get_default_db_path, parse_memory_type, process_structured_plan,
    start_mcp_server, start_mcp_server_with_api,
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
        command: ArtifactCommands,
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
}


#[derive(Subcommand)]
enum ArtifactCommands {
    /// Initialize artifact directory structure
    Init,

    /// Create a new project constitution
    CreateConstitution {
        /// Project name
        #[arg(short, long)]
        project: String,

        /// Core principles (can be specified multiple times)
        #[arg(short = 'P', long)]
        principle: Vec<String>,

        /// Quality gates (can be specified multiple times)
        #[arg(short = 'q', long)]
        quality_gate: Vec<String>,

        /// Constraints (can be specified multiple times)
        #[arg(short = 'c', long)]
        constraint: Vec<String>,

        /// Namespace for memory entry (default: project:PROJECT_NAME)
        #[arg(short, long)]
        namespace: Option<String>,
    },

    /// Create a new feature specification
    CreateFeatureSpec {
        /// Feature ID (kebab-case, e.g., "user-auth-jwt")
        #[arg(short, long)]
        id: String,

        /// Feature name (e.g., "User Authentication")
        #[arg(short, long)]
        name: String,

        /// Parent feature ID (for sub-features)
        #[arg(short, long)]
        parent: Option<String>,

        /// Functional requirements (can be specified multiple times)
        #[arg(short = 'r', long)]
        requirement: Vec<String>,

        /// Success criteria (can be specified multiple times)
        #[arg(short = 's', long)]
        success_criterion: Vec<String>,

        /// Constitution memory ID to link to
        #[arg(short = 'C', long)]
        constitution_id: Option<String>,

        /// Namespace for memory entry (default: project:CURRENT_PROJECT)
        #[arg(short = 'N', long)]
        namespace: Option<String>,
    },

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
        "mnemosyne={},iroh=warn,iroh_net=warn,iroh::net::magicsock=error,tokio::sync::broadcast=error,tokio_stream=error",
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
            use mnemosyne_core::artifacts::{
                Constitution, FeatureSpec, Artifact as ArtifactTrait,
                ArtifactWorkflow, parse_frontmatter,
            };
            use mnemosyne_core::types::Namespace;
            use std::fs;

            match command {
                ArtifactCommands::CreateConstitution {
                    project,
                    principle,
                    quality_gate,
                    constraint,
                    namespace,
                } => {
                    println!("Creating project constitution for '{}'...", project);

                    // Ensure artifact directory exists
                    let artifacts_dir = PathBuf::from(".mnemosyne/artifacts");
                    if !artifacts_dir.exists() {
                        eprintln!("✗ Artifact directory not found. Run 'mnemosyne artifact init' first.");
                        std::process::exit(1);
                    }

                    // Validate that at least one principle is provided
                    if principle.is_empty() {
                        eprintln!("✗ At least one principle is required (use --principle)");
                        std::process::exit(1);
                    }

                    // Initialize storage and workflow
                    let db_path = get_db_path(cli.db_path.clone());
                    let storage = Arc::new(
                        LibsqlStorage::new_with_validation(ConnectionMode::Local(db_path), true).await?
                    );
                    let workflow = ArtifactWorkflow::new(artifacts_dir.clone(), storage)?;

                    // Build constitution
                    let mut builder = Constitution::builder(project.clone());
                    for p in principle {
                        builder = builder.principle(p);
                    }
                    for gate in quality_gate {
                        builder = builder.quality_gate(gate);
                    }
                    for c in constraint {
                        builder = builder.constraint(c);
                    }
                    let mut constitution = builder.build();

                    // Determine namespace
                    let ns = if let Some(ns_str) = namespace {
                        // Parse namespace string (e.g., "project:myapp")
                        if ns_str.starts_with("project:") {
                            let name = ns_str.strip_prefix("project:").unwrap().to_string();
                            Namespace::Project { name }
                        } else if ns_str == "global" {
                            Namespace::Global
                        } else {
                            eprintln!("✗ Invalid namespace format. Use 'global' or 'project:NAME'");
                            std::process::exit(1);
                        }
                    } else {
                        // Default to project namespace
                        Namespace::Project { name: project.clone() }
                    };

                    // Save constitution
                    let memory_id = workflow.save_constitution(&mut constitution, ns).await?;

                    println!("{} Constitution saved!", icons::status::success());
                    println!("   Memory ID: {}", memory_id);
                    println!("   File: {}", constitution.file_path().display());
                    println!();
                    println!("Next steps:");
                    println!("  - View: mnemosyne artifact show constitution");
                    println!("  - Edit: $EDITOR .mnemosyne/artifacts/{}", constitution.file_path().display());
                    println!("  - Create feature spec: mnemosyne artifact create-feature-spec ...");

                    Ok(())
                }
                ArtifactCommands::CreateFeatureSpec {
                    id,
                    name,
                    parent,
                    requirement,
                    success_criterion,
                    constitution_id,
                    namespace,
                } => {
                    println!("Creating feature specification '{}'...", name);

                    // Ensure artifact directory exists
                    let artifacts_dir = PathBuf::from(".mnemosyne/artifacts");
                    if !artifacts_dir.exists() {
                        eprintln!("✗ Artifact directory not found. Run 'mnemosyne artifact init' first.");
                        std::process::exit(1);
                    }

                    // Initialize storage and workflow
                    let db_path = get_db_path(cli.db_path.clone());
                    let storage = Arc::new(
                        LibsqlStorage::new_with_validation(ConnectionMode::Local(db_path), true).await?
                    );
                    let workflow = ArtifactWorkflow::new(artifacts_dir.clone(), storage)?;

                    // Build feature spec
                    let mut builder = FeatureSpec::builder(id.clone(), name.clone());
                    if let Some(p) = parent {
                        builder = builder.parent_feature(p);
                    }
                    for req in requirement {
                        builder = builder.requirement(req);
                    }
                    for criterion in success_criterion {
                        builder = builder.success_criterion(criterion);
                    }
                    let mut spec = builder.build();

                    // Determine namespace
                    let ns = if let Some(ns_str) = namespace {
                        // Parse namespace string
                        if ns_str.starts_with("project:") {
                            let proj_name = ns_str.strip_prefix("project:").unwrap().to_string();
                            Namespace::Project { name: proj_name }
                        } else if ns_str == "global" {
                            Namespace::Global
                        } else {
                            eprintln!("✗ Invalid namespace format. Use 'global' or 'project:NAME'");
                            std::process::exit(1);
                        }
                    } else {
                        // Try to infer project name from git root or use "default"
                        let project_name = std::env::current_dir()
                            .ok()
                            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                            .unwrap_or_else(|| "default".to_string());
                        Namespace::Project { name: project_name }
                    };

                    // Save feature spec
                    let memory_id = workflow.save_feature_spec(&mut spec, ns, constitution_id).await?;

                    println!("{} Feature spec saved!", icons::status::success());
                    println!("   Memory ID: {}", memory_id);
                    println!("   File: {}", spec.file_path().display());
                    if let Some(ref const_id) = spec.metadata().references.first() {
                        println!("   Linked to constitution: {}", const_id);
                    }
                    println!();
                    println!("Next steps:");
                    println!("  - View: mnemosyne artifact show {}", id);
                    println!("  - Edit: $EDITOR .mnemosyne/artifacts/{}", spec.file_path().display());
                    println!("  - List all specs: mnemosyne artifact list --artifact-type spec");

                    Ok(())
                }
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
        Some(Commands::Doctor { verbose, fix, json }) => {
            use mnemosyne_core::health::{run_health_checks, print_health_summary};

            debug!("Running health checks...");

            // Get database path
            let db_path = get_db_path(cli.db_path);

            // Create storage instance
            let storage = LibsqlStorage::from_path(&db_path).await?;

            // Run health checks
            let summary = run_health_checks(&storage, verbose, fix).await?;

            // Output results
            if json {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            } else {
                print_health_summary(&summary, verbose);
            }

            // Exit with appropriate code
            match summary.status {
                mnemosyne_core::health::CheckStatus::Pass => std::process::exit(0),
                mnemosyne_core::health::CheckStatus::Warn => std::process::exit(1),
                mnemosyne_core::health::CheckStatus::Fail => std::process::exit(2),
            }
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
