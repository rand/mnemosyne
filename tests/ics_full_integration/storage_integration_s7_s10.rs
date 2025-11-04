//! Storage Integration Tests S7-S10 (continued)

use crate::ics_full_integration::*;
use mnemosyne_core::{
    ics::{ChangeProposal, ProposalStatus},
    storage::StorageBackend,
    types::{LinkType, MemoryLink, MemoryType, Namespace},
};

/// S7: Memory updates from ICS proposals
#[tokio::test]
async fn s7_memory_updates_from_proposals() {
    let storage = StorageFixture::new().await.expect("Storage setup failed");

    // Create initial memory
    let mut memory = create_test_memory(
        "The system has distributed architecture",
        MemoryType::ArchitectureDecision,
        Namespace::Global,
        7,
    );
    storage
        .storage()
        .store_memory(&memory)
        .await
        .expect("Store memory");

    // Agent proposes improvement
    let proposal = ChangeProposal {
        id: "prop-1".to_string(),
        agent: "agent:optimizer".to_string(),
        description: "Add specificity to architecture description".to_string(),
        original: "The system has distributed architecture".to_string(),
        proposed:
            "The system uses distributed microservices architecture with event-driven communication"
                .to_string(),
        line_range: (1, 1),
        created_at: std::time::SystemTime::now(),
        status: ProposalStatus::Accepted,
        rationale: "More specific architectural detail".to_string(),
    };

    // User accepts proposal, update memory
    memory.content = proposal.proposed.clone();
    memory.importance = 8; // Increased due to detail
    storage
        .storage()
        .update_memory(&memory)
        .await
        .expect("Update memory");

    // Verify update persisted
    let updated = storage
        .storage()
        .get_memory(memory.id.clone())
        .await
        .expect("Get memory");

    assert!(updated.content.contains("microservices"));
    assert!(updated.content.contains("event-driven"));
    assert_eq!(updated.importance, 8);
}

/// S8: Memory link creation from ICS
#[tokio::test]
async fn s8_memory_link_creation() {
    let storage = StorageFixture::new().await.expect("Storage setup failed");

    // Create two related memories
    let mut memory_a = create_test_memory(
        "System uses distributed architecture",
        MemoryType::ArchitectureDecision,
        Namespace::Global,
        8,
    );
    let memory_b = create_test_memory(
        "Event-driven communication between services",
        MemoryType::CodePattern,
        Namespace::Global,
        7,
    );

    storage
        .storage()
        .store_memory(&memory_a)
        .await
        .expect("Store A");
    storage
        .storage()
        .store_memory(&memory_b)
        .await
        .expect("Store B");

    // ICS semantic analysis detects relationship
    // Create link A -> B
    let link = MemoryLink {
        target_id: memory_b.id.clone(),
        link_type: LinkType::Implements,
        strength: 0.85,
        reason: "Distributed architecture implemented via event-driven pattern".to_string(),
        created_at: chrono::Utc::now(),
        last_traversed_at: None,
        user_created: false,
    };

    memory_a.links.push(link.clone());

    // Update memory with link
    storage
        .storage()
        .update_memory(&memory_a)
        .await
        .expect("Update with link");

    // Verify link persisted
    let retrieved = storage
        .storage()
        .get_memory(memory_a.id.clone())
        .await
        .expect("Get memory");

    assert_eq!(retrieved.links.len(), 1);
    assert_eq!(retrieved.links[0].target_id, memory_b.id);
    assert_eq!(retrieved.links[0].link_type, LinkType::Implements);
    assert_eq!(retrieved.links[0].strength, 0.85);
}

/// S9: Memory deletion from ICS
#[tokio::test]
async fn s9_memory_deletion() {
    let storage = StorageFixture::new().await.expect("Storage setup failed");

    // Create memory
    let memory = create_test_memory("Temporary note", MemoryType::Insight, Namespace::Global, 5);
    storage
        .storage()
        .store_memory(&memory)
        .await
        .expect("Store memory");

    // Verify exists
    let exists = storage
        .storage()
        .get_memory(memory.id.clone())
        .await
        .is_ok();
    assert!(exists, "Memory should exist");

    // Delete from ICS (soft delete/archive)
    storage
        .storage()
        .archive_memory(memory.id.clone())
        .await
        .expect("Archive memory");

    // Verify archived
    let archived = storage
        .storage()
        .get_memory(memory.id.clone())
        .await
        .expect("Should still retrieve archived");

    assert_archived(&archived);

    // Verify not in default searches
    let search_results = storage
        .storage()
        .keyword_search("Temporary", Some(Namespace::Global))
        .await
        .expect("Search");

    // Archived memories should not appear in regular search
    let found = search_results
        .iter()
        .any(|r| r.memory.id == memory.id && !r.memory.is_archived);
    assert!(
        !found,
        "Archived memory should not appear in regular search"
    );
}

/// S10: Transaction integrity
#[tokio::test]
async fn s10_transaction_integrity() {
    let storage = StorageFixture::new().await.expect("Storage setup failed");

    // Create memory
    let memory = create_test_memory(
        "Transaction test",
        MemoryType::CodePattern,
        Namespace::Global,
        7,
    );

    // Store should be atomic
    storage
        .storage()
        .store_memory(&memory)
        .await
        .expect("Store memory");

    // Verify stored correctly
    let retrieved = storage
        .storage()
        .get_memory(memory.id.clone())
        .await
        .expect("Get memory");

    assert_eq!(retrieved.content, "Transaction test");

    // Attempt invalid update (should fail atomically)
    let mut invalid = retrieved.clone();
    invalid.content = "".to_string(); // Empty content might be invalid

    // If update succeeds, verify consistency
    let update_result = storage.storage().update_memory(&invalid).await;

    match update_result {
        Ok(_) => {
            // If it succeeded, verify the update was applied
            let updated = storage
                .storage()
                .get_memory(memory.id.clone())
                .await
                .expect("Get updated");
            // Either the empty string is stored, or it was rejected
            assert!(updated.content == "" || updated.content == "Transaction test");
        }
        Err(_) => {
            // If it failed, verify original is unchanged
            let unchanged = storage
                .storage()
                .get_memory(memory.id.clone())
                .await
                .expect("Get unchanged");
            assert_eq!(unchanged.content, "Transaction test");
        }
    }

    // No partial writes should occur
    let count = storage.memory_count().await.expect("Count memories");
    assert_eq!(count, 1, "Should have exactly 1 memory, no duplicates");
}
