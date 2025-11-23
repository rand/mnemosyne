//! Iroh Transport - Remote Communication Implementation
//!
//! Implements the RemoteTransport trait using Iroh P2P networking.
//! Handles connection establishment and stream management.

use crate::orchestration::messages::AgentMessage;
use crate::orchestration::network::endpoint::AgentEndpoint;
use crate::orchestration::network::protocol::AgentProtocol;
use crate::orchestration::network::router::RemoteTransport;
use async_trait::async_trait;
use iroh::base::node_addr::NodeAddr;
use iroh::net::NodeId;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Transport implementation using Iroh
pub struct IrohTransport {
    /// Reference to the shared agent endpoint
    endpoint: Arc<RwLock<Option<AgentEndpoint>>>,
}

impl IrohTransport {
    /// Create a new Iroh transport
    pub fn new(endpoint: Arc<RwLock<Option<AgentEndpoint>>>) -> Self {
        Self { endpoint }
    }
}

#[async_trait]
impl RemoteTransport for IrohTransport {
    async fn send(&self, node_id: &str, message: &AgentMessage) -> Result<(), String> {
        // Acquire read lock on endpoint
        let endpoint_lock = self.endpoint.read().await;
        let endpoint = endpoint_lock
            .as_ref()
            .ok_or_else(|| "Agent endpoint not initialized".to_string())?;

        // Parse node ID
        let node_id = NodeId::from_str(node_id)
            .map_err(|e| format!("Invalid node ID: {}", e))?;
        
        // Construct NodeAddr (direct connection by ID)
        // In a real scenario, we might need relay URLs or direct addresses,
        // but Iroh handles discovery via magical infrastructure.
        let node_addr = NodeAddr::new(node_id);

        // Connect to the remote agent
        let connection = endpoint
            .connect(&node_addr)
            .await
            .map_err(|e| format!("Failed to connect: {}", e))?;

        // Open a bidirectional stream
        let (mut send_stream, _recv_stream) = endpoint
            .open_stream(&connection)
            .await
            .map_err(|e| format!("Failed to open stream: {}", e))?;

        // Send the message using AgentProtocol
        AgentProtocol::send_message(&mut send_stream, message)
            .await
            .map_err(|e| format!("Failed to send message: {}", e))?;
            
        // Finish the stream to signal we are done sending
        send_stream
            .finish()
            .map_err(|e| format!("Failed to finish stream: {}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_transport_creation() {
        let endpoint = Arc::new(RwLock::new(None));
        let _transport = IrohTransport::new(endpoint);
    }
}
