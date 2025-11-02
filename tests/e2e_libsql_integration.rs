//! End-to-End Integration Test for LibSQL Migration
//!
//! This test exercises the complete system functionality to verify
//! the LibSQL migration works correctly in real-world scenarios.

use mnemosyne_core::{
    ConnectionMode, LibsqlStorage, MemoryNote, MemoryType, Namespace, StorageBackend,
};
use std::time::Duration;
use tokio::time::sleep;

/// Helper to create a test memory
fn create_test_memory(
    content: &str,
    memory_type: MemoryType,
    importance: u8,
    namespace: Namespace,
) -> MemoryNote {
    use mnemosyne_core::MemoryId;

    MemoryNote {
        id: MemoryId::new(),
        namespace,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        content: content.to_string(),
        summary: format!("Summary: {}", content),
        keywords: vec!["test".to_string(), "integration".to_string()],
        tags: vec!["e2e".to_string()],
        context: "End-to-end integration test".to_string(),
        memory_type,
        importance,
        confidence: 0.9,
        links: vec![],
        related_files: vec![],
        related_entities: vec![],
        access_count: 0,
        last_accessed_at: chrono::Utc::now(),
        expires_at: None,
        is_archived: false,
        superseded_by: None,
        embedding: None,
        embedding_model: "test-model".to_string(),
    }
}

#[tokio::test]
async fn test_e2e_complete_workflow() {
    println!("\n=== E2E Test: Complete Workflow ===\n");

    // 1. Create storage with unique temp file
    let db_path = format!("/tmp/e2e_test_{}.db", uuid::Uuid::new_v4());
    println!("1. Creating LibSQL storage at: {}", db_path);

    let storage = LibsqlStorage::new_with_validation(
        ConnectionMode::Local(db_path.clone()),
        true, // create_if_missing
    )
    .await
    .expect("Failed to create storage");
    println!("   ✓ Storage created successfully");

    // 2. Store multiple memories in different namespaces
    println!("\n2. Storing test memories...");

    let mem1 = create_test_memory(
        "Decided to use Rust for high-performance memory system",
        MemoryType::ArchitectureDecision,
        9,
        Namespace::Project {
            name: "mnemosyne".to_string(),
        },
    );

    let mem2 = create_test_memory(
        "LibSQL provides native vector search without extensions",
        MemoryType::Insight,
        8,
        Namespace::Project {
            name: "mnemosyne".to_string(),
        },
    );

    let mem3 = create_test_memory(
        "Always use Result<T, E> for error handling in Rust",
        MemoryType::CodePattern,
        7,
        Namespace::Global,
    );

    storage
        .store_memory(&mem1)
        .await
        .expect("Failed to store mem1");
    println!("   ✓ Stored memory 1 (Architecture Decision)");

    storage
        .store_memory(&mem2)
        .await
        .expect("Failed to store mem2");
    println!("   ✓ Stored memory 2 (Insight)");

    storage
        .store_memory(&mem3)
        .await
        .expect("Failed to store mem3");
    println!("   ✓ Stored memory 3 (Code Pattern)");

    // 3. Test retrieval by ID
    println!("\n3. Testing retrieval by ID...");
    let retrieved = storage
        .get_memory(mem1.id)
        .await
        .expect("Failed to retrieve memory");

    assert_eq!(retrieved.id, mem1.id);
    assert_eq!(retrieved.content, mem1.content);
    println!("   ✓ Retrieved memory matches original");

    // 4. Test keyword search
    println!("\n4. Testing keyword search...");
    let keyword_results = storage
        .keyword_search(
            "Rust performance",
            Some(Namespace::Project {
                name: "mnemosyne".to_string(),
            }),
        )
        .await
        .expect("Keyword search failed");

    println!("   Found {} results", keyword_results.len());
    assert!(
        !keyword_results.is_empty(),
        "Should find Rust-related memories"
    );
    println!("   ✓ Keyword search working");

    // 5. Test hybrid search
    println!("\n5. Testing hybrid search...");
    let hybrid_results = storage
        .hybrid_search(
            "vector search",
            Some(Namespace::Project {
                name: "mnemosyne".to_string(),
            }),
            10,
            false,
        )
        .await
        .expect("Hybrid search failed");

    println!("   Found {} results", hybrid_results.len());
    assert!(
        !hybrid_results.is_empty(),
        "Should find vector search memories"
    );

    for (i, result) in hybrid_results.iter().enumerate() {
        println!(
            "   {}. [Score: {:.3}] {}",
            i + 1,
            result.score,
            &result.memory.content[..result.memory.content.len().min(60)]
        );
    }
    println!("   ✓ Hybrid search working");

    // 6. Test namespace isolation
    println!("\n6. Testing namespace isolation...");
    let project_count = storage
        .count_memories(Some(Namespace::Project {
            name: "mnemosyne".to_string(),
        }))
        .await
        .expect("Failed to count project memories");

    let global_count = storage
        .count_memories(Some(Namespace::Global))
        .await
        .expect("Failed to count global memories");

    println!("   Project namespace: {} memories", project_count);
    println!("   Global namespace: {} memories", global_count);
    assert_eq!(project_count, 2, "Should have 2 project memories");
    assert_eq!(global_count, 1, "Should have 1 global memory");
    println!("   ✓ Namespace isolation working");

    // 7. Test update
    println!("\n7. Testing memory update...");
    let mut updated_mem = mem1.clone();
    updated_mem.content = "Updated: Decided to use Rust and LibSQL for memory system".to_string();
    updated_mem.importance = 10;

    storage
        .update_memory(&updated_mem)
        .await
        .expect("Failed to update memory");

    let retrieved_updated = storage
        .get_memory(mem1.id)
        .await
        .expect("Failed to retrieve updated memory");

    assert_eq!(retrieved_updated.content, updated_mem.content);
    assert_eq!(retrieved_updated.importance, 10);
    println!("   ✓ Memory update working");

    // 8. Test archival
    println!("\n8. Testing memory archival...");
    storage
        .archive_memory(mem3.id)
        .await
        .expect("Failed to archive memory");

    let archived = storage
        .get_memory(mem3.id)
        .await
        .expect("Failed to retrieve archived memory");

    assert!(archived.is_archived, "Memory should be archived");
    println!("   ✓ Memory archival working");

    // Verify archived memories are excluded from search
    let search_after_archive = storage
        .hybrid_search("Result error handling", Some(Namespace::Global), 10, false)
        .await
        .expect("Search failed");

    println!(
        "   Search results after archival: {}",
        search_after_archive.len()
    );
    assert_eq!(
        search_after_archive.len(),
        0,
        "Archived memories should not appear in search"
    );
    println!("   ✓ Archived memories excluded from search");

    // 9. Test list memories
    println!("\n9. Testing list memories...");
    let recent = storage
        .list_memories(
            Some(Namespace::Project {
                name: "mnemosyne".to_string(),
            }),
            10,
            mnemosyne_core::storage::MemorySortOrder::Recent,
        )
        .await
        .expect("Failed to list memories");

    println!("   Found {} recent memories", recent.len());
    assert_eq!(recent.len(), 2, "Should list 2 active project memories");
    println!("   ✓ List memories working");

    // 10. Test persistence - close and reopen
    println!("\n10. Testing data persistence...");
    drop(storage);
    println!("   Closed storage");

    sleep(Duration::from_millis(100)).await;

    let storage2 = LibsqlStorage::new(ConnectionMode::Local(db_path.clone()))
        .await
        .expect("Failed to reopen storage");
    println!("   Reopened storage");

    let persisted = storage2
        .get_memory(mem1.id)
        .await
        .expect("Failed to retrieve from reopened storage");

    assert_eq!(persisted.id, mem1.id);
    assert_eq!(persisted.content, updated_mem.content);
    println!("   ✓ Data persisted correctly across restarts");

    // 11. Test total count
    println!("\n11. Testing total memory count...");
    let total = storage2
        .count_memories(None)
        .await
        .expect("Failed to count all memories");

    println!("   Total active memories: {}", total);
    assert_eq!(total, 2, "Should have 2 active memories (1 was archived)");
    println!("   ✓ Total count correct");

    // Cleanup
    println!("\n12. Cleanup...");
    drop(storage2);
    std::fs::remove_file(&db_path).ok();
    println!("   ✓ Test database removed");

    println!("\n=== E2E Test: ✅ ALL CHECKS PASSED ===\n");
}

#[tokio::test]
async fn test_e2e_vector_search_with_embeddings() {
    println!("\n=== E2E Test: Vector Search with Embeddings ===\n");
    println!("Testing LibSQL native vector search with F32_BLOB embeddings");

    let db_path = format!("/tmp/e2e_vector_test_{}.db", uuid::Uuid::new_v4());
    println!("Creating storage at: {}", db_path);

    let storage = LibsqlStorage::new_with_validation(
        ConnectionMode::Local(db_path.clone()),
        true, // create_if_missing
    )
    .await
    .expect("Failed to create storage");

    // Create memories with mock embeddings
    let mut mem1 = create_test_memory(
        "Machine learning model training",
        MemoryType::Insight,
        8,
        Namespace::Global,
    );
    mem1.embedding = Some(vec![0.1; 384]); // Mock 384-dim embedding

    let mut mem2 = create_test_memory(
        "Deep learning neural networks",
        MemoryType::Insight,
        7,
        Namespace::Global,
    );
    mem2.embedding = Some(vec![0.15; 384]); // Similar embedding

    let mut mem3 = create_test_memory(
        "Database query optimization",
        MemoryType::CodePattern,
        6,
        Namespace::Global,
    );
    mem3.embedding = Some(vec![0.9; 384]); // Different embedding

    storage.store_memory(&mem1).await.expect("Failed to store");
    storage.store_memory(&mem2).await.expect("Failed to store");
    storage.store_memory(&mem3).await.expect("Failed to store");

    println!("Stored 3 memories with embeddings");

    // Test vector search
    let query_embedding = vec![0.12; 384]; // Close to mem1 and mem2

    let results = storage
        .vector_search(&query_embedding, 3, None)
        .await
        .expect("Vector search failed");

    println!("\nVector search results:");
    for (i, (memory_id, score)) in results.iter().enumerate() {
        // Fetch memory to display content
        let mem = storage.get_memory(*memory_id).await.expect("Should get memory");
        println!(
            "{}. [Score: {:.3}] {}",
            i + 1,
            score,
            &mem.content[..mem.content.len().min(50)]
        );
    }

    assert_eq!(results.len(), 3, "Should return 3 results");

    // The most similar should be mem1 or mem2 (closer embeddings)
    let (top_memory_id, _score) = &results[0];
    assert!(
        *top_memory_id == mem1.id || *top_memory_id == mem2.id,
        "Top result should be ML-related memory"
    );

    println!("\n✓ Vector search correctly ranked by similarity");

    // Cleanup
    drop(storage);
    std::fs::remove_file(&db_path).ok();

    println!("\n=== E2E Test: ✅ VECTOR SEARCH PASSED ===\n");
}

#[tokio::test]
async fn test_e2e_graph_traversal() {
    println!("\n=== E2E Test: Graph Traversal ===\n");

    let db_path = format!("/tmp/e2e_graph_test_{}.db", uuid::Uuid::new_v4());
    let storage = LibsqlStorage::new_with_validation(
        ConnectionMode::Local(db_path.clone()),
        true, // create_if_missing
    )
    .await
    .expect("Failed to create storage");

    // Create a chain of related memories
    let mem1 = create_test_memory(
        "Architecture: Microservices design",
        MemoryType::ArchitectureDecision,
        9,
        Namespace::Global,
    );

    let mut mem2 = create_test_memory(
        "Pattern: Event-driven architecture",
        MemoryType::CodePattern,
        8,
        Namespace::Global,
    );

    let mut mem3 = create_test_memory(
        "Implementation: Message queue system",
        MemoryType::CodePattern,
        7,
        Namespace::Global,
    );

    // Store memories
    storage.store_memory(&mem1).await.unwrap();
    storage.store_memory(&mem2).await.unwrap();
    storage.store_memory(&mem3).await.unwrap();

    println!("Stored 3 memories");

    // Create links manually by updating with links
    use mnemosyne_core::{LinkType, MemoryLink};

    mem2.links.push(MemoryLink {
        target_id: mem1.id,
        link_type: LinkType::Implements,
        strength: 0.9,
        reason: "Event-driven implements microservices".to_string(),
        created_at: chrono::Utc::now(),
        last_traversed_at: None,
        user_created: false,
    });

    mem3.links.push(MemoryLink {
        target_id: mem2.id,
        link_type: LinkType::Implements,
        strength: 0.8,
        reason: "Message queue implements event-driven".to_string(),
        created_at: chrono::Utc::now(),
        last_traversed_at: None,
        user_created: false,
    });

    storage.update_memory(&mem2).await.unwrap();
    storage.update_memory(&mem3).await.unwrap();

    println!("Created link chain: mem1 ← mem2 ← mem3");

    // Test graph traversal starting from mem3
    let graph_results = storage
        .graph_traverse(&[mem3.id], 2, None)
        .await
        .expect("Graph traversal failed");

    println!("\nGraph traversal from mem3 (max 2 hops):");
    println!("Found {} connected memories", graph_results.len());

    for memory in &graph_results {
        println!("  - {}", &memory.content[..memory.content.len().min(50)]);
    }

    // Should find mem3, mem2, and mem1 (traversing the link chain)
    assert!(
        graph_results.len() >= 2,
        "Should find at least 2 connected memories"
    );

    let ids: Vec<_> = graph_results.iter().map(|m| m.id).collect();
    assert!(ids.contains(&mem3.id), "Should include starting memory");

    println!("\n✓ Graph traversal working correctly");

    // Cleanup
    drop(storage);
    std::fs::remove_file(&db_path).ok();

    println!("\n=== E2E Test: ✅ GRAPH TRAVERSAL PASSED ===\n");
}
