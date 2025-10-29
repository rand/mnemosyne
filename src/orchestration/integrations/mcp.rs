//! MCP Server Integration
//!
//! The MCP (Model Context Protocol) server provides JSON-RPC tools for
//! Claude Code to interact with Mnemosyne. This module documents the
//! integration points between MCP and the orchestration system.
//!
//! # Architecture
//!
//! MCP Server <--> Orchestration Engine
//!     |               |
//!     |               +-- Evolution Jobs
//!     |               +-- Work Queue
//!     |               +-- Event Sourcing
//!     |
//!     +-- Tools: remember, recall, context, etc.
//!
//! # Integration Approach
//!
//! The MCP server and orchestration engine are separate processes:
//! - **MCP Server**: JSON-RPC stdio server (spawned by Claude Code)
//! - **Orchestration**: Native Rust engine (launched by `mnemosyne orchestrate`)
//!
//! ## Triggering Orchestration via CLI
//!
//! ```bash
//! # Launch orchestrated session from CLI
//! mnemosyne orchestrate --plan "Implement feature X"
//! ```
//!
//! ## MCP Tools Available
//!
//! The following MCP tools are available to Claude Code:
//! - `mnemosyne.recall`: Search memories
//! - `mnemosyne.remember`: Store new memory
//! - `mnemosyne.context`: Get memory context
//! - `mnemosyne.graph`: Traverse memory graph
//! - `mnemosyne.update`: Update existing memory
//! - `mnemosyne.delete`: Delete memory
//! - `mnemosyne.consolidate`: Merge similar memories
//! - `mnemosyne.list`: List recent memories
//!
//! ## Future Enhancements
//!
//! Potential MCP tools for direct orchestration access:
//! - `mnemosyne.orchestrate.submit`: Submit work item
//! - `mnemosyne.orchestrate.status`: Query work status
//! - `mnemosyne.orchestrate.events`: Get event history
//!
//! These would require:
//! 1. Shared storage/IPC between MCP server and orchestration engine
//! 2. Event-based communication (pubsub or filesystem watching)
//! 3. Process coordination

/// MCP Integration (documentation only)
///
/// This module serves as documentation for how MCP and orchestration
/// integrate. See module-level docs for details.
pub struct McpIntegration;

impl McpIntegration {
    /// Get available MCP tools
    ///
    /// Returns the list of MCP tool names that are available for
    /// interacting with Mnemosyne's memory system.
    pub fn available_tools() -> Vec<&'static str> {
        vec![
            "mnemosyne.recall",
            "mnemosyne.remember",
            "mnemosyne.context",
            "mnemosyne.graph",
            "mnemosyne.update",
            "mnemosyne.delete",
            "mnemosyne.consolidate",
            "mnemosyne.list",
        ]
    }

    /// Check if orchestration can be triggered via CLI
    ///
    /// Returns true, since `mnemosyne orchestrate` command is available.
    pub fn supports_cli_orchestration() -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_available_tools() {
        let tools = McpIntegration::available_tools();
        assert_eq!(tools.len(), 8);
        assert!(tools.contains(&"mnemosyne.recall"));
        assert!(tools.contains(&"mnemosyne.remember"));
    }

    #[test]
    fn test_cli_orchestration_support() {
        assert!(McpIntegration::supports_cli_orchestration());
    }
}
