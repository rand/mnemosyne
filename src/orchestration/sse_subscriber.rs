//! SSE Subscriber for CLI Event Streaming
//!
//! Subscribes to the API server's `/events/stream` endpoint via Server-Sent Events (SSE)
//! and forwards relevant CLI events to the orchestrator for coordination.
//!
//! # Purpose
//!
//! Enables bidirectional event flow:
//! - CLI → API server → Dashboard (observability)
//! - CLI → API server → SSE → Orchestrator (coordination)
//!
//! This allows the orchestrator to react to:
//! - Memory operations (remember/recall)
//! - CLI commands (status, doctor, export, etc.)
//! - Session lifecycle (started/ended)
//! - Database operations
//!
//! # Architecture
//!
//! ```text
//! CLI Command
//!     ↓ (HTTP POST /events/emit)
//! API Server
//!     ↓ (SSE /events/stream)
//! SSE Subscriber (this module)
//!     ↓ (CliEventReceived message)
//! Orchestrator Actor
//! ```
//!
//! # Reconnection
//!
//! Uses exponential backoff for reconnection:
//! - Base: 1 second
//! - Max: 60 seconds
//! - Resets on successful connection

use crate::api::Event as ApiEvent;
use crate::orchestration::events::AgentEvent;
use crate::orchestration::messages::OrchestratorMessage;
use eventsource_client as es;
use ractor::ActorRef;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

/// SSE subscriber configuration
#[derive(Debug, Clone)]
pub struct SseSubscriberConfig {
    /// API server URL
    pub api_url: String,
    /// Initial reconnection delay
    pub reconnect_delay_secs: u64,
    /// Maximum reconnection delay
    pub max_reconnect_delay_secs: u64,
}

impl Default for SseSubscriberConfig {
    fn default() -> Self {
        Self {
            api_url: "http://localhost:3000".to_string(),
            reconnect_delay_secs: 1,
            max_reconnect_delay_secs: 60,
        }
    }
}

/// SSE subscriber for orchestrator event subscription
pub struct SseSubscriber {
    /// Configuration
    config: SseSubscriberConfig,
    /// Reference to orchestrator
    orchestrator: ActorRef<OrchestratorMessage>,
    /// Shutdown signal
    shutdown_rx: broadcast::Receiver<()>,
}

impl SseSubscriber {
    /// Create a new SSE subscriber
    pub fn new(
        config: SseSubscriberConfig,
        orchestrator: ActorRef<OrchestratorMessage>,
        shutdown_rx: broadcast::Receiver<()>,
    ) -> Self {
        Self {
            config,
            orchestrator,
            shutdown_rx,
        }
    }

    /// Start subscribing to SSE events
    ///
    /// Runs in a loop with reconnection on failure.
    /// Returns when shutdown signal is received.
    pub async fn run(mut self) {
        info!("SSE subscriber starting: {}/events/stream", self.config.api_url);

        let mut reconnect_delay = self.config.reconnect_delay_secs;

        loop {
            // Check for shutdown before connecting
            if self.shutdown_rx.try_recv().is_ok() {
                info!("SSE subscriber: shutdown signal received");
                break;
            }

            // Build SSE client
            let stream_url = format!("{}/events/stream", self.config.api_url);
            debug!("SSE subscriber: connecting to {}", stream_url);

            let mut client = match es::ClientBuilder::for_url(&stream_url) {
                Ok(builder) => {
                    let reconnect_opts = es::ReconnectOptions::reconnect(true)
                        .retry_initial(false) // We handle our own reconnection logic
                        .delay(Duration::from_secs(reconnect_delay))
                        .build();
                    builder.reconnect(reconnect_opts).build()
                }
                Err(e) => {
                    error!("SSE subscriber: failed to build client: {}", e);
                    // Wait before retry
                    tokio::time::sleep(Duration::from_secs(reconnect_delay)).await;
                    reconnect_delay = (reconnect_delay * 2).min(self.config.max_reconnect_delay_secs);
                    continue;
                }
            };

            // Stream events
            match self.stream_events(&mut client).await {
                Ok(_) => {
                    info!("SSE subscriber: stream ended gracefully");
                    // Reset reconnection delay on successful connection
                    reconnect_delay = self.config.reconnect_delay_secs;
                }
                Err(e) => {
                    warn!("SSE subscriber: stream error: {}", e);
                    // Exponential backoff
                    reconnect_delay = (reconnect_delay * 2).min(self.config.max_reconnect_delay_secs);
                    debug!("SSE subscriber: reconnecting in {} seconds", reconnect_delay);
                    tokio::time::sleep(Duration::from_secs(reconnect_delay)).await;
                }
            }

            // Check for shutdown after error
            if self.shutdown_rx.try_recv().is_ok() {
                info!("SSE subscriber: shutdown signal received after error");
                break;
            }
        }

        info!("SSE subscriber stopped");
    }

    /// Stream events from SSE client
    async fn stream_events(&mut self, client: &mut impl es::Client) -> Result<(), String> {
        use tokio_stream::StreamExt;

        info!("SSE subscriber: connected, streaming events");

        let mut stream = client.stream();

        loop {
            // Check for shutdown
            if self.shutdown_rx.try_recv().is_ok() {
                debug!("SSE subscriber: shutdown during streaming");
                return Ok(());
            }

            // Receive next SSE event with timeout
            let event_result = tokio::time::timeout(
                Duration::from_secs(5), // Check shutdown every 5s
                stream.next(),
            )
            .await;

            match event_result {
                Ok(Some(Ok(es::SSE::Connected(_)))) => {
                    // Connection established
                    debug!("SSE subscriber: connected to event stream");
                }
                Ok(Some(Ok(es::SSE::Event(event)))) => {
                    debug!("SSE subscriber: received event (type: {})", event.event_type);
                    if let Err(e) = self.handle_sse_event(event).await {
                        warn!("SSE subscriber: failed to handle event: {}", e);
                    }
                }
                Ok(Some(Ok(es::SSE::Comment(_comment)))) => {
                    // SSE comment (keepalive), ignore
                    debug!("SSE subscriber: received keepalive comment");
                }
                Ok(Some(Err(e))) => {
                    return Err(format!("SSE stream error: {}", e));
                }
                Ok(None) => {
                    // Stream ended
                    return Ok(());
                }
                Err(_timeout) => {
                    // Timeout elapsed, loop again to check shutdown
                    continue;
                }
            }
        }
    }

    /// Handle a single SSE event
    async fn handle_sse_event(&self, event: es::Event) -> Result<(), String> {
        // Parse event data as ApiEvent
        let api_event: ApiEvent = serde_json::from_str(&event.data)
            .map_err(|e| format!("Failed to parse event JSON: {}", e))?;

        // Convert to AgentEvent (if applicable)
        if let Some(agent_event) = convert_api_event_to_agent_event(&api_event) {
            // Send to orchestrator
            debug!(
                "SSE subscriber: forwarding CLI event to orchestrator: {}",
                agent_event.summary()
            );

            let message = OrchestratorMessage::CliEventReceived {
                event: agent_event,
            };

            if let Err(e) = self.orchestrator.cast(message) {
                warn!(
                    "SSE subscriber: failed to send CliEventReceived to orchestrator: {}",
                    e
                );
            }
        } else {
            // Event type not relevant for orchestration
            debug!(
                "SSE subscriber: skipping non-CLI event (id: {})",
                api_event.id
            );
        }

        Ok(())
    }
}

/// Convert API Event to AgentEvent (if applicable)
///
/// Only CLI-related events are converted for orchestrator coordination.
/// Returns None for events that are purely observability (dashboard-only).
fn convert_api_event_to_agent_event(api_event: &ApiEvent) -> Option<AgentEvent> {
    use crate::api::EventType;

    match &api_event.event_type {
        // CLI command events
        EventType::CliCommandStarted {
            command,
            args,
            timestamp,
        } => Some(AgentEvent::CliCommandStarted {
            command: command.clone(),
            args: args.clone(),
            timestamp: *timestamp,
        }),

        EventType::CliCommandCompleted {
            command,
            duration_ms,
            result_summary,
            ..
        } => Some(AgentEvent::CliCommandCompleted {
            command: command.clone(),
            duration_ms: *duration_ms,
            result_summary: result_summary.clone(),
        }),

        EventType::CliCommandFailed {
            command,
            error,
            duration_ms,
            ..
        } => Some(AgentEvent::CliCommandFailed {
            command: command.clone(),
            error: error.clone(),
            duration_ms: *duration_ms,
        }),

        // Memory operations
        EventType::MemoryStored {
            memory_id,
            summary,
            ..
        } => Some(AgentEvent::RememberExecuted {
            content_preview: summary.clone(),
            importance: 5, // Default
            memory_id: crate::types::MemoryId::from_string(memory_id).unwrap_or_default(),
        }),

        EventType::MemoryRecalled {
            query,
            count,
            ..
        } => Some(AgentEvent::RecallExecuted {
            query: query.clone(),
            result_count: *count,
            duration_ms: 0, // Not available in API event
        }),

        // Database operations
        EventType::DatabaseOperation {
            operation,
            table,
            affected_rows,
            duration_ms,
            ..
        } => Some(AgentEvent::DatabaseOperation {
            operation: operation.clone(),
            table: table.clone(),
            affected_rows: *affected_rows,
            duration_ms: *duration_ms,
        }),

        EventType::SearchPerformed {
            query,
            search_type,
            result_count,
            duration_ms,
            ..
        } => Some(AgentEvent::SearchPerformed {
            query: query.clone(),
            search_type: search_type.clone(),
            result_count: *result_count,
            duration_ms: *duration_ms,
        }),

        // Session lifecycle
        EventType::SessionStarted {
            instance_id,
            timestamp,
        } => Some(AgentEvent::SessionStarted {
            instance_id: instance_id.clone(),
            timestamp: *timestamp,
        }),

        // Other events are dashboard-only (not needed for orchestration)
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::EventType;

    #[test]
    fn test_convert_cli_command_started() {
        let api_event = ApiEvent {
            id: "test-1".to_string(),
            instance_id: None,
            event_type: EventType::CliCommandStarted {
                command: "status".to_string(),
                args: vec![],
                timestamp: chrono::Utc::now(),
            },
        };

        let agent_event = convert_api_event_to_agent_event(&api_event);
        assert!(agent_event.is_some());

        if let Some(AgentEvent::CliCommandStarted { command, .. }) = agent_event {
            assert_eq!(command, "status");
        } else {
            panic!("Expected CliCommandStarted");
        }
    }

    #[test]
    fn test_convert_memory_stored() {
        let api_event = ApiEvent {
            id: "test-2".to_string(),
            instance_id: None,
            event_type: EventType::MemoryStored {
                memory_id: "mem-123".to_string(),
                summary: "Test memory".to_string(),
                timestamp: chrono::Utc::now(),
            },
        };

        let agent_event = convert_api_event_to_agent_event(&api_event);
        assert!(agent_event.is_some());

        if let Some(AgentEvent::RememberExecuted { content_preview, .. }) = agent_event {
            assert_eq!(content_preview, "Test memory");
        } else {
            panic!("Expected RememberExecuted");
        }
    }

    #[test]
    fn test_convert_heartbeat_skipped() {
        let api_event = ApiEvent {
            id: "test-3".to_string(),
            instance_id: None,
            event_type: EventType::Heartbeat {
                instance_id: "test".to_string(),
                timestamp: chrono::Utc::now(),
            },
        };

        let agent_event = convert_api_event_to_agent_event(&api_event);
        assert!(agent_event.is_none()); // Heartbeat is dashboard-only
    }
}
