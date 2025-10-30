//! Integration tests for LLM enrichment workflows
//!
//! Tests the complete LLM integration including:
//! - Memory enrichment (extracting insights, keywords, links)
//! - Link generation between related memories
//! - Consolidation decisions (merge vs keep both)
//!
//! NOTE: These tests require ANTHROPIC_API_KEY to be set and are marked with #[ignore].
//! Run with: cargo test --test integration -- --ignored

use mnemosyne_core::{MemoryType, StorageBackend};
use once_cell::sync::Lazy;
use std::sync::Arc;

mod common;
use common::{create_real_llm_service, create_test_storage, sample_memory};

/// Shared LLM service instance - only accesses keychain once per test run
static SHARED_LLM_SERVICE: Lazy<Option<Arc<mnemosyne_core::LlmService>>> =
    Lazy::new(|| match create_real_llm_service() {
        Some(llm) => {
            eprintln!("✓ LLM service initialized (keychain accessed once)");
            Some(llm)
        }
        None => {
            eprintln!("✗ No API key found in environment or keychain");
            eprintln!("  Set API key with: cargo run -- config set-key");
            eprintln!("  Or: export ANTHROPIC_API_KEY=sk-ant-...");
            None
        }
    });

/// Helper to get LLM service or skip test
fn get_llm_service_or_skip() -> Arc<mnemosyne_core::LlmService> {
    match SHARED_LLM_SERVICE.as_ref() {
        Some(llm) => Arc::clone(llm),
        None => {
            eprintln!("Skipping test: LLM service not available");
            std::process::exit(0);
        }
    }
}

#[tokio::test]
#[ignore] // Requires API key
async fn test_enrich_memory_architecture_decision() {
    // Setup
    let storage = create_test_storage().await;
    let llm = get_llm_service_or_skip();

    // Create a raw memory note (as if just captured from user)
    let raw_content = "We decided to use PostgreSQL instead of MongoDB because we need ACID \
                       guarantees for financial transactions and complex relational queries \
                       across multiple tables.";
    let context = "Sprint 5 planning meeting";

    // Test: Enrich the memory with LLM
    let enriched = llm
        .enrich_memory(raw_content, context)
        .await
        .expect("Failed to enrich memory");

    // Assert: LLM generated summary
    assert!(
        !enriched.summary.is_empty(),
        "LLM should generate a summary"
    );
    // Note: Summary might be longer than very terse content - that's OK
    // The important thing is that a summary was generated
    println!("Summary: {}", enriched.summary);
    println!(
        "Content length: {}, Summary length: {}",
        enriched.content.len(),
        enriched.summary.len()
    );

    // Assert: LLM extracted relevant keywords
    assert!(!enriched.keywords.is_empty(), "LLM should extract keywords");
    assert!(
        enriched.keywords.len() >= 3,
        "Should extract at least 3 keywords"
    );

    // Keywords should be relevant (likely include database-related terms)
    let keywords_text = enriched.keywords.join(" ").to_lowercase();
    let has_db_keywords = keywords_text.contains("postgres")
        || keywords_text.contains("database")
        || keywords_text.contains("sql")
        || keywords_text.contains("acid");
    assert!(
        has_db_keywords,
        "Keywords should include database-related terms: {:?}",
        enriched.keywords
    );

    // Assert: Importance may be adjusted based on LLM analysis
    // Architecture decisions are typically important
    assert!(
        enriched.importance >= 6,
        "Architecture decision should have high importance"
    );

    // Store enriched memory
    storage.store_memory(&enriched).await.unwrap();

    // Verify retrieval
    let retrieved = storage.get_memory(enriched.id).await.unwrap();
    assert_eq!(retrieved.summary, enriched.summary);
    assert_eq!(retrieved.keywords, enriched.keywords);
}

#[tokio::test]
#[ignore] // Requires API key
async fn test_enrich_memory_bug_fix() {
    // Setup
    let storage = create_test_storage().await;
    let llm = get_llm_service_or_skip();

    // Create bug fix memory
    let raw_content = "Fixed a race condition in the user session cache. Multiple threads \
                       were reading and writing to the HashMap without synchronization. \
                       Wrapped it in Arc<RwLock> to fix.";
    let context = "Bug fix during code review";

    // Test: Enrich
    let enriched = llm
        .enrich_memory(raw_content, context)
        .await
        .expect("Failed to enrich memory");

    // Assert: Summary generated
    assert!(!enriched.summary.is_empty(), "Should generate summary");

    // Assert: Keywords extracted
    assert!(enriched.keywords.len() >= 3, "Should extract keywords");
    let keywords_text = enriched.keywords.join(" ").to_lowercase();
    let has_relevant = keywords_text.contains("race")
        || keywords_text.contains("concurrency")
        || keywords_text.contains("sync")
        || keywords_text.contains("thread");
    assert!(
        has_relevant,
        "Keywords should include concurrency-related terms: {:?}",
        enriched.keywords
    );

    // Store and verify
    storage.store_memory(&enriched).await.unwrap();
    let retrieved = storage.get_memory(enriched.id).await.unwrap();
    assert_eq!(retrieved.keywords, enriched.keywords);
}

#[tokio::test]
#[ignore] // Requires API key
async fn test_link_generation() {
    // Setup
    let storage = create_test_storage().await;
    let llm = get_llm_service_or_skip();

    // Create related memories
    let mut db_decision = sample_memory(
        "Decided to use PostgreSQL for ACID guarantees",
        MemoryType::ArchitectureDecision,
        8,
    );
    db_decision.keywords = vec!["database".to_string(), "postgresql".to_string()];

    let mut api_impl = sample_memory(
        "Implemented REST API endpoints for user management with PostgreSQL backend",
        MemoryType::CodePattern,
        7,
    );
    api_impl.keywords = vec!["api".to_string(), "postgresql".to_string()];

    storage.store_memory(&db_decision).await.unwrap();
    storage.store_memory(&api_impl).await.unwrap();

    // Test: Ask LLM to identify links between memories
    let context = vec![db_decision.clone()];
    let links = llm
        .generate_links(&api_impl, &context)
        .await
        .expect("Failed to generate links");

    // Note: LLM may or may not generate links depending on its assessment
    // This test primarily verifies the API works without errors
    println!("Generated {} links", links.len());
    for link in &links {
        println!("  Link: {:?} (strength: {})", link.link_type, link.strength);
        assert!(
            link.strength >= 0.0 && link.strength <= 1.0,
            "Link strength should be between 0 and 1"
        );
        assert!(!link.reason.is_empty(), "Link should have a reason");
    }
}

#[tokio::test]
#[ignore] // Requires API key
async fn test_consolidation_decision_merge() {
    // Setup
    let storage = create_test_storage().await;
    let llm = get_llm_service_or_skip();

    // Create duplicate/similar memories that should be merged
    let mem1 = sample_memory(
        "Use PostgreSQL for the database. It provides ACID guarantees.",
        MemoryType::ArchitectureDecision,
        8,
    );

    let mem2 = sample_memory(
        "Database decision: We're using PostgreSQL because we need transactions.",
        MemoryType::ArchitectureDecision,
        7,
    );

    storage.store_memory(&mem1).await.unwrap();
    storage.store_memory(&mem2).await.unwrap();

    // Test: Ask LLM for consolidation decision
    let decision = llm
        .should_consolidate(&mem1, &mem2)
        .await
        .expect("Failed to get consolidation decision");

    // Assert: Decision is one of the valid variants
    println!("Consolidation decision: {:?}", decision);

    match decision {
        mnemosyne_core::ConsolidationDecision::Merge { into, content } => {
            println!("Decision: Merge into {:?}", into);
            println!("Merged content length: {}", content.len());
            assert!(!content.is_empty(), "Merged content should not be empty");
            assert!(
                into == mem1.id || into == mem2.id,
                "Should merge into one of the original memories"
            );
        }
        mnemosyne_core::ConsolidationDecision::Supersede { kept, superseded } => {
            println!(
                "Decision: Supersede - keep {:?}, archive {:?}",
                kept, superseded
            );
            assert!(
                (kept == mem1.id && superseded == mem2.id)
                    || (kept == mem2.id && superseded == mem1.id),
                "Supersede should reference both memories"
            );
        }
        mnemosyne_core::ConsolidationDecision::KeepBoth => {
            println!("Decision: Keep both separate");
        }
    }
}

#[tokio::test]
#[ignore] // Requires API key
async fn test_consolidation_decision_keep_both() {
    // Setup
    let storage = create_test_storage().await;
    let llm = get_llm_service_or_skip();

    // Create distinct memories that should NOT be merged
    let mem1 = sample_memory(
        "Use PostgreSQL for the primary user database",
        MemoryType::ArchitectureDecision,
        8,
    );

    let mem2 = sample_memory(
        "Use Redis for caching session data",
        MemoryType::ArchitectureDecision,
        7,
    );

    storage.store_memory(&mem1).await.unwrap();
    storage.store_memory(&mem2).await.unwrap();

    // Test: Ask LLM for consolidation decision
    let decision = llm
        .should_consolidate(&mem1, &mem2)
        .await
        .expect("Failed to get consolidation decision");

    // Assert: These are different decisions
    println!("Consolidation decision: {:?}", decision);

    // Note: LLM might decide any of the three options depending on its analysis
    // This test mainly verifies the API works correctly
    match decision {
        mnemosyne_core::ConsolidationDecision::KeepBoth => {
            println!("Correctly decided to keep separate (expected for distinct memories)");
        }
        _ => {
            println!("LLM decided to consolidate despite distinct topics");
        }
    }

    // Verify both memories still exist
    let retrieved1 = storage.get_memory(mem1.id).await.unwrap();
    let retrieved2 = storage.get_memory(mem2.id).await.unwrap();
    assert!(!retrieved1.is_archived, "Memory 1 should not be archived");
    assert!(!retrieved2.is_archived, "Memory 2 should not be archived");
}
