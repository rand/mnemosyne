//! Agent Protocol - Message Format for P2P Communication
//!
//! Defines the protocol for sending AgentMessage over Iroh streams:
//! - Message serialization (bincode)
//! - Stream framing
//! - Error handling

use crate::error::{MnemosyneError, Result};
use crate::orchestration::messages::AgentMessage;
use iroh::net::endpoint::{RecvStream, SendStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Agent protocol for P2P communication
pub struct AgentProtocol;

impl AgentProtocol {
    /// Send a message over a stream
    pub async fn send_message(
        send: &mut SendStream,
        message: &AgentMessage,
    ) -> Result<()> {
        // Serialize message with bincode
        let data = bincode::serialize(message)
            .map_err(|e| MnemosyneError::SerializationError(e.to_string()))?;

        // Write length prefix (4 bytes, big-endian)
        let len = data.len() as u32;
        send.write_all(&len.to_be_bytes())
            .await
            .map_err(|e| MnemosyneError::NetworkError(e.to_string()))?;

        // Write message data
        send.write_all(&data)
            .await
            .map_err(|e| MnemosyneError::NetworkError(e.to_string()))?;

        // Flush stream
        send.flush()
            .await
            .map_err(|e| MnemosyneError::NetworkError(e.to_string()))?;

        Ok(())
    }

    /// Receive a message from a stream
    pub async fn recv_message(recv: &mut RecvStream) -> Result<AgentMessage> {
        // Read length prefix (4 bytes)
        let mut len_bytes = [0u8; 4];
        recv.read_exact(&mut len_bytes)
            .await
            .map_err(|e| MnemosyneError::NetworkError(e.to_string()))?;

        let len = u32::from_be_bytes(len_bytes) as usize;

        // Validate length (max 10MB)
        if len > 10 * 1024 * 1024 {
            return Err(MnemosyneError::NetworkError(
                format!("Message too large: {} bytes", len)
            ));
        }

        // Read message data
        let mut data = vec![0u8; len];
        recv.read_exact(&mut data)
            .await
            .map_err(|e| MnemosyneError::NetworkError(e.to_string()))?;

        // Deserialize message
        let message = bincode::deserialize(&data)
            .map_err(|e| MnemosyneError::SerializationError(e.to_string()))?;

        Ok(message)
    }

    /// Send and receive a request-response pair
    pub async fn request_response(
        send: &mut SendStream,
        recv: &mut RecvStream,
        request: &AgentMessage,
    ) -> Result<AgentMessage> {
        // Send request
        Self::send_message(send, request).await?;

        // Receive response
        Self::recv_message(recv).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::messages::OrchestratorMessage;

    // Note: Testing actual network I/O requires two endpoints
    // These tests verify serialization only

    #[test]
    fn test_message_serialization() {
        let message = AgentMessage::Orchestrator(OrchestratorMessage::Initialize);

        let data = bincode::serialize(&message).unwrap();
        let deserialized: AgentMessage = bincode::deserialize(&data).unwrap();

        assert!(matches!(
            deserialized,
            AgentMessage::Orchestrator(OrchestratorMessage::Initialize)
        ));
    }
}
