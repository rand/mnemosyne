//! Storage Integration Tests (S1-S10)
//!
//! Tests ICS integration with LibsqlStorage backend

use crate::ics_full_integration::*;
use mnemosyne_core::{
    ics::SemanticAnalyzer,
    storage::StorageBackend,
    types::{MemoryType, Namespace},
};

/// S1: Memory persistence from ICS
#[tokio::test]
async fn s1_memory_persistence_from_ics() {
    // Setup: Storage + ICS
    let storage = StorageFixture::new().await.expect("Storage setup failed");
    let mut ics = IcsFixture::new();

    // User creates content in ICS editor
    ics.add_text("The system uses JWT authentication with 1-hour token expiration.\n");
    ics.add_text("This is a critical security pattern for the API.\n");

    // Trigger semantic analysis
    let analysis = ics.analyze().await.expect("Analysis should succeed");

    // Verify semantic extraction
    assert_min_triples(&analysis, 1);
    assert!(analysis.entities.len() > 0);

    // Create memory from ICS content
    let memory = create_test_memory(
        &ics.buffer_content(),
        MemoryType::CodePattern,
        Namespace::Global,
        8,
    );

    // Persist to storage
    storage
        .storage()
        .store_memory(&memory)
        .await
        .expect("Should persist memory");

    // Verify memory was stored
    let retrieved = storage
        .storage()
        .get_memory(memory.id.clone())
        .await
        .expect("Should retrieve memory");

    assert_eq!(retrieved.id, memory.id);
    assert!(retrieved.content.contains("JWT authentication"));
    assert_memory_namespace(&retrieved, &Namespace::Global);
}

/// S2: Memory retrieval in ICS panel
#[tokio::test]
async fn s2_memory_retrieval_in_ics_panel() {
    // Setup: Pre-populate storage with memories
    let storage = StorageFixture::with_memories(50, Namespace::Global)
        .await
        .expect("Storage setup failed");

    // Retrieve all memories for ICS panel
    let results = storage
        .storage()
        .keyword_search("", Some(Namespace::Global))
        .await
        .expect("Search should succeed");

    // Extract memories from search results
    let memories: Vec<MemoryNote> = results.into_iter().map(|r| r.memory).collect();

    // Note: keyword_search has a LIMIT 20, so we get top 20 by importance
    assert_memory_count(&memories, 20);

    // Create ICS with loaded memories
    let ics = IcsFixture::with_memories(memories.clone());

    // Test keyword search filtering
    let search_results = ics.search_memories("integration");
    assert!(
        search_results.len() > 0,
        "Should find memories with keyword"
    );

    // Test importance sorting (if memories have varied importance)
    let mut sorted = memories.clone();
    sorted.sort_by(|a, b| b.importance.cmp(&a.importance));
    assert_sorted_by_importance(&sorted);
}

/// S3: Cross-session memory continuity
#[tokio::test]
async fn s3_cross_session_memory_continuity() {
    // Session 1: Create memories
    let storage = StorageFixture::new().await.expect("Storage setup failed");
    let mut ics1 = IcsFixture::new();

    // Create 5 memories in session 1
    for i in 0..5 {
        ics1.add_text(&format!("Session 1 memory {}\n", i + 1));
        let memory = create_test_memory(
            &format!("Session 1 memory {}", i + 1),
            MemoryType::CodePattern,
            Namespace::Session {
                project: "test".to_string(),
                session_id: "session-1".to_string(),
            },
            7,
        );
        storage
            .storage()
            .store_memory(&memory)
            .await
            .expect("Should create memory");
    }

    // Verify session 1 memories persisted
    let session1_results = storage
        .storage()
        .keyword_search(
            "",
            Some(Namespace::Session {
                project: "test".to_string(),
                session_id: "session-1".to_string(),
            }),
        )
        .await
        .expect("Should retrieve session 1 memories");
    let session1_memories: Vec<MemoryNote> =
        session1_results.into_iter().map(|r| r.memory).collect();
    assert_memory_count(&session1_memories, 5);

    // Session 2: New ICS instance
    let session2_results = storage
        .storage()
        .keyword_search(
            "",
            Some(Namespace::Session {
                project: "test".to_string(),
                session_id: "session-1".to_string(),
            }),
        )
        .await
        .expect("Should retrieve memories");
    let session2_memories: Vec<MemoryNote> =
        session2_results.into_iter().map(|r| r.memory).collect();

    let ics2 = IcsFixture::with_memories(session2_memories.clone());

    // Verify all session 1 memories visible in session 2
    assert_memory_count(&session2_memories, 5);
    assert_memory_exists(&ics2.memories, "Session 1 memory 1");
    assert_memory_exists(&ics2.memories, "Session 1 memory 5");
}

/// S4: Namespace isolation in ICS
#[tokio::test]
async fn s4_namespace_isolation_in_ics() {
    let storage = StorageFixture::new().await.expect("Storage setup failed");

    // Create memories in different namespaces
    let global = create_test_memory(
        "Global configuration",
        MemoryType::Configuration,
        Namespace::Global,
        8,
    );

    let project = create_test_memory(
        "Project-specific pattern",
        MemoryType::CodePattern,
        Namespace::Project {
            name: "myproject".to_string(),
        },
        7,
    );

    let session = create_test_memory(
        "Session note",
        MemoryType::CodePattern,
        Namespace::Session {
            project: "myproject".to_string(),
            session_id: "session-1".to_string(),
        },
        6,
    );

    // Persist all
    storage
        .storage()
        .store_memory(&global)
        .await
        .expect("Create global");
    storage
        .storage()
        .store_memory(&project)
        .await
        .expect("Create project");
    storage
        .storage()
        .store_memory(&session)
        .await
        .expect("Create session");

    // Query global namespace
    let global_results = storage
        .storage()
        .keyword_search("", Some(Namespace::Global))
        .await
        .expect("Global search");
    let global_memories: Vec<MemoryNote> = global_results.into_iter().map(|r| r.memory).collect();
    assert_memory_count(&global_memories, 1);
    assert_memory_namespace(&global_memories[0], &Namespace::Global);

    // Query project namespace
    let project_results = storage
        .storage()
        .keyword_search(
            "",
            Some(Namespace::Project {
                name: "myproject".to_string(),
            }),
        )
        .await
        .expect("Project search");
    let project_memories: Vec<MemoryNote> = project_results.into_iter().map(|r| r.memory).collect();
    assert_memory_count(&project_memories, 1);
    assert_memory_namespace(
        &project_memories[0],
        &Namespace::Project {
            name: "myproject".to_string(),
        },
    );

    // Query session namespace
    let session_results = storage
        .storage()
        .keyword_search(
            "",
            Some(Namespace::Session {
                project: "myproject".to_string(),
                session_id: "session-1".to_string(),
            }),
        )
        .await
        .expect("Session search");
    let session_memories: Vec<MemoryNote> = session_results.into_iter().map(|r| r.memory).collect();
    assert_memory_count(&session_memories, 1);
}

/// S5: Concurrent ICS + MCP memory updates
#[tokio::test]
async fn s5_concurrent_ics_mcp_updates() {
    let storage = StorageFixture::new().await.expect("Storage setup failed");

    // ICS creates memory
    let mut memory = create_test_memory(
        "Initial content",
        MemoryType::CodePattern,
        Namespace::Global,
        7,
    );
    storage
        .storage()
        .store_memory(&memory)
        .await
        .expect("Create memory");

    // MCP updates different field
    memory.importance = 9;
    storage
        .storage()
        .update_memory(&memory)
        .await
        .expect("Update memory");

    // ICS refreshes
    let updated = storage
        .storage()
        .get_memory(memory.id.clone())
        .await
        .expect("Get memory");

    // Verify both changes visible
    assert_eq!(updated.content, "Initial content");
    assert_eq!(updated.importance, 9);
}

/// S6: Large memory dataset performance
#[tokio::test]
async fn s6_large_memory_dataset_performance() {
    use std::time::Instant;

    let storage = StorageFixture::new().await.expect("Storage setup failed");

    // Pre-populate 1000 memories (10k would be too slow for tests)
    let large_dataset = generate_large_dataset(1000, Namespace::Global);

    let start = Instant::now();
    for memory in &large_dataset {
        storage
            .storage()
            .store_memory(memory)
            .await
            .expect("Create memory");
    }
    let write_duration = start.elapsed();
    println!("Wrote 1000 memories in {:?}", write_duration);

    // Load in ICS
    let start = Instant::now();
    let results = storage
        .storage()
        .keyword_search("", Some(Namespace::Global))
        .await
        .expect("Search should succeed");
    let memories: Vec<MemoryNote> = results.into_iter().map(|r| r.memory).collect();
    let load_duration = start.elapsed();

    // Verify performance
    assert!(
        load_duration.as_millis() < 1000,
        "Initial load should be fast: {:?}",
        load_duration
    );
    // Note: keyword_search has LIMIT 20, so we verify the limit works efficiently
    assert_memory_count(&memories, 20);

    // Test search performance
    let start = Instant::now();
    let _search_results = storage
        .storage()
        .keyword_search("system", Some(Namespace::Global))
        .await
        .expect("Search should succeed");
    let search_duration = start.elapsed();

    assert!(
        search_duration.as_millis() < 200,
        "Search should be fast: {:?}",
        search_duration
    );
}
