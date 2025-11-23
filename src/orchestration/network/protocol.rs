//! Agent Protocol - Message Format for P2P Communication
//!
//! Defines the protocol for sending AgentMessage over Iroh streams:
//! - Message serialization (bincode)
//! - Stream framing
//! - Error handling

use crate::error::{MnemosyneError, Result};
use crate::orchestration::messages::AgentMessage;
use iroh::net::endpoint::{RecvStream, SendStream};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HandshakeMessage {
    Hello { secret: Option<String> },
    Ack,
    Reject,
}

/// Wire format for agent messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WireMessage {
    /// Handshake message for authentication
    Handshake(HandshakeMessage),
    /// Version 1: Direct wrapper around AgentMessage
    V1(AgentMessage),
}

/// Agent protocol for P2P communication
pub struct AgentProtocol;

impl AgentProtocol {
    /// Perform handshake as the initiator (client)
    pub async fn handshake_initiator(
        send: &mut SendStream,
        recv: &mut RecvStream,
        secret: Option<String>,
    ) -> Result<()> {
        // Send Hello
        let hello = WireMessage::Handshake(HandshakeMessage::Hello { secret });
        Self::send_wire_message(send, &hello).await?;

        // Recv Ack
        let response = Self::recv_wire_message(recv).await?;
        match response {
            WireMessage::Handshake(HandshakeMessage::Ack) => Ok(()),
            WireMessage::Handshake(HandshakeMessage::Reject) => {
                Err(MnemosyneError::NetworkError("Handshake rejected".into()))
            }
            _ => Err(MnemosyneError::NetworkError(
                "Invalid handshake response".into(),
            )),
        }
    }

    /// Perform handshake as the responder (server)
    pub async fn handshake_responder(
        send: &mut SendStream,
        recv: &mut RecvStream,
        expected_secret: Option<String>,
    ) -> Result<()> {
        // Recv Hello
        let msg = Self::recv_wire_message(recv).await?;

        match msg {
            WireMessage::Handshake(HandshakeMessage::Hello { secret }) => {
                // Check secret
                // If expected_secret is None, we accept any secret (or no secret)
                // If expected_secret is Some, the received secret must match
                let valid = match &expected_secret {
                    None => true, // No secret required
                    Some(expected) => secret.as_ref() == Some(expected),
                };

                if valid {
                    let ack = WireMessage::Handshake(HandshakeMessage::Ack);
                    Self::send_wire_message(send, &ack).await?;
                    Ok(())
                } else {
                    let reject = WireMessage::Handshake(HandshakeMessage::Reject);
                    Self::send_wire_message(send, &reject).await?;
                    Err(MnemosyneError::NetworkError("Invalid secret".into()))
                }
            }
            _ => Err(MnemosyneError::NetworkError("Expected Hello".into())),
        }
    }

    /// Send a raw wire message
    async fn send_wire_message(send: &mut SendStream, wire_msg: &WireMessage) -> Result<()> {
        // Serialize message with bincode
        let data = bincode::serialize(wire_msg)
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

    /// Receive a raw wire message
    async fn recv_wire_message(recv: &mut RecvStream) -> Result<WireMessage> {
        // Read length prefix (4 bytes)
        let mut len_bytes = [0u8; 4];
        recv.read_exact(&mut len_bytes)
            .await
            .map_err(|e| MnemosyneError::NetworkError(e.to_string()))?;

        let len = u32::from_be_bytes(len_bytes) as usize;

        // Validate length (max 10MB)
        if len > 10 * 1024 * 1024 {
            return Err(MnemosyneError::NetworkError(format!(
                "Message too large: {} bytes",
                len
            )));
        }

        // Read message data
        let mut data = vec![0u8; len];
        recv.read_exact(&mut data)
            .await
            .map_err(|e| MnemosyneError::NetworkError(e.to_string()))?;

        // Deserialize message
        bincode::deserialize(&data).map_err(|e| MnemosyneError::SerializationError(e.to_string()))
    }

    /// Send a message over a stream
    pub async fn send_message(send: &mut SendStream, message: &AgentMessage) -> Result<()> {
        // Wrap in WireMessage
        let wire_msg = WireMessage::V1(message.clone());
        Self::send_wire_message(send, &wire_msg).await
    }

    /// Receive a message from a stream
    pub async fn recv_message(recv: &mut RecvStream) -> Result<AgentMessage> {
        let wire_msg = Self::recv_wire_message(recv).await?;

        match wire_msg {
            WireMessage::V1(msg) => Ok(msg),
            WireMessage::Handshake(_) => Err(MnemosyneError::NetworkError(
                "Unexpected handshake message".into(),
            )),
        }
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

    #[test]
    fn test_wire_message_serialization() {
        let message = AgentMessage::Orchestrator(OrchestratorMessage::Initialize);
        let wire_msg = WireMessage::V1(message);

        let data = bincode::serialize(&wire_msg).unwrap();
        let deserialized: WireMessage = bincode::deserialize(&data).unwrap();

        match deserialized {
            WireMessage::V1(msg) => {
                assert!(matches!(
                    msg,
                    AgentMessage::Orchestrator(OrchestratorMessage::Initialize)
                ));
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_handshake_serialization() {
        let handshake = WireMessage::Handshake(HandshakeMessage::Hello {
            secret: Some("secret".into()),
        });

        let data = bincode::serialize(&handshake).unwrap();
        let deserialized: WireMessage = bincode::deserialize(&data).unwrap();

        match deserialized {
            WireMessage::Handshake(HandshakeMessage::Hello { secret }) => {
                assert_eq!(secret, Some("secret".into()));
            }
            _ => panic!("Wrong message type"),
        }
    }
}
