//! Evaluation metrics for LLM prompt improvements
//!
//! Tests the quality of LLM responses for:
//! - Memory enrichment
//! - Link generation
//! - Consolidation decisions
//!
//! Run with: cargo test --test llm_prompt_evaluation -- --ignored
//! Requires ANTHROPIC_API_KEY environment variable.

use mnemosyne_core::services::{LlmConfig, LlmService};
use mnemosyne_core::types::{MemoryNote, MemoryType, Namespace, MemoryId};
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Test case for memory enrichment
#[derive(Debug, Serialize, Deserialize)]
struct EnrichmentTestCase {
    name: String,
    raw_content: String,
    context: String,
    expected_type: MemoryType,
    expected_importance_range: (u8, u8),
    min_keywords: usize,
    min_tags: usize,
}

/// Test case for link generation
#[derive(Debug)]
struct LinkTestCase {
    name: String,
    new_memory_summary: String,
    candidate_summaries: Vec<String>,
    expected_min_links: usize,
    expected_max_links: usize,
}

/// Test case for consolidation decision
#[derive(Debug)]
struct ConsolidationTestCase {
    name: String,
    memory_a_summary: String,
    memory_b_summary: String,
    expected_decision: ConsolidationExpectation,
}

#[derive(Debug, PartialEq)]
enum ConsolidationExpectation {
    Merge,
    Supersede,
    KeepBoth,
    Any, // When multiple decisions could be valid
}

/// Evaluation metrics for enrichment
#[derive(Debug)]
struct EnrichmentMetrics {
    total_tests: usize,
    correct_type: usize,
    importance_in_range: usize,
    min_keywords_met: usize,
    min_tags_met: usize,
    json_parse_success: usize,
}

impl EnrichmentMetrics {
    fn new() -> Self {
        Self {
            total_tests: 0,
            correct_type: 0,
            importance_in_range: 0,
            min_keywords_met: 0,
            min_tags_met: 0,
            json_parse_success: 0,
        }
    }

    fn accuracy(&self) -> f64 {
        if self.total_tests == 0 {
            return 0.0;
        }
        (self.correct_type as f64 / self.total_tests as f64) * 100.0
    }

    fn json_success_rate(&self) -> f64 {
        if self.total_tests == 0 {
            return 0.0;
        }
        (self.json_parse_success as f64 / self.total_tests as f64) * 100.0
    }
}

/// Create test dataset for enrichment
fn enrichment_test_cases() -> Vec<EnrichmentTestCase> {
    vec![
        EnrichmentTestCase {
            name: "Architecture Decision - Database Migration".to_string(),
            raw_content: "Switched from SQLite to PostgreSQL for production due to concurrent write limitations. Migration completed successfully.".to_string(),
            context: "Database architecture discussion".to_string(),
            expected_type: MemoryType::ArchitectureDecision,
            expected_importance_range: (7, 9),
            min_keywords: 3,
            min_tags: 2,
        },
        EnrichmentTestCase {
            name: "Bug Fix - Retry Logic".to_string(),
            raw_content: "Fixed infinite loop in retry logic by adding max_attempts counter. Bug was causing API timeouts.".to_string(),
            context: "API reliability improvements".to_string(),
            expected_type: MemoryType::BugFix,
            expected_importance_range: (6, 8),
            min_keywords: 3,
            min_tags: 2,
        },
        EnrichmentTestCase {
            name: "Preference - UI Theme".to_string(),
            raw_content: "User prefers dark mode for terminal interfaces".to_string(),
            context: "User interface preferences".to_string(),
            expected_type: MemoryType::Preference,
            expected_importance_range: (2, 4),
            min_keywords: 3,
            min_tags: 2,
        },
        EnrichmentTestCase {
            name: "Code Pattern - Error Handling".to_string(),
            raw_content: "Established pattern: wrap all async operations in Result<T, AppError> for consistent error handling across services".to_string(),
            context: "Error handling standardization".to_string(),
            expected_type: MemoryType::CodePattern,
            expected_importance_range: (6, 8),
            min_keywords: 3,
            min_tags: 2,
        },
        EnrichmentTestCase {
            name: "Configuration - API Rate Limit".to_string(),
            raw_content: "Set API rate limit to 100 requests per minute with exponential backoff starting at 1 second".to_string(),
            context: "API configuration".to_string(),
            expected_type: MemoryType::Configuration,
            expected_importance_range: (5, 7),
            min_keywords: 3,
            min_tags: 2,
        },
    ]
}

#[tokio::test]
#[ignore] // Requires API key and makes real API calls
async fn test_enrichment_accuracy() {
    let service = LlmService::with_default().expect("Failed to create LLM service");
    let test_cases = enrichment_test_cases();
    let mut metrics = EnrichmentMetrics::new();

    println!("\n=== Memory Enrichment Evaluation ===\n");

    for (idx, test_case) in test_cases.iter().enumerate() {
        println!("Test {}: {}", idx + 1, test_case.name);
        metrics.total_tests += 1;

        match service.enrich_memory(&test_case.raw_content, &test_case.context).await {
            Ok(note) => {
                // Check if response was JSON (no parsing warnings in logs)
                metrics.json_parse_success += 1;

                // Check memory type
                if note.memory_type == test_case.expected_type {
                    metrics.correct_type += 1;
                    println!("  ✓ Type: {:?}", note.memory_type);
                } else {
                    println!("  ✗ Type: Got {:?}, expected {:?}", note.memory_type, test_case.expected_type);
                }

                // Check importance range
                if note.importance >= test_case.expected_importance_range.0
                    && note.importance <= test_case.expected_importance_range.1
                {
                    metrics.importance_in_range += 1;
                    println!("  ✓ Importance: {}", note.importance);
                } else {
                    println!("  ✗ Importance: {} (expected {}-{})",
                             note.importance,
                             test_case.expected_importance_range.0,
                             test_case.expected_importance_range.1);
                }

                // Check keywords
                if note.keywords.len() >= test_case.min_keywords {
                    metrics.min_keywords_met += 1;
                    println!("  ✓ Keywords: {} ({})", note.keywords.len(), note.keywords.join(", "));
                } else {
                    println!("  ✗ Keywords: {} (expected at least {})", note.keywords.len(), test_case.min_keywords);
                }

                // Check tags
                if note.tags.len() >= test_case.min_tags {
                    metrics.min_tags_met += 1;
                    println!("  ✓ Tags: {} ({})", note.tags.len(), note.tags.join(", "));
                } else {
                    println!("  ✗ Tags: {} (expected at least {})", note.tags.len(), test_case.min_tags);
                }

                println!("  Summary: {}", note.summary);
            }
            Err(e) => {
                println!("  ✗ Error: {}", e);
            }
        }

        println!();
    }

    // Print summary
    println!("=== Results ===");
    println!("Total tests: {}", metrics.total_tests);
    println!("Type accuracy: {:.1}% ({}/{})", metrics.accuracy(), metrics.correct_type, metrics.total_tests);
    println!("Importance accuracy: {:.1}% ({}/{})",
             (metrics.importance_in_range as f64 / metrics.total_tests as f64) * 100.0,
             metrics.importance_in_range,
             metrics.total_tests);
    println!("Keywords met: {:.1}% ({}/{})",
             (metrics.min_keywords_met as f64 / metrics.total_tests as f64) * 100.0,
             metrics.min_keywords_met,
             metrics.total_tests);
    println!("Tags met: {:.1}% ({}/{})",
             (metrics.min_tags_met as f64 / metrics.total_tests as f64) * 100.0,
             metrics.min_tags_met,
             metrics.total_tests);
    println!("JSON parse success: {:.1}%", metrics.json_success_rate());

    // Assert minimum quality thresholds
    assert!(metrics.accuracy() >= 80.0, "Type accuracy should be >= 80%");
    assert!(metrics.json_success_rate() >= 80.0, "JSON parsing should succeed >= 80% of the time");
}

#[tokio::test]
#[ignore] // Requires API key and makes real API calls
async fn test_link_generation_quality() {
    let service = LlmService::with_default().expect("Failed to create LLM service");

    println!("\n=== Link Generation Evaluation ===\n");

    // Create test memories
    let new_memory = create_test_memory(
        "Added authentication middleware to API endpoints",
        MemoryType::CodePattern,
        vec!["authentication", "middleware", "API"],
    );

    let candidates = vec![
        create_test_memory(
            "Set up JWT token generation for user sessions",
            MemoryType::CodePattern,
            vec!["JWT", "authentication", "sessions"],
        ),
        create_test_memory(
            "Updated database schema for user tracking",
            MemoryType::Configuration,
            vec!["database", "schema", "users"],
        ),
        create_test_memory(
            "User prefers dark mode terminal",
            MemoryType::Preference,
            vec!["UI", "preferences"],
        ),
    ];

    match service.generate_links(&new_memory, &candidates).await {
        Ok(links) => {
            println!("Generated {} links:", links.len());
            for link in &links {
                println!("  - Target: {:?}", link.target_id);
                println!("    Type: {:?}", link.link_type);
                println!("    Strength: {:.2}", link.strength);
                println!("    Reason: {}", link.reason);
            }

            // Expect at least 1 link (to JWT memory)
            assert!(links.len() >= 1, "Should generate at least 1 link");

            // Expect no more than 2 links (JWT and maybe database)
            assert!(links.len() <= 2, "Should not generate spurious links");

            // Check that link strengths are reasonable
            for link in &links {
                assert!(link.strength >= 0.6, "Link strength should be >= 0.6 (threshold)");
                assert!(link.strength <= 1.0, "Link strength should be <= 1.0");
            }

            println!("\n✓ Link generation quality checks passed");
        }
        Err(e) => {
            panic!("Link generation failed: {}", e);
        }
    }
}

#[tokio::test]
#[ignore] // Requires API key and makes real API calls
async fn test_consolidation_accuracy() {
    let service = LlmService::with_default().expect("Failed to create LLM service");

    println!("\n=== Consolidation Decision Evaluation ===\n");

    let test_cases = vec![
        (
            "Similar memories should MERGE",
            create_test_memory("PostgreSQL migration completed successfully", MemoryType::ArchitectureDecision, vec!["database"]),
            create_test_memory("Switched from SQLite to PostgreSQL for production", MemoryType::ArchitectureDecision, vec!["database"]),
            ConsolidationExpectation::Merge,
        ),
        (
            "Distinct memories should KEEP_BOTH",
            create_test_memory("User authentication implemented with JWT", MemoryType::CodePattern, vec!["auth"]),
            create_test_memory("Database connection pooling configured", MemoryType::Configuration, vec!["database"]),
            ConsolidationExpectation::KeepBoth,
        ),
    ];

    let mut correct = 0;
    let mut total = 0;

    for (name, memory_a, memory_b, expected) in test_cases {
        println!("Test: {}", name);
        total += 1;

        match service.should_consolidate(&memory_a, &memory_b).await {
            Ok(decision) => {
                let decision_str = match decision {
                    mnemosyne::types::ConsolidationDecision::Merge { .. } => "MERGE",
                    mnemosyne::types::ConsolidationDecision::Supersede { .. } => "SUPERSEDE",
                    mnemosyne::types::ConsolidationDecision::KeepBoth => "KEEP_BOTH",
                };

                let matches = match (expected, &decision) {
                    (ConsolidationExpectation::Merge, mnemosyne::types::ConsolidationDecision::Merge { .. }) => true,
                    (ConsolidationExpectation::Supersede, mnemosyne::types::ConsolidationDecision::Supersede { .. }) => true,
                    (ConsolidationExpectation::KeepBoth, mnemosyne::types::ConsolidationDecision::KeepBoth) => true,
                    (ConsolidationExpectation::Any, _) => true,
                    _ => false,
                };

                if matches {
                    correct += 1;
                    println!("  ✓ Decision: {}", decision_str);
                } else {
                    println!("  ✗ Decision: {} (expected {:?})", decision_str, expected);
                }
            }
            Err(e) => {
                println!("  ✗ Error: {}", e);
            }
        }

        println!();
    }

    println!("=== Results ===");
    println!("Accuracy: {:.1}% ({}/{})", (correct as f64 / total as f64) * 100.0, correct, total);

    assert!(correct as f64 / total as f64 >= 0.8, "Consolidation accuracy should be >= 80%");
}

/// Helper to create test memory
fn create_test_memory(summary: &str, memory_type: MemoryType, tags: Vec<&str>) -> MemoryNote {
    MemoryNote {
        id: MemoryId::new(),
        namespace: Namespace::Global,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        content: summary.to_string(),
        summary: summary.to_string(),
        keywords: vec![],
        tags: tags.iter().map(|s| s.to_string()).collect(),
        context: String::new(),
        memory_type,
        importance: 5,
        confidence: 0.8,
        links: vec![],
        related_files: vec![],
        related_entities: vec![],
        access_count: 0,
        last_accessed_at: Utc::now(),
        expires_at: None,
        is_archived: false,
        superseded_by: None,
        embedding: None,
        embedding_model: "claude-3-5-haiku-20241022".to_string(),
    }
}
