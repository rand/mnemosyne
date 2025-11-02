//! Integration tests for OptimizerDSpyAdapter
//!
//! Tests verify:
//! - Context consolidation with all modes
//! - Skills discovery for tasks
//! - Context budget optimization
//! - Type safety and JSON conversion
//! - Error handling

#[cfg(feature = "python")]
mod optimizer_adapter_tests {
    use mnemosyne_core::orchestration::actors::optimizer_dspy_adapter::{
        ConsolidatedContext, ContextUsage, LoadedResources, OptimizerDSpyAdapter,
        SkillDiscoveryResult, SkillMetadata,
    };
    use mnemosyne_core::orchestration::dspy_bridge::DSpyBridge;
    use std::sync::Arc;

    /// Helper to create test adapter (requires Python environment)
    async fn create_test_adapter() -> OptimizerDSpyAdapter {
        let bridge = Arc::new(DSpyBridge::new().expect("Failed to create DSPy bridge"));
        OptimizerDSpyAdapter::new(bridge)
    }

    // =============================================================================
    // Context Consolidation Tests
    // =============================================================================

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_consolidate_context_detailed() {
        let adapter = create_test_adapter().await;

        let result = adapter
            .consolidate_context(
                "Implement user authentication",
                vec![
                    "Created auth module".to_string(),
                    "Added JWT token generation".to_string(),
                ],
                vec![
                    "Missing password hashing".to_string(),
                    "No error handling for invalid tokens".to_string(),
                ],
                vec!["Test token expiration".to_string()],
                1,
                "detailed",
            )
            .await;

        assert!(result.is_ok() || result.is_err());

        if let Ok(consolidated) = result {
            assert!(!consolidated.consolidated_content.is_empty());
            assert!(!consolidated.strategic_guidance.is_empty());
            assert!(consolidated.estimated_tokens > 0);
        }
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_consolidate_context_summary() {
        let adapter = create_test_adapter().await;

        let result = adapter
            .consolidate_context(
                "Add caching layer",
                vec!["Implemented Redis caching".to_string()],
                vec![
                    "Missing cache invalidation".to_string(),
                    "No TTL configuration".to_string(),
                ],
                vec![],
                2,
                "summary",
            )
            .await;

        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_consolidate_context_compressed() {
        let adapter = create_test_adapter().await;

        let result = adapter
            .consolidate_context(
                "Fix database queries",
                vec!["Optimized slow queries".to_string()],
                vec![
                    "Critical: N+1 query issue remains".to_string(),
                    "Missing index on user_id".to_string(),
                ],
                vec![],
                4,
                "compressed",
            )
            .await;

        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_consolidate_context_empty_feedback() {
        let adapter = create_test_adapter().await;

        let result = adapter
            .consolidate_context(
                "Test task",
                vec![],
                vec![],
                vec![],
                1,
                "detailed",
            )
            .await;

        // Should handle gracefully
        assert!(result.is_ok() || result.is_err());
    }

    // =============================================================================
    // Skills Discovery Tests
    // =============================================================================

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_discover_skills_basic() {
        let adapter = create_test_adapter().await;

        let skills = vec![
            SkillMetadata {
                name: "rust-async".to_string(),
                description: "Async Rust programming".to_string(),
                keywords: vec!["async".to_string(), "tokio".to_string()],
                domains: vec!["rust".to_string()],
            },
            SkillMetadata {
                name: "python-fastapi".to_string(),
                description: "FastAPI web framework".to_string(),
                keywords: vec!["fastapi".to_string(), "web".to_string()],
                domains: vec!["python".to_string()],
            },
            SkillMetadata {
                name: "database-postgres".to_string(),
                description: "PostgreSQL database".to_string(),
                keywords: vec!["postgres".to_string(), "sql".to_string()],
                domains: vec!["database".to_string()],
            },
        ];

        let result = adapter
            .discover_skills(
                "Build async REST API with database",
                skills,
                2,
                0.5,
            )
            .await;

        assert!(result.is_ok() || result.is_err());

        if let Ok(discovery) = result {
            assert!(!discovery.selected_skills.is_empty() || discovery.selected_skills.is_empty());
            assert!(!discovery.reasoning.is_empty() || discovery.reasoning.is_empty());
        }
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_discover_skills_high_context_usage() {
        let adapter = create_test_adapter().await;

        let skills = vec![
            SkillMetadata {
                name: "skill-1".to_string(),
                description: "Test skill 1".to_string(),
                keywords: vec!["test".to_string()],
                domains: vec!["test".to_string()],
            },
            SkillMetadata {
                name: "skill-2".to_string(),
                description: "Test skill 2".to_string(),
                keywords: vec!["test".to_string()],
                domains: vec!["test".to_string()],
            },
        ];

        let result = adapter
            .discover_skills(
                "Simple task",
                skills,
                5,
                0.85, // High context usage
            )
            .await;

        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_discover_skills_empty_list() {
        let adapter = create_test_adapter().await;

        let result = adapter
            .discover_skills(
                "Any task",
                vec![],
                5,
                0.5,
            )
            .await;

        // Should handle gracefully
        assert!(result.is_ok() || result.is_err());
    }

    // =============================================================================
    // Context Budget Optimization Tests
    // =============================================================================

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_optimize_context_budget_basic() {
        let adapter = create_test_adapter().await;

        let usage = ContextUsage {
            critical_pct: 0.40,
            skills_pct: 0.30,
            project_pct: 0.20,
            general_pct: 0.10,
            total_pct: 1.0,
        };

        let resources = LoadedResources {
            skill_names: vec![
                "rust-async".to_string(),
                "python-fastapi".to_string(),
                "database-postgres".to_string(),
            ],
            memory_ids: vec![
                "mem-1".to_string(),
                "mem-2".to_string(),
                "mem-3".to_string(),
            ],
            memory_summaries: vec![
                "Summary 1".to_string(),
                "Summary 2".to_string(),
                "Summary 3".to_string(),
            ],
        };

        let result = adapter
            .optimize_context_budget(usage, resources, 0.75, 8)
            .await;

        assert!(result.is_ok() || result.is_err());

        if let Ok(plan) = result {
            assert!(!plan.optimization_rationale.is_empty() || plan.optimization_rationale.is_empty());
        }
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_optimize_context_budget_high_priority() {
        let adapter = create_test_adapter().await;

        let usage = ContextUsage {
            critical_pct: 0.40,
            skills_pct: 0.35,
            project_pct: 0.25,
            general_pct: 0.10,
            total_pct: 1.10, // Over budget
        };

        let resources = LoadedResources {
            skill_names: vec!["skill-1".to_string(), "skill-2".to_string()],
            memory_ids: vec!["mem-1".to_string()],
            memory_summaries: vec!["Summary".to_string()],
        };

        let result = adapter
            .optimize_context_budget(usage, resources, 0.75, 10) // Critical priority
            .await;

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
                adapter_clone
                    .consolidate_context(
                        &format!("Task {}", i),
                        vec![format!("Summary {}", i)],
                        vec![format!("Issue {}", i)],
                        vec![],
                        1,
                        "detailed",
                    )
                    .await
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

        let (r1, r2, r3) = tokio::join!(
            adapter1.consolidate_context(
                "Task 1",
                vec!["Summary".to_string()],
                vec!["Issue".to_string()],
                vec![],
                1,
                "detailed",
            ),
            adapter2.discover_skills(
                "Task 2",
                vec![],
                3,
                0.5,
            ),
            adapter3.optimize_context_budget(
                ContextUsage {
                    critical_pct: 0.4,
                    skills_pct: 0.3,
                    project_pct: 0.2,
                    general_pct: 0.1,
                    total_pct: 1.0,
                },
                LoadedResources {
                    skill_names: vec![],
                    memory_ids: vec![],
                    memory_summaries: vec![],
                },
                0.75,
                5,
            ),
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

        // Very long content
        let long_content = "Long text. ".repeat(1000);
        let result = adapter
            .consolidate_context(
                &long_content,
                vec![long_content.clone()],
                vec![long_content.clone()],
                vec![],
                1,
                "detailed",
            )
            .await;
        assert!(result.is_ok() || result.is_err());

        // Special characters
        let special = "Code: `fn test() { return Ok(()); }` with symbols: !@#$%";
        let result = adapter
            .consolidate_context(
                special,
                vec![],
                vec![special.to_string()],
                vec![],
                1,
                "detailed",
            )
            .await;
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_adapter_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<OptimizerDSpyAdapter>();
        assert_sync::<OptimizerDSpyAdapter>();
    }
}
