//! Integration test for embedding re-generation on content update
//!
//! Verifies that when memory content is updated, embeddings are regenerated

use mnemosyne_core::services::embeddings::EmbeddingService;
use mnemosyne_core::storage::libsql::{ConnectionMode, LibsqlStorage};
use mnemosyne_core::storage::StorageBackend;
use mnemosyne_core::types::{MemoryId, MemoryNote, MemoryType, Namespace};
use mnemosyne_core::LlmConfig;
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn test_embedding_regeneration_on_content_update() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test_embedding_update.db");

    // Create storage backend
    let storage: Arc<dyn StorageBackend> = Arc::new(
        LibsqlStorage::new_with_validation(
            ConnectionMode::Local(db_path.to_str().unwrap().to_string()),
            true, // create_if_missing
        )
        .await
        .unwrap(),
    );

    // Create embedding service
    let llm_config = LlmConfig::default();
    let embeddings = Arc::new(EmbeddingService::new(
        "test-key".to_string(),
        llm_config.clone(),
    ));

    // Create initial memory with content
    let namespace = Namespace::Project {
        name: "test-project".to_string(),
    };
    let memory_id = MemoryId::new();
    let initial_content = "Rust programming language features memory safety";

    // Generate initial embedding
    let initial_embedding = embeddings
        .generate_embedding(initial_content)
        .await
        .expect("Failed to generate initial embedding");

    let memory = MemoryNote {
        id: memory_id.clone(),
        namespace: namespace.clone(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        content: initial_content.to_string(),
        summary: format!("Summary: {}", initial_content),
        keywords: vec!["rust".to_string(), "memory".to_string()],
        tags: vec!["test".to_string()],
        context: "Test memory".to_string(),
        memory_type: MemoryType::Insight,
        importance: 5,
        confidence: 0.9,
        links: vec![],
        related_files: vec![],
        related_entities: vec![],
        access_count: 0,
        last_accessed_at: chrono::Utc::now(),
        expires_at: None,
        is_archived: false,
        superseded_by: None,
        embedding: Some(initial_embedding.clone()),
        embedding_model: "test-model".to_string(),
    };

    // Store memory
    storage
        .store_memory(&memory)
        .await
        .expect("Failed to store initial memory");

    // Update memory content (simulating MCP update)
    let updated_content = "Python programming language emphasizes code readability";
    let updated_embedding = embeddings
        .generate_embedding(updated_content)
        .await
        .expect("Failed to generate updated embedding");

    let mut updated_memory = memory.clone();
    updated_memory.content = updated_content.to_string();
    updated_memory.embedding = Some(updated_embedding.clone());
    updated_memory.updated_at = chrono::Utc::now();

    // Update in storage
    storage
        .update_memory(&updated_memory)
        .await
        .expect("Failed to update memory");

    // Retrieve and verify
    let retrieved = storage
        .get_memory(memory_id.clone())
        .await
        .expect("Failed to retrieve memory");

    // Verify content was updated
    assert_eq!(retrieved.content, updated_content);

    // Verify embedding exists
    assert!(retrieved.embedding.is_some(), "Embedding should exist");

    // Verify embedding changed (should be different from initial)
    let retrieved_embedding = retrieved.embedding.unwrap();
    assert_eq!(
        retrieved_embedding.len(),
        updated_embedding.len(),
        "Embedding dimensions should match"
    );

    // Calculate cosine similarity to verify they're different
    // (same content would have similarity ~1.0, different content much lower)
    let dot_product: f32 = initial_embedding
        .iter()
        .zip(retrieved_embedding.iter())
        .map(|(a, b)| a * b)
        .sum();
    let mag_a: f32 = initial_embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = retrieved_embedding
        .iter()
        .map(|x| x * x)
        .sum::<f32>()
        .sqrt();
    let similarity = dot_product / (mag_a * mag_b);

    // Embeddings for different content should have lower similarity
    assert!(
        similarity < 0.95,
        "Embedding should have changed significantly (similarity: {})",
        similarity
    );

    println!("✅ Embedding regeneration test passed");
    println!("   Initial content: {}", initial_content);
    println!("   Updated content: {}", updated_content);
    println!("   Embedding similarity: {:.3}", similarity);
}

#[tokio::test]
async fn test_embedding_consistency() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test_embedding_consistency.db");

    let _storage: Arc<dyn StorageBackend> = Arc::new(
        LibsqlStorage::new_with_validation(
            ConnectionMode::Local(db_path.to_str().unwrap().to_string()),
            true, // create_if_missing
        )
        .await
        .unwrap(),
    );

    let llm_config = LlmConfig::default();
    let embeddings = Arc::new(EmbeddingService::new("test-key".to_string(), llm_config));

    // Test that same content produces similar embeddings
    let content = "Database architecture decisions";

    let embedding1 = embeddings.generate_embedding(content).await.unwrap();
    let embedding2 = embeddings.generate_embedding(content).await.unwrap();

    // Calculate similarity
    let dot_product: f32 = embedding1
        .iter()
        .zip(embedding2.iter())
        .map(|(a, b)| a * b)
        .sum();
    let mag_a: f32 = embedding1.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = embedding2.iter().map(|x| x * x).sum::<f32>().sqrt();
    let similarity = dot_product / (mag_a * mag_b);

    // Same content should produce very similar embeddings (close to 1.0)
    assert!(
        similarity > 0.95,
        "Same content should produce similar embeddings (similarity: {})",
        similarity
    );

    println!(
        "✅ Embedding consistency test passed (similarity: {:.3})",
        similarity
    );
}
