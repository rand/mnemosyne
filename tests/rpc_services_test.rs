//! Integration tests for RPC services
//!
//! These tests verify the MemoryService RPC implementation using a mock storage backend.

#![cfg(feature = "rpc")]

use mnemosyne_core::{
    rpc::{
        generated::{
            memory_service_server::MemoryService, GetContextRequest, GetMemoryRequest,
            GraphTraverseRequest, ListMemoriesRequest, RecallRequest, SemanticSearchRequest,
            StoreMemoryRequest,
        },
        services::MemoryServiceImpl,
    },
    storage::{libsql::LibsqlStorage, StorageBackend},
};
use std::sync::Arc;
use tonic::Request;

/// Helper to create a test storage backend with temporary file database
async fn create_test_storage() -> Arc<dyn StorageBackend> {
    use mnemosyne_core::storage::libsql::ConnectionMode;

    // Use a temporary file instead of :memory: because libSQL's :memory: mode
    // creates isolated databases per connection, so migrations wouldn't persist
    let temp_file = format!("/tmp/mnemosyne_rpc_test_{}.db", uuid::Uuid::new_v4());

    Arc::new(
        LibsqlStorage::new_with_validation(
            ConnectionMode::Local(temp_file),
            true, // create_if_missing - required for test databases
        )
        .await
        .expect("Failed to create test storage"),
    )
}

/// Helper to create test namespace
fn test_namespace() -> mnemosyne_core::rpc::generated::Namespace {
    mnemosyne_core::rpc::generated::Namespace {
        namespace: Some(
            mnemosyne_core::rpc::generated::namespace::Namespace::Project(
                mnemosyne_core::rpc::generated::ProjectNamespace {
                    name: "test-project".to_string(),
                },
            ),
        ),
    }
}

#[tokio::test]
async fn test_store_and_get_memory() {
    let storage = create_test_storage().await;
    let service = MemoryServiceImpl::new(storage.clone(), None);

    // Store a memory
    let store_request = Request::new(StoreMemoryRequest {
        content: "Test memory content".to_string(),
        namespace: Some(test_namespace()),
        importance: Some(8),
        context: Some("Test context".to_string()),
        tags: vec!["test".to_string(), "rpc".to_string()],
        memory_type: Some(
            mnemosyne_core::rpc::generated::MemoryType::Insight as i32,
        ),
        skip_llm_enrichment: true,
    });

    let store_response = service
        .store_memory(store_request)
        .await
        .expect("Failed to store memory");

    let memory_id = store_response.into_inner().memory_id;
    assert!(!memory_id.is_empty(), "Memory ID should not be empty");

    // Retrieve the memory
    let get_request = Request::new(GetMemoryRequest {
        memory_id: memory_id.clone(),
    });

    let get_response = service
        .get_memory(get_request)
        .await
        .expect("Failed to get memory");

    let memory = get_response.into_inner().memory.expect("Memory not found");
    assert_eq!(memory.content, "Test memory content");
    assert_eq!(memory.importance, 8);
    assert_eq!(memory.tags, vec!["test", "rpc"]);
}

#[tokio::test]
async fn test_update_memory() {
    let storage = create_test_storage().await;
    let service = MemoryServiceImpl::new(storage.clone(), None);

    // Store a memory first
    let store_request = Request::new(StoreMemoryRequest {
        content: "Original content".to_string(),
        namespace: Some(test_namespace()),
        importance: Some(5),
        context: None,
        tags: vec!["original".to_string()],
        memory_type: None,
        skip_llm_enrichment: true,
    });

    let store_response = service
        .store_memory(store_request)
        .await
        .expect("Failed to store memory");

    let memory_id = store_response.into_inner().memory_id;

    // Update the memory
    let update_request = Request::new(mnemosyne_core::rpc::generated::UpdateMemoryRequest {
        memory_id: memory_id.clone(),
        content: Some("Updated content".to_string()),
        importance: Some(9),
        tags: vec![],
        add_tags: vec!["updated".to_string()],
        remove_tags: vec![],
        re_enrich: false,
    });

    let update_response = service
        .update_memory(update_request)
        .await
        .expect("Failed to update memory");

    let updated_memory = update_response
        .into_inner()
        .memory
        .expect("Updated memory not found");

    assert_eq!(updated_memory.content, "Updated content");
    assert_eq!(updated_memory.importance, 9);
    assert!(updated_memory.tags.contains(&"updated".to_string()));
}

#[tokio::test]
async fn test_list_memories() {
    let storage = create_test_storage().await;
    let service = MemoryServiceImpl::new(storage.clone(), None);

    // Store multiple memories
    for i in 1..=5 {
        let store_request = Request::new(StoreMemoryRequest {
            content: format!("Memory {}", i),
            namespace: Some(test_namespace()),
            importance: Some(i),
            context: None,
            tags: vec!["test".to_string()],
            memory_type: None,
            skip_llm_enrichment: true,
        });

        service
            .store_memory(store_request)
            .await
            .expect("Failed to store memory");
    }

    // List memories
    let list_request = Request::new(ListMemoriesRequest {
        namespace: Some(test_namespace()),
        limit: 10,
        offset: None,
        memory_types: vec![],
        tags: vec![],
        min_importance: None,
        include_archived: false,
        sort_by: "importance".to_string(),
        sort_desc: true,
    });

    let list_response = service
        .list_memories(list_request)
        .await
        .expect("Failed to list memories");

    let response = list_response.into_inner();
    assert!(response.memories.len() >= 5, "Should have at least 5 memories");
    assert_eq!(response.total_count as usize, response.memories.len());
}

#[tokio::test]
async fn test_recall_search() {
    let storage = create_test_storage().await;
    let service = MemoryServiceImpl::new(storage.clone(), None);

    // Store memories with searchable content
    let contents = vec![
        "Rust is a systems programming language",
        "Python is great for data science",
        "JavaScript runs in browsers",
    ];

    for content in contents {
        let store_request = Request::new(StoreMemoryRequest {
            content: content.to_string(),
            namespace: Some(test_namespace()),
            importance: Some(7),
            context: None,
            tags: vec![],
            memory_type: None,
            skip_llm_enrichment: true,
        });

        service
            .store_memory(store_request)
            .await
            .expect("Failed to store memory");
    }

    // Search for Rust-related memories
    let recall_request = Request::new(RecallRequest {
        query: "Rust programming".to_string(),
        namespace: Some(test_namespace()),
        max_results: 5,
        min_importance: None,
        memory_types: vec![],
        tags: vec![],
        include_archived: false,
        semantic_weight: None,
        fts_weight: None,
        graph_weight: None,
    });

    let recall_response = service
        .recall(recall_request)
        .await
        .expect("Failed to recall memories");

    let response = recall_response.into_inner();
    assert!(
        !response.results.is_empty(),
        "Should find at least one matching memory"
    );
    assert_eq!(response.query, "Rust programming");
}

#[tokio::test]
async fn test_semantic_search() {
    let storage = create_test_storage().await;
    let service = MemoryServiceImpl::new(storage.clone(), None);

    // Create a dummy embedding vector (384d for LibSQL schema)
    let embedding = vec![0.1f32; 384];

    let search_request = Request::new(SemanticSearchRequest {
        embedding: embedding.clone(),
        namespace: Some(test_namespace()),
        max_results: 10,
        min_importance: None,
        include_archived: false,
    });

    // This should not fail even with empty database
    let search_response = service
        .semantic_search(search_request)
        .await
        .expect("Semantic search should not fail");

    let results = search_response.into_inner().results;
    // Empty database should return empty results
    assert_eq!(results.len(), 0, "Empty database should return no results");
}

#[tokio::test]
async fn test_semantic_search_validation() {
    let storage = create_test_storage().await;
    let service = MemoryServiceImpl::new(storage.clone(), None);

    // Test with empty embedding vector
    let search_request = Request::new(SemanticSearchRequest {
        embedding: vec![],
        namespace: None,
        max_results: 10,
        min_importance: None,
        include_archived: false,
    });

    let result = service.semantic_search(search_request).await;
    assert!(
        result.is_err(),
        "Empty embedding vector should return error"
    );

    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn test_graph_traverse() {
    let storage = create_test_storage().await;
    let service = MemoryServiceImpl::new(storage.clone(), None);

    // Store a seed memory
    let store_request = Request::new(StoreMemoryRequest {
        content: "Seed memory".to_string(),
        namespace: Some(test_namespace()),
        importance: Some(8),
        context: None,
        tags: vec![],
        memory_type: None,
        skip_llm_enrichment: true,
    });

    let store_response = service
        .store_memory(store_request)
        .await
        .expect("Failed to store seed memory");

    let seed_id = store_response.into_inner().memory_id;

    // Traverse graph from seed
    let traverse_request = Request::new(GraphTraverseRequest {
        seed_ids: vec![seed_id.clone()],
        max_hops: 2,
        link_types: vec![],
        min_link_strength: None,
    });

    let traverse_response = service
        .graph_traverse(traverse_request)
        .await
        .expect("Failed to traverse graph");

    let response = traverse_response.into_inner();
    assert!(
        !response.memories.is_empty(),
        "Should return at least the seed memory"
    );
}

#[tokio::test]
async fn test_graph_traverse_validation() {
    let storage = create_test_storage().await;
    let service = MemoryServiceImpl::new(storage.clone(), None);

    // Test with empty seed IDs
    let traverse_request = Request::new(GraphTraverseRequest {
        seed_ids: vec![],
        max_hops: 2,
        link_types: vec![],
        min_link_strength: None,
    });

    let result = service.graph_traverse(traverse_request).await;
    assert!(result.is_err(), "Empty seed IDs should return error");

    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn test_get_context() {
    let storage = create_test_storage().await;
    let service = MemoryServiceImpl::new(storage.clone(), None);

    // Store a memory
    let store_request = Request::new(StoreMemoryRequest {
        content: "Context test memory".to_string(),
        namespace: Some(test_namespace()),
        importance: Some(7),
        context: None,
        tags: vec![],
        memory_type: None,
        skip_llm_enrichment: true,
    });

    let store_response = service
        .store_memory(store_request)
        .await
        .expect("Failed to store memory");

    let memory_id = store_response.into_inner().memory_id;

    // Get context
    let context_request = Request::new(GetContextRequest {
        memory_ids: vec![memory_id.clone()],
        include_links: true,
        max_linked_depth: 1,
    });

    let context_response = service
        .get_context(context_request)
        .await
        .expect("Failed to get context");

    let response = context_response.into_inner();
    assert_eq!(response.memories.len(), 1, "Should return requested memory");
    assert_eq!(response.memories[0].id, memory_id);
}

#[tokio::test]
async fn test_get_context_validation() {
    let storage = create_test_storage().await;
    let service = MemoryServiceImpl::new(storage.clone(), None);

    // Test with empty memory IDs
    let context_request = Request::new(GetContextRequest {
        memory_ids: vec![],
        include_links: false,
        max_linked_depth: 0,
    });

    let result = service.get_context(context_request).await;
    assert!(result.is_err(), "Empty memory IDs should return error");

    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn test_delete_memory() {
    let storage = create_test_storage().await;
    let service = MemoryServiceImpl::new(storage.clone(), None);

    // Store a memory
    let store_request = Request::new(StoreMemoryRequest {
        content: "Memory to delete".to_string(),
        namespace: Some(test_namespace()),
        importance: Some(5),
        context: None,
        tags: vec![],
        memory_type: None,
        skip_llm_enrichment: true,
    });

    let store_response = service
        .store_memory(store_request)
        .await
        .expect("Failed to store memory");

    let memory_id = store_response.into_inner().memory_id;

    // Delete the memory
    let delete_request = Request::new(mnemosyne_core::rpc::generated::DeleteMemoryRequest {
        memory_id: memory_id.clone(),
    });

    let delete_response = service
        .delete_memory(delete_request)
        .await
        .expect("Failed to delete memory");

    assert!(
        delete_response.into_inner().success,
        "Delete should succeed"
    );

    // Try to get the deleted memory (should still exist but be archived)
    let get_request = Request::new(GetMemoryRequest {
        memory_id: memory_id.clone(),
    });

    let get_response = service.get_memory(get_request).await;
    // Note: Archived memories can still be retrieved, they're just marked as archived
    assert!(get_response.is_ok(), "Archived memory should still exist");
}
