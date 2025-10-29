//! Orchestrated Claude Code Session Launcher
//!
//! This module provides functionality to launch Claude Code sessions with
//! full multi-agent orchestration and Mnemosyne integration.
//!
//! # Features
//! - Auto-detect Claude Code binary
//! - Generate agent configurations (Orchestrator, Optimizer, Reviewer, Executor)
//! - Configure MCP server integration
//! - Load project context at session start
//! - Support for sub-agent spawning
//!
//! # Usage
//! ```no_run
//! use mnemosyne_core::launcher;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     launcher::launch_orchestrated_session(None, None).await?;
//!     Ok(())
//! }
//! ```

pub mod agents;
pub mod context;
pub mod mcp;

use crate::error::{MnemosyneError, Result};
use std::path::PathBuf;
use std::process::Command;
use tracing::{debug, info, warn};

/// Configuration for launching Claude Code sessions
#[derive(Debug, Clone)]
pub struct LauncherConfig {
    /// Primary agent role for this session (default: Executor)
    pub primary_agent_role: agents::AgentRole,

    /// Enable sub-agent spawning (default: true)
    pub enable_subagents: bool,

    /// Maximum concurrent agents (default: 4)
    pub max_concurrent_agents: u8,

    /// Mnemosyne namespace (auto-detect from git if None)
    pub mnemosyne_namespace: Option<String>,

    /// Database path (use default if None)
    pub mnemosyne_db_path: Option<String>,

    /// Load context at session start (default: true)
    pub load_context_on_start: bool,

    /// Context loading configuration
    pub context_config: context::ContextLoadConfig,

    /// Permission mode for Claude Code (default: "default")
    pub permission_mode: String,

    /// Model to use (default: "sonnet")
    pub model: String,

    /// Enable session hooks (default: true)
    pub enable_hooks: bool,

    /// Initial prompt to send to Claude Code (optional)
    pub initial_prompt: Option<String>,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            primary_agent_role: agents::AgentRole::Executor,
            enable_subagents: true,
            max_concurrent_agents: 4,
            mnemosyne_namespace: None,
            mnemosyne_db_path: None,
            load_context_on_start: true,
            context_config: context::ContextLoadConfig::default(),
            permission_mode: "default".to_string(),
            model: "sonnet".to_string(),
            enable_hooks: true,
            initial_prompt: None,
        }
    }
}

/// Main Claude Code launcher
pub struct ClaudeCodeLauncher {
    config: LauncherConfig,
    claude_binary: PathBuf,
}

impl ClaudeCodeLauncher {
    /// Create a new launcher with default configuration
    pub fn new() -> Result<Self> {
        Self::with_config(LauncherConfig::default())
    }

    /// Create a new launcher with custom configuration
    pub fn with_config(config: LauncherConfig) -> Result<Self> {
        let claude_binary = detect_claude_binary()?;

        Ok(Self {
            config,
            claude_binary,
        })
    }

    /// Launch an orchestrated Claude Code session
    pub async fn launch(&self) -> Result<()> {
        info!("Launching orchestrated Claude Code session");
        debug!("Configuration: {:?}", self.config);

        // STEP 1: Initialize storage backend FIRST (eager initialization)
        let db_path = self.config.mnemosyne_db_path.clone()
            .unwrap_or_else(|| get_default_db_path());

        let storage = match crate::storage::libsql::LibsqlStorage::new(
            crate::storage::libsql::ConnectionMode::Local(db_path.clone())
        ).await {
            Ok(s) => {
                info!("Storage initialized: {}", db_path);
                std::sync::Arc::new(s)
            }
            Err(e) => {
                warn!("Could not initialize storage for context loading: {}", e);
                warn!("Launching without startup context");
                return self.launch_without_context().await;
            }
        };

        // STEP 1.5: Initialize OrchestrationEngine
        let orchestration_config = crate::orchestration::SupervisionConfig {
            max_restarts: 3,
            restart_window_secs: 60,
            enable_subagents: self.config.enable_subagents,
            max_concurrent_agents: self.config.max_concurrent_agents as usize,
        };

        let mut orchestration_engine = match crate::orchestration::OrchestrationEngine::new(
            storage.clone(),
            orchestration_config
        ).await {
            Ok(mut engine) => {
                // Start the engine to spawn all 4 agents
                if let Err(e) = engine.start().await {
                    warn!("Could not start orchestration engine: {}", e);
                    warn!("Continuing without orchestration");
                    None
                } else {
                    info!("Orchestration engine started with all 4 agents");
                    Some(engine)
                }
            }
            Err(e) => {
                warn!("Could not initialize orchestration engine: {}", e);
                warn!("Continuing without orchestration");
                None
            }
        };

        // STEP 2: Generate startup context with timeout protection
        let startup_prompt = if self.config.load_context_on_start {
            match tokio::time::timeout(
                std::time::Duration::from_millis(500),
                self.generate_startup_context_with_storage(storage.clone())
            ).await {
                Ok(Ok(context)) => {
                    info!("Loaded startup context ({} bytes)", context.len());
                    debug!("Context preview: {}...", &context.chars().take(100).collect::<String>());
                    context
                }
                Ok(Err(e)) => {
                    warn!("Context loading failed: {}", e);
                    String::new()
                }
                Err(_) => {
                    warn!("Context loading timed out (>500ms)");
                    String::new()
                }
            }
        } else {
            debug!("Context loading disabled by configuration");
            String::new()
        };

        // STEP 3: Generate agent and MCP configurations
        let agent_config = self.generate_agent_config()?;
        let mcp_config = self.generate_mcp_config()?;

        // STEP 4: Build command arguments
        let args = self.build_command_args(&agent_config, &mcp_config, &startup_prompt);

        debug!("Launching Claude Code with {} bytes of startup context", startup_prompt.len());

        // STEP 5: Execute Claude Code with orchestration engine running
        let status = Command::new(&self.claude_binary)
            .args(&args)
            .status()
            .map_err(|e| MnemosyneError::Other(format!("Failed to launch Claude Code: {}", e)))?;

        // STEP 6: Graceful shutdown of orchestration engine
        if let Some(mut engine) = orchestration_engine {
            info!("Shutting down orchestration engine");
            if let Err(e) = engine.stop().await {
                warn!("Error during orchestration shutdown: {}", e);
            }
        }

        if !status.success() {
            return Err(MnemosyneError::Other(format!(
                "Claude Code exited with status: {:?}",
                status.code()
            )));
        }

        Ok(())
    }

    /// Launch without context (fallback for storage errors)
    async fn launch_without_context(&self) -> Result<()> {
        info!("Launching without startup context");

        let agent_config = self.generate_agent_config()?;
        let mcp_config = self.generate_mcp_config()?;
        let args = self.build_command_args(&agent_config, &mcp_config, &String::new());

        let status = Command::new(&self.claude_binary)
            .args(&args)
            .status()
            .map_err(|e| MnemosyneError::Other(format!("Failed to launch Claude Code: {}", e)))?;

        if !status.success() {
            return Err(MnemosyneError::Other(format!(
                "Claude Code exited with status: {:?}",
                status.code()
            )));
        }

        Ok(())
    }

    /// Generate agent configuration JSON
    fn generate_agent_config(&self) -> Result<String> {
        let agents = agents::AgentDefinition::default_orchestration_agents();
        agents::AgentDefinition::agents_to_json(&agents)
    }

    /// Generate MCP configuration JSON
    fn generate_mcp_config(&self) -> Result<String> {
        let generator = mcp::McpConfigGenerator {
            mnemosyne_binary_path: get_mnemosyne_binary_path()?,
            namespace: self.config.mnemosyne_namespace.clone()
                .unwrap_or_else(|| detect_namespace()),
            db_path: self.config.mnemosyne_db_path.clone()
                .unwrap_or_else(|| get_default_db_path()),
            agent_role: self.config.primary_agent_role.clone(),
        };

        generator.generate_config()
    }

    /// Generate startup context prompt with storage backend
    async fn generate_startup_context_with_storage(
        &self,
        storage: std::sync::Arc<dyn crate::storage::StorageBackend>,
    ) -> Result<String> {
        let namespace = self.config.mnemosyne_namespace.clone()
            .unwrap_or_else(|| detect_namespace());

        let loader = context::ContextLoader::new(storage);

        loader.generate_startup_prompt(
            &namespace,
            &self.config.context_config
        ).await
    }

    /// Build command-line arguments for Claude Code
    fn build_command_args(&self, agent_config: &str, mcp_config: &str, startup_prompt: &str) -> Vec<String> {
        let mut args = vec![
            "--agents".to_string(),
            agent_config.to_string(),
            "--mcp-config".to_string(),
            mcp_config.to_string(),
            "--permission-mode".to_string(),
            self.config.permission_mode.clone(),
            "--model".to_string(),
            self.config.model.clone(),
        ];

        if !startup_prompt.is_empty() {
            args.push("--append-system-prompt".to_string());
            args.push(startup_prompt.to_string());
        }

        // Add initial prompt if provided
        if let Some(ref prompt) = self.config.initial_prompt {
            args.push("--prompt".to_string());
            args.push(prompt.clone());
        }

        args
    }
}

/// Detect Claude Code binary location
pub fn detect_claude_binary() -> Result<PathBuf> {
    // Try common locations
    let paths = vec![
        "claude",                          // In PATH
        "/usr/local/bin/claude",           // Common install location
        "/opt/homebrew/bin/claude",        // Homebrew on Apple Silicon
        "/home/linuxbrew/.linuxbrew/bin/claude", // Homebrew on Linux
    ];

    for path in paths {
        if let Ok(output) = Command::new(path).arg("--version").output() {
            if output.status.success() {
                debug!("Found Claude Code at: {}", path);
                return Ok(PathBuf::from(path));
            }
        }
    }

    // Try `which claude`
    if let Ok(output) = Command::new("which").arg("claude").output() {
        if output.status.success() {
            if let Ok(path_str) = String::from_utf8(output.stdout) {
                let path = path_str.trim();
                if !path.is_empty() {
                    debug!("Found Claude Code via 'which': {}", path);
                    return Ok(PathBuf::from(path));
                }
            }
        }
    }

    Err(MnemosyneError::Other(
        "Claude Code binary not found. Please ensure Claude Code is installed and in your PATH.".to_string()
    ))
}

/// Get mnemosyne binary path
fn get_mnemosyne_binary_path() -> Result<String> {
    // Try to find mnemosyne binary
    if let Ok(output) = Command::new("which").arg("mnemosyne").output() {
        if output.status.success() {
            if let Ok(path) = String::from_utf8(output.stdout) {
                let path = path.trim().to_string();
                if !path.is_empty() {
                    return Ok(path);
                }
            }
        }
    }

    // Fallback to "mnemosyne" (assume in PATH)
    Ok("mnemosyne".to_string())
}

/// Detect namespace from current directory
fn detect_namespace() -> String {
    // Try to detect from git
    if let Ok(output) = Command::new("git")
        .args(&["rev-parse", "--show-toplevel"])
        .output()
    {
        if output.status.success() {
            if let Ok(git_root) = String::from_utf8(output.stdout) {
                let git_root = git_root.trim();
                if let Some(project_name) = PathBuf::from(git_root).file_name() {
                    if let Some(name) = project_name.to_str() {
                        return format!("project:{}", name);
                    }
                }
            }
        }
    }

    // Fallback to global
    "global".to_string()
}

/// Get default database path, checking for project database first
fn get_default_db_path() -> String {
    // Check for project-specific database in .mnemosyne/
    let project_db = PathBuf::from(".mnemosyne").join("project.db");
    if project_db.exists() {
        return project_db.to_string_lossy().to_string();
    }

    // Fall back to XDG default
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("mnemosyne")
        .join("mnemosyne.db")
        .to_string_lossy()
        .to_string()
}

/// Launch an orchestrated Claude Code session (convenience function)
pub async fn launch_orchestrated_session(
    db_path: Option<String>,
    initial_prompt: Option<String>,
) -> Result<()> {
    let mut config = LauncherConfig::default();
    config.mnemosyne_db_path = db_path;
    config.initial_prompt = initial_prompt;

    let launcher = ClaudeCodeLauncher::with_config(config)?;
    launcher.launch().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_claude_binary() {
        // This test will fail if Claude Code is not installed
        // That's expected - it's more of a verification test
        match detect_claude_binary() {
            Ok(path) => {
                println!("Found Claude Code at: {:?}", path);
                assert!(path.exists() || path.to_string_lossy() == "claude");
            }
            Err(e) => {
                println!("Claude Code not found (expected if not installed): {}", e);
            }
        }
    }

    #[test]
    fn test_detect_namespace() {
        let namespace = detect_namespace();
        assert!(!namespace.is_empty());
        assert!(namespace == "global" || namespace.starts_with("project:"));
    }

    #[test]
    fn test_default_launcher_config() {
        let config = LauncherConfig::default();
        assert!(matches!(config.primary_agent_role, agents::AgentRole::Executor));
        assert!(config.enable_subagents);
        assert_eq!(config.max_concurrent_agents, 4);
        assert_eq!(config.permission_mode, "default");
        assert_eq!(config.model, "sonnet");
    }

    #[test]
    fn test_get_default_db_path() {
        let path = get_default_db_path();
        // Should contain either project.db (if project-specific exists) or mnemosyne.db (XDG default)
        assert!(
            path.contains("project.db") || path.contains("mnemosyne.db"),
            "Expected path to contain either 'project.db' or 'mnemosyne.db', got: {}",
            path
        );
    }
}
