//! Interactive Prompts for Coordination
//!
//! Provides user-friendly interactive prompts for coordination decisions.
//!
//! # Features
//!
//! - Join request approval/denial
//! - Conflict resolution options
//! - Mode selection (isolated vs coordinated)
//! - File scope specification
//! - Timeout adjustment

use crate::error::{MnemosyneError, Result};
use crate::orchestration::branch_registry::{CoordinationMode, WorkIntent};
use crate::orchestration::conflict_detector::ConflictSeverity;
use crate::orchestration::identity::AgentId;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::path::PathBuf;

/// Prompt for join request approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRequestPrompt {
    /// Requesting agent ID
    pub agent_id: AgentId,

    /// Target branch
    pub branch: String,

    /// Requested work intent
    pub intent: WorkIntent,

    /// Requested coordination mode
    pub mode: CoordinationMode,

    /// Other agents on this branch
    pub other_agents: Vec<AgentId>,
}

/// User's decision on join request
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JoinDecision {
    /// Approve the request
    Approve,

    /// Deny the request
    Deny,

    /// Approve with coordinated mode
    ApproveCoordinated,

    /// Approve with read-only intent
    ApproveReadOnly,
}

/// Prompt for conflict resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictPrompt {
    /// Conflicting file path
    pub file_path: PathBuf,

    /// Agents involved
    pub agents: Vec<AgentId>,

    /// Conflict severity
    pub severity: ConflictSeverity,

    /// Suggested actions
    pub suggestions: Vec<String>,
}

/// User's decision on conflict
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictDecision {
    /// Continue anyway
    Continue,

    /// Partition work (specify file scopes)
    Partition,

    /// Release assignment
    Release,

    /// Wait for other agent
    Wait,
}

/// Interactive prompter
pub struct InteractivePrompter {
    /// Enable interactive mode (false for automated/testing)
    interactive: bool,
}

impl InteractivePrompter {
    /// Create a new interactive prompter
    pub fn new(interactive: bool) -> Self {
        Self { interactive }
    }

    /// Prompt for join request decision
    pub fn prompt_join_request(&self, prompt: &JoinRequestPrompt) -> Result<JoinDecision> {
        if !self.interactive {
            // Auto-approve in non-interactive mode
            return Ok(JoinDecision::Approve);
        }

        println!("\n╔═══════════════════════════════════════════╗");
        println!("║      Branch Join Request                  ║");
        println!("╚═══════════════════════════════════════════╝\n");

        println!(
            "Agent {} wants to join branch '{}'",
            prompt.agent_id, prompt.branch
        );
        println!("Intent: {:?}", prompt.intent);
        println!("Mode: {:?}", prompt.mode);

        if !prompt.other_agents.is_empty() {
            println!(
                "\n⚠ {} other agent(s) currently on this branch:",
                prompt.other_agents.len()
            );
            for (i, agent_id) in prompt.other_agents.iter().enumerate() {
                println!("  {}. {}", i + 1, agent_id);
            }
        }

        println!("\nOptions:");
        println!("  1. Approve");
        println!("  2. Deny");
        println!("  3. Approve with coordinated mode");
        println!("  4. Approve with read-only access");
        println!("  q. Quit");

        print!("\nYour choice: ");
        io::stdout().flush().map_err(MnemosyneError::Io)?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(MnemosyneError::Io)?;

        match input.trim() {
            "1" => Ok(JoinDecision::Approve),
            "2" => Ok(JoinDecision::Deny),
            "3" => Ok(JoinDecision::ApproveCoordinated),
            "4" => Ok(JoinDecision::ApproveReadOnly),
            "q" | "Q" => Err(MnemosyneError::Other("User cancelled".to_string())),
            _ => {
                println!("Invalid choice. Defaulting to Approve.");
                Ok(JoinDecision::Approve)
            }
        }
    }

    /// Prompt for conflict resolution
    pub fn prompt_conflict_resolution(&self, prompt: &ConflictPrompt) -> Result<ConflictDecision> {
        if !self.interactive {
            // Auto-continue in non-interactive mode
            return Ok(ConflictDecision::Continue);
        }

        println!("\n╔═══════════════════════════════════════════╗");
        println!("║      Conflict Detected                    ║");
        println!("╚═══════════════════════════════════════════╝\n");

        println!("File: {}", prompt.file_path.display());
        println!("Severity: {:?}", prompt.severity);
        println!("Agents: {}", prompt.agents.len());

        if !prompt.suggestions.is_empty() {
            println!("\nSuggestions:");
            for suggestion in &prompt.suggestions {
                println!("  • {}", suggestion);
            }
        }

        println!("\nOptions:");
        println!("  1. Continue (accept risk)");
        println!("  2. Partition work (specify file scopes)");
        println!("  3. Release assignment");
        println!("  4. Wait for other agent");
        println!("  q. Quit");

        print!("\nYour choice: ");
        io::stdout().flush().map_err(MnemosyneError::Io)?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(MnemosyneError::Io)?;

        match input.trim() {
            "1" => Ok(ConflictDecision::Continue),
            "2" => Ok(ConflictDecision::Partition),
            "3" => Ok(ConflictDecision::Release),
            "4" => Ok(ConflictDecision::Wait),
            "q" | "Q" => Err(MnemosyneError::Other("User cancelled".to_string())),
            _ => {
                println!("Invalid choice. Defaulting to Continue.");
                Ok(ConflictDecision::Continue)
            }
        }
    }

    /// Prompt for coordination mode selection
    pub fn prompt_coordination_mode(&self) -> Result<CoordinationMode> {
        if !self.interactive {
            return Ok(CoordinationMode::Isolated); // Default per user requirement
        }

        println!("\n╔═══════════════════════════════════════════╗");
        println!("║      Select Coordination Mode             ║");
        println!("╚═══════════════════════════════════════════╝\n");

        println!("Coordination modes:");
        println!("  1. Isolated (default) - Block other agents");
        println!("  2. Coordinated - Allow multiple agents with conflict detection");

        print!("\nYour choice [1]: ");
        io::stdout().flush().map_err(MnemosyneError::Io)?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(MnemosyneError::Io)?;

        match input.trim() {
            "" | "1" => Ok(CoordinationMode::Isolated),
            "2" => Ok(CoordinationMode::Coordinated),
            _ => {
                println!("Invalid choice. Defaulting to Isolated.");
                Ok(CoordinationMode::Isolated)
            }
        }
    }

    /// Prompt for work intent selection
    pub fn prompt_work_intent(&self) -> Result<WorkIntent> {
        if !self.interactive {
            return Ok(WorkIntent::ReadOnly); // Safest default
        }

        println!("\n╔═══════════════════════════════════════════╗");
        println!("║      Select Work Intent                   ║");
        println!("╚═══════════════════════════════════════════╝\n");

        println!("Work intents:");
        println!("  1. Read-only - View code only (auto-approved)");
        println!("  2. Write specific files - Modify listed files");
        println!("  3. Full branch - Complete write access");

        print!("\nYour choice [1]: ");
        io::stdout().flush().map_err(MnemosyneError::Io)?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(MnemosyneError::Io)?;

        match input.trim() {
            "" | "1" => Ok(WorkIntent::ReadOnly),
            "2" => {
                // Prompt for file paths
                println!("\nEnter file paths (comma-separated):");
                print!("> ");
                io::stdout().flush().map_err(MnemosyneError::Io)?;

                let mut files_input = String::new();
                io::stdin()
                    .read_line(&mut files_input)
                    .map_err(MnemosyneError::Io)?;

                let files: Vec<PathBuf> = files_input
                    .trim()
                    .split(',')
                    .map(|s| PathBuf::from(s.trim()))
                    .collect();

                if files.is_empty() {
                    println!("No files specified. Defaulting to read-only.");
                    Ok(WorkIntent::ReadOnly)
                } else {
                    Ok(WorkIntent::Write(files))
                }
            }
            "3" => Ok(WorkIntent::FullBranch),
            _ => {
                println!("Invalid choice. Defaulting to read-only.");
                Ok(WorkIntent::ReadOnly)
            }
        }
    }

    /// Display informational message
    pub fn display_info(&self, title: &str, message: &str) {
        if !self.interactive {
            return;
        }

        println!("\n╔═══════════════════════════════════════════╗");
        println!("║ {:<41} ║", title);
        println!("╚═══════════════════════════════════════════╝\n");
        println!("{}", message);
        println!();
    }

    /// Display success message
    pub fn display_success(&self, message: &str) {
        if !self.interactive {
            return;
        }

        println!("\n✓ {}", message);
    }

    /// Display error message
    pub fn display_error(&self, message: &str) {
        if !self.interactive {
            return;
        }

        eprintln!("\n✗ Error: {}", message);
    }

    /// Display warning message
    pub fn display_warning(&self, message: &str) {
        if !self.interactive {
            return;
        }

        println!("\n⚠ Warning: {}", message);
    }

    /// Confirm action with yes/no prompt
    pub fn confirm(&self, message: &str, default_yes: bool) -> Result<bool> {
        if !self.interactive {
            return Ok(default_yes);
        }

        let prompt_suffix = if default_yes { " [Y/n]: " } else { " [y/N]: " };
        print!("{}{}", message, prompt_suffix);
        io::stdout().flush().map_err(MnemosyneError::Io)?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(MnemosyneError::Io)?;

        match input.trim().to_lowercase().as_str() {
            "" => Ok(default_yes),
            "y" | "yes" => Ok(true),
            "n" | "no" => Ok(false),
            _ => {
                println!(
                    "Invalid input. Defaulting to {}.",
                    if default_yes { "yes" } else { "no" }
                );
                Ok(default_yes)
            }
        }
    }
}

impl Default for InteractivePrompter {
    fn default() -> Self {
        Self::new(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_non_interactive_defaults() {
        let prompter = InteractivePrompter::new(false);

        // Non-interactive mode should use safe defaults
        let mode = prompter.prompt_coordination_mode().unwrap();
        assert_eq!(mode, CoordinationMode::Isolated);

        let intent = prompter.prompt_work_intent().unwrap();
        assert_eq!(intent, WorkIntent::ReadOnly);

        let confirm = prompter.confirm("Test?", true).unwrap();
        assert!(confirm);

        let confirm = prompter.confirm("Test?", false).unwrap();
        assert!(!confirm);
    }

    #[test]
    fn test_join_prompt_structure() {
        let prompt = JoinRequestPrompt {
            agent_id: AgentId::new(),
            branch: "main".to_string(),
            intent: WorkIntent::ReadOnly,
            mode: CoordinationMode::Isolated,
            other_agents: vec![],
        };

        // Verify structure is valid
        assert_eq!(prompt.branch, "main");
    }

    #[test]
    fn test_conflict_prompt_structure() {
        let prompt = ConflictPrompt {
            file_path: PathBuf::from("src/main.rs"),
            agents: vec![AgentId::new(), AgentId::new()],
            severity: ConflictSeverity::Warning,
            suggestions: vec!["Partition work".to_string()],
        };

        assert_eq!(prompt.agents.len(), 2);
        assert_eq!(prompt.suggestions.len(), 1);
    }
}
