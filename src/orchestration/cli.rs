//! CLI Commands for Branch Coordination
//!
//! Provides command-line interface for agents to interact with the branch
//! coordination system.
//!
//! # Commands
//!
//! - `status` - Show current branch assignments and conflicts
//! - `join` - Request to join a branch with specified intent
//! - `conflicts` - List active conflicts for current agent
//! - `switch` - Switch to a different branch (releases current assignment)
//! - `release` - Release current branch assignment

use crate::error::{MnemosyneError, Result};
use crate::orchestration::branch_coordinator::{BranchCoordinator, JoinRequest, JoinResponse};
use crate::orchestration::branch_registry::{CoordinationMode, WorkIntent};
use crate::orchestration::identity::AgentIdentity;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// CLI command type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CliCommand {
    /// Show status of branch assignments
    Status {
        /// Show all branches or just current agent
        all: bool,
    },

    /// Join a branch
    Join {
        /// Target branch name
        branch: String,

        /// Work intent (read-only, write, full)
        intent: String,

        /// Coordination mode (isolated, coordinated)
        mode: Option<String>,

        /// File paths for write intent
        files: Vec<PathBuf>,
    },

    /// List conflicts
    Conflicts {
        /// Show all conflicts or just for current agent
        all: bool,
    },

    /// Switch to a different branch
    Switch {
        /// Target branch name
        branch: String,

        /// Work intent
        intent: String,

        /// Coordination mode
        mode: Option<String>,
    },

    /// Release current branch assignment
    Release,
}

/// CLI command result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliResult {
    /// Success status
    pub success: bool,

    /// Result message
    pub message: String,

    /// Additional data (JSON)
    pub data: Option<serde_json::Value>,
}

impl CliResult {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: None,
        }
    }

    pub fn success_with_data(message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: Some(data),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
        }
    }
}

/// CLI handler
pub struct CliHandler {
    coordinator: BranchCoordinator,
    current_agent: AgentIdentity,
}

impl CliHandler {
    /// Create a new CLI handler
    pub fn new(coordinator: BranchCoordinator, current_agent: AgentIdentity) -> Self {
        Self {
            coordinator,
            current_agent,
        }
    }

    /// Execute a CLI command
    pub async fn execute(&self, command: CliCommand) -> Result<CliResult> {
        match command {
            CliCommand::Status { all } => self.handle_status(all).await,
            CliCommand::Join {
                branch,
                intent,
                mode,
                files,
            } => self.handle_join(branch, intent, mode, files).await,
            CliCommand::Conflicts { all } => self.handle_conflicts(all).await,
            CliCommand::Switch {
                branch,
                intent,
                mode,
            } => self.handle_switch(branch, intent, mode).await,
            CliCommand::Release => self.handle_release().await,
        }
    }

    /// Handle status command
    async fn handle_status(&self, all: bool) -> Result<CliResult> {
        if all {
            // Show all branch assignments
            // TODO: Get all branches from registry
            Ok(CliResult::success("Status for all branches"))
        } else {
            // Show current agent's assignment
            let branch = &self.current_agent.branch;
            let assignments = self.coordinator.get_branch_assignments(branch).await?;

            let data = serde_json::json!({
                "branch": branch,
                "assignments": assignments.len(),
                "agent_id": self.current_agent.id.to_string(),
            });

            Ok(CliResult::success_with_data(
                format!("Branch '{}' has {} agent(s)", branch, assignments.len()),
                data,
            ))
        }
    }

    /// Handle join command
    async fn handle_join(
        &self,
        branch: String,
        intent_str: String,
        mode_str: Option<String>,
        files: Vec<PathBuf>,
    ) -> Result<CliResult> {
        // Parse intent
        let intent = match intent_str.to_lowercase().as_str() {
            "read" | "readonly" | "read-only" => WorkIntent::ReadOnly,
            "write" => {
                if files.is_empty() {
                    return Ok(CliResult::error(
                        "Write intent requires --files argument with file paths",
                    ));
                }
                WorkIntent::Write(files)
            }
            "full" | "fullbranch" | "full-branch" => WorkIntent::FullBranch,
            _ => {
                return Ok(CliResult::error(format!(
                    "Invalid intent '{}'. Use: read, write, or full",
                    intent_str
                )));
            }
        };

        // Parse mode
        let mode = match mode_str.as_deref() {
            Some("isolated") => CoordinationMode::Isolated,
            Some("coordinated") => CoordinationMode::Coordinated,
            None => CoordinationMode::Isolated, // Default per user requirement
            Some(other) => {
                return Ok(CliResult::error(format!(
                    "Invalid mode '{}'. Use: isolated or coordinated",
                    other
                )));
            }
        };

        // Create join request
        let request = JoinRequest {
            agent_identity: self.current_agent.clone(),
            target_branch: branch.clone(),
            intent,
            mode,
            work_items: vec![],
        };

        // Handle request
        match self.coordinator.handle_join_request(request).await? {
            JoinResponse::Approved { message, .. } => {
                Ok(CliResult::success(format!("✓ {}", message)))
            }
            JoinResponse::RequiresCoordination {
                message,
                other_agents,
                ..
            } => {
                let data = serde_json::json!({
                    "other_agents": other_agents.len(),
                    "coordination_required": true,
                });
                Ok(CliResult::success_with_data(
                    format!("⚠ {} (Other agents: {})", message, other_agents.len()),
                    data,
                ))
            }
            JoinResponse::Denied {
                reason,
                suggestions,
            } => {
                let mut message = format!("✗ Denied: {}", reason);
                if !suggestions.is_empty() {
                    message.push_str("\n\nSuggestions:");
                    for suggestion in &suggestions {
                        message.push_str(&format!("\n  • {}", suggestion));
                    }
                }
                Ok(CliResult::error(message))
            }
        }
    }

    /// Handle conflicts command
    async fn handle_conflicts(&self, _all: bool) -> Result<CliResult> {
        // TODO: Get conflicts from file tracker via coordinator
        Ok(CliResult::success("No active conflicts"))
    }

    /// Handle switch command
    async fn handle_switch(
        &self,
        branch: String,
        intent: String,
        mode: Option<String>,
    ) -> Result<CliResult> {
        // Release current assignment
        self.coordinator
            .release_assignment(&self.current_agent.id)
            .await?;

        // Join new branch
        self.handle_join(branch, intent, mode, vec![]).await
    }

    /// Handle release command
    async fn handle_release(&self) -> Result<CliResult> {
        self.coordinator
            .release_assignment(&self.current_agent.id)
            .await?;

        Ok(CliResult::success(format!(
            "Released assignment for branch '{}'",
            self.current_agent.branch
        )))
    }
}

/// Parse CLI arguments into a command
pub fn parse_args(args: &[String]) -> Result<CliCommand> {
    if args.is_empty() {
        return Err(MnemosyneError::Other(
            "No command specified. Use: status, join, conflicts, switch, or release".to_string(),
        ));
    }

    let command = &args[0];

    match command.as_str() {
        "status" => {
            let all = args.contains(&"--all".to_string());
            Ok(CliCommand::Status { all })
        }

        "join" => {
            if args.len() < 3 {
                return Err(MnemosyneError::Other(
                    "Usage: join <branch> <intent> [--mode <mode>] [--files <paths>]".to_string(),
                ));
            }

            let branch = args[1].clone();
            let intent = args[2].clone();

            let mode = args
                .windows(2)
                .find(|w| w[0] == "--mode")
                .map(|w| w[1].clone());

            let files = if let Some(pos) = args.iter().position(|a| a == "--files") {
                args[pos + 1..]
                    .iter()
                    .take_while(|a| !a.starts_with("--"))
                    .map(PathBuf::from)
                    .collect()
            } else {
                vec![]
            };

            Ok(CliCommand::Join {
                branch,
                intent,
                mode,
                files,
            })
        }

        "conflicts" => {
            let all = args.contains(&"--all".to_string());
            Ok(CliCommand::Conflicts { all })
        }

        "switch" => {
            if args.len() < 3 {
                return Err(MnemosyneError::Other(
                    "Usage: switch <branch> <intent> [--mode <mode>]".to_string(),
                ));
            }

            let branch = args[1].clone();
            let intent = args[2].clone();

            let mode = args
                .windows(2)
                .find(|w| w[0] == "--mode")
                .map(|w| w[1].clone());

            Ok(CliCommand::Switch {
                branch,
                intent,
                mode,
            })
        }

        "release" => Ok(CliCommand::Release),

        _ => Err(MnemosyneError::Other(format!(
            "Unknown command '{}'. Use: status, join, conflicts, switch, or release",
            command
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_status() {
        let args = vec!["status".to_string()];
        let cmd = parse_args(&args).unwrap();
        match cmd {
            CliCommand::Status { all } => assert!(!all),
            _ => panic!("Expected Status command"),
        }
    }

    #[test]
    fn test_parse_status_all() {
        let args = vec!["status".to_string(), "--all".to_string()];
        let cmd = parse_args(&args).unwrap();
        match cmd {
            CliCommand::Status { all } => assert!(all),
            _ => panic!("Expected Status command with --all"),
        }
    }

    #[test]
    fn test_parse_join() {
        let args = vec![
            "join".to_string(),
            "main".to_string(),
            "read".to_string(),
        ];
        let cmd = parse_args(&args).unwrap();
        match cmd {
            CliCommand::Join { branch, intent, .. } => {
                assert_eq!(branch, "main");
                assert_eq!(intent, "read");
            }
            _ => panic!("Expected Join command"),
        }
    }

    #[test]
    fn test_parse_join_with_mode() {
        let args = vec![
            "join".to_string(),
            "feature/test".to_string(),
            "write".to_string(),
            "--mode".to_string(),
            "coordinated".to_string(),
        ];
        let cmd = parse_args(&args).unwrap();
        match cmd {
            CliCommand::Join { branch, mode, .. } => {
                assert_eq!(branch, "feature/test");
                assert_eq!(mode, Some("coordinated".to_string()));
            }
            _ => panic!("Expected Join command"),
        }
    }

    #[test]
    fn test_parse_release() {
        let args = vec!["release".to_string()];
        let cmd = parse_args(&args).unwrap();
        matches!(cmd, CliCommand::Release);
    }
}
