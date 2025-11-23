use super::orchestration::OrchestrationStatus;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info, warn};

/// IPC Message for internal communication between IPC server and Orchestration engine
#[derive(Debug)]
pub enum IpcMessage {
    /// Request for status
    GetStatus(oneshot::Sender<OrchestrationStatus>),
}

/// IPC Command sent over the wire
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "command", content = "args")]
pub enum IpcCommand {
    /// Get status
    #[serde(rename = "status")]
    GetStatus,
}

/// Start the IPC server
pub async fn start_ipc_server(
    socket_path: PathBuf,
    tx: mpsc::Sender<IpcMessage>,
) -> Result<()> {
    // Remove existing socket if present
    if socket_path.exists() {
        tokio::fs::remove_file(&socket_path).await.map_err(|e| {
            crate::error::MnemosyneError::Other(format!("Failed to remove existing socket: {}", e))
        })?;
    }

    info!("Starting IPC server on {}", socket_path.display());
    let listener = UnixListener::bind(&socket_path).map_err(|e| {
        crate::error::MnemosyneError::Other(format!("Failed to bind IPC socket: {}", e))
    })?;

    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((mut stream, _addr)) => {
                    let tx = tx.clone();
                    tokio::spawn(async move {
                        let (reader, mut writer) = stream.split();
                        let mut reader = BufReader::new(reader);
                        let mut line = String::new();

                        // Read command
                        match reader.read_line(&mut line).await {
                            Ok(0) => return, // connection closed
                            Ok(_) => {
                                // Parse command
                                let command_result: std::result::Result<IpcCommand, _> = serde_json::from_str(&line);
                                match command_result {
                                    Ok(IpcCommand::GetStatus) => {
                                        let (resp_tx, resp_rx) = oneshot::channel();
                                        if let Err(e) = tx.send(IpcMessage::GetStatus(resp_tx)).await {
                                            error!("Failed to send IPC message to engine: {}", e);
                                            return;
                                        }

                                        match resp_rx.await {
                                            Ok(status) => {
                                                match serde_json::to_string(&status) {
                                                    Ok(json) => {
                                                        if let Err(e) = writer.write_all(json.as_bytes()).await {
                                                            error!("Failed to write IPC response: {}", e);
                                                        }
                                                        // Write newline delimiter
                                                        if let Err(e) = writer.write_all(b"\n").await {
                                                            error!("Failed to write newline: {}", e);
                                                        }
                                                    }
                                                    Err(e) => error!("Failed to serialize status: {}", e),
                                                }
                                            }
                                            Err(e) => error!("Failed to receive response from engine: {}", e),
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Invalid IPC command received: {}. Error: {}", line, e);
                                    }
                                }
                            }
                            Err(e) => error!("Failed to read from IPC socket: {}", e),
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept IPC connection: {}", e);
                }
            }
        }
    });

    Ok(())
}

/// Client to query status via IPC
pub async fn query_status(socket_path: &Path) -> Result<OrchestrationStatus> {
    use tokio::io::AsyncWriteExt;
    use tokio::net::UnixStream;

    let mut stream = UnixStream::connect(socket_path).await.map_err(|e| {
        crate::error::MnemosyneError::Other(format!("Failed to connect to IPC socket: {}", e))
    })?;

    let command = IpcCommand::GetStatus;
    let json = serde_json::to_string(&command).map_err(|e| {
        crate::error::MnemosyneError::Other(format!("Failed to serialize command: {}", e))
    })?;

    stream.write_all(json.as_bytes()).await.map_err(|e| {
        crate::error::MnemosyneError::Other(format!("Failed to write to IPC socket: {}", e))
    })?;
    stream.write_all(b"\n").await.map_err(|e| {
        crate::error::MnemosyneError::Other(format!("Failed to write newline: {}", e))
    })?;

    let reader = BufReader::new(stream);
    let mut lines = reader.lines();

    if let Some(line) = lines.next_line().await.map_err(|e| {
        crate::error::MnemosyneError::Other(format!("Failed to read from IPC socket: {}", e))
    })? {
        let status: OrchestrationStatus = serde_json::from_str(&line).map_err(|e| {
            crate::error::MnemosyneError::Other(format!("Failed to deserialize status: {}", e))
        })?;
        Ok(status)
    } else {
        Err(crate::error::MnemosyneError::Other(
            "IPC socket closed without response".to_string(),
        ))
    }
}
