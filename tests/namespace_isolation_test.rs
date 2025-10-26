//! Integration tests for namespace isolation and hierarchy
//!
//! Tests that memories are properly isolated by namespace and that
//! the namespace hierarchy (Global -> Project -> Session) works correctly.

use mnemosyne::{MemoryType, Namespace, StorageBackend};

mod common;
use common::{create_test_storage, sample_memory};

#[tokio::test]
async fn test_project_namespace_isolation() {
    // Setup
    let storage = create_test_storage().await;

    // Create memories in different project namespaces
    let mut app1_mem = sample_memory(
        "App1 uses PostgreSQL",
        MemoryType::ArchitectureDecision,
        8,
    );
    app1_mem.namespace = Namespace::Project {
        name: "app1".to_string(),
    };
    app1_mem.keywords = vec!["database".to_string()];

    let mut app2_mem = sample_memory(
        "App2 uses MongoDB",
        MemoryType::ArchitectureDecision,
        8,
    );
    app2_mem.namespace = Namespace::Project {
        name: "app2".to_string(),
    };
    app2_mem.keywords = vec!["database".to_string()];

    storage.store_memory(&app1_mem).await.unwrap();
    storage.store_memory(&app2_mem).await.unwrap();

    // Test: Search in app1 namespace only
    let results = storage
        .hybrid_search(
            "database",
            Some(Namespace::Project { name: "app1".to_string() }),
            10,
            false,
        )
        .await
        .unwrap();

    // Assert: Only app1 memory returned
    assert_eq!(results.len(), 1, "Should return only 1 memory from app1");
    assert_eq!(
        results[0].memory.id, app1_mem.id,
        "Should be the app1 memory"
    );
    assert!(
        results[0].memory.content.contains("PostgreSQL"),
        "Should be PostgreSQL memory, not MongoDB"
    );

    // Test: Search in app2 namespace only
    let results = storage
        .hybrid_search(
            "database",
            Some(Namespace::Project { name: "app2".to_string() }),
            10,
            false,
        )
        .await
        .unwrap();

    // Assert: Only app2 memory returned
    assert_eq!(results.len(), 1, "Should return only 1 memory from app2");
    assert_eq!(
        results[0].memory.id, app2_mem.id,
        "Should be the app2 memory"
    );
    assert!(
        results[0].memory.content.contains("MongoDB"),
        "Should be MongoDB memory, not PostgreSQL"
    );
}

#[tokio::test]
async fn test_global_search_includes_all_namespaces() {
    // Setup
    let storage = create_test_storage().await;

    // Create memories in different namespaces
    let mut global_mem = sample_memory("Global config", MemoryType::Configuration, 5);
    global_mem.namespace = Namespace::Global;
    global_mem.keywords = vec!["config".to_string()];

    let mut project_mem = sample_memory("Project config", MemoryType::Configuration, 5);
    project_mem.namespace = Namespace::Project { name: "myapp".to_string() };
    project_mem.keywords = vec!["config".to_string()];

    storage.store_memory(&global_mem).await.unwrap();
    storage.store_memory(&project_mem).await.unwrap();

    // Test: Search with no namespace filter (global search)
    let results = storage
        .hybrid_search("config", None, 10, false)
        .await
        .unwrap();

    // Assert: Both memories returned
    assert_eq!(results.len(), 2, "Should return both memories");

    let namespaces: Vec<_> = results.iter().map(|r| r.memory.namespace.clone()).collect();
    assert!(
        namespaces.contains(&Namespace::Global),
        "Should include global namespace"
    );
    assert!(
        namespaces.contains(&Namespace::Project { name: "myapp".to_string() }),
        "Should include project namespace"
    );
}

#[tokio::test]
async fn test_session_namespace_hierarchy() {
    // Setup
    let storage = create_test_storage().await;

    // Create memories in hierarchical namespaces
    let mut global_mem = sample_memory("Global insight", MemoryType::Insight, 5);
    global_mem.namespace = Namespace::Global;
    global_mem.keywords = vec!["insight".to_string()];

    let mut project_mem = sample_memory("Project insight", MemoryType::Insight, 6);
    project_mem.namespace = Namespace::Project { name: "myapp".to_string() };
    project_mem.keywords = vec!["insight".to_string()];

    let mut session_mem = sample_memory("Session insight", MemoryType::Insight, 7);
    session_mem.namespace = Namespace::Session {
        project: "myapp".to_string(),
        session_id: "session_123".to_string(),
    };
    session_mem.keywords = vec!["insight".to_string()];

    storage.store_memory(&global_mem).await.unwrap();
    storage.store_memory(&project_mem).await.unwrap();
    storage.store_memory(&session_mem).await.unwrap();

    // Test: Search in session namespace
    let results = storage
        .hybrid_search(
            "insight",
            Some(Namespace::Session {
                project: "myapp".to_string(),
                session_id: "session_123".to_string(),
            }),
            10,
            false,
        )
        .await
        .unwrap();

    // Assert: Only session memory returned (strict isolation for now)
    // Note: This tests current behavior - future may implement hierarchical search
    assert_eq!(
        results.len(),
        1,
        "Should return only session-specific memory"
    );
    assert_eq!(
        results[0].memory.id, session_mem.id,
        "Should be the session memory"
    );
}

#[tokio::test]
async fn test_list_memories_by_namespace() {
    use mnemosyne::storage::MemorySortOrder;

    // Setup
    let storage = create_test_storage().await;

    // Create multiple memories in same namespace
    for i in 0..5 {
        let mut mem = sample_memory(
            &format!("App memory {}", i),
            MemoryType::Insight,
            5 + i as u8,
        );
        mem.namespace = Namespace::Project { name: "myapp".to_string() };
        storage.store_memory(&mem).await.unwrap();
    }

    // Create memories in different namespace
    for i in 0..3 {
        let mut mem = sample_memory(
            &format!("Other app memory {}", i),
            MemoryType::Insight,
            5,
        );
        mem.namespace = Namespace::Project { name: "otherapp".to_string() };
        storage.store_memory(&mem).await.unwrap();
    }

    // Test: List memories for myapp
    let results = storage
        .list_memories(
            Some(Namespace::Project { name: "myapp".to_string() }),
            10,
            MemorySortOrder::Recent,
        )
        .await
        .unwrap();

    // Assert: Only myapp memories returned
    assert_eq!(results.len(), 5, "Should return 5 myapp memories");

    for mem in &results {
        assert_eq!(
            mem.namespace,
            Namespace::Project { name: "myapp".to_string() },
            "All memories should be from myapp namespace"
        );
    }

    // Test: List memories for otherapp
    let results = storage
        .list_memories(
            Some(Namespace::Project { name: "otherapp".to_string() }),
            10,
            MemorySortOrder::Recent,
        )
        .await
        .unwrap();

    // Assert: Only otherapp memories returned
    assert_eq!(results.len(), 3, "Should return 3 otherapp memories");
}

#[tokio::test]
async fn test_count_memories_by_namespace() {
    // Setup
    let storage = create_test_storage().await;

    // Create memories in different namespaces
    for i in 0..10 {
        let mut mem = sample_memory(&format!("Memory {}", i), MemoryType::Insight, 5);
        mem.namespace = if i < 7 {
            Namespace::Project { name: "app1".to_string() }
        } else {
            Namespace::Project { name: "app2".to_string() }
        };
        storage.store_memory(&mem).await.unwrap();
    }

    // Test: Count app1 memories
    let count = storage
        .count_memories(Some(Namespace::Project { name: "app1".to_string() }))
        .await
        .unwrap();
    assert_eq!(count, 7, "Should have 7 app1 memories");

    // Test: Count app2 memories
    let count = storage
        .count_memories(Some(Namespace::Project { name: "app2".to_string() }))
        .await
        .unwrap();
    assert_eq!(count, 3, "Should have 3 app2 memories");

    // Test: Count all memories
    let count = storage.count_memories(None).await.unwrap();
    assert_eq!(count, 10, "Should have 10 total memories");
}

#[tokio::test]
async fn test_namespace_serialization_consistency() {
    // Setup
    let storage = create_test_storage().await;

    // Create memory with complex namespace
    let mut mem = sample_memory("Test memory", MemoryType::Insight, 5);
    mem.namespace = Namespace::Session {
        project: "my-complex-project-name".to_string(),
        session_id: "session_2025-10-26_abc123".to_string(),
    };

    // Store and retrieve
    storage.store_memory(&mem).await.unwrap();
    let retrieved = storage.get_memory(mem.id).await.unwrap();

    // Assert: Namespace preserved exactly
    assert_eq!(
        retrieved.namespace, mem.namespace,
        "Namespace should be preserved through storage roundtrip"
    );

    match retrieved.namespace {
        Namespace::Session { project, session_id } => {
            assert_eq!(project, "my-complex-project-name");
            assert_eq!(session_id, "session_2025-10-26_abc123");
        }
        _ => panic!("Namespace should be Session type"),
    }
}

#[tokio::test]
async fn test_update_memory_preserves_namespace() {
    // Setup
    let storage = create_test_storage().await;

    // Create memory in specific namespace
    let mut mem = sample_memory("Original content", MemoryType::Insight, 5);
    mem.namespace = Namespace::Project { name: "myapp".to_string() };

    storage.store_memory(&mem).await.unwrap();

    // Update memory
    mem.content = "Updated content".to_string();
    storage.update_memory(&mem).await.unwrap();

    // Retrieve and verify
    let retrieved = storage.get_memory(mem.id).await.unwrap();

    assert_eq!(
        retrieved.namespace,
        Namespace::Project { name: "myapp".to_string() },
        "Namespace should be preserved after update"
    );
    assert_eq!(retrieved.content, "Updated content");
}

#[tokio::test]
async fn test_archived_memories_excluded_from_search() {
    // Setup
    let storage = create_test_storage().await;

    // Create and archive a memory
    let mut mem = sample_memory("Archived memory", MemoryType::Insight, 5);
    mem.namespace = Namespace::Project { name: "myapp".to_string() };
    mem.keywords = vec!["archived".to_string()];

    storage.store_memory(&mem).await.unwrap();
    storage.archive_memory(mem.id).await.unwrap();

    // Test: Search should not return archived memory
    let results = storage
        .hybrid_search(
            "archived",
            Some(Namespace::Project { name: "myapp".to_string() }),
            10,
            false,
        )
        .await
        .unwrap();

    assert_eq!(
        results.len(),
        0,
        "Archived memories should not appear in search results"
    );

    // Test: Count should also exclude archived
    let count = storage
        .count_memories(Some(Namespace::Project { name: "myapp".to_string() }))
        .await
        .unwrap();

    assert_eq!(count, 0, "Archived memories should not be counted");
}
