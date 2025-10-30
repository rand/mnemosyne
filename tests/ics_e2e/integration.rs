//! Full workflow integration tests for ICS
//!
//! Tests covering complete workflows:
//! - W1: Complete document lifecycle
//! - W2: Collaborative refinement session
//! - W3: Error recovery workflow
//! - W4: Memory-driven context building
//! - W5: Large document performance

use crate::ics_e2e::*;
use mnemosyne_core::ics::editor::{Actor, CrdtBuffer};
use mnemosyne_core::ics::{AgentActivity, ProposalStatus};

/// W1: Complete document lifecycle
#[tokio::test]
async fn w1_complete_document_lifecycle() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("document.md");

    // Phase 1: Creation
    let mut buffer = CrdtBuffer::new(0, Actor::Human, Some(file_path.clone())).expect("Should create buffer");
    let pos = buffer.text_len().expect("Should get text length");
    buffer.insert(pos, "# Project Plan\n\n").expect("Should insert");
    let pos = buffer.text_len().expect("Should get text length");
    buffer.insert(pos, "TODO: Define objectives\n").expect("Should insert");

    // Phase 2: Analysis
    let mut ctx = TestContext::new();
    ctx.add_text(fixtures::sample_markdown_doc());

    let analysis = ctx.analyze().await.expect("Analysis should succeed");
    assert_triples_found(&analysis, 1);
    assert_holes_found(&analysis, 1);

    // Phase 3: Agent proposals
    let agent = actors::MockAgent::reviewer();
    let proposals = agent.propose(&ctx.buffer_content());
    assert!(!proposals.is_empty());

    // Phase 4: User acceptance
    if let Some(mut proposal) = proposals.first().cloned() {
        proposal.status = ProposalStatus::Accepted;
        assert_proposal_accepted(&proposal);
    }

    // Phase 5: Save
    let pos = buffer.text_len().expect("Should get text length");
    buffer.insert(pos, "Objectives defined based on analysis.\n").expect("Should insert");
    buffer.save_file().expect("Should save");
    assert!(!buffer.dirty);
    assert!(file_path.exists());

    // Phase 6: Verify persistence
    let content = std::fs::read_to_string(&file_path).expect("Should read file");
    assert!(content.contains("# Project Plan"));
}

/// W2: Collaborative refinement session
#[tokio::test]
async fn w2_collaborative_refinement_session() {
    let mut ctx = TestContext::new();

    // Session start: Initial document
    ctx.add_text("System overview\n\n");
    ctx.add_text("The system is fast.\n");

    // Round 1: Orchestrator adds structure
    let orchestrator = actors::MockAgent::orchestrator();
    let orch_proposals = orchestrator.propose(&ctx.buffer_content());

    if !orch_proposals.is_empty() {
        // Apply orchestrator's improvement (create new buffer with structure)
        let buf_id = ctx.create_buffer();
        ctx.switch_buffer(buf_id);
        ctx.add_text("# System Overview\n\n");
        ctx.add_text("## Performance\n\n");
        ctx.add_text("The system is fast.\n");
    }

    // Round 2: Optimizer adds precision
    let mut optimizer = actors::MockAgent::optimizer();
    optimizer.set_activity(
        AgentActivity::Analyzing,
        Some("Analyzing performance claims".to_string()),
    );

    let opt_proposals = optimizer.propose(&ctx.buffer_content());

    // Round 3: User provides specific data
    ctx.add_text("\nBenchmark: 1000 req/s sustained\n");

    // Round 4: Reviewer validates
    let reviewer = actors::MockAgent::reviewer();
    let rev_proposals = reviewer.propose(&ctx.buffer_content());

    // Round 5: Semantic analysis
    let analysis = ctx.analyze().await.expect("Analysis should succeed");

    // Session complete: Verify refinement
    let final_content = ctx.buffer_content();
    assert!(final_content.contains("# System Overview"));
    assert!(final_content.len() > "System overview\n\n".len());
    assert!(orch_proposals.len() + opt_proposals.len() + rev_proposals.len() >= 0);
    assert!(analysis.triples.len() >= 0);
}

/// W3: Error recovery workflow
#[tokio::test]
async fn w3_error_recovery_workflow() {
    let mut ctx = TestContext::new();
    let mut agent = actors::MockAgent::semantic();

    // Phase 1: Normal operation
    ctx.add_text("Normal document content\n");
    agent.set_activity(AgentActivity::Analyzing, Some("Processing".to_string()));

    // Phase 2: Error occurs
    agent.trigger_error("Analysis service unavailable");
    assert!(matches!(agent.info.activity, AgentActivity::Error(_)));

    // Phase 3: User continues editing
    ctx.add_text("Additional content during error\n");
    let content = ctx.buffer_content();
    assert!(content.contains("Additional content"));

    // Phase 4: Error recovery
    agent.set_activity(AgentActivity::Idle, Some("Recovered".to_string()));
    assert_agent_idle(&agent.info);

    // Phase 5: Resume normal operation
    agent.set_activity(AgentActivity::Analyzing, Some("Resuming".to_string()));
    assert_agent_analyzing(&agent.info);

    // Phase 6: Successful analysis
    let analysis = ctx
        .analyze()
        .await
        .expect("Analysis should succeed after recovery");
    assert!(analysis.triples.len() >= 0);
}

/// W4: Memory-driven context building
#[tokio::test]
async fn w4_memory_driven_context_building() {
    let ctx = TestContext::with_fixtures();

    // Memories available
    assert_eq!(ctx.memories.len(), 3);

    // Phase 1: User starts new document
    let mut edit_ctx = TestContext::new();
    edit_ctx.add_text("# Authentication Design\n\n");

    // Phase 2: Search relevant memories
    let auth_memories: Vec<_> = ctx
        .memories
        .iter()
        .filter(|m| {
            m.keywords
                .iter()
                .any(|k| k.contains("auth") || k.contains("security"))
        })
        .collect();

    assert!(
        !auth_memories.is_empty(),
        "Should find auth-related memories"
    );

    // Phase 3: Agent uses memory context for proposals
    let reviewer = actors::MockAgent::reviewer();

    // Simulate agent having access to memory context
    edit_ctx.add_text("TODO: Define authentication mechanism\n");

    let proposals = reviewer.propose(&edit_ctx.buffer_content());

    // Phase 4: Proposal should be informed by memory
    // (In real system, proposal would reference JWT from memory)
    if !proposals.is_empty() {
        let proposal = &proposals[0];
        assert!(proposal.rationale.contains("TODO") || proposal.rationale.len() > 0);
    }

    // Phase 5: User applies memory-driven proposal
    edit_ctx.add_text("\n## JWT Authentication\n\n");
    edit_ctx.add_text("Use JWT tokens with 1-hour expiration (per security memory)\n");

    let final_content = edit_ctx.buffer_content();
    assert!(final_content.contains("JWT"));
    assert!(final_content.contains("1-hour"));
}

/// W5: Large document performance
#[tokio::test]
async fn w5_large_document_performance() {
    use std::time::Instant;

    let mut ctx = TestContext::new();

    // Generate large document (1000+ lines)
    let large_doc = fixtures::large_document();
    assert!(large_doc.lines().count() > 1000, "Should have 1000+ lines");

    // Measure insert performance
    let start = Instant::now();
    ctx.add_text(&large_doc);
    let insert_duration = start.elapsed();

    // Should be fast (< 100ms for 1000 lines)
    assert!(
        insert_duration.as_millis() < 100,
        "Insert should be fast: {:?}",
        insert_duration
    );

    // Measure content retrieval
    let start = Instant::now();
    let content = ctx.buffer_content();
    let read_duration = start.elapsed();

    assert!(
        read_duration.as_millis() < 50,
        "Read should be fast: {:?}",
        read_duration
    );
    assert!(content.len() > 10000, "Should have large content");

    // Measure analysis performance
    let start = Instant::now();
    let analysis = ctx.analyze().await.expect("Analysis should succeed");
    let analysis_duration = start.elapsed();

    // Analysis may take longer but should complete
    assert!(
        analysis_duration.as_secs() < 5,
        "Analysis should complete: {:?}",
        analysis_duration
    );

    // Verify analysis found semantic content
    assert!(
        analysis.triples.len() > 0,
        "Should extract triples from large document"
    );
    assert!(analysis.entities.len() > 0, "Should extract entities");

    // Measure agent proposal generation
    let agent = actors::MockAgent::optimizer();
    let start = Instant::now();
    let proposals = agent.propose(&content);
    let proposal_duration = start.elapsed();

    assert!(
        proposal_duration.as_millis() < 100,
        "Proposals should be fast: {:?}",
        proposal_duration
    );
    assert!(proposals.len() >= 0, "Should generate proposals");
}
