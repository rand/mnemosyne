//! Full Workflow Integration Tests (W1-W8)
//!
//! End-to-end workflows combining multiple subsystems

use crate::ics_full_integration::*;
use mnemosyne_core::{
    ics::{ChangeProposal, ProposalStatus, SemanticAnalyzer},
    storage::StorageBackend,
    types::{LinkType, MemoryLink, MemoryType, Namespace},
};
use std::time::Duration;

/// W1: Complete ICS session workflow
#[tokio::test]
async fn w1_complete_ics_session() {
    let storage = StorageFixture::new().await.expect("Storage setup failed");
    let mut ics = IcsFixture::new();

    // Phase 1: User creates content in ICS
    ics.add_text("# System Architecture\n\n");
    ics.add_text("The authentication system uses JWT tokens with Redis session storage.\n");
    ics.add_text("Token expiration is set to 1 hour for security.\n");

    // Phase 2: Semantic analysis
    let analysis = ics.analyze().await.expect("Analysis should succeed");
    assert_min_triples(&analysis, 1);

    // Phase 3: Create memory from content
    let memory = create_test_memory(
        &ics.buffer_content(),
        MemoryType::ArchitectureDecision,
        Namespace::Global,
        8,
    );

    // Phase 4: Store memory
    storage
        .storage()
        .store_memory(&memory)
        .await
        .expect("Store memory");

    // Phase 5: Verify retrieval
    let retrieved = storage
        .storage()
        .get_memory(memory.id.clone())
        .await
        .expect("Retrieve memory");

    assert!(retrieved.content.contains("JWT"));
    assert_eq!(retrieved.importance, 8);

    // Phase 6: Search for memory
    let search_results = storage
        .storage()
        .keyword_search("authentication", Some(Namespace::Global))
        .await
        .expect("Search");

    assert!(
        search_results.iter().any(|r| r.memory.id == memory.id),
        "Should find stored memory via search"
    );
}

/// W2: Multi-agent collaboration workflow
#[tokio::test]
async fn w2_multi_agent_collaboration() {
    let storage = StorageFixture::new().await.expect("Storage setup failed");

    // Agent 1: Architecture agent creates decision
    let arch_memory = create_test_memory(
        "System uses microservices architecture with event-driven communication",
        MemoryType::ArchitectureDecision,
        Namespace::Global,
        9,
    );

    storage
        .storage()
        .store_memory(&arch_memory)
        .await
        .expect("Store arch");

    // Agent 2: Implementation agent creates pattern
    let impl_memory = create_test_memory(
        "Event bus implemented using RabbitMQ with message acknowledgment",
        MemoryType::CodePattern,
        Namespace::Global,
        8,
    );

    storage
        .storage()
        .store_memory(&impl_memory)
        .await
        .expect("Store impl");

    // Agent 3: QA agent creates test strategy
    let test_memory = create_test_memory(
        "Integration tests verify event delivery and message ordering",
        MemoryType::CodePattern,
        Namespace::Global,
        7,
    );

    storage
        .storage()
        .store_memory(&test_memory)
        .await
        .expect("Store test");

    // Create links between memories (semantic relationships)
    let mut arch_with_links = arch_memory.clone();
    arch_with_links.links.push(MemoryLink {
        target_id: impl_memory.id.clone(),
        link_type: LinkType::Implements,
        strength: 0.9,
        reason: "Implementation of architectural pattern".to_string(),
        created_at: chrono::Utc::now(),
    });

    storage
        .storage()
        .update_memory(&arch_with_links)
        .await
        .expect("Update with link");

    // Verify collaboration: retrieve and check links
    let retrieved = storage
        .storage()
        .get_memory(arch_memory.id.clone())
        .await
        .expect("Get arch memory");

    assert_eq!(retrieved.links.len(), 1);
    assert_eq!(retrieved.links[0].link_type, LinkType::Implements);
}

/// W3: Memory lifecycle workflow
#[tokio::test]
async fn w3_memory_lifecycle() {
    let storage = StorageFixture::new().await.expect("Storage setup failed");

    // Phase 1: Create memory
    let mut memory = create_test_memory(
        "Initial implementation note",
        MemoryType::CodePattern,
        Namespace::Global,
        5,
    );

    storage
        .storage()
        .store_memory(&memory)
        .await
        .expect("Create");

    // Phase 2: Access memory multiple times
    for _ in 0..10 {
        storage
            .storage()
            .increment_access(memory.id.clone())
            .await
            .expect("Increment access");
    }

    // Phase 3: Evolution - increase importance based on access
    let accessed = storage
        .storage()
        .get_memory(memory.id.clone())
        .await
        .expect("Get");

    memory.importance = 8; // Increased from 5
    memory.access_count = accessed.access_count;

    storage
        .storage()
        .update_memory(&memory)
        .await
        .expect("Update importance");

    // Phase 4: Supersede with better version
    let improved_memory = create_test_memory(
        "Improved implementation with error handling",
        MemoryType::CodePattern,
        Namespace::Global,
        9,
    );

    storage
        .storage()
        .store_memory(&improved_memory)
        .await
        .expect("Store improved");

    // Mark original as superseded
    memory.superseded_by = Some(improved_memory.id.clone());
    storage
        .storage()
        .update_memory(&memory)
        .await
        .expect("Mark superseded");

    // Phase 5: Eventually archive old version
    storage
        .storage()
        .archive_memory(memory.id.clone())
        .await
        .expect("Archive");

    let archived = storage
        .storage()
        .get_memory(memory.id.clone())
        .await
        .expect("Get archived");

    assert_archived(&archived);
    assert_eq!(archived.superseded_by, Some(improved_memory.id));
}

/// W4: Search and retrieval workflow (keyword → vector → hybrid)
#[tokio::test]
async fn w4_search_retrieval_workflow() {
    let storage = StorageFixture::new().await.expect("Storage setup failed");

    // Create diverse memories
    let memories = vec![
        (
            "JWT authentication with bcrypt password hashing",
            vec!["jwt", "auth", "security"],
        ),
        (
            "OAuth2 authorization flow implementation",
            vec!["oauth", "auth", "security"],
        ),
        (
            "Database connection pooling with PostgreSQL",
            vec!["database", "postgres", "pooling"],
        ),
        (
            "Redis cache for session management",
            vec!["redis", "cache", "session"],
        ),
        (
            "React component state management patterns",
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

    // Phase 1: Keyword search for authentication
    let keyword_results = storage
        .storage()
        .keyword_search("auth", Some(Namespace::Global))
        .await
        .expect("Keyword search");

    assert!(
        keyword_results.len() >= 2,
        "Should find auth-related memories"
    );

    // Phase 2: Keyword search for database
    let db_results = storage
        .storage()
        .keyword_search("database", Some(Namespace::Global))
        .await
        .expect("Database search");

    assert!(db_results.len() >= 1, "Should find database memory");

    // Phase 3: Empty search (returns all, limited to 20)
    let all_results = storage
        .storage()
        .keyword_search("", Some(Namespace::Global))
        .await
        .expect("All search");

    assert!(all_results.len() >= 5, "Should return multiple memories");

    // Verify results are ranked by importance
    assert_sorted_by_importance(
        &all_results
            .iter()
            .map(|r| r.memory.clone())
            .collect::<Vec<_>>(),
    );
}

/// W5: Proposal generation and acceptance workflow
#[tokio::test]
async fn w5_proposal_workflow() {
    let storage = StorageFixture::new().await.expect("Storage setup failed");

    // Phase 1: Create initial memory
    let mut memory = create_test_memory(
        "The system needs better error handling",
        MemoryType::Constraint,
        Namespace::Global,
        6,
    );

    storage
        .storage()
        .store_memory(&memory)
        .await
        .expect("Store");

    // Phase 2: Agent generates proposal
    let proposal = ChangeProposal {
        id: "proposal-1".to_string(),
        agent: "agent:optimizer".to_string(),
        description: "Add specific error handling strategy".to_string(),
        original: "The system needs better error handling".to_string(),
        proposed: "The system implements error handling with custom error types, \
                   structured logging, and graceful degradation patterns"
            .to_string(),
        line_range: (1, 1),
        created_at: std::time::SystemTime::now(),
        status: ProposalStatus::Pending,
        rationale: "Vague constraint should specify concrete implementation".to_string(),
    };

    // Phase 3: User reviews and accepts proposal
    let mut accepted_proposal = proposal.clone();
    accepted_proposal.status = ProposalStatus::Accepted;

    assert_proposal_accepted(&accepted_proposal);

    // Phase 4: Update memory with accepted changes
    memory.content = accepted_proposal.proposed.clone();
    memory.importance = 8; // Increased due to detail

    storage
        .storage()
        .update_memory(&memory)
        .await
        .expect("Update with proposal");

    // Phase 5: Verify changes persisted
    let updated = storage
        .storage()
        .get_memory(memory.id.clone())
        .await
        .expect("Get updated");

    assert!(updated.content.contains("custom error types"));
    assert!(updated.content.contains("structured logging"));
    assert_eq!(updated.importance, 8);
}

/// W6: Cross-session memory continuity workflow
#[tokio::test]
async fn w6_cross_session_continuity() {
    let storage = StorageFixture::new().await.expect("Storage setup failed");

    // === Session 1 ===
    let session1_ns = Namespace::Session {
        project: "myapp".to_string(),
        session_id: "session-2024-01-01".to_string(),
    };

    // Create memories in session 1
    for i in 0..5 {
        let memory = create_test_memory(
            &format!("Session 1 decision {}: Database choice", i + 1),
            MemoryType::ArchitectureDecision,
            session1_ns.clone(),
            8,
        );

        storage
            .storage()
            .store_memory(&memory)
            .await
            .expect("Store s1");
    }

    // Promote one memory to project level
    let important_memory = create_test_memory(
        "Chosen PostgreSQL for ACID guarantees and JSON support",
        MemoryType::ArchitectureDecision,
        Namespace::Project {
            name: "myapp".to_string(),
        },
        9,
    );

    storage
        .storage()
        .store_memory(&important_memory)
        .await
        .expect("Store project memory");

    // === Session 2 (days later) ===
    let session2_ns = Namespace::Session {
        project: "myapp".to_string(),
        session_id: "session-2024-01-05".to_string(),
    };

    // Retrieve project-level memories (continuity)
    let project_results = storage
        .storage()
        .keyword_search(
            "",
            Some(Namespace::Project {
                name: "myapp".to_string(),
            }),
        )
        .await
        .expect("Project search");

    assert!(
        project_results.len() >= 1,
        "Should find project-level memories from previous session"
    );

    // Session 1 memories not visible in session 2
    let session2_results = storage
        .storage()
        .keyword_search("", Some(session2_ns.clone()))
        .await
        .expect("Session 2 search");

    // Session 2 is new, so should have no memories yet
    assert_eq!(session2_results.len(), 0, "New session should start empty");

    // Create memory in session 2
    let session2_memory = create_test_memory(
        "Session 2: Implementing PostgreSQL connection pool",
        MemoryType::CodePattern,
        session2_ns.clone(),
        7,
    );

    storage
        .storage()
        .store_memory(&session2_memory)
        .await
        .expect("Store s2");

    // Verify namespace isolation maintained
    let s2_check = storage
        .storage()
        .keyword_search("", Some(session2_ns))
        .await
        .expect("S2 check");

    assert_eq!(s2_check.len(), 1, "Session 2 should have 1 memory now");
}

/// W7: Large-scale data processing workflow
#[tokio::test]
async fn w7_large_scale_processing() {
    use std::time::Instant;

    let storage = StorageFixture::new().await.expect("Storage setup failed");

    // Phase 1: Batch insert 500 memories
    let start = Instant::now();
    for i in 0..500 {
        let memory_type = match i % 7 {
            0 => MemoryType::ArchitectureDecision,
            1 => MemoryType::CodePattern,
            2 => MemoryType::BugFix,
            3 => MemoryType::Configuration,
            4 => MemoryType::Constraint,
            5 => MemoryType::Insight,
            _ => MemoryType::Entity,
        };

        let memory = create_test_memory(
            &format!("Batch memory {} with content", i + 1),
            memory_type,
            Namespace::Global,
            ((i % 10) + 1) as u8, // 1-10 instead of 0-9
        );

        storage
            .storage()
            .store_memory(&memory)
            .await
            .expect("Batch store");
    }
    let insert_duration = start.elapsed();
    println!("Inserted 500 memories in {:?}", insert_duration);

    // Phase 2: Bulk search
    let search_start = Instant::now();
    let results = storage
        .storage()
        .keyword_search("", Some(Namespace::Global))
        .await
        .expect("Bulk search");
    let search_duration = search_start.elapsed();

    println!("Searched 500 memories in {:?}", search_duration);

    // Phase 3: Verify performance
    assert!(
        search_duration.as_millis() < 500,
        "Bulk search should be fast"
    );

    // Returns top 20 by importance
    assert_memory_count(
        &results.iter().map(|r| r.memory.clone()).collect::<Vec<_>>(),
        20,
    );

    // Phase 4: Update batch
    let update_start = Instant::now();
    for result in results.iter().take(10) {
        let mut updated = result.memory.clone();
        // Cap importance at 10 (CHECK constraint: importance BETWEEN 1 AND 10)
        updated.importance = (updated.importance.saturating_add(1)).min(10);

        storage
            .storage()
            .update_memory(&updated)
            .await
            .expect("Bulk update");
    }
    let update_duration = update_start.elapsed();

    println!("Updated 10 memories in {:?}", update_duration);

    assert!(update_duration.as_millis() < 200, "Updates should be fast");
}

/// W8: Error recovery and resilience workflow
#[tokio::test]
async fn w8_error_recovery_resilience() {
    let storage = StorageFixture::new().await.expect("Storage setup failed");

    // Phase 1: Normal operation
    let memory = create_test_memory(
        "Normal memory creation",
        MemoryType::CodePattern,
        Namespace::Global,
        7,
    );

    storage
        .storage()
        .store_memory(&memory)
        .await
        .expect("Normal store");

    // Phase 2: Attempt to retrieve non-existent memory (error case)
    use mnemosyne_core::types::MemoryId;
    let fake_id = MemoryId::new();

    let result = storage.storage().get_memory(fake_id.clone()).await;
    assert!(
        result.is_err(),
        "Should error when retrieving non-existent memory"
    );

    // Phase 3: Continue normal operations after error
    let memory2 = create_test_memory(
        "Recovery after error",
        MemoryType::Insight,
        Namespace::Global,
        8,
    );

    storage
        .storage()
        .store_memory(&memory2)
        .await
        .expect("Store after error");

    // Phase 4: Verify system still functional
    let search = storage
        .storage()
        .keyword_search("", Some(Namespace::Global))
        .await
        .expect("Search after error");

    assert!(search.len() >= 2, "System should continue functioning");

    // Phase 5: Test invalid update (empty content)
    let mut invalid = memory.clone();
    invalid.content = "".to_string();

    let update_result = storage.storage().update_memory(&invalid).await;

    // System should either accept empty content or reject gracefully
    match update_result {
        Ok(_) => {
            // Empty content accepted
            let check = storage
                .storage()
                .get_memory(memory.id.clone())
                .await
                .expect("Get after empty update");
            // Content is either empty or unchanged
            assert!(
                check.content.is_empty() || check.content == "Normal memory creation",
                "Content should be empty or unchanged"
            );
        }
        Err(_) => {
            // Update rejected, verify original unchanged
            let check = storage
                .storage()
                .get_memory(memory.id.clone())
                .await
                .expect("Get after rejected update");
            assert_eq!(
                check.content, "Normal memory creation",
                "Original should be unchanged"
            );
        }
    }

    // Phase 6: Final verification - system still operational
    let final_search = storage
        .storage()
        .keyword_search("", Some(Namespace::Global))
        .await
        .expect("Final search");

    assert!(final_search.len() >= 2, "System remains operational");
}
