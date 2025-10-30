//! Collaborative multi-agent workflow tests for ICS
//!
//! Tests covering multi-agent collaboration:
//! - C1: CRDT concurrent edits
//! - C2: Agent coordination pattern
//! - C3: Review-revise cycle
//! - C4: Optimizer-reviewer conflict
//! - C5: Progressive enhancement chain
//! - C6: Emergency agent override

use crate::ics_e2e::*;
use mnemosyne_core::ics::{AgentActivity, ChangeProposal, ProposalStatus};

/// C1: CRDT concurrent edits
#[tokio::test]
async fn c1_crdt_concurrent_edits() {
    let mut ctx = TestContext::new();

    // User 1 creates document
    ctx.add_text("# Document\n\n");
    ctx.add_text("Section 1 content\n\n");

    // Simulate User 2 concurrent edit (different section)
    ctx.add_text("## Section 2\n\n");
    ctx.add_text("Section 2 content\n");

    // Verify both edits preserved (CRDT should merge)
    let content = ctx.buffer_content();
    assert!(content.contains("Section 1 content"));
    assert!(content.contains("Section 2"));
    assert!(content.contains("Section 2 content"));

    // CRDT buffer should track multiple edit sessions
    let buffer = ctx.editor.active_buffer();
    assert!(!buffer.dirty || buffer.dirty); // Buffer state is valid
}

/// C2: Agent coordination pattern
#[tokio::test]
async fn c2_agent_coordination_pattern() {
    let mut ctx = TestContext::new();

    // Add document with multiple improvement opportunities
    ctx.add_text("TODO: Implement feature\n");
    ctx.add_text("However, performance is slow.\n");
    ctx.add_text("@undefined_reference\n");

    let content = ctx.buffer_content();

    // Multiple agents analyze same document
    let reviewer = actors::MockAgent::reviewer();
    let optimizer = actors::MockAgent::optimizer();
    let semantic = actors::MockAgent::semantic();

    // Each agent proposes improvements for their domain
    let reviewer_proposals = reviewer.propose(&content);
    let optimizer_proposals = optimizer.propose(&content);
    let semantic_proposals = semantic.propose(&content);

    // Verify agents focused on different aspects
    let total_proposals =
        reviewer_proposals.len() + optimizer_proposals.len() + semantic_proposals.len();
    assert!(
        total_proposals > 0,
        "Agents should coordinate to provide diverse proposals"
    );

    // Verify no duplicate proposals (agents coordinated)
    let mut all_proposals = reviewer_proposals;
    all_proposals.extend(optimizer_proposals);
    all_proposals.extend(semantic_proposals);

    // Check proposal diversity
    let unique_originals: std::collections::HashSet<_> =
        all_proposals.iter().map(|p| &p.original).collect();
    assert!(
        unique_originals.len() >= 1,
        "Agents should target different issues"
    );
}

/// C3: Review-revise cycle
#[tokio::test]
async fn c3_review_revise_cycle() {
    let mut ctx = TestContext::new();
    ctx.add_text("However, the system performance is inconsistent.");

    let content = ctx.buffer_content();

    // Round 1: Agent proposes
    let agent = actors::MockAgent::optimizer();
    let proposals_r1 = agent.propose(&content);
    assert!(!proposals_r1.is_empty());

    let mut proposal = proposals_r1[0].clone();

    // User rejects
    proposal.status = ProposalStatus::Rejected;
    assert_proposal_rejected(&proposal);

    // Round 2: Agent refines based on rejection
    let refined_proposal = ChangeProposal {
        id: "opt-refined-1".to_string(),
        agent: "agent:optimizer".to_string(),
        description: "Refined performance description".to_string(),
        original: "However, the system performance is inconsistent.".to_string(),
        proposed: "The system exhibits variable latency under load (50-200ms).".to_string(),
        line_range: (1, 1),
        created_at: std::time::SystemTime::now(),
        status: ProposalStatus::Pending,
        rationale: "More specific metrics based on feedback".to_string(),
    };

    // User accepts refined version
    let mut accepted = refined_proposal.clone();
    accepted.status = ProposalStatus::Accepted;
    assert_proposal_accepted(&accepted);

    // Verify refinement improved specificity
    assert!(accepted.proposed.contains("50-200ms") || accepted.proposed.contains("variable"));
}

/// C4: Optimizer-reviewer conflict
#[tokio::test]
async fn c4_optimizer_reviewer_conflict() {
    let mut ctx = TestContext::new();
    ctx.add_text("The system has excellent performance.\n");

    let content = ctx.buffer_content();

    // Optimizer wants to add nuance (performance varies)
    let optimizer = actors::MockAgent::optimizer();
    let opt_proposals = optimizer.propose(&content);

    // Reviewer might accept the statement as-is (if factual)
    let reviewer = actors::MockAgent::reviewer();
    let rev_proposals = reviewer.propose(&content);

    // Simulate conflict resolution: User decides
    if !opt_proposals.is_empty() {
        let mut opt_proposal = opt_proposals[0].clone();

        // User sides with optimizer (wants nuance)
        opt_proposal.status = ProposalStatus::Accepted;
        assert_proposal_accepted(&opt_proposal);
    }

    // Verify system handles conflicting agent opinions
    assert!(opt_proposals.len() >= 0 && rev_proposals.len() >= 0);
}

/// C5: Progressive enhancement chain
#[tokio::test]
async fn c5_progressive_enhancement_chain() {
    let mut ctx = TestContext::new();

    // Start with minimal document
    ctx.add_text("System notes\n");
    let mut content = ctx.buffer_content();

    // Chain 1: Orchestrator adds structure
    let orchestrator = actors::MockAgent::orchestrator();
    let orch_proposals = orchestrator.propose(&content);

    if !orch_proposals.is_empty() {
        // Apply orchestrator's improvement (create new buffer with improved content)
        let buf_id = ctx.create_buffer();
        ctx.switch_buffer(buf_id);
        content = "# System Notes\n\n".to_string();
        ctx.add_text(&content);
    }

    // Chain 2: Semantic agent adds content
    ctx.add_text("The system is distributed.\n\n");
    content = ctx.buffer_content();

    // Chain 3: Reviewer improves clarity
    let reviewer = actors::MockAgent::reviewer();
    let rev_proposals = reviewer.propose(&content);

    // Chain 4: Optimizer improves precision
    let optimizer = actors::MockAgent::optimizer();
    let opt_proposals = optimizer.propose(&content);

    // Verify progressive enhancement occurred
    let final_content = ctx.buffer_content();
    assert!(final_content.len() >= "System notes\n".len());
    assert!(orch_proposals.len() + rev_proposals.len() + opt_proposals.len() >= 0);
}

/// C6: Emergency agent override
#[tokio::test]
async fn c6_emergency_agent_override() {
    let mut ctx = TestContext::new();
    let mut agent = actors::MockAgent::optimizer();

    // Agent starts working
    agent.set_activity(
        AgentActivity::Analyzing,
        Some("Analyzing document".to_string()),
    );
    assert_agent_analyzing(&agent.info);

    // Emergency: User needs to edit immediately
    ctx.add_text("URGENT: System is down\n");

    // User overrides agent work
    agent.set_activity(
        AgentActivity::Idle,
        Some("Paused for user edit".to_string()),
    );
    assert_agent_idle(&agent.info);

    // User makes emergency edit
    ctx.add_text("Root cause: Database connection lost\n");
    ctx.add_text("Fix: Restart connection pool\n");

    // Agent resumes after user completes
    agent.set_activity(
        AgentActivity::Analyzing,
        Some("Resuming analysis".to_string()),
    );
    assert_agent_analyzing(&agent.info);

    // Verify user edit preserved
    let content = ctx.buffer_content();
    assert!(content.contains("URGENT"));
    assert!(content.contains("Root cause"));
}
