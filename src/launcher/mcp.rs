//! MCP Configuration Generation
//!
//! Generates MCP (Model Context Protocol) server configuration for
//! Mnemosyne integration with Claude Code.

use crate::error::{MnemosyneError, Result};
use crate::launcher::agents::AgentRole;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Command to run the MCP server
    pub command: String,

    /// Arguments for the command
    pub args: Vec<String>,

    /// Environment variables
    pub env: HashMap<String, String>,

    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// MCP configuration generator
#[derive(Debug, Clone)]
pub struct McpConfigGenerator {
    /// Path to mnemosyne binary
    pub mnemosyne_binary_path: String,

    /// Namespace for memories
    pub namespace: String,

    /// Database path
    pub db_path: String,

    /// Agent role for this session
    pub agent_role: AgentRole,
}

impl McpConfigGenerator {
    /// Generate MCP configuration JSON string
    pub fn generate_config(&self) -> Result<String> {
        let mut env = HashMap::new();
        env.insert("MNEMOSYNE_NAMESPACE".to_string(), self.namespace.clone());
        env.insert("MNEMOSYNE_DB_PATH".to_string(), self.db_path.clone());
        env.insert(
            "MNEMOSYNE_AGENT_ROLE".to_string(),
            self.agent_role.as_str().to_string(),
        );
        env.insert("RUST_LOG".to_string(), "info".to_string());

        let server_config = McpServerConfig {
            command: self.mnemosyne_binary_path.clone(),
            args: vec!["serve".to_string()],
            env,
            description: Some("Mnemosyne - Project-aware agentic memory system".to_string()),
        };

        let config = json!({
            "mcpServers": {
                "mnemosyne": server_config
            }
        });

        serde_json::to_string(&config)
            .map_err(|e| MnemosyneError::Other(format!("Failed to serialize MCP config: {}", e)))
    }

    /// Generate MCP configuration with custom server name
    pub fn generate_config_with_name(&self, server_name: &str) -> Result<String> {
        let mut env = HashMap::new();
        env.insert("MNEMOSYNE_NAMESPACE".to_string(), self.namespace.clone());
        env.insert("MNEMOSYNE_DB_PATH".to_string(), self.db_path.clone());
        env.insert(
            "MNEMOSYNE_AGENT_ROLE".to_string(),
            self.agent_role.as_str().to_string(),
        );
        env.insert("RUST_LOG".to_string(), "info".to_string());

        let server_config = McpServerConfig {
            command: self.mnemosyne_binary_path.clone(),
            args: vec!["serve".to_string()],
            env,
            description: Some(format!("Mnemosyne - {} memory system", self.namespace)),
        };

        let config = json!({
            "mcpServers": {
                server_name: server_config
            }
        });

        serde_json::to_string(&config)
            .map_err(|e| MnemosyneError::Other(format!("Failed to serialize MCP config: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_config_generation() {
        let generator = McpConfigGenerator {
            mnemosyne_binary_path: "/usr/local/bin/mnemosyne".to_string(),
            namespace: "project:test".to_string(),
            db_path: "/path/to/db.db".to_string(),
            agent_role: AgentRole::Executor,
        };

        let config_json = generator.generate_config().unwrap();

        // Verify it's valid JSON
        let config: serde_json::Value = serde_json::from_str(&config_json).unwrap();

        assert!(config["mcpServers"]["mnemosyne"].is_object());
        assert_eq!(
            config["mcpServers"]["mnemosyne"]["command"],
            "/usr/local/bin/mnemosyne"
        );
        assert_eq!(config["mcpServers"]["mnemosyne"]["args"][0], "serve");
        assert_eq!(
            config["mcpServers"]["mnemosyne"]["env"]["MNEMOSYNE_NAMESPACE"],
            "project:test"
        );
        assert_eq!(
            config["mcpServers"]["mnemosyne"]["env"]["MNEMOSYNE_DB_PATH"],
            "/path/to/db.db"
        );
        assert_eq!(
            config["mcpServers"]["mnemosyne"]["env"]["MNEMOSYNE_AGENT_ROLE"],
            "executor"
        );
    }

    #[test]
    fn test_mcp_config_with_custom_name() {
        let generator = McpConfigGenerator {
            mnemosyne_binary_path: "mnemosyne".to_string(),
            namespace: "global".to_string(),
            db_path: "~/.local/share/mnemosyne/mnemosyne.db".to_string(),
            agent_role: AgentRole::Orchestrator,
        };

        let config_json = generator
            .generate_config_with_name("mnemosyne-global")
            .unwrap();
        let config: serde_json::Value = serde_json::from_str(&config_json).unwrap();

        assert!(config["mcpServers"]["mnemosyne-global"].is_object());
        assert_eq!(
            config["mcpServers"]["mnemosyne-global"]["env"]["MNEMOSYNE_AGENT_ROLE"],
            "orchestrator"
        );
    }

    #[test]
    fn test_different_agent_roles() {
        let roles = vec![
            AgentRole::Orchestrator,
            AgentRole::Optimizer,
            AgentRole::Reviewer,
            AgentRole::Executor,
        ];

        for role in roles {
            let generator = McpConfigGenerator {
                mnemosyne_binary_path: "mnemosyne".to_string(),
                namespace: "test".to_string(),
                db_path: "test.db".to_string(),
                agent_role: role.clone(),
            };

            let config_json = generator.generate_config().unwrap();
            let config: serde_json::Value = serde_json::from_str(&config_json).unwrap();

            assert_eq!(
                config["mcpServers"]["mnemosyne"]["env"]["MNEMOSYNE_AGENT_ROLE"],
                role.as_str()
            );
        }
    }
}
