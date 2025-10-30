//! Status Line Integration
//!
//! Provides status line information for display in terminal prompts or status bars.
//!
//! # Status Line Format
//!
//! ```text
//! [branch:main|mode:isolated|conflicts:0]
//! [branch:feature/test|mode:coordinated(2)|conflicts:1⚠]
//! ```
//!
//! # Integration
//!
//! This module can be integrated with:
//! - Shell prompts (PS1, PROMPT)
//! - Terminal multiplexers (tmux, screen)
//! - IDEs and editors
//! - Status bars (i3bar, waybar)

use crate::error::Result;
use crate::orchestration::branch_coordinator::BranchCoordinator;
use crate::orchestration::branch_registry::CoordinationMode;
use crate::orchestration::identity::AgentIdentity;
use serde::{Deserialize, Serialize};

/// Status line information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusLine {
    /// Current branch
    pub branch: String,

    /// Coordination mode
    pub mode: CoordinationMode,

    /// Number of agents on this branch
    pub agent_count: usize,

    /// Number of active conflicts
    pub conflict_count: usize,

    /// Is current agent blocked
    pub blocked: bool,
}

impl StatusLine {
    /// Format as compact string for status line display
    pub fn format_compact(&self) -> String {
        let mode_str = match self.mode {
            CoordinationMode::Isolated => "iso".to_string(),
            CoordinationMode::Coordinated => format!("coord({})", self.agent_count),
        };

        let conflict_indicator = if self.conflict_count > 0 {
            format!("|conflicts:{}⚠", self.conflict_count)
        } else {
            String::new()
        };

        let blocked_indicator = if self.blocked { "|BLOCKED" } else { "" };

        format!(
            "[{}|{}{}{}]",
            self.branch, mode_str, conflict_indicator, blocked_indicator
        )
    }

    /// Format as detailed string with full labels
    pub fn format_detailed(&self) -> String {
        let mode_str = match self.mode {
            CoordinationMode::Isolated => "isolated".to_string(),
            CoordinationMode::Coordinated => {
                format!("coordinated ({} agents)", self.agent_count)
            }
        };

        let mut parts = vec![
            format!("Branch: {}", self.branch),
            format!("Mode: {}", mode_str),
        ];

        if self.conflict_count > 0 {
            parts.push(format!("⚠ {} conflict(s)", self.conflict_count));
        }

        if self.blocked {
            parts.push("❌ BLOCKED".to_string());
        }

        parts.join(" | ")
    }

    /// Format as JSON for programmatic consumption
    pub fn format_json(&self) -> Result<String> {
        serde_json::to_string(self)
            .map_err(|e| crate::error::MnemosyneError::Other(format!("JSON serialization failed: {}", e)))
    }

    /// Create color-coded ANSI string (for terminal display)
    pub fn format_ansi(&self) -> String {
        let branch_color = "\x1b[36m"; // Cyan
        let mode_color = match self.mode {
            CoordinationMode::Isolated => "\x1b[33m", // Yellow
            CoordinationMode::Coordinated => "\x1b[32m", // Green
        };
        let conflict_color = "\x1b[31m"; // Red
        let reset = "\x1b[0m";

        let mode_str = match self.mode {
            CoordinationMode::Isolated => "iso".to_string(),
            CoordinationMode::Coordinated => format!("coord({})", self.agent_count),
        };

        let mut result = format!(
            "[{}{}{}|{}{}{}",
            branch_color, self.branch, reset, mode_color, mode_str, reset
        );

        if self.conflict_count > 0 {
            result.push_str(&format!(
                "|{}conflicts:{}⚠{}",
                conflict_color, self.conflict_count, reset
            ));
        }

        if self.blocked {
            result.push_str(&format!("|{}BLOCKED{}", conflict_color, reset));
        }

        result.push(']');
        result
    }
}

/// Status line provider
pub struct StatusLineProvider {
    coordinator: BranchCoordinator,
    agent: AgentIdentity,
}

impl StatusLineProvider {
    /// Create a new status line provider
    pub fn new(coordinator: BranchCoordinator, agent: AgentIdentity) -> Self {
        Self { coordinator, agent }
    }

    /// Get current status line information
    pub async fn get_status(&self) -> Result<StatusLine> {
        let branch = &self.agent.branch;

        // Get assignments for current branch
        let assignments = self.coordinator.get_branch_assignments(branch).await?;

        // Determine mode (check current agent's assignment)
        let mode = assignments
            .iter()
            .find(|a| a.agent_id == self.agent.id)
            .map(|a| a.mode)
            .unwrap_or(CoordinationMode::Isolated);

        // Count agents
        let agent_count = assignments.len();

        // Get conflict count from coordinator
        let conflict_count = self.coordinator.get_agent_conflict_count(&self.agent.id)
            .unwrap_or(0); // Gracefully handle errors by showing 0 conflicts

        // TODO: Check if blocked
        let blocked = false;

        Ok(StatusLine {
            branch: branch.clone(),
            mode,
            agent_count,
            conflict_count,
            blocked,
        })
    }

    /// Get status line formatted for display
    pub async fn get_formatted(&self, format: StatusLineFormat) -> Result<String> {
        let status = self.get_status().await?;

        Ok(match format {
            StatusLineFormat::Compact => status.format_compact(),
            StatusLineFormat::Detailed => status.format_detailed(),
            StatusLineFormat::Json => status.format_json()?,
            StatusLineFormat::Ansi => status.format_ansi(),
        })
    }
}

/// Status line format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusLineFormat {
    /// Compact format for narrow displays
    Compact,

    /// Detailed format with full labels
    Detailed,

    /// JSON format for programmatic use
    Json,

    /// ANSI color-coded format
    Ansi,
}

/// Shell integration helper
pub struct ShellIntegration;

impl ShellIntegration {
    /// Generate bash prompt integration
    pub fn bash_prompt() -> &'static str {
        r#"
# Add to ~/.bashrc or ~/.bash_profile

mnemosyne_prompt() {
    if command -v mnemosyne-status &> /dev/null; then
        mnemosyne-status --format ansi 2>/dev/null || echo ""
    fi
}

# Add to PS1:
# PS1="$(mnemosyne_prompt) $PS1"
"#
    }

    /// Generate zsh prompt integration
    pub fn zsh_prompt() -> &'static str {
        r#"
# Add to ~/.zshrc

mnemosyne_prompt() {
    if command -v mnemosyne-status &> /dev/null; then
        mnemosyne-status --format ansi 2>/dev/null || echo ""
    fi
}

# Add to PROMPT:
# PROMPT="$(mnemosyne_prompt) $PROMPT"
"#
    }

    /// Generate tmux status bar integration
    pub fn tmux_status() -> &'static str {
        r#"
# Add to ~/.tmux.conf

set -g status-right '#(mnemosyne-status --format compact 2>/dev/null) | %H:%M'
"#
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_compact_isolated() {
        let status = StatusLine {
            branch: "main".to_string(),
            mode: CoordinationMode::Isolated,
            agent_count: 1,
            conflict_count: 0,
            blocked: false,
        };

        assert_eq!(status.format_compact(), "[main|iso]");
    }

    #[test]
    fn test_format_compact_coordinated() {
        let status = StatusLine {
            branch: "feature/test".to_string(),
            mode: CoordinationMode::Coordinated,
            agent_count: 3,
            conflict_count: 0,
            blocked: false,
        };

        assert_eq!(status.format_compact(), "[feature/test|coord(3)]");
    }

    #[test]
    fn test_format_compact_with_conflicts() {
        let status = StatusLine {
            branch: "main".to_string(),
            mode: CoordinationMode::Coordinated,
            agent_count: 2,
            conflict_count: 1,
            blocked: false,
        };

        assert_eq!(status.format_compact(), "[main|coord(2)|conflicts:1⚠]");
    }

    #[test]
    fn test_format_compact_blocked() {
        let status = StatusLine {
            branch: "main".to_string(),
            mode: CoordinationMode::Isolated,
            agent_count: 1,
            conflict_count: 0,
            blocked: true,
        };

        assert_eq!(status.format_compact(), "[main|iso|BLOCKED]");
    }

    #[test]
    fn test_format_detailed() {
        let status = StatusLine {
            branch: "main".to_string(),
            mode: CoordinationMode::Coordinated,
            agent_count: 2,
            conflict_count: 1,
            blocked: false,
        };

        let detailed = status.format_detailed();
        assert!(detailed.contains("Branch: main"));
        assert!(detailed.contains("coordinated (2 agents)"));
        assert!(detailed.contains("1 conflict(s)"));
    }

    #[test]
    fn test_format_json() {
        let status = StatusLine {
            branch: "main".to_string(),
            mode: CoordinationMode::Isolated,
            agent_count: 1,
            conflict_count: 0,
            blocked: false,
        };

        let json = status.format_json().unwrap();
        assert!(json.contains("\"branch\":\"main\""));
    }
}
