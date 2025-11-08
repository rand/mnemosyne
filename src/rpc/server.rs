//! gRPC server setup

use crate::rpc::generated::health_service_server::HealthServiceServer;
use crate::rpc::generated::memory_service_server::MemoryServiceServer;
use crate::rpc::services::{HealthServiceImpl, MemoryServiceImpl};
use anyhow::Result;
use tonic::transport::Server;
use tracing::info;

pub struct RpcServer {
    health_service: HealthServiceImpl,
    memory_service: MemoryServiceImpl,
}

impl RpcServer {
    pub fn new() -> Self {
        Self {
            health_service: HealthServiceImpl::new(),
            memory_service: MemoryServiceImpl::new(),
        }
    }

    pub async fn serve(self, addr: impl Into<String>) -> Result<()> {
        let addr_str = addr.into();
        let addr = addr_str.parse()?;

        info!("Starting mnemosyne RPC server on {}", addr);

        Server::builder()
            .add_service(HealthServiceServer::new(self.health_service))
            .add_service(MemoryServiceServer::new(self.memory_service))
            .serve(addr)
            .await?;

        Ok(())
    }
}

impl Default for RpcServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let _server = RpcServer::new();
    }
}
