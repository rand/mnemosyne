//! MCP server startup command

use super::event_helpers;
use super::helpers::start_mcp_server;
use mnemosyne_core::error::Result;
use mnemosyne_core::orchestration::events::AgentEvent;

/// Handle MCP server startup command
pub async fn handle(db_path: Option<String>) -> Result<()> {
    let start_time = std::time::Instant::now();
    let instance_id = std::process::id().to_string();

    // Emit server started event
    event_helpers::emit_domain_event(AgentEvent::ServerStarted {
        server_type: "mcp".to_string(),
        listen_addr: "stdio".to_string(),
        instance_id: instance_id.clone(),
    })
    .await;

    // Start MCP server (automatically handles API server startup)
    let result = start_mcp_server(db_path).await;

    // Emit server stopped event
    let uptime_ms = start_time.elapsed().as_millis() as u64;
    event_helpers::emit_domain_event(AgentEvent::ServerStopped {
        server_type: "mcp".to_string(),
        uptime_ms,
        requests_handled: 0, // MCP doesn't track request count in stdio mode
    })
    .await;

    result
}
