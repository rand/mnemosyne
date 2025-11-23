//! Shared helper functions for CLI commands
//!
//! This module contains utility functions used across multiple CLI commands,
//! including database path resolution, MCP server startup, and JSON parsing.

use mnemosyne_core::{
    error::Result, mcp::EventSink, services::embeddings::EmbeddingService, ConfigManager,
    ConnectionMode, LibsqlStorage, LlmConfig, LlmService, McpServer, ToolHandler,
};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Get the default database path using XDG_DATA_HOME standard
pub fn get_default_db_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("mnemosyne")
        .join("mnemosyne.db")
}

/// Get the database path from CLI arg, env var, project dir, or default
pub fn get_db_path(cli_path: Option<String>) -> String {
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
pub fn process_structured_plan(plan: &serde_json::Value) {
    // Try to extract tasks from various common JSON structures
    let tasks = extract_tasks_from_plan(plan);

    if tasks.is_empty() {
        println!("  9  No tasks found in plan structure");
        println!("  Expected JSON with 'tasks', 'phases', or 'steps' field");
        return;
    }

    println!("  Found {} task(s):", tasks.len());
    println!();

    for (i, task) in tasks.iter().enumerate() {
        println!("  {}. {}", i + 1, task);
    }

    println!();
    println!("  9  Structured execution not yet fully implemented");
    println!("  Falling back to prompt-based orchestration");
}

/// Extract tasks from various JSON plan formats
pub fn extract_tasks_from_plan(plan: &serde_json::Value) -> Vec<String> {
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
pub fn extract_task_description(task: &serde_json::Value) -> Option<String> {
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

/// Detect if an API server is already running on ports 3000-3010
pub async fn detect_api_server() -> Option<String> {
    for port in 3000..=3010 {
        let url = format!("http://127.0.0.1:{}/health", port);
        match reqwest::get(&url).await {
            Ok(response) if response.status().is_success() => {
                debug!("Found API server at port {}", port);
                return Some(format!("http://127.0.0.1:{}", port));
            }
            _ => continue,
        }
    }
    debug!("No API server found on ports 3000-3010");
    None
}

/// Start MCP server in stdio mode
pub async fn start_mcp_server(db_path_arg: Option<String>) -> Result<()> {
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
                model: "claude-haiku-4-5-20251001".to_string(),
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

    // Try to start API server for dashboard connectivity
    use mnemosyne_core::api::{ApiServer, ApiServerConfig};
    use mnemosyne_core::mcp::tools::EventSink;
    use std::net::SocketAddr;

    let socket_addr: SocketAddr = "127.0.0.1:3000".parse().expect("Valid socket address");
    let api_config = ApiServerConfig {
        addr: socket_addr,
        event_capacity: 1000,
    };

    // Try to bind port 3000 (owner mode) or connect to existing server (client mode)
    let (event_sink, api_server_task) = match tokio::net::TcpListener::bind(socket_addr).await {
        Ok(listener) => {
            // Owner mode: We successfully bound port 3000, start API server
            drop(listener); // Release the listener, ApiServer will rebind

            let api_server = ApiServer::new(api_config);
            let event_broadcaster = api_server.broadcaster().clone();

            info!("API server starting on port 3000 (owner mode)");
            info!("Dashboard: mnemosyne-dash --api http://127.0.0.1:3000");

            let api_task = tokio::spawn(async move {
                if let Err(e) = api_server.serve().await {
                    warn!("API server error: {}", e);
                }
            });

            (EventSink::Local(event_broadcaster), Some(api_task))
        }
        Err(_) => {
            // Port taken, try to connect to existing API server (client mode)
            if let Some(api_url) = detect_api_server().await {
                info!(
                    "Connecting to existing API server at {} (client mode)",
                    api_url
                );
                let client = reqwest::Client::new();
                (EventSink::Remote { client, api_url }, None)
            } else {
                warn!("Port 3000 in use but no API server found - events will not be broadcast");
                warn!("Dashboard may not show activity from this MCP server");
                (EventSink::None, None)
            }
        }
    };

    // Initialize tool handler with event sink
    let tool_handler =
        ToolHandler::new_with_event_sink(Arc::new(storage), llm, embeddings, event_sink);

    // Create and run MCP server
    let mcp_server = McpServer::new(tool_handler);

    // Run MCP server (and API server if owner mode) with graceful shutdown
    if let Some(api_task) = api_server_task {
        // Owner mode: Run both MCP and API server
        tokio::select! {
            result = mcp_server.run() => {
                result?;
            }
            result = api_task => {
                if let Err(e) = result {
                    warn!("API server task failed: {}", e);
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Received shutdown signal, stopping servers gracefully...");
            }
        }
    } else {
        // Client mode: Run only MCP server
        tokio::select! {
            result = mcp_server.run() => {
                result?;
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Received shutdown signal, stopping MCP server gracefully...");
            }
        }
    }

    info!("MCP server shut down complete");
    Ok(())
}

#[allow(dead_code)]
pub async fn start_mcp_server_with_api(
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
            model: "claude-haiku-4-5-20251001".to_string(),
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
    let event_sink = EventSink::Local(event_broadcaster);
    let tool_handler =
        ToolHandler::new_with_event_sink(Arc::new(storage), llm, embeddings, event_sink);

    // Create MCP server
    let mcp_server = McpServer::new(tool_handler);

    // Spawn API server in background (non-critical)
    let _api_handle = tokio::spawn(async move {
        if let Err(e) = api_server.serve().await {
            warn!("{}", e);
        }
    });

    // Run MCP server (critical) with graceful shutdown on signals
    tokio::select! {
        result = mcp_server.run() => {
            result?;
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal, stopping MCP server gracefully...");
        }
    }

    info!("Shutdown complete");
    Ok(())
}

/// Parse memory type from string with support for aliases
pub fn parse_memory_type(type_str: &str) -> mnemosyne_core::MemoryType {
    match type_str.to_lowercase().as_str() {
        // Canonical names and aliases
        "architecture_decision" | "architecture" | "decision" => {
            mnemosyne_core::MemoryType::ArchitectureDecision
        }
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
        "implementation_plan" | "plan" | "impl_plan" => {
            mnemosyne_core::MemoryType::ImplementationPlan
        }
        "task_breakdown" | "tasks" | "breakdown" => mnemosyne_core::MemoryType::TaskBreakdown,
        "quality_checklist" | "checklist" | "qa" => mnemosyne_core::MemoryType::QualityChecklist,
        "clarification" | "clarify" => mnemosyne_core::MemoryType::Clarification,
        // Default to Insight for unknown types
        _ => mnemosyne_core::MemoryType::Insight,
    }
}
