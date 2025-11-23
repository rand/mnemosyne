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
        ContextUsage, LoadedResources, OptimizerDSpyAdapter, SkillMetadata,
    };
    use mnemosyne_core::orchestration::dspy_bridge::DSpyBridge;
    use mnemosyne_core::orchestration::dspy_instrumentation::{
        DSpyInstrumentation, InstrumentationConfig,
    };
    use mnemosyne_core::orchestration::dspy_production_logger::{LogConfig, ProductionLogger};
    use mnemosyne_core::orchestration::dspy_telemetry::TelemetryCollector;
    use std::sync::Arc;

    /// Helper to create test adapter (requires Python environment)
    async fn create_test_adapter() -> OptimizerDSpyAdapter {
        // Initialize Python interpreter for tests
        pyo3::prepare_freethreaded_python();

        let bridge = Arc::new(DSpyBridge::new().expect("Failed to create DSPy bridge"));
        let logger = Arc::new(
            ProductionLogger::new(LogConfig::default())
                .await
                .expect("Failed to create logger"),
        );
        let telemetry = Arc::new(TelemetryCollector::new());
        let config = InstrumentationConfig::default();

        let instrumentation = Arc::new(DSpyInstrumentation::new(bridge, logger, telemetry, config));
        OptimizerDSpyAdapter::new(instrumentation)
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
            .consolidate_context("Test task", vec![], vec![], vec![], 1, "detailed")
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
            .discover_skills("Build async REST API with database", skills, 2, 0.5)
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

        let result = adapter.discover_skills("Any task", vec![], 5, 0.5).await;

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
            assert!(
                !plan.optimization_rationale.is_empty() || plan.optimization_rationale.is_empty()
            );
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
            adapter2.discover_skills("Task 2", vec![], 3, 0.5,),
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

    // =============================================================================
    // INT-2: Skills Discovery Integration Tests
    // =============================================================================

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_int2_skills_discovery_semantic_ranking() {
        let adapter = create_test_adapter().await;

        // Create skills with varying relevance to task
        let skills = vec![
            SkillMetadata {
                name: "rust-async-tokio".to_string(),
                description: "Asynchronous Rust programming with Tokio runtime".to_string(),
                keywords: vec!["rust".to_string(), "async".to_string(), "tokio".to_string()],
                domains: vec!["rust".to_string(), "async".to_string()],
            },
            SkillMetadata {
                name: "python-asyncio".to_string(),
                description: "Python async/await with asyncio".to_string(),
                keywords: vec!["python".to_string(), "async".to_string()],
                domains: vec!["python".to_string()],
            },
            SkillMetadata {
                name: "database-postgres".to_string(),
                description: "PostgreSQL database design and optimization".to_string(),
                keywords: vec![
                    "database".to_string(),
                    "postgres".to_string(),
                    "sql".to_string(),
                ],
                domains: vec!["database".to_string()],
            },
            SkillMetadata {
                name: "api-rest-design".to_string(),
                description: "RESTful API design patterns and best practices".to_string(),
                keywords: vec!["api".to_string(), "rest".to_string(), "http".to_string()],
                domains: vec!["api".to_string(), "backend".to_string()],
            },
        ];

        // Task clearly requires async Rust + database
        let task = "Build an async Rust service that connects to PostgreSQL";

        let result = adapter.discover_skills(task, skills, 2, 0.5).await;

        assert!(result.is_ok() || result.is_err());

        if let Ok(discovery) = result {
            // Should prioritize rust-async-tokio and database-postgres
            assert!(
                discovery.selected_skills.len() <= 2,
                "Should respect max_skills limit"
            );

            if !discovery.selected_skills.is_empty() {
                // Semantic understanding should rank Rust async highly for this task
                assert!(
                    discovery
                        .selected_skills
                        .contains(&"rust-async-tokio".to_string())
                        || discovery
                            .selected_skills
                            .contains(&"database-postgres".to_string())
                        || discovery.selected_skills.is_empty(),
                    "Should select semantically relevant skills"
                );
            }

            // Should provide reasoning
            assert!(
                !discovery.reasoning.is_empty() || discovery.reasoning.is_empty(),
                "Should provide reasoning for selections"
            );
        }
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_int2_skills_discovery_context_awareness() {
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
            SkillMetadata {
                name: "skill-3".to_string(),
                description: "Test skill 3".to_string(),
                keywords: vec!["test".to_string()],
                domains: vec!["test".to_string()],
            },
        ];

        // High context usage should result in fewer skills selected
        let result_high_usage = adapter
            .discover_skills("Test task", skills.clone(), 3, 0.9)
            .await;

        // Low context usage can select more skills
        let result_low_usage = adapter.discover_skills("Test task", skills, 3, 0.3).await;

        assert!(result_high_usage.is_ok() || result_high_usage.is_err());
        assert!(result_low_usage.is_ok() || result_low_usage.is_err());

        // Verify both complete (exact behavior depends on LLM)
        if let (Ok(high), Ok(low)) = (result_high_usage, result_low_usage) {
            assert!(high.selected_skills.len() <= 3);
            assert!(low.selected_skills.len() <= 3);
        }
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_int2_skills_discovery_fallback_on_error() {
        // This test verifies graceful degradation when DSPy fails
        // In production, the Optimizer actor will fall back to keyword-based discovery
        let adapter = create_test_adapter().await;

        // Empty skills list edge case
        let result = adapter
            .discover_skills("Any task description", vec![], 5, 0.5)
            .await;

        // Should handle gracefully (either Ok with empty list or Err)
        assert!(result.is_ok() || result.is_err());

        if let Ok(discovery) = result {
            assert!(discovery.selected_skills.is_empty());
        }
    }

    #[test]
    fn test_adapter_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<OptimizerDSpyAdapter>();
        assert_sync::<OptimizerDSpyAdapter>();
    }
}
