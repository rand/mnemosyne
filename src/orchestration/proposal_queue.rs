//! Proposal Queue for Agent-to-ICS Communication
//!
//! Provides a thread-safe queue for agents to send change proposals to ICS.
//! ICS can poll the queue to receive and display proposals.

use crate::ics::proposals::ChangeProposal;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Proposal queue for agent-to-ICS communication
#[derive(Clone)]
pub struct ProposalQueue {
    /// Sender for agents to push proposals
    sender: mpsc::UnboundedSender<ChangeProposal>,
    /// Receiver for ICS to poll proposals (wrapped in Arc for cloning)
    receiver: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<ChangeProposal>>>,
}

impl ProposalQueue {
    /// Create a new proposal queue
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            sender,
            receiver: Arc::new(tokio::sync::Mutex::new(receiver)),
        }
    }

    /// Send a proposal to the queue (called by agents)
    ///
    /// This is a non-blocking operation that always succeeds unless the receiver
    /// has been dropped.
    pub fn send(&self, proposal: ChangeProposal) -> Result<(), SendError> {
        self.sender
            .send(proposal)
            .map_err(|_| SendError::ReceiverDropped)
    }

    /// Try to receive all pending proposals (called by ICS)
    ///
    /// Returns all proposals currently in the queue without blocking.
    /// If the queue is empty, returns an empty vector.
    pub async fn try_recv_all(&self) -> Vec<ChangeProposal> {
        let mut proposals = Vec::new();
        let mut receiver = self.receiver.lock().await;

        while let Ok(proposal) = receiver.try_recv() {
            proposals.push(proposal);
        }

        proposals
    }

    /// Receive a single proposal (blocking until one arrives)
    ///
    /// This will block until a proposal is available or the sender is dropped.
    pub async fn recv(&self) -> Option<ChangeProposal> {
        let mut receiver = self.receiver.lock().await;
        receiver.recv().await
    }

    /// Check if there are any pending proposals
    pub async fn has_pending(&self) -> bool {
        let receiver = self.receiver.lock().await;
        !receiver.is_empty()
    }

    /// Get a sender handle for agents to use
    pub fn get_sender(&self) -> ProposalSender {
        ProposalSender {
            sender: self.sender.clone(),
        }
    }
}

impl Default for ProposalQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Sender handle for agents
///
/// This is a lightweight handle that agents can use to send proposals.
/// It can be cloned and sent across threads.
#[derive(Clone)]
pub struct ProposalSender {
    sender: mpsc::UnboundedSender<ChangeProposal>,
}

impl ProposalSender {
    /// Send a proposal
    pub fn send(&self, proposal: ChangeProposal) -> Result<(), SendError> {
        self.sender
            .send(proposal)
            .map_err(|_| SendError::ReceiverDropped)
    }
}

/// Error types for sending proposals
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendError {
    /// The receiver (ICS) has been dropped
    ReceiverDropped,
}

impl std::fmt::Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SendError::ReceiverDropped => write!(f, "Receiver dropped"),
        }
    }
}

impl std::error::Error for SendError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ics::proposals::ProposalStatus;
    use std::time::SystemTime;

    fn create_test_proposal(id: &str) -> ChangeProposal {
        ChangeProposal {
            id: id.to_string(),
            agent: "TestAgent".to_string(),
            description: "Test proposal".to_string(),
            original: "old text".to_string(),
            proposed: "new text".to_string(),
            line_range: (1, 2),
            created_at: SystemTime::now(),
            status: ProposalStatus::Pending,
            rationale: "Testing".to_string(),
        }
    }

    #[tokio::test]
    async fn test_send_and_receive() {
        let queue = ProposalQueue::new();
        let proposal = create_test_proposal("test-1");

        // Send a proposal
        queue.send(proposal.clone()).unwrap();

        // Receive it
        let received = queue.recv().await.unwrap();
        assert_eq!(received.id, "test-1");
    }

    #[tokio::test]
    async fn test_try_recv_all() {
        let queue = ProposalQueue::new();

        // Send multiple proposals
        queue.send(create_test_proposal("test-1")).unwrap();
        queue.send(create_test_proposal("test-2")).unwrap();
        queue.send(create_test_proposal("test-3")).unwrap();

        // Receive all at once
        let proposals = queue.try_recv_all().await;
        assert_eq!(proposals.len(), 3);
        assert_eq!(proposals[0].id, "test-1");
        assert_eq!(proposals[1].id, "test-2");
        assert_eq!(proposals[2].id, "test-3");
    }

    #[tokio::test]
    async fn test_try_recv_all_empty() {
        let queue = ProposalQueue::new();

        // Try to receive from empty queue
        let proposals = queue.try_recv_all().await;
        assert_eq!(proposals.len(), 0);
    }

    #[tokio::test]
    async fn test_has_pending() {
        let queue = ProposalQueue::new();

        // Initially empty
        assert!(!queue.has_pending().await);

        // Send a proposal
        queue.send(create_test_proposal("test-1")).unwrap();

        // Now has pending
        assert!(queue.has_pending().await);

        // Receive it
        queue.recv().await.unwrap();

        // Empty again
        assert!(!queue.has_pending().await);
    }

    #[tokio::test]
    async fn test_sender_handle() {
        let queue = ProposalQueue::new();
        let sender = queue.get_sender();

        // Send via sender handle
        sender.send(create_test_proposal("test-1")).unwrap();

        // Receive via queue
        let received = queue.recv().await.unwrap();
        assert_eq!(received.id, "test-1");
    }

    #[tokio::test]
    async fn test_multiple_senders() {
        let queue = ProposalQueue::new();
        let sender1 = queue.get_sender();
        let sender2 = queue.get_sender();

        // Send from different senders
        sender1.send(create_test_proposal("from-1")).unwrap();
        sender2.send(create_test_proposal("from-2")).unwrap();

        // Receive all
        let proposals = queue.try_recv_all().await;
        assert_eq!(proposals.len(), 2);
    }
}
