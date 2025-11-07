//! Integration tests for vector storage with sqlite-vec
//!
//! Tests the dual storage approach:
//! - rusqlite with sqlite-vec for vector operations with connection pooling
//! - Same database file as libsql
//! - Vector search functionality

use mnemosyne_core::storage::vectors::SqliteVectorStorage;
use mnemosyne_core::types::MemoryId;
use tempfile::TempDir;

/// Create a test vector storage instance
async fn create_test_storage() -> (SqliteVectorStorage, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test_vectors.db");
    let storage = SqliteVectorStorage::new(db_path, 1536).unwrap();
    storage.create_vec_table().await.unwrap();
    (storage, temp_dir)
}

/// Generate a random normalized embedding vector
fn generate_embedding(seed: u32, dimensions: usize) -> Vec<f32> {
    let mut vec: Vec<f32> = (0..dimensions)
        .map(|i| ((seed + i as u32) as f32).sin())
        .collect();

    // Normalize to unit length for cosine similarity
    let magnitude: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    vec.iter_mut().for_each(|x| *x /= magnitude);
    vec
}

#[tokio::test]
async fn test_store_and_retrieve_vector() {
    let (storage, _temp) = create_test_storage().await;
    let memory_id = MemoryId::new();
    let embedding = generate_embedding(42, 1536);

    // Store vector
    storage.store_vector(&memory_id, &embedding).await.unwrap();

    // Retrieve vector
    let retrieved = storage.get_vector(&memory_id).await.unwrap();
    assert!(retrieved.is_some(), "Vector should be found");

    let retrieved = retrieved.unwrap();
    assert_eq!(
        retrieved.len(),
        1536,
        "Vector should have correct dimensions"
    );

    // Check values are close (floating point comparison)
    for (a, b) in embedding.iter().zip(retrieved.iter()) {
        assert!(
            (a - b).abs() < 0.001,
            "Values should match within tolerance"
        );
    }
}

#[tokio::test]
async fn test_dimension_mismatch_error() {
    let (storage, _temp) = create_test_storage().await;
    let memory_id = MemoryId::new();
    let wrong_embedding = vec![1.0; 512]; // Wrong dimension (512 instead of 1536)

    let result = storage.store_vector(&memory_id, &wrong_embedding).await;
    assert!(result.is_err(), "Should fail with dimension mismatch");
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("dimension mismatch"),
        "Error should mention dimension mismatch"
    );
}

#[tokio::test]
async fn test_knn_search_with_similar_vectors() {
    let (storage, _temp) = create_test_storage().await;

    // Create a base vector
    let base_embedding = generate_embedding(100, 1536);

    // Create similar vector (small perturbation)
    let mut similar_embedding = base_embedding.clone();
    for i in 0..10 {
        similar_embedding[i] *= 1.01; // Small change
    }

    // Re-normalize
    let magnitude: f32 = similar_embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    similar_embedding.iter_mut().for_each(|x| *x /= magnitude);

    // Create different vector
    let different_embedding = generate_embedding(999, 1536);

    // Store vectors
    let id_base = MemoryId::new();
    let id_similar = MemoryId::new();
    let id_different = MemoryId::new();

    storage.store_vector(&id_base, &base_embedding).await.unwrap();
    storage
        .store_vector(&id_similar, &similar_embedding)
        .await
        .unwrap();
    storage
        .store_vector(&id_different, &different_embedding)
        .await
        .unwrap();

    // Search with base embedding
    let results = storage.search_similar(&base_embedding, 10, 0.5).await.unwrap();

    assert!(
        results.len() >= 2,
        "Should find at least base and similar vectors"
    );

    // First result should be exact match (id_base)
    let (found_id, similarity) = &results[0];
    assert_eq!(*found_id, id_base, "First result should be exact match");
    assert!(
        *similarity > 0.99,
        "Exact match should have very high similarity: {}",
        similarity
    );

    // Second result should be similar vector
    let (found_id, similarity) = &results[1];
    assert_eq!(
        *found_id, id_similar,
        "Second result should be similar vector"
    );
    assert!(
        *similarity > 0.9,
        "Similar vector should have high similarity: {}",
        similarity
    );

    // Different vector might or might not be in results depending on threshold
    // but if present, should have lower similarity
    if results.len() > 2 {
        let (_, similarity) = &results[2];
        assert!(
            *similarity < 0.95,
            "Different vector should have lower similarity"
        );
    }
}

#[tokio::test]
async fn test_delete_vector() {
    let (storage, _temp) = create_test_storage().await;
    let memory_id = MemoryId::new();
    let embedding = generate_embedding(42, 1536);

    // Store and verify
    storage.store_vector(&memory_id, &embedding).await.unwrap();
    assert!(
        storage.get_vector(&memory_id).await.unwrap().is_some(),
        "Vector should exist after storing"
    );

    // Delete and verify
    storage.delete_vector(&memory_id).await.unwrap();
    assert!(
        storage.get_vector(&memory_id).await.unwrap().is_none(),
        "Vector should not exist after deletion"
    );
}

#[tokio::test]
async fn test_count_vectors() {
    let (storage, _temp) = create_test_storage().await;

    assert_eq!(
        storage.count_vectors().await.unwrap(),
        0,
        "Should start with 0 vectors"
    );

    // Store some vectors
    for i in 0..5 {
        let id = MemoryId::new();
        let embedding = generate_embedding(i, 1536);
        storage.store_vector(&id, &embedding).await.unwrap();
    }

    assert_eq!(
        storage.count_vectors().await.unwrap(),
        5,
        "Should have 5 vectors after storing"
    );
}

#[tokio::test]
async fn test_batch_store_vectors() {
    let (mut storage, _temp) = create_test_storage().await;

    let vectors: Vec<(MemoryId, Vec<f32>)> = (0..10)
        .map(|i| (MemoryId::new(), generate_embedding(i, 1536)))
        .collect();

    let count = storage.batch_store_vectors(&vectors).await.unwrap();
    assert_eq!(count, 10, "Should store all 10 vectors");
    assert_eq!(
        storage.count_vectors().await.unwrap(),
        10,
        "Should have 10 vectors in storage"
    );
}

#[tokio::test]
async fn test_replace_existing_vector() {
    let (storage, _temp) = create_test_storage().await;
    let memory_id = MemoryId::new();

    // Store first vector
    let embedding1 = generate_embedding(1, 1536);
    storage.store_vector(&memory_id, &embedding1).await.unwrap();

    // Store second vector with same ID (should replace)
    let embedding2 = generate_embedding(2, 1536);
    storage.store_vector(&memory_id, &embedding2).await.unwrap();

    // Should only have one vector
    assert_eq!(
        storage.count_vectors().await.unwrap(),
        1,
        "Should have only 1 vector after replacement"
    );

    // Retrieved vector should match second embedding
    let retrieved = storage.get_vector(&memory_id).await.unwrap().unwrap();
    for (a, b) in embedding2.iter().zip(retrieved.iter()) {
        assert!((a - b).abs() < 0.001);
    }
}

#[tokio::test]
async fn test_search_with_limit() {
    let (mut storage, _temp) = create_test_storage().await;

    // Store 20 vectors
    let embeddings: Vec<(MemoryId, Vec<f32>)> = (0..20)
        .map(|i| (MemoryId::new(), generate_embedding(i, 1536)))
        .collect();

    storage.batch_store_vectors(&embeddings).await.unwrap();

    // Search with limit of 5
    let query = generate_embedding(0, 1536);
    let results = storage.search_similar(&query, 5, 0.0).await.unwrap();

    assert_eq!(
        results.len(),
        5,
        "Should return exactly 5 results when limit is 5"
    );
}

#[tokio::test]
async fn test_search_with_min_similarity_threshold() {
    let (storage, _temp) = create_test_storage().await;

    // Create three vectors with known relationships
    let base = generate_embedding(100, 1536);

    // Very similar vector
    let mut very_similar = base.clone();
    for i in 0..5 {
        very_similar[i] *= 1.005;
    }
    let magnitude: f32 = very_similar.iter().map(|x| x * x).sum::<f32>().sqrt();
    very_similar.iter_mut().for_each(|x| *x /= magnitude);

    // Completely different vector
    let different = generate_embedding(999, 1536);

    let id1 = MemoryId::new();
    let id2 = MemoryId::new();
    let id3 = MemoryId::new();

    storage.store_vector(&id1, &base).await.unwrap();
    storage.store_vector(&id2, &very_similar).await.unwrap();
    storage.store_vector(&id3, &different).await.unwrap();

    // Search with high threshold (0.95)
    let results = storage.search_similar(&base, 10, 0.95).await.unwrap();

    // Should only find base and very_similar
    assert!(
        results.len() >= 2 && results.len() <= 3,
        "Should find 2-3 results with high threshold, got {}",
        results.len()
    );

    for (id, similarity) in &results {
        assert!(
            *similarity >= 0.95,
            "All results should meet minimum similarity threshold"
        );
    }
}

#[tokio::test]
async fn test_empty_search_returns_empty() {
    let (storage, _temp) = create_test_storage().await;

    // Search in empty storage
    let query = generate_embedding(0, 1536);
    let results = storage.search_similar(&query, 10, 0.0).await.unwrap();

    assert_eq!(
        results.len(),
        0,
        "Should return empty results for empty storage"
    );
}

#[tokio::test]
async fn test_nonexistent_vector_returns_none() {
    let (storage, _temp) = create_test_storage().await;
    let nonexistent_id = MemoryId::new();

    let result = storage.get_vector(&nonexistent_id).await.unwrap();
    assert!(
        result.is_none(),
        "Should return None for nonexistent vector"
    );
}

/// Performance test: Search should be fast even with many vectors
#[tokio::test]
#[ignore] // Run with: cargo test --test integration_vector_storage -- --ignored
async fn test_search_performance_10k_vectors() {
    let (mut storage, _temp) = create_test_storage().await;

    // Store 10,000 vectors
    println!("Storing 10,000 vectors...");
    let start = std::time::Instant::now();

    let batch_size = 1000;
    for batch in 0..10 {
        let vectors: Vec<(MemoryId, Vec<f32>)> = (0..batch_size)
            .map(|i| {
                (
                    MemoryId::new(),
                    generate_embedding(batch * batch_size + i, 1536),
                )
            })
            .collect();
        storage.batch_store_vectors(&vectors).await.unwrap();
    }

    let store_duration = start.elapsed();
    println!("Stored 10,000 vectors in {:?}", store_duration);

    // Perform search
    let query = generate_embedding(5000, 1536);
    let search_start = std::time::Instant::now();
    let results = storage.search_similar(&query, 10, 0.5).await.unwrap();
    let search_duration = search_start.elapsed();

    println!("Search completed in {:?}", search_duration);
    println!("Found {} results", results.len());

    // Assert search is under 10ms (target performance)
    assert!(
        search_duration.as_millis() < 10,
        "Search should complete in under 10ms, took {:?}",
        search_duration
    );
}
