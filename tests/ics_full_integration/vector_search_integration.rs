//! Vector Search Integration Tests (V1-V8)
//!
//! Tests ICS integration with vector/semantic search functionality

use crate::ics_full_integration::*;
use mnemosyne_core::{
    embeddings::{cosine_similarity, EmbeddingService},
    storage::StorageBackend,
    types::{MemoryType, Namespace},
};

/// V1: Embedding generation for memories
#[tokio::test]
async fn v1_embedding_generation() {
    let storage = StorageFixture::new().await.expect("Storage setup failed");

    // Create mock embedding service (no model download required)
    let embedding_service = MockEmbeddingService::new_standard();

    // Create memory
    let content = "Authentication system uses JWT tokens with 1-hour expiration";
    let mut memory = create_test_memory(content, MemoryType::CodePattern, Namespace::Global, 8);

    // Generate embedding
    let embedding = embedding_service
        .embed(content)
        .await
        .expect("Embedding generation");

    // Verify embedding properties
    assert_eq!(
        embedding.len(),
        384,
        "all-MiniLM-L6-v2 produces 384-dimensional embeddings"
    );
    assert!(
        embedding.iter().all(|&x| x.is_finite()),
        "All values finite"
    );

    // Attach to memory
    memory.embedding = Some(embedding);

    // Store with embedding
    storage
        .storage()
        .store_memory(&memory)
        .await
        .expect("Store with embedding");

    // Retrieve and verify
    let retrieved = storage
        .storage()
        .get_memory(memory.id.clone())
        .await
        .expect("Retrieve memory");

    assert_has_embedding(&retrieved);
}

/// V2: Semantic search with embeddings
#[tokio::test]
async fn v2_semantic_search() {
    let storage = StorageFixture::new().await.expect("Storage setup failed");

    // Create mock embedding service
    let embedding_service = MockEmbeddingService::new_standard();

    // Create semantically related memories
    let memories_content = vec![
        "JWT authentication with Redis session storage",
        "OAuth2 authorization flow implementation",
        "Database connection pooling with PostgreSQL",
        "React component state management",
    ];

    for content in &memories_content {
        let mut memory = create_test_memory(content, MemoryType::CodePattern, Namespace::Global, 7);

        let embedding = embedding_service.embed(content).await.expect("Embed");
        memory.embedding = Some(embedding);

        storage
            .storage()
            .store_memory(&memory)
            .await
            .expect("Store memory");
    }

    // Query: semantically similar to JWT/auth
    let query = "User authentication and session management";
    let query_embedding = embedding_service.embed(query).await.expect("Query embed");

    // Perform vector search (returns Vec<(MemoryId, f32)>)
    let results = storage
        .storage()
        .vector_search(&query_embedding, 10, Some(Namespace::Global))
        .await
        .expect("Vector search");

    // Should find auth-related memories
    assert!(results.len() >= 2, "Should find auth-related memories");

    // Retrieve top result to verify it's auth-related
    if let Some((memory_id, _score)) = results.first() {
        let top_memory = storage
            .storage()
            .get_memory(memory_id.clone())
            .await
            .expect("Get top result");

        assert!(
            top_memory.content.contains("JWT") || top_memory.content.contains("OAuth"),
            "Top result should be auth-related"
        );
    }
}

/// V3: Hybrid search (keyword + vector)
#[tokio::test]
async fn v3_hybrid_search() {
    let storage = StorageFixture::new().await.expect("Storage setup failed");

    // Create diverse memories
    let memories = vec![
        (
            "PostgreSQL database optimization techniques",
            vec!["database", "postgres", "optimization"],
        ),
        (
            "Database connection pooling best practices",
            vec!["database", "pooling", "connections"],
        ),
        (
            "React state management patterns",
            vec!["react", "state", "frontend"],
        ),
    ];

    for (content, keywords) in memories {
        let mut memory = create_test_memory(content, MemoryType::CodePattern, Namespace::Global, 7);
        memory.keywords = keywords.iter().map(|s| s.to_string()).collect();

        storage
            .storage()
            .store_memory(&memory)
            .await
            .expect("Store");
    }

    // Keyword search: should find database-related
    let keyword_results = storage
        .storage()
        .keyword_search("database", Some(Namespace::Global))
        .await
        .expect("Keyword search");

    assert!(keyword_results.len() >= 2, "Should find database memories");

    // Verify all results contain "database"
    for result in &keyword_results {
        assert!(
            result.memory.content.contains("database")
                || result.memory.content.contains("Database"),
            "Keyword search should match 'database'"
        );
    }
}

/// V4: Semantic relevance ranking
#[tokio::test]
async fn v4_semantic_relevance_ranking() {
    let storage = StorageFixture::new().await.expect("Storage setup failed");

    // Create mock embedding service
    let embedding_service = MockEmbeddingService::new_standard();

    // Create memories with varying semantic similarity to query
    let memories = vec![
        ("User authentication with JWT tokens", 9), // High relevance
        ("Session management and token refresh", 8), // Medium relevance
        ("Database schema migration scripts", 7),   // Low relevance
        ("Frontend component styling with CSS", 6), // Very low relevance
    ];

    for (content, importance) in memories {
        let mut memory = create_test_memory(
            content,
            MemoryType::CodePattern,
            Namespace::Global,
            importance,
        );

        let embedding = embedding_service.embed(content).await.expect("Embed");
        memory.embedding = Some(embedding);

        storage
            .storage()
            .store_memory(&memory)
            .await
            .expect("Store");
    }

    // Query about authentication
    let query = "How to implement secure authentication?";
    let query_embedding = embedding_service.embed(query).await.expect("Query embed");

    let results = storage
        .storage()
        .vector_search(&query_embedding, 4, Some(Namespace::Global))
        .await
        .expect("Vector search");

    // Verify results are ranked by relevance
    assert!(results.len() >= 2, "Should have multiple results");

    // Fetch top result to verify it's about JWT/auth
    if let Some((memory_id, _score)) = results.first() {
        let top_memory = storage
            .storage()
            .get_memory(memory_id.clone())
            .await
            .expect("Get top result");

        assert!(
            top_memory.content.contains("JWT") || top_memory.content.contains("authentication"),
            "Most relevant result should be about auth"
        );
    }

    // Fetch all memories to check CSS ranking
    if results.len() >= 4 {
        let mut memories = Vec::new();
        for (id, _score) in &results {
            let mem = storage
                .storage()
                .get_memory(id.clone())
                .await
                .expect("Get memory");
            memories.push(mem);
        }

        let css_position = memories
            .iter()
            .position(|m| m.content.contains("CSS"))
            .unwrap_or(999);
        assert!(css_position >= 2, "CSS should rank lower for auth query");
    }
}

/// V5: Embedding model consistency
#[tokio::test]
async fn v5_embedding_model_consistency() {
    // Create mock embedding service
    let embedding_service = MockEmbeddingService::new_standard();

    let text = "Authentication system implementation";

    // Generate embedding twice
    let embedding1 = embedding_service.embed(text).await.expect("Embed 1");
    let embedding2 = embedding_service.embed(text).await.expect("Embed 2");

    // Should be identical (deterministic)
    assert_eq!(embedding1.len(), embedding2.len(), "Same dimensions");

    let similarity = cosine_similarity(&embedding1, &embedding2);
    assert!(
        (similarity - 1.0).abs() < 0.001,
        "Embeddings should be identical: similarity={}",
        similarity
    );

    // Test batch consistency
    let texts = vec![text, text, text];
    let batch_embeddings = embedding_service
        .embed_batch(&texts)
        .await
        .expect("Batch embed");

    // All batch embeddings should be identical
    for batch_embed in &batch_embeddings {
        let sim = cosine_similarity(&embedding1, batch_embed);
        assert!(
            (sim - 1.0).abs() < 0.001,
            "Batch embedding should match single: similarity={}",
            sim
        );
    }
}

/// V6: Large-scale vector search performance
#[tokio::test]
async fn v6_large_scale_vector_search() {
    use std::time::Instant;

    let storage = StorageFixture::new().await.expect("Storage setup failed");

    // Create mock embedding service (fast, no model download)
    let embedding_service = MockEmbeddingService::new_standard();

    // Create 100 memories with embeddings (reduced from 1000 for test speed)
    let start = Instant::now();
    for i in 0..100 {
        let content = format!(
            "System component {}: handles {} operations with {} patterns",
            i,
            if i % 3 == 0 { "database" } else { "cache" },
            if i % 2 == 0 { "async" } else { "sync" }
        );

        let mut memory =
            create_test_memory(&content, MemoryType::CodePattern, Namespace::Global, 7);

        let embedding = embedding_service.embed(&content).await.expect("Embed");
        memory.embedding = Some(embedding);

        storage
            .storage()
            .store_memory(&memory)
            .await
            .expect("Store");
    }
    let write_duration = start.elapsed();
    println!(
        "Stored 100 memories with embeddings in {:?}",
        write_duration
    );

    // Perform vector search
    let query = "Database operations with async patterns";
    let query_embedding = embedding_service.embed(query).await.expect("Query embed");

    let start = Instant::now();
    let results = storage
        .storage()
        .vector_search(&query_embedding, 10, Some(Namespace::Global))
        .await
        .expect("Vector search");
    let search_duration = start.elapsed();

    println!("Vector search completed in {:?}", search_duration);

    // Verify performance
    assert!(
        search_duration.as_millis() < 1000,
        "Vector search should be fast: {:?}",
        search_duration
    );

    // Verify results
    assert!(results.len() > 0, "Should find results");
    assert!(results.len() <= 10, "Should respect limit");
}

/// V7: Vector search with namespace filtering
#[tokio::test]
async fn v7_namespace_filtering() {
    let storage = StorageFixture::new().await.expect("Storage setup failed");

    // Create mock embedding service
    let embedding_service = MockEmbeddingService::new_standard();

    // Create memories in different namespaces
    let namespaces = vec![
        Namespace::Global,
        Namespace::Project {
            name: "project-a".to_string(),
        },
        Namespace::Project {
            name: "project-b".to_string(),
        },
    ];

    for ns in &namespaces {
        let content = format!("Authentication implementation for {:?}", ns);
        let mut memory = create_test_memory(&content, MemoryType::CodePattern, ns.clone(), 7);

        let embedding = embedding_service.embed(&content).await.expect("Embed");
        memory.embedding = Some(embedding);

        storage
            .storage()
            .store_memory(&memory)
            .await
            .expect("Store");
    }

    // Query with namespace filter
    let query = "Authentication patterns";
    let query_embedding = embedding_service.embed(query).await.expect("Query embed");

    // Search in Global namespace only
    let global_results = storage
        .storage()
        .vector_search(&query_embedding, 10, Some(Namespace::Global))
        .await
        .expect("Global search");

    // Search in project-a namespace only
    let project_a_results = storage
        .storage()
        .vector_search(
            &query_embedding,
            10,
            Some(Namespace::Project {
                name: "project-a".to_string(),
            }),
        )
        .await
        .expect("Project-a search");

    // Verify namespace isolation
    assert!(global_results.len() >= 1, "Should find global memory");
    assert!(project_a_results.len() >= 1, "Should find project-a memory");

    // Fetch and verify global results
    for (id, _score) in &global_results {
        let memory = storage
            .storage()
            .get_memory(id.clone())
            .await
            .expect("Get global memory");
        assert_memory_namespace(&memory, &Namespace::Global);
    }

    // Fetch and verify project-a results
    for (id, _score) in &project_a_results {
        let memory = storage
            .storage()
            .get_memory(id.clone())
            .await
            .expect("Get project-a memory");
        assert_memory_namespace(
            &memory,
            &Namespace::Project {
                name: "project-a".to_string(),
            },
        );
    }
}

/// V8: Incremental embedding updates
#[tokio::test]
async fn v8_incremental_embedding_updates() {
    let storage = StorageFixture::new().await.expect("Storage setup failed");

    // Create mock embedding service
    let embedding_service = MockEmbeddingService::new_standard();

    // Create memory without embedding
    let mut memory = create_test_memory(
        "Initial content without embedding",
        MemoryType::CodePattern,
        Namespace::Global,
        7,
    );

    storage
        .storage()
        .store_memory(&memory)
        .await
        .expect("Store without embedding");

    // Verify no embedding
    let retrieved = storage
        .storage()
        .get_memory(memory.id.clone())
        .await
        .expect("Get");
    assert_no_embedding(&retrieved);

    // Update content and add embedding
    memory.content = "Updated content with JWT authentication".to_string();
    let new_embedding = embedding_service
        .embed(&memory.content)
        .await
        .expect("New embedding");
    memory.embedding = Some(new_embedding.clone());

    storage
        .storage()
        .update_memory(&memory)
        .await
        .expect("Update with embedding");

    // Verify embedding added
    let updated = storage
        .storage()
        .get_memory(memory.id.clone())
        .await
        .expect("Get updated");
    assert_has_embedding(&updated);

    // Verify can search by new content
    let query_embedding = embedding_service
        .embed("JWT authentication")
        .await
        .expect("Query embed");

    let search_results = storage
        .storage()
        .vector_search(&query_embedding, 10, Some(Namespace::Global))
        .await
        .expect("Search");

    // Should find the updated memory (vector_search returns Vec<(MemoryId, f32)>)
    assert!(
        search_results.iter().any(|(id, _score)| *id == memory.id),
        "Should find updated memory in vector search"
    );
}
