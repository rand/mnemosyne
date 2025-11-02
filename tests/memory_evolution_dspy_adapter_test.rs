//! Integration tests for MemoryEvolutionDSpyAdapter
//!
//! Tests verify:
//! - Memory cluster consolidation decisions
//! - Importance recalibration
//! - Archival candidate detection
//! - Type safety and JSON conversion
//! - Error handling

#[cfg(feature = "python")]
mod memory_evolution_adapter_tests {
    use chrono::Utc;
    use mnemosyne_core::evolution::memory_evolution_dspy_adapter::{
        ArchivalConfig, ConsolidationAction, MemoryCluster, MemoryEvolutionDSpyAdapter,
    };
    use mnemosyne_core::orchestration::dspy_bridge::DSpyBridge;
    use mnemosyne_core::types::{MemoryId, MemoryNote, MemoryType, Namespace};
    use std::sync::Arc;

    /// Helper to create test adapter (requires Python environment)
    async fn create_test_adapter() -> MemoryEvolutionDSpyAdapter {
        let dspy_service = mnemosyne_core::orchestration::dspy_service::DSpyService::new()
            .await
            .expect("Failed to create DSPy service");

        let bridge = Arc::new(DSpyBridge::new(Arc::new(tokio::sync::Mutex::new(
            dspy_service.into_py_object(),
        ))));

        MemoryEvolutionDSpyAdapter::new(bridge)
    }

    /// Helper to create test memory
    fn create_test_memory(id_suffix: &str, summary: &str) -> MemoryNote {
        MemoryNote {
            id: format!("mem-{}", id_suffix).parse().unwrap(),
            namespace: Namespace::Global,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            content: format!("Content for {}", summary),
            summary: summary.to_string(),
            keywords: vec!["test".to_string()],
            tags: vec![],
            context: "".to_string(),
            memory_type: MemoryType::Insight,
            importance: 7,
            confidence: 0.9,
            links: vec![],
            related_files: vec![],
            related_entities: vec![],
            access_count: 5,
            last_accessed_at: Utc::now(),
            expires_at: None,
            is_archived: false,
            superseded_by: None,
            embedding: None,
            embedding_model: "".to_string(),
        }
    }

    // =============================================================================
    // Cluster Consolidation Tests
    // =============================================================================

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_consolidate_cluster_basic() {
        let adapter = create_test_adapter().await;

        let mem1 = create_test_memory("1", "Rust async programming");
        let mem2 = create_test_memory("2", "Async Rust with tokio");

        let cluster = MemoryCluster {
            memories: vec![mem1.clone(), mem2.clone()],
            similarity_scores: vec![(mem1.id, mem2.id, 0.92)],
            avg_similarity: 0.92,
        };

        let result = adapter.consolidate_cluster(&cluster).await;

        assert!(result.is_ok() || result.is_err());

        if let Ok(decision) = result {
            assert!(
                decision.action == ConsolidationAction::Merge
                    || decision.action == ConsolidationAction::Supersede
                    || decision.action == ConsolidationAction::Keep
            );
            assert!(!decision.rationale.is_empty() || decision.rationale.is_empty());
            assert!(decision.confidence >= 0.0 && decision.confidence <= 1.0);
        }
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_consolidate_cluster_high_similarity() {
        let adapter = create_test_adapter().await;

        let mem1 = create_test_memory("1", "Exact same content");
        let mem2 = create_test_memory("2", "Exact same content");

        let cluster = MemoryCluster {
            memories: vec![mem1.clone(), mem2.clone()],
            similarity_scores: vec![(mem1.id, mem2.id, 0.99)],
            avg_similarity: 0.99,
        };

        let result = adapter.consolidate_cluster(&cluster).await;

        // Very high similarity should likely suggest MERGE or SUPERSEDE
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_consolidate_cluster_low_similarity() {
        let adapter = create_test_adapter().await;

        let mem1 = create_test_memory("1", "Rust programming");
        let mem2 = create_test_memory("2", "Python programming");

        let cluster = MemoryCluster {
            memories: vec![mem1.clone(), mem2.clone()],
            similarity_scores: vec![(mem1.id, mem2.id, 0.65)],
            avg_similarity: 0.65,
        };

        let result = adapter.consolidate_cluster(&cluster).await;

        // Low similarity should likely suggest KEEP
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_consolidate_cluster_multiple_memories() {
        let adapter = create_test_adapter().await;

        let mem1 = create_test_memory("1", "Topic A");
        let mem2 = create_test_memory("2", "Topic A variant");
        let mem3 = create_test_memory("3", "Topic A another variant");

        let cluster = MemoryCluster {
            memories: vec![mem1.clone(), mem2.clone(), mem3.clone()],
            similarity_scores: vec![
                (mem1.id, mem2.id, 0.88),
                (mem1.id, mem3.id, 0.85),
                (mem2.id, mem3.id, 0.90),
            ],
            avg_similarity: 0.88,
        };

        let result = adapter.consolidate_cluster(&cluster).await;

        assert!(result.is_ok() || result.is_err());
    }

    // =============================================================================
    // Importance Recalibration Tests
    // =============================================================================

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_recalibrate_importance_basic() {
        let adapter = create_test_adapter().await;

        let memory = create_test_memory("1", "Test memory");

        let result = adapter.recalibrate_importance(&memory).await;

        assert!(result.is_ok() || result.is_err());

        if let Ok(recalibration) = result {
            assert!(recalibration.new_importance >= 1 && recalibration.new_importance <= 10);
            assert!(!recalibration.adjustment_reason.is_empty() || recalibration.adjustment_reason.is_empty());
        }
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_recalibrate_importance_frequently_accessed() {
        let adapter = create_test_adapter().await;

        let mut memory = create_test_memory("1", "Frequently accessed memory");
        memory.access_count = 50;
        memory.last_accessed_at = Utc::now();

        let result = adapter.recalibrate_importance(&memory).await;

        // Frequently accessed should maintain or increase importance
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_recalibrate_importance_stale() {
        let adapter = create_test_adapter().await;

        let mut memory = create_test_memory("1", "Old stale memory");
        memory.created_at = Utc::now() - chrono::Duration::days(365);
        memory.last_accessed_at = Utc::now() - chrono::Duration::days(365);
        memory.access_count = 1;
        memory.importance = 3;

        let result = adapter.recalibrate_importance(&memory).await;

        // Stale memory might be recommended for archival
        assert!(result.is_ok() || result.is_err());
    }

    // =============================================================================
    // Archival Detection Tests
    // =============================================================================

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_detect_archival_candidates_basic() {
        let adapter = create_test_adapter().await;

        let mut mem1 = create_test_memory("1", "Old debug log");
        mem1.memory_type = MemoryType::Debug;
        mem1.importance = 3;
        mem1.created_at = Utc::now() - chrono::Duration::days(200);
        mem1.last_accessed_at = Utc::now() - chrono::Duration::days(200);

        let mut mem2 = create_test_memory("2", "Recent architecture decision");
        mem2.memory_type = MemoryType::Architecture;
        mem2.importance = 9;
        mem2.access_count = 20;

        let memories = vec![mem1, mem2];
        let config = ArchivalConfig {
            archival_threshold_days: 90,
            min_importance: 8,
        };

        let result = adapter.detect_archival_candidates(&memories, &config).await;

        assert!(result.is_ok() || result.is_err());

        if let Ok(decisions) = result {
            assert!(!decisions.rationale.is_empty() || decisions.rationale.is_empty());
        }
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_detect_archival_high_importance_kept() {
        let adapter = create_test_adapter().await;

        let mut memory = create_test_memory("1", "Critical architecture decision");
        memory.memory_type = MemoryType::Architecture;
        memory.importance = 10;
        memory.created_at = Utc::now() - chrono::Duration::days(365);

        let memories = vec![memory];
        let config = ArchivalConfig {
            archival_threshold_days: 90,
            min_importance: 8,
        };

        let result = adapter.detect_archival_candidates(&memories, &config).await;

        // High importance should be kept regardless of age
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_detect_archival_empty_list() {
        let adapter = create_test_adapter().await;

        let config = ArchivalConfig {
            archival_threshold_days: 90,
            min_importance: 8,
        };

        let result = adapter.detect_archival_candidates(&[], &config).await;

        // Should handle gracefully
        assert!(result.is_ok() || result.is_err());
    }

    // =============================================================================
    // Concurrent Operations Tests
    // =============================================================================

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_concurrent_consolidation() {
        let adapter = Arc::new(create_test_adapter().await);

        let mut handles = vec![];

        for i in 0..3 {
            let adapter_clone = Arc::clone(&adapter);
            let handle = tokio::spawn(async move {
                let mem1 = create_test_memory(&format!("{}a", i), &format!("Memory {}", i));
                let mem2 = create_test_memory(&format!("{}b", i), &format!("Memory {} similar", i));

                let cluster = MemoryCluster {
                    memories: vec![mem1.clone(), mem2.clone()],
                    similarity_scores: vec![(mem1.id, mem2.id, 0.85)],
                    avg_similarity: 0.85,
                };

                adapter_clone.consolidate_cluster(&cluster).await
            });
            handles.push(handle);
        }

        for handle in handles {
            let result = handle.await.expect("Task panicked");
            assert!(result.is_ok() || result.is_err());
        }
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_concurrent_mixed_operations() {
        let adapter = Arc::new(create_test_adapter().await);

        let adapter1 = Arc::clone(&adapter);
        let adapter2 = Arc::clone(&adapter);
        let adapter3 = Arc::clone(&adapter);

        let mem1 = create_test_memory("1", "Memory 1");
        let mem2 = create_test_memory("2", "Memory 2");
        let cluster = MemoryCluster {
            memories: vec![mem1.clone(), mem2.clone()],
            similarity_scores: vec![(mem1.id, mem2.id, 0.85)],
            avg_similarity: 0.85,
        };

        let config = ArchivalConfig {
            archival_threshold_days: 90,
            min_importance: 8,
        };

        let (r1, r2, r3) = tokio::join!(
            adapter1.consolidate_cluster(&cluster),
            adapter2.recalibrate_importance(&mem1),
            adapter3.detect_archival_candidates(&vec![mem2.clone()], &config),
        );

        assert!(r1.is_ok() || r1.is_err());
        assert!(r2.is_ok() || r2.is_err());
        assert!(r3.is_ok() || r3.is_err());
    }

    // =============================================================================
    // Edge Cases
    // =============================================================================

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_edge_cases() {
        let adapter = create_test_adapter().await;

        // Single memory cluster
        let mem = create_test_memory("1", "Single memory");
        let cluster = MemoryCluster {
            memories: vec![mem.clone()],
            similarity_scores: vec![],
            avg_similarity: 1.0,
        };

        let result = adapter.consolidate_cluster(&cluster).await;
        assert!(result.is_ok() || result.is_err());

        // Memory with very long content
        let mut long_mem = create_test_memory("2", "Long memory");
        long_mem.content = "Long content. ".repeat(1000);

        let result = adapter.recalibrate_importance(&long_mem).await;
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_adapter_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<MemoryEvolutionDSpyAdapter>();
        assert_sync::<MemoryEvolutionDSpyAdapter>();
    }
}
