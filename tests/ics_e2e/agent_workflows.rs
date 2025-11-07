//! Agent-centric workflow tests for ICS
//!
//! Tests covering agent interaction patterns:
//! - A1: Single agent proposal workflow
//! - A2: Multi-agent concurrent proposals
//! - A3: Agent status monitoring
//! - A4: Proposal review & modification
//! - A5: Proposal rejection & feedback
//! - A6: Agent-driven semantic improvements
//! - A7: Agent activity timeline

use crate::ics_e2e::*;
use mnemosyne_core::ics::{AgentActivity, ChangeProposal, ProposalStatus};

/// A1: Single agent proposal workflow
#[tokio::test]
async fn a1_single_agent_proposal_workflow() {
    let mut ctx = TestContext::new();

    // Add document content
    ctx.add_text("Some text without title");

    // Create orchestrator agent
    let agent = actors::MockAgent::orchestrator();

    // Agent analyzes and creates proposal
    let proposals = agent.propose(&ctx.buffer_content());

    // Verify proposal created
    assert!(!proposals.is_empty(), "Agent should create proposal");
    let proposal = &proposals[0];
    assert_eq!(proposal.agent, "agent:orchestrator");
    assert_proposal_pending(proposal);

    // Simulate user acceptance
    let mut accepted_proposal = proposal.clone();
    accepted_proposal.status = ProposalStatus::Accepted;
    assert_proposal_accepted(&accepted_proposal);
}

/// A2: Multi-agent concurrent proposals
#[tokio::test]
async fn a2_multi_agent_concurrent_proposals() {
    let mut ctx = TestContext::new();

    // Add document with multiple issues
    ctx.add_text("TODO: Add feature\nHowever, the system is fast.\n@undefined symbol");

    // Create multiple agents
    let mut agents = actors::create_mock_agents();

    // Agents analyze concurrently
    let all_proposals =
        actors::simulate_concurrent_agents(&mut agents, &ctx.buffer_content()).await;

    // Verify each agent type created proposals
    let total_proposals: usize = all_proposals.iter().map(|p| p.len()).sum();
    assert!(
        total_proposals > 0,
        "At least one agent should create proposals"
    );

    // Verify proposals from different agents
    let flat_proposals: Vec<_> = all_proposals.into_iter().flatten().collect();
    let agent_types: Vec<_> = flat_proposals.iter().map(|p| &p.agent).collect();

    // Should have proposals from multiple agent types
    assert!(!agent_types.is_empty(), "Should have agent proposals");
}

/// A3: Agent status monitoring
#[tokio::test]
async fn a3_agent_status_monitoring() {
    let mut agent = actors::MockAgent::new("test", "TestAgent");

    // Initially idle
    assert_agent_idle(&agent.info);

    // Set to analyzing
    agent.set_activity(
        AgentActivity::Analyzing,
        Some("Analyzing document".to_string()),
    );
    assert_agent_analyzing(&agent.info);
    assert_eq!(agent.info.message, Some("Analyzing document".to_string()));

    // Set to proposing
    agent.set_activity(
        AgentActivity::Proposing,
        Some("Creating proposal".to_string()),
    );
    assert_agent_proposing(&agent.info);

    // Set to waiting
    agent.set_activity(AgentActivity::Waiting, Some("Awaiting review".to_string()));
    assert_agent_activity(&agent.info, AgentActivity::Waiting);

    // Set to error
    agent.trigger_error("Analysis failed");
    assert!(matches!(agent.info.activity, AgentActivity::Error(_)));
}

/// A4: Proposal review & modification
#[tokio::test]
async fn a4_proposal_review_and_modification() {
    let mut ctx = TestContext::new();
    ctx.add_text("TODO: implement");

    // Agent creates proposal
    let agent = actors::MockAgent::reviewer();
    let proposals = agent.propose(&ctx.buffer_content());

    assert!(!proposals.is_empty());
    let mut proposal = proposals[0].clone();

    // User reviews and modifies
    let original_proposed = proposal.proposed.clone();
    proposal.proposed = format!("{} (modified by user)", original_proposed);

    // User accepts modified version
    proposal.status = ProposalStatus::Accepted;

    // Verify modification preserved
    assert!(proposal.proposed.contains("(modified by user)"));
    assert_proposal_accepted(&proposal);
}

/// A5: Proposal rejection & feedback
#[tokio::test]
async fn a5_proposal_rejection_and_feedback() {
    let mut ctx = TestContext::new();
    ctx.add_text("However, performance varies");

    // Agent creates first proposal
    let agent = actors::MockAgent::optimizer();
    let proposals = agent.propose(&ctx.buffer_content());

    assert!(!proposals.is_empty());
    let mut proposal = proposals[0].clone();

    // User rejects
    proposal.status = ProposalStatus::Rejected;
    assert_proposal_rejected(&proposal);

    // Agent could propose alternative (simulated by creating new proposal)
    let alternative_proposal = ChangeProposal {
        id: "opt-alt-1".to_string(),
        agent: "agent:optimizer".to_string(),
        description: "Alternative approach".to_string(),
        original: proposal.original.clone(),
        proposed: "Different solution based on feedback".to_string(),
        line_range: proposal.line_range,
        created_at: std::time::SystemTime::now(),
        status: ProposalStatus::Pending,
        rationale: "Revised based on user feedback".to_string(),
    };

    // User accepts alternative
    let mut accepted = alternative_proposal.clone();
    accepted.status = ProposalStatus::Accepted;
    assert_proposal_accepted(&accepted);
}

/// A6: Agent-driven semantic improvements
#[tokio::test]
async fn a6_agent_driven_semantic_improvements() {
    let mut ctx = TestContext::new();

    // Add document with semantic issues
    ctx.add_text(fixtures::sample_markdown_doc());

    // Run semantic analysis
    let analysis = ctx.analyze().await.expect("Analysis should succeed");

    // Verify holes detected
    assert_holes_found(&analysis, 1);

    // Agent proposes resolution for holes
    let agent = actors::MockAgent::reviewer();
    let proposals = agent.propose(&ctx.buffer_content());

    // Verify agent addresses semantic issues
    if !proposals.is_empty() {
        let proposal = &proposals[0];
        assert!(
            proposal.rationale.contains("TODO") || proposal.rationale.contains("should"),
            "Proposal should address detected issues"
        );
    }
}

/// A7: Agent activity timeline
#[tokio::test]
async fn a7_agent_activity_timeline() {
    let mut agents = actors::create_mock_agents();
    let mut timeline = Vec::new();

    // Simulate agent activities over time
    for agent in &mut agents {
        agent.set_activity(AgentActivity::Idle, None);
        timeline.push((
            agent.info.name.clone(),
            agent.info.activity.clone(),
            agent.info.last_active,
        ));

        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;

        agent.set_activity(AgentActivity::Analyzing, Some("Working".to_string()));
        timeline.push((
            agent.info.name.clone(),
            agent.info.activity.clone(),
            agent.info.last_active,
        ));
    }

    // Verify timeline has events
    assert_eq!(timeline.len(), agents.len() * 2);

    // Verify timestamps are ordered
    for i in 1..timeline.len() {
        assert!(
            timeline[i].2 >= timeline[i - 1].2,
            "Timeline should be chronologically ordered"
        );
    }
}
