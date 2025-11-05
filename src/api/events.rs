//! Event types and Server-Sent Events (SSE) endpoint

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Event type discriminant
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventType {
    /// Agent started
    AgentStarted {
        agent_id: String,
        timestamp: DateTime<Utc>,
    },
    /// Agent completed task
    AgentCompleted {
        agent_id: String,
        result: String,
        timestamp: DateTime<Utc>,
    },
    /// Agent failed
    AgentFailed {
        agent_id: String,
        error: String,
        timestamp: DateTime<Utc>,
    },
    /// Memory stored
    MemoryStored {
        memory_id: String,
        summary: String,
        timestamp: DateTime<Utc>,
    },
    /// Memory recalled
    MemoryRecalled {
        query: String,
        count: usize,
        timestamp: DateTime<Utc>,
    },
    /// Context file modified
    ContextModified {
        file: String,
        timestamp: DateTime<Utc>,
    },
    /// Context validated
    ContextValidated {
        file: String,
        errors: Vec<String>,
        timestamp: DateTime<Utc>,
    },
    /// System health update
    HealthUpdate {
        memory_mb: f32,
        cpu_percent: f32,
        timestamp: DateTime<Utc>,
    },
    /// Session started
    SessionStarted {
        instance_id: String,
        timestamp: DateTime<Utc>,
    },
    /// Heartbeat (published periodically when idle)
    Heartbeat {
        instance_id: String,
        timestamp: DateTime<Utc>,
    },
}

/// Event wrapper with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Event ID (for deduplication)
    pub id: String,
    /// Instance ID (for multi-instance coordination)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    /// Event payload
    #[serde(flatten)]
    pub event_type: EventType,
}

impl Event {
    /// Create new event
    pub fn new(event_type: EventType) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            instance_id: None,
            event_type,
        }
    }

    /// Create new event with instance ID
    pub fn new_with_instance(event_type: EventType, instance_id: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            instance_id: Some(instance_id),
            event_type,
        }
    }

    /// Create agent started event
    pub fn agent_started(agent_id: String) -> Self {
        Self::new(EventType::AgentStarted {
            agent_id,
            timestamp: Utc::now(),
        })
    }

    /// Create agent completed event
    pub fn agent_completed(agent_id: String, result: String) -> Self {
        Self::new(EventType::AgentCompleted {
            agent_id,
            result,
            timestamp: Utc::now(),
        })
    }

    /// Create agent failed event
    pub fn agent_failed(agent_id: String, error: String) -> Self {
        Self::new(EventType::AgentFailed {
            agent_id,
            error,
            timestamp: Utc::now(),
        })
    }

    /// Create memory stored event
    pub fn memory_stored(memory_id: String, summary: String) -> Self {
        Self::new(EventType::MemoryStored {
            memory_id,
            summary,
            timestamp: Utc::now(),
        })
    }

    /// Create memory recalled event
    pub fn memory_recalled(query: String, count: usize) -> Self {
        Self::new(EventType::MemoryRecalled {
            query,
            count,
            timestamp: Utc::now(),
        })
    }

    /// Create context modified event
    pub fn context_modified(file: String) -> Self {
        Self::new(EventType::ContextModified {
            file,
            timestamp: Utc::now(),
        })
    }

    /// Create context validated event
    pub fn context_validated(file: String, errors: Vec<String>) -> Self {
        Self::new(EventType::ContextValidated {
            file,
            errors,
            timestamp: Utc::now(),
        })
    }

    /// Create health update event
    pub fn health_update(memory_mb: f32, cpu_percent: f32) -> Self {
        Self::new(EventType::HealthUpdate {
            memory_mb,
            cpu_percent,
            timestamp: Utc::now(),
        })
    }

    /// Create session started event
    pub fn session_started(instance_id: String) -> Self {
        Self::new(EventType::SessionStarted {
            instance_id,
            timestamp: Utc::now(),
        })
    }

    /// Create heartbeat event
    pub fn heartbeat(instance_id: String) -> Self {
        Self::new(EventType::Heartbeat {
            instance_id,
            timestamp: Utc::now(),
        })
    }

    /// Convert to SSE data format
    pub fn to_sse(&self) -> String {
        format!(
            "id: {}\ndata: {}\n\n",
            self.id,
            serde_json::to_string(&self).unwrap_or_else(|_| "{}".to_string())
        )
    }
}

/// Event broadcaster using tokio broadcast channel
#[derive(Clone)]
pub struct EventBroadcaster {
    tx: broadcast::Sender<Event>,
}

impl EventBroadcaster {
    /// Create new broadcaster with channel capacity
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Broadcast event to all subscribers
    pub fn broadcast(&self, event: Event) -> Result<usize, broadcast::error::SendError<Event>> {
        self.tx.send(event)
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }

    /// Get subscriber count
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Default for EventBroadcaster {
    fn default() -> Self {
        Self::new(1000) // Default capacity: 1000 events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = Event::agent_started("executor".to_string());
        match event.event_type {
            EventType::AgentStarted { agent_id, .. } => {
                assert_eq!(agent_id, "executor");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_sse_format() {
        let event = Event::memory_stored(
            "mem-123".to_string(),
            "Test memory".to_string(),
        );
        let sse = event.to_sse();
        assert!(sse.contains("id:"));
        assert!(sse.contains("data:"));
        assert!(sse.contains("memory_stored"));
    }

    #[tokio::test]
    async fn test_broadcaster() {
        let broadcaster = EventBroadcaster::new(10);
        let mut rx = broadcaster.subscribe();

        let event = Event::agent_started("test".to_string());
        broadcaster.broadcast(event.clone()).unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received.id, event.id);
    }
}
