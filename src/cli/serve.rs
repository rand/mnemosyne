//! MCP server startup command

use mnemosyne_core::error::Result;
use super::helpers::start_mcp_server;

/// Handle MCP server startup command
pub async fn handle(db_path: Option<String>) -> Result<()> {
    // Start MCP server (automatically handles API server startup)
    start_mcp_server(db_path).await
}
