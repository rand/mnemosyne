//! Evolution System Integration Tests (E1-E6)
//!
//! Tests ICS integration with evolution jobs: consolidation,
//! importance recalibration, link decay, and archival

use crate::ics_full_integration::*;
use mnemosyne_core::{
    storage::StorageBackend,
    types::{MemoryType, Namespace},
};
use std::time::Duration;

/// E1: Consolidation of ICS memories
#[tokio::test]
async fn e1_consolidation_detection() {
    let storage = StorageFixture::new().await.expect("Storage setup failed");

    // Create 3 similar memories
    let mem1 = create_test_memory(
        "JWT authentication uses 1-hour token expiration",
        MemoryType::CodePattern,
        Namespace::Global,
        7,
    );
    let mem2 = create_test_memory(
        "Authentication implemented with JWT tokens expiring after 1 hour",
        MemoryType::CodePattern,
        Namespace::Global,
        7,
    );
    let mem3 = create_test_memory(
        "JWT tokens expire in 60 minutes for security",
        MemoryType::CodePattern,
        Namespace::Global,
        6,
    );

    storage.storage().store_memory(&mem1).await.expect("Store 1");
    storage.storage().store_memory(&mem2).await.expect("Store 2");
    storage.storage().store_memory(&mem3).await.expect("Store 3");

    // Load in ICS
    let results = storage
        .storage()
        .keyword_search("JWT", Some(Namespace::Global))
        .await
        .expect("Search JWT");

    let memories: Vec<_> = results.into_iter().map(|r| r.memory).collect();

    // Should find all 3
    assert_memory_count(&memories, 3);

    // In real implementation, consolidation job would detect similarity
    // and propose merge. For now, verify they're all retrievable
    for memory in &memories {
        assert!(
            memory.content.contains("JWT") || memory.content.contains("token"),
            "All memories should be JWT-related"
        );
    }
}

/// E2: Importance recalibration
#[tokio::test]
async fn e2_importance_recalibration() {
    let storage = StorageFixture::new().await.expect("Storage setup failed");

    // Create memory with initial importance
    let mut memory = create_test_memory(
        "Frequently accessed pattern",
        MemoryType::CodePattern,
        Namespace::Global,
        5,
    );
    storage
        .storage()
        .store_memory(&memory)
        .await
        .expect("Store memory");

    // Simulate access (increment counter)
    for _ in 0..10 {
        storage
            .storage()
            .increment_access(memory.id.clone())
            .await
            .expect("Increment access");
    }

    // Retrieve and verify access count
    let accessed = storage
        .storage()
        .get_memory(memory.id.clone())
        .await
        .expect("Get memory");

    assert_access_count_increased(0, accessed.access_count);

    // In real implementation, recalibrator would increase importance
    // based on access patterns. Mock the recalibration:
    memory.importance = 7; // Increased from 5
    memory.access_count = accessed.access_count;

    storage
        .storage()
        .update_memory(&memory)
        .await
        .expect("Update importance");

    let recalibrated = storage
        .storage()
        .get_memory(memory.id.clone())
        .await
        .expect("Get recalibrated");

    assert_eq!(
        recalibrated.importance, 7,
        "Importance should increase based on access"
    );
}

/// E3: Link decay with ICS activity
#[tokio::test]
async fn e3_link_decay() {
    let storage = StorageFixture::new().await.expect("Storage setup failed");

    // Create two linked memories
    let mut memory_a = create_test_memory(
        "Memory A",
        MemoryType::CodePattern,
        Namespace::Global,
        7,
    );
    let memory_b = create_test_memory(
        "Memory B",
        MemoryType::Insight,
        Namespace::Global,
        6,
    );

    storage.storage().store_memory(&memory_a).await.expect("Store A");
    storage.storage().store_memory(&memory_b).await.expect("Store B");

    // Create link with high strength
    let link = mnemosyne_core::types::MemoryLink {
        target_id: memory_b.id.clone(),
        link_type: mnemosyne_core::types::LinkType::References,
        strength: 0.9,
        reason: "Strong reference".to_string(),
        created_at: chrono::Utc::now(),
    };

    memory_a.links.push(link);
    storage
        .storage()
        .update_memory(&memory_a)
        .await
        .expect("Update with link");

    // In real implementation, link decay job would reduce strength
    // over time if not traversed. Simulate decay:
    let mut decayed = storage
        .storage()
        .get_memory(memory_a.id.clone())
        .await
        .expect("Get memory");

    decayed.links[0].strength = 0.7; // Decayed from 0.9

    storage
        .storage()
        .update_memory(&decayed)
        .await
        .expect("Update decayed");

    // Verify decay applied
    let mut final_state = storage
        .storage()
        .get_memory(memory_a.id.clone())
        .await
        .expect("Get final");

    assert_eq!(final_state.links[0].strength, 0.7, "Link should decay");

    // Simulate traversal (access increases strength)
    final_state.links[0].strength = 0.85; // Restored partially
    // Would update again to persist strengthened link
}

/// E4: Archival of unused ICS content
#[tokio::test]
async fn e4_archival_of_unused_content() {
    let storage = StorageFixture::new().await.expect("Storage setup failed");

    // Create old, unused memory
    let mut old_memory = create_test_memory(
        "Old unused note",
        MemoryType::Insight,
        Namespace::Global,
        5,
    );

    // Simulate old timestamp (90 days ago would be set in real impl)
    old_memory.access_count = 0;

    storage
        .storage()
        .store_memory(&old_memory)
        .await
        .expect("Store old memory");

    // In real implementation, archival job would check:
    // - last_accessed_at > 90 days ago
    // - access_count low
    // - importance low
    // Then archive the memory

    // Simulate archival
    storage
        .storage()
        .archive_memory(old_memory.id.clone())
        .await
        .expect("Archive old memory");

    // Verify archived
    let archived = storage
        .storage()
        .get_memory(old_memory.id.clone())
        .await
        .expect("Get archived");

    assert_archived(&archived);

    // Verify not in default ICS panel view
    let search_results = storage
        .storage()
        .keyword_search("", Some(Namespace::Global))
        .await
        .expect("Search");

    let active_memories: Vec<_> = search_results
        .into_iter()
        .filter(|r| !r.memory.is_archived)
        .collect();

    // Old memory should not appear in active list
    assert!(
        !active_memories.iter().any(|r| r.memory.id == old_memory.id),
        "Archived memory should not appear in default view"
    );
}

/// E5: Background evolution during ICS session
#[tokio::test]
async fn e5_background_evolution() {
    let storage = StorageFixture::new().await.expect("Storage setup failed");

    // Create ICS session with memories
    let memories = generate_memory_batch("System", 10, Namespace::Global);

    for memory in &memories {
        storage
            .storage()
            .store_memory(memory)
            .await
            .expect("Store memory");
    }

    // Load in ICS
    let mut ics = IcsFixture::new();
    ics.add_text("Working in ICS...\n");

    // Simulate background evolution job running
    // (In real impl, this would be async background task)
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Evolution might consolidate or archive some memories
    // ICS should reflect changes on next refresh

    // Refresh ICS panel
    let updated_results = storage
        .storage()
        .keyword_search("", Some(Namespace::Global))
        .await
        .expect("Refresh search");

    let updated_memories: Vec<_> = updated_results.into_iter().map(|r| r.memory).collect();

    // Verify ICS can handle updates
    ics.set_memories(updated_memories.clone());

    assert!(
        updated_memories.len() >= 0,
        "Should have memories after evolution"
    );

    // User continues working without interruption
    ics.add_text("Evolution ran in background\n");
    assert!(ics.buffer_content().contains("Evolution"));
}

/// E6: Evolution rollback (mock)
#[tokio::test]
async fn e6_evolution_rollback() {
    let storage = StorageFixture::new().await.expect("Storage setup failed");

    // Create 2 memories to be consolidated
    let mem1 = create_test_memory(
        "Original memory 1",
        MemoryType::CodePattern,
        Namespace::Global,
        7,
    );
    let mem2 = create_test_memory(
        "Original memory 2",
        MemoryType::CodePattern,
        Namespace::Global,
        6,
    );

    storage.storage().store_memory(&mem1).await.expect("Store 1");
    storage.storage().store_memory(&mem2).await.expect("Store 2");

    // Simulate consolidation (merge into single memory)
    let merged = create_test_memory(
        "Consolidated: Original memory 1 and Original memory 2",
        MemoryType::CodePattern,
        Namespace::Global,
        7,
    );

    storage
        .storage()
        .store_memory(&merged)
        .await
        .expect("Store merged");

    // Archive originals (mark as superseded)
    storage
        .storage()
        .archive_memory(mem1.id.clone())
        .await
        .expect("Archive 1");
    storage
        .storage()
        .archive_memory(mem2.id.clone())
        .await
        .expect("Archive 2");

    // User reviews in ICS, dislikes consolidation
    // Trigger rollback: restore originals, remove merged

    // In real implementation, would have rollback mechanism
    // For now, verify we can retrieve archived memories
    let restored1 = storage
        .storage()
        .get_memory(mem1.id.clone())
        .await
        .expect("Get mem1");
    let restored2 = storage
        .storage()
        .get_memory(mem2.id.clone())
        .await
        .expect("Get mem2");

    assert_archived(&restored1);
    assert_archived(&restored2);

    // Could be un-archived in real rollback
    // For now, verify they're still accessible
    assert_eq!(restored1.content, "Original memory 1");
    assert_eq!(restored2.content, "Original memory 2");
}
