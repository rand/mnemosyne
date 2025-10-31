//! Network Layer - Iroh P2P Communication
//!
//! Provides distributed agent communication using Iroh QUIC networking:
//! - Agent endpoints with keypair identity
//! - Protocol handlers for agent messages
//! - Hybrid routing (local Ractor vs remote Iroh)
//! - Peer discovery and connection management

pub mod endpoint;
pub mod protocol;
pub mod router;

pub use endpoint::{AgentEndpoint, AgentKeypair};
pub use protocol::AgentProtocol;
pub use router::MessageRouter;

use crate::error::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Network layer managing all distributed communication
pub struct NetworkLayer {
    /// Local agent endpoint
    endpoint: Arc<RwLock<Option<AgentEndpoint>>>,

    /// Message router (WIP)
    #[allow(dead_code)]
    router: Arc<MessageRouter>,

    /// Whether network layer is started
    started: Arc<RwLock<bool>>,
}

impl NetworkLayer {
    /// Create a new network layer
    pub async fn new() -> Result<Self> {
        Ok(Self {
            endpoint: Arc::new(RwLock::new(None)),
            router: Arc::new(MessageRouter::new()),
            started: Arc::new(RwLock::new(false)),
        })
    }

    /// Start the network layer
    pub async fn start(&self) -> Result<()> {
        let mut started = self.started.write().await;
        if *started {
            return Ok(());
        }

        tracing::debug!("Starting network layer");

        // Create agent endpoint
        let endpoint = AgentEndpoint::new().await?;

        tracing::debug!("Agent endpoint created: {}", endpoint.node_id());

        // Store endpoint
        {
            let mut ep = self.endpoint.write().await;
            *ep = Some(endpoint);
        }

        *started = true;

        tracing::debug!("Network layer started");
        Ok(())
    }

    /// Stop the network layer
    pub async fn stop(&self) -> Result<()> {
        let mut started = self.started.write().await;
        if !*started {
            return Ok(());
        }

        tracing::info!("Stopping network layer");

        // Close endpoint
        {
            let mut ep = self.endpoint.write().await;
            if let Some(endpoint) = ep.take() {
                endpoint.close().await?;
            }
        }

        *started = false;

        tracing::info!("Network layer stopped");
        Ok(())
    }

    /// Get endpoint node ID
    pub async fn node_id(&self) -> Option<String> {
        let ep = self.endpoint.read().await;
        ep.as_ref().map(|e| e.node_id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_layer_lifecycle() {
        let layer = NetworkLayer::new().await.unwrap();

        // Test start/stop
        layer.start().await.unwrap();
        assert!(layer.node_id().await.is_some());

        layer.stop().await.unwrap();
    }
}
