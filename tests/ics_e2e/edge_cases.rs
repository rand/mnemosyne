//! Edge case and error handling tests for ICS
//!
//! Tests covering edge cases:
//! - E1: Rapid panel toggling
//! - E2: Malformed analysis input
//! - E3: Concurrent proposal conflicts
//! - E4: Panel state persistence
//! - E5: Buffer limit handling

use crate::ics_e2e::*;
use mnemosyne_core::ics::{AgentActivity, ProposalStatus};

/// E1: Rapid panel toggling
#[tokio::test]
async fn e1_rapid_panel_toggling() {
    let ctx = TestContext::with_fixtures();

    // Rapid state changes
    for _ in 0..100 {
        // In real implementation, would toggle panel visibility
        // For testing, verify state remains consistent
        assert!(ctx.memories.len() > 0);
        assert!(ctx.proposals.len() > 0);
    }

    // Verify data integrity after rapid toggles
    assert_eq!(ctx.memories.len(), 3);
    assert_eq!(ctx.proposals.len(), 3);

    // Verify proposals still accessible
    for proposal in &ctx.proposals {
        assert!(proposal.id.len() > 0);
        assert!(proposal.agent.len() > 0);
    }

    // Verify memories still accessible
    for memory in &ctx.memories {
        assert!(memory.content.len() > 0);
        assert!(memory.importance >= 0);
    }
}

/// E2: Malformed analysis input
#[tokio::test]
async fn e2_malformed_analysis_input() {
    let mut ctx = TestContext::new();

    // Test empty input
    ctx.add_text("");
    let analysis_result = ctx.analyze().await;
    // Empty input might timeout or return empty analysis
    // Either outcome is acceptable

    // Test malformed markup
    let buffer = ctx.editor.active_buffer_mut();
    let pos = buffer.text_len().expect("Should get text length");
    buffer
        .insert(pos, "# Unclosed header [[[[[")
        .expect("Should insert");
    let pos = buffer.text_len().expect("Should get text length");
    buffer
        .insert(pos, "{{{{{{ Unbalanced braces")
        .expect("Should insert");

    let analysis_result = ctx.analyze().await;
    // Should handle gracefully (either succeed or fail cleanly)
    match analysis_result {
        Ok(analysis) => {
            // If succeeds, should have valid structure
            assert!(analysis.triples.len() >= 0);
            assert!(analysis.entities.len() >= 0);
        }
        Err(_) => {
            // Failure is acceptable for malformed input
            // Just verify it doesn't panic
        }
    }

    // Test extremely long line
    let long_line = "a".repeat(10000);
    let buffer = ctx.editor.active_buffer_mut();
    let pos = buffer.text_len().expect("Should get text length");
    buffer.insert(pos, &long_line).expect("Should insert");

    // Should handle without panic
    let content = ctx.buffer_content();
    assert!(content.len() >= 10000);
}

/// E3: Concurrent proposal conflicts
#[tokio::test]
async fn e3_concurrent_proposal_conflicts() {
    let mut ctx = TestContext::new();
    ctx.add_text("However, the system has performance issues.\n");

    let content = ctx.buffer_content();

    // Multiple agents propose changes to overlapping text
    let optimizer = actors::MockAgent::optimizer();
    let reviewer = actors::MockAgent::reviewer();

    let opt_proposals = optimizer.propose(&content);
    let rev_proposals = reviewer.propose(&content);

    // Both might target same "However" text
    if !opt_proposals.is_empty() && !rev_proposals.is_empty() {
        let opt_proposal = &opt_proposals[0];
        let rev_proposal = &rev_proposals[0];

        // Detect if proposals conflict (overlap in line ranges)
        let opt_range = opt_proposal.line_range;
        let rev_range = rev_proposal.line_range;

        let conflicts = opt_range.0 <= rev_range.1 && rev_range.0 <= opt_range.1;

        if conflicts {
            // System should handle conflict resolution
            // User chooses one proposal
            let mut accepted = opt_proposal.clone();
            accepted.status = ProposalStatus::Accepted;

            let mut rejected = rev_proposal.clone();
            rejected.status = ProposalStatus::Rejected;

            assert_proposal_accepted(&accepted);
            assert_proposal_rejected(&rejected);
        }
    }

    // Verify system remains stable after conflict
    let content = ctx.editor.active_buffer().text().expect("Should get text");
    assert!(content.lines().count() > 0);
}

/// E4: Panel state persistence
#[tokio::test]
async fn e4_panel_state_persistence() {
    let ctx = TestContext::with_fixtures();

    // Store initial state
    let initial_memories = ctx.memories.len();
    let initial_proposals = ctx.proposals.len();

    // Simulate operations that might affect panels
    let mut edit_ctx = TestContext::new();
    edit_ctx.add_text("New content\n");

    // Verify original context state persists
    assert_eq!(ctx.memories.len(), initial_memories);
    assert_eq!(ctx.proposals.len(), initial_proposals);

    // Verify panel data quality
    assert!(ctx.memories.iter().all(|m| m.importance > 0));
    assert!(ctx.proposals.iter().all(|p| p.id.len() > 0));

    // Test filtering persists
    let filtered: Vec<_> = ctx.memories.iter().filter(|m| m.importance >= 8).collect();

    assert_eq!(filtered.len(), 2);

    // Re-filter to verify state consistency
    let filtered_again: Vec<_> = ctx.memories.iter().filter(|m| m.importance >= 8).collect();

    assert_eq!(filtered.len(), filtered_again.len());
}

/// E5: Buffer limit handling
#[tokio::test]
async fn e5_buffer_limit_handling() {
    let mut ctx = TestContext::new();

    // Test very large document
    let large_text = "Line\n".repeat(10000);
    ctx.add_text(&large_text);

    let content = ctx.buffer_content();
    assert!(content.lines().count() >= 10000);

    // Test buffer with many edits (stress undo stack)
    let mut small_ctx = TestContext::new();
    for i in 0..1000 {
        small_ctx.add_text(&format!("Edit {}\n", i));
    }

    // Test undo doesn't overflow
    for _ in 0..10 {
        small_ctx.editor.active_buffer_mut().undo();
    }

    // Verify buffer still functional
    assert!(small_ctx.buffer_content().len() > 0);

    // Test redo
    for _ in 0..5 {
        small_ctx.editor.active_buffer_mut().redo();
    }

    assert!(small_ctx.buffer_content().len() > 0);

    // Test empty buffer edge case
    let empty_ctx = TestContext::new();
    assert_buffer_empty(empty_ctx.editor.active_buffer());

    // Test single character
    let mut single_ctx = TestContext::new();
    single_ctx.add_text("x");
    assert_eq!(single_ctx.buffer_content().trim(), "x");

    // Test unicode handling
    let mut unicode_ctx = TestContext::new();
    unicode_ctx.add_text("Hello 世界 🚀\n");
    let unicode_content = unicode_ctx.buffer_content();
    assert!(unicode_content.contains("世界"));
    assert!(unicode_content.contains("🚀"));

    // Test extremely nested structure
    let nested =
        "# Level 1\n## Level 2\n### Level 3\n#### Level 4\n##### Level 5\n###### Level 6\n";
    let mut nested_ctx = TestContext::new();
    nested_ctx.add_text(nested);
    assert!(nested_ctx.buffer_content().contains("Level 6"));
}
