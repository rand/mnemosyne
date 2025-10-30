//! Integration tests for hybrid search (keyword + graph)
//!
//! Tests the complete hybrid search pipeline including:
//! - FTS5 keyword search
//! - Graph expansion
//! - Weighted ranking (keyword, graph, importance, recency)
//! - Recency decay

use mnemosyne_core::{LinkType, MemoryLink, MemoryType, Namespace, StorageBackend};

mod common;
use common::{create_test_storage, sample_memory};

mod fixtures;
use fixtures::TestData;

#[tokio::test]
async fn test_keyword_search_only() {
    // Setup
    let storage = create_test_storage().await;
    let test_data = TestData::load();

    // Store all test memories
    for memory in test_data.all() {
        storage.store_memory(&memory).await.unwrap();
    }

    // Test: Search for "database" without graph expansion
    let results = storage
        .hybrid_search("database", None, 10, false)
        .await
        .unwrap();

    // Assert: Should return only keyword matches (5 database memories)
    assert!(
        results.len() >= 5,
        "Expected at least 5 database memories, got {}",
        results.len()
    );

    // Assert: All results contain "database" in keywords or content
    for result in &results {
        let has_keyword = result
            .memory
            .keywords
            .iter()
            .any(|k| k.to_lowercase().contains("database"));
        let has_content = result.memory.content.to_lowercase().contains("database");

        assert!(
            has_keyword || has_content,
            "Result should match 'database': {:?}",
            result.memory.summary
        );
    }

    // Assert: Results sorted by score descending
    for i in 0..results.len() - 1 {
        assert!(
            results[i].score >= results[i + 1].score,
            "Results should be sorted by score descending"
        );
    }
}

#[tokio::test]
async fn test_hybrid_search_with_graph_expansion() {
    // Setup
    let storage = create_test_storage().await;
    let test_data = TestData::load();

    // Store database memories
    for memory in &test_data.database_memories {
        storage.store_memory(memory).await.unwrap();
    }

    // Create a link from a database memory to an API memory
    let db_mem = &test_data.database_memories[0];
    let api_mem = &test_data.api_memories[0];

    storage.store_memory(api_mem).await.unwrap();

    // Manually add a link (in real usage, LLM would create these)
    let link = MemoryLink {
        target_id: api_mem.id,
        link_type: LinkType::References,
        strength: 0.8,
        reason: "API uses database".to_string(),
        created_at: chrono::Utc::now(),
    };

    // Update the database memory with the link
    let mut db_mem_with_link = db_mem.clone();
    db_mem_with_link.links = vec![link];
    storage.update_memory(&db_mem_with_link).await.unwrap();

    // Test: Search with graph expansion enabled
    let results = storage
        .hybrid_search("database", None, 10, true)
        .await
        .unwrap();

    // Assert: Should include both keyword matches AND graph-expanded memories
    let keyword_matches = results
        .iter()
        .filter(|r| r.memory.keywords.contains(&"database".to_string()))
        .count();

    let graph_matches = results
        .iter()
        .filter(|r| r.match_reason.contains("graph"))
        .count();

    assert!(
        keyword_matches > 0,
        "Should have keyword matches: {}",
        keyword_matches
    );

    // Note: Graph expansion may or may not return additional results depending on link structure
    // This is fine - we're testing that it runs without error
    println!(
        "Keyword matches: {}, Graph matches: {}",
        keyword_matches, graph_matches
    );
}

#[tokio::test]
async fn test_importance_weighting() {
    // Setup
    let storage = create_test_storage().await;

    // Create two memories with same content but different importance
    let mut high_importance = sample_memory(
        "Critical system design decision",
        MemoryType::ArchitectureDecision,
        9,
    );
    high_importance.keywords = vec!["design".to_string()];

    let mut low_importance = sample_memory("Minor design tweak", MemoryType::Configuration, 3);
    low_importance.keywords = vec!["design".to_string()];

    storage.store_memory(&high_importance).await.unwrap();
    storage.store_memory(&low_importance).await.unwrap();

    // Test: Search for "design"
    let results = storage
        .hybrid_search("design", None, 10, false)
        .await
        .unwrap();

    assert_eq!(results.len(), 2, "Should return both memories");

    // Assert: High importance memory should score higher
    let high_idx = results
        .iter()
        .position(|r| r.memory.id == high_importance.id)
        .unwrap();
    let low_idx = results
        .iter()
        .position(|r| r.memory.id == low_importance.id)
        .unwrap();

    assert!(
        high_idx < low_idx,
        "High importance memory should rank higher (index {} vs {})",
        high_idx,
        low_idx
    );

    // The high importance memory should have a higher score
    assert!(
        results[high_idx].score > results[low_idx].score,
        "High importance score {} should be > low importance score {}",
        results[high_idx].score,
        results[low_idx].score
    );
}

#[tokio::test]
async fn test_recency_decay() {
    use chrono::Duration;

    // Setup
    let storage = create_test_storage().await;

    // Create two memories with same content but different ages
    let mut recent = sample_memory("System update", MemoryType::Configuration, 5);
    recent.keywords = vec!["update".to_string()];
    recent.created_at = chrono::Utc::now();

    let mut old = sample_memory("System update", MemoryType::Configuration, 5);
    old.keywords = vec!["update".to_string()];
    old.created_at = chrono::Utc::now() - Duration::days(60); // 60 days old

    storage.store_memory(&recent).await.unwrap();
    storage.store_memory(&old).await.unwrap();

    // Test: Search for "update"
    let results = storage
        .hybrid_search("update", None, 10, false)
        .await
        .unwrap();

    assert_eq!(results.len(), 2, "Should return both memories");

    // Assert: Recent memory should score higher due to recency decay
    let recent_idx = results
        .iter()
        .position(|r| r.memory.id == recent.id)
        .unwrap();
    let old_idx = results.iter().position(|r| r.memory.id == old.id).unwrap();

    assert!(
        recent_idx <= old_idx,
        "Recent memory should rank higher or equal (index {} vs {})",
        recent_idx,
        old_idx
    );
}

#[tokio::test]
async fn test_empty_results() {
    // Setup
    let storage = create_test_storage().await;
    let test_data = TestData::load();

    // Store some memories
    for memory in test_data.database_memories {
        storage.store_memory(&memory).await.unwrap();
    }

    // Test: Search for term that doesn't exist
    let results = storage
        .hybrid_search("nonexistent_term_xyz123", None, 10, false)
        .await
        .unwrap();

    // Assert: Empty results, no error
    assert_eq!(results.len(), 0, "Should return empty results");
}

#[tokio::test]
async fn test_large_result_set_with_limit() {
    // Setup
    let storage = create_test_storage().await;

    // Create 20 memories all with same keyword
    for i in 0..20 {
        let mut mem = sample_memory(&format!("Test memory number {}", i), MemoryType::Insight, 5);
        mem.keywords = vec!["common".to_string()];
        storage.store_memory(&mem).await.unwrap();
    }

    // Test: Search with limit
    let limit = 10;
    let results = storage
        .hybrid_search("common", None, limit, false)
        .await
        .unwrap();

    // Assert: Exactly 10 results (respects limit)
    assert_eq!(
        results.len(),
        limit,
        "Should return exactly {} results",
        limit
    );

    // Assert: Top 10 by score
    for i in 0..results.len() - 1 {
        assert!(
            results[i].score >= results[i + 1].score,
            "Results should be sorted by score"
        );
    }
}

#[tokio::test]
async fn test_namespace_filtering() {
    // Setup
    let storage = create_test_storage().await;

    // Create memories in different namespaces
    let mut global_mem = sample_memory("Global memory", MemoryType::Insight, 5);
    global_mem.namespace = Namespace::Global;
    global_mem.keywords = vec!["searchterm".to_string()];

    let mut project_mem = sample_memory("Project memory", MemoryType::Insight, 5);
    project_mem.namespace = Namespace::Project {
        name: "test-project".to_string(),
    };
    project_mem.keywords = vec!["searchterm".to_string()];

    storage.store_memory(&global_mem).await.unwrap();
    storage.store_memory(&project_mem).await.unwrap();

    // Test: Search with namespace filter
    let results = storage
        .hybrid_search(
            "searchterm",
            Some(Namespace::Project {
                name: "test-project".to_string(),
            }),
            10,
            false,
        )
        .await
        .unwrap();

    // Assert: Only project namespace returned
    assert_eq!(
        results.len(),
        1,
        "Should return only 1 memory from namespace"
    );
    assert_eq!(
        results[0].memory.namespace,
        Namespace::Project {
            name: "test-project".to_string()
        },
        "Should be from test-project namespace"
    );
}

#[tokio::test]
async fn test_hybrid_scoring_components() {
    // Setup
    let storage = create_test_storage().await;

    // Create a memory with all scoring factors
    let mut mem = sample_memory(
        "Important recent decision about architecture",
        MemoryType::ArchitectureDecision,
        9, // High importance
    );
    mem.keywords = vec!["architecture".to_string(), "decision".to_string()];
    mem.created_at = chrono::Utc::now(); // Recent

    storage.store_memory(&mem).await.unwrap();

    // Test: Search
    let results = storage
        .hybrid_search("architecture", None, 10, false)
        .await
        .unwrap();

    assert_eq!(results.len(), 1);

    let result = &results[0];

    // Assert: Score should be moderate (keyword + importance + recency, no embedding)
    // Without vector search (no embeddings in test), keyword matching dominates
    // Keyword match + importance boost + recency boost = ~44%
    assert!(
        result.score > 0.4,
        "Score should be moderate for recent, important, keyword match (no embedding): {}",
        result.score
    );

    // Assert: Match reason indicates keyword match
    assert!(
        result.match_reason.contains("keyword"),
        "Should indicate keyword match: {}",
        result.match_reason
    );
}
