//! CRDT synchronization protocol for multi-agent collaboration

use super::{Actor, CrdtBuffer};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

/// Sync message sent between buffers and agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMessage {
    /// Unique message ID
    pub id: Uuid,

    /// Buffer ID this message relates to
    pub buffer_id: usize,

    /// Actor sending the message
    pub from: Actor,

    /// Message payload
    pub payload: SyncPayload,
}

/// Sync message payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncPayload {
    /// CRDT changes to merge
    Changes(Vec<u8>),

    /// Full document state (for new subscribers)
    FullState(Vec<u8>),

    /// Request full state from buffer owner
    RequestState,

    /// Cursor position update (for awareness)
    CursorUpdate { line: usize, column: usize },

    /// Agent is proposing a change (not yet applied)
    Proposal {
        description: String,
        changes: Vec<u8>,
    },
}

/// Synchronization coordinator for a buffer
pub struct SyncCoordinator {
    /// Buffer being synchronized
    buffer_id: usize,

    /// Local actor
    local_actor: Actor,

    /// Broadcast channel for sending updates
    tx: broadcast::Sender<SyncMessage>,

    /// Receiver for incoming updates
    rx: broadcast::Receiver<SyncMessage>,

    /// Channel for sending to agents
    agent_tx: Option<mpsc::UnboundedSender<SyncMessage>>,
}

impl SyncCoordinator {
    /// Create new sync coordinator
    pub fn new(buffer_id: usize, local_actor: Actor) -> Self {
        let (tx, rx) = broadcast::channel(100);

        Self {
            buffer_id,
            local_actor,
            tx,
            rx,
            agent_tx: None,
        }
    }

    /// Connect to agent channel
    pub fn connect_agent_channel(&mut self, tx: mpsc::UnboundedSender<SyncMessage>) {
        self.agent_tx = Some(tx);
    }

    /// Subscribe to sync messages
    pub fn subscribe(&self) -> broadcast::Receiver<SyncMessage> {
        self.tx.subscribe()
    }

    /// Broadcast local changes
    pub fn broadcast_changes(&self, buffer: &mut CrdtBuffer) -> Result<()> {
        let changes = buffer.get_changes()?;
        if !changes.is_empty() {
            let msg = SyncMessage {
                id: Uuid::new_v4(),
                buffer_id: self.buffer_id,
                from: self.local_actor,
                payload: SyncPayload::Changes(changes),
            };

            // Broadcast to local subscribers
            let _ = self.tx.send(msg.clone());

            // Send to agents if connected
            if let Some(agent_tx) = &self.agent_tx {
                let _ = agent_tx.send(msg);
            }
        }

        Ok(())
    }

    /// Broadcast full state (for new subscribers)
    pub fn broadcast_state(&self, buffer: &mut CrdtBuffer) -> Result<()> {
        let state = buffer.save_state();

        let msg = SyncMessage {
            id: Uuid::new_v4(),
            buffer_id: self.buffer_id,
            from: self.local_actor,
            payload: SyncPayload::FullState(state),
        };

        let _ = self.tx.send(msg);

        Ok(())
    }

    /// Broadcast cursor position
    pub fn broadcast_cursor(&self, line: usize, column: usize) -> Result<()> {
        let msg = SyncMessage {
            id: Uuid::new_v4(),
            buffer_id: self.buffer_id,
            from: self.local_actor,
            payload: SyncPayload::CursorUpdate { line, column },
        };

        let _ = self.tx.send(msg);

        Ok(())
    }

    /// Receive next sync message (non-blocking)
    pub fn try_recv(&mut self) -> Option<SyncMessage> {
        match self.rx.try_recv() {
            Ok(msg) => {
                // Ignore our own messages
                if msg.from == self.local_actor {
                    None
                } else {
                    Some(msg)
                }
            }
            Err(broadcast::error::TryRecvError::Empty) => None,
            Err(broadcast::error::TryRecvError::Lagged(_)) => {
                // We lagged behind, request full state
                let msg = SyncMessage {
                    id: Uuid::new_v4(),
                    buffer_id: self.buffer_id,
                    from: self.local_actor,
                    payload: SyncPayload::RequestState,
                };
                let _ = self.tx.send(msg);
                None
            }
            Err(broadcast::error::TryRecvError::Closed) => None,
        }
    }

    /// Apply sync message to buffer
    pub fn apply_message(&self, msg: &SyncMessage, buffer: &mut CrdtBuffer) -> Result<()> {
        match &msg.payload {
            SyncPayload::Changes(changes) => {
                buffer.merge_changes(changes)?;
            }
            SyncPayload::FullState(state) => {
                buffer.load_state(state)?;
            }
            SyncPayload::RequestState => {
                // Send our full state
                self.broadcast_state(buffer)?;
            }
            SyncPayload::CursorUpdate { .. } => {
                // Cursor updates are handled by UI layer
            }
            SyncPayload::Proposal { .. } => {
                // Proposals are handled by UI layer (show as suggestions)
            }
        }

        Ok(())
    }
}

/// Awareness information for collaborative editing
#[derive(Debug, Clone)]
pub struct Awareness {
    /// Actor's cursor position
    pub cursor: Option<(usize, usize)>,

    /// Actor's current selection
    pub selection: Option<(usize, usize)>,

    /// Last seen timestamp
    pub last_seen: std::time::Instant,
}

/// Awareness tracker for all collaborators
pub struct AwarenessTracker {
    /// Awareness state for each actor
    states: std::collections::HashMap<Actor, Awareness>,
}

impl AwarenessTracker {
    /// Create new awareness tracker
    pub fn new() -> Self {
        Self {
            states: std::collections::HashMap::new(),
        }
    }

    /// Update actor's awareness
    pub fn update(&mut self, actor: Actor, cursor: Option<(usize, usize)>) {
        self.states.insert(
            actor,
            Awareness {
                cursor,
                selection: None,
                last_seen: std::time::Instant::now(),
            },
        );
    }

    /// Get awareness for an actor
    pub fn get(&self, actor: &Actor) -> Option<&Awareness> {
        self.states.get(actor)
    }

    /// Get all active collaborators (seen in last 10 seconds)
    pub fn active_collaborators(&self) -> Vec<Actor> {
        let now = std::time::Instant::now();
        self.states
            .iter()
            .filter(|(_, awareness)| now.duration_since(awareness.last_seen).as_secs() < 10)
            .map(|(actor, _)| *actor)
            .collect()
    }

    /// Clean up stale awareness (not seen in 30 seconds)
    pub fn cleanup(&mut self) {
        let now = std::time::Instant::now();
        self.states
            .retain(|_, awareness| now.duration_since(awareness.last_seen).as_secs() < 30);
    }
}

impl Default for AwarenessTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sync_coordinator_broadcast() {
        let coord = SyncCoordinator::new(0, Actor::Human);
        let mut subscriber = coord.subscribe();

        let mut buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();
        coord.broadcast_state(&mut buffer).unwrap();

        let msg = subscriber.recv().await.unwrap();
        assert_eq!(msg.buffer_id, 0);
        assert_eq!(msg.from, Actor::Human);
        assert!(matches!(msg.payload, SyncPayload::FullState(_)));
    }

    #[test]
    fn test_awareness_tracker() {
        let mut tracker = AwarenessTracker::new();
        tracker.update(Actor::Human, Some((1, 5)));
        tracker.update(Actor::Optimizer, Some((2, 10)));

        let active = tracker.active_collaborators();
        assert_eq!(active.len(), 2);
        assert!(active.contains(&Actor::Human));
        assert!(active.contains(&Actor::Optimizer));
    }

    #[test]
    fn test_awareness_cleanup() {
        let mut tracker = AwarenessTracker::new();
        tracker.update(Actor::Human, Some((1, 5)));

        // Manually age the awareness
        if let Some(awareness) = tracker.states.get_mut(&Actor::Human) {
            awareness.last_seen = std::time::Instant::now() - std::time::Duration::from_secs(31);
        }

        tracker.cleanup();
        assert_eq!(tracker.active_collaborators().len(), 0);
    }
}
