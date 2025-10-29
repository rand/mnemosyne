//! Custom assertions for ICS E2E tests
//!
//! Provides domain-specific assertions for:
//! - Buffer state verification
//! - Semantic analysis results
//! - Proposal state
//! - Agent activity
//! - CRDT attribution

use mnemosyne_core::ics::*;
use mnemosyne_core::ics::editor::*;

/// Assert buffer contains exact text
pub fn assert_buffer_contains(buffer: &TextBuffer, expected: &str) {
    let content = buffer.content.to_string();
    assert!(
        content.contains(expected),
        "Buffer should contain '{}' but was '{}'",
        expected,
        content
    );
}

/// Assert buffer equals exact text
pub fn assert_buffer_equals(buffer: &TextBuffer, expected: &str) {
    let content = buffer.content.to_string();
    assert_eq!(
        content, expected,
        "Buffer content mismatch"
    );
}

/// Assert buffer is empty
pub fn assert_buffer_empty(buffer: &TextBuffer) {
    let content = buffer.content.to_string();
    assert!(
        content.is_empty(),
        "Buffer should be empty but contained: '{}'",
        content
    );
}

/// Assert buffer line count
pub fn assert_buffer_lines(buffer: &TextBuffer, expected_lines: usize) {
    let line_count = buffer.content.len_lines();
    assert_eq!(
        line_count, expected_lines,
        "Expected {} lines but got {}",
        expected_lines, line_count
    );
}

/// Assert cursor position
pub fn assert_cursor_at(buffer: &TextBuffer, line: usize, column: usize) {
    let pos = &buffer.cursor.position;
    assert_eq!(
        (pos.line, pos.column),
        (line, column),
        "Expected cursor at ({}, {}) but was at ({}, {})",
        line, column, pos.line, pos.column
    );
}

/// Assert semantic analysis found triples
pub fn assert_triples_found(analysis: &SemanticAnalysis, min_count: usize) {
    assert!(
        analysis.triples.len() >= min_count,
        "Expected at least {} triples but found {}",
        min_count,
        analysis.triples.len()
    );
}

/// Assert specific triple exists
pub fn assert_triple_exists(
    analysis: &SemanticAnalysis,
    subject: &str,
    predicate: &str,
    object: &str,
) {
    let found = analysis.triples.iter().any(|t| {
        t.subject.contains(subject) && t.predicate == predicate && t.object.contains(object)
    });
    assert!(
        found,
        "Expected triple '{}' {} '{}' not found in analysis",
        subject, predicate, object
    );
}

/// Assert typed holes found
pub fn assert_holes_found(analysis: &SemanticAnalysis, min_count: usize) {
    assert!(
        analysis.holes.len() >= min_count,
        "Expected at least {} holes but found {}",
        min_count,
        analysis.holes.len()
    );
}

/// Assert specific hole kind exists
pub fn assert_hole_kind_exists(analysis: &SemanticAnalysis, kind: HoleKind) {
    let found = analysis.holes.iter().any(|h| h.kind == kind);
    assert!(
        found,
        "Expected hole of kind {:?} not found in analysis",
        kind
    );
}

/// Assert entities extracted
pub fn assert_entities_found(analysis: &SemanticAnalysis, expected_entities: &[&str]) {
    for entity in expected_entities {
        assert!(
            analysis.entities.contains_key(*entity),
            "Expected entity '{}' not found in analysis",
            entity
        );
    }
}

/// Assert entity count
pub fn assert_entity_count(analysis: &SemanticAnalysis, entity: &str, expected_count: usize) {
    let count = analysis.entities.get(entity).copied().unwrap_or(0);
    assert_eq!(
        count, expected_count,
        "Expected entity '{}' to appear {} times but found {}",
        entity, expected_count, count
    );
}

/// Assert proposal has specific status
pub fn assert_proposal_status(proposal: &ChangeProposal, expected_status: ProposalStatus) {
    assert_eq!(
        proposal.status, expected_status,
        "Expected proposal {} to have status {:?} but was {:?}",
        proposal.id, expected_status, proposal.status
    );
}

/// Assert proposal is pending
pub fn assert_proposal_pending(proposal: &ChangeProposal) {
    assert_proposal_status(proposal, ProposalStatus::Pending);
}

/// Assert proposal is accepted
pub fn assert_proposal_accepted(proposal: &ChangeProposal) {
    assert_proposal_status(proposal, ProposalStatus::Accepted);
}

/// Assert proposal is rejected
pub fn assert_proposal_rejected(proposal: &ChangeProposal) {
    assert_proposal_status(proposal, ProposalStatus::Rejected);
}

/// Assert agent activity status
pub fn assert_agent_activity(agent: &AgentInfo, expected_activity: AgentActivity) {
    assert_eq!(
        agent.activity, expected_activity,
        "Expected agent '{}' to have activity {:?} but was {:?}",
        agent.name, expected_activity, agent.activity
    );
}

/// Assert agent is idle
pub fn assert_agent_idle(agent: &AgentInfo) {
    assert!(
        matches!(agent.activity, AgentActivity::Idle),
        "Expected agent '{}' to be idle but was {:?}",
        agent.name, agent.activity
    );
}

/// Assert agent is analyzing
pub fn assert_agent_analyzing(agent: &AgentInfo) {
    assert!(
        matches!(agent.activity, AgentActivity::Analyzing),
        "Expected agent '{}' to be analyzing but was {:?}",
        agent.name, agent.activity
    );
}

/// Assert agent is proposing
pub fn assert_agent_proposing(agent: &AgentInfo) {
    assert!(
        matches!(agent.activity, AgentActivity::Proposing),
        "Expected agent '{}' to be proposing but was {:?}",
        agent.name, agent.activity
    );
}

/// Assert diagnostic severity
pub fn assert_diagnostic_severity(diagnostic: &Diagnostic, expected_severity: Severity) {
    assert_eq!(
        diagnostic.severity, expected_severity,
        "Expected diagnostic at line {} to have severity {:?} but was {:?}",
        diagnostic.position.line, expected_severity, diagnostic.severity
    );
}

/// Assert diagnostics count by severity
pub fn assert_diagnostics_count(diagnostics: &[Diagnostic], severity: Severity, expected_count: usize) {
    let count = diagnostics.iter().filter(|d| d.severity == severity).count();
    assert_eq!(
        count, expected_count,
        "Expected {} diagnostics with severity {:?} but found {}",
        expected_count, severity, count
    );
}

/// Assert no diagnostics (clean document)
pub fn assert_no_diagnostics(diagnostics: &[Diagnostic]) {
    assert!(
        diagnostics.is_empty(),
        "Expected no diagnostics but found {}",
        diagnostics.len()
    );
}

/// Assert CRDT attribution for actor
pub fn assert_attribution_by_actor(attributions: &[Attribution], actor: Actor, min_count: usize) {
    let count = attributions.iter().filter(|a| a.actor == actor).count();
    assert!(
        count >= min_count,
        "Expected at least {} attributions by {:?} but found {}",
        min_count, actor, count
    );
}

/// Assert attribution exists for range
pub fn assert_attribution_in_range(
    attributions: &[Attribution],
    start: usize,
    end: usize,
    expected_actor: Actor,
) {
    let found = attributions.iter().any(|a| {
        a.range.0 <= start && a.range.1 >= end && a.actor == expected_actor
    });
    assert!(
        found,
        "Expected attribution by {:?} in range ({}, {}) not found",
        expected_actor, start, end
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_assertions() {
        let mut buffer = TextBuffer::new(0, None);
        assert_buffer_empty(&buffer);

        buffer.insert("test");
        assert_buffer_contains(&buffer, "test");
        assert_buffer_equals(&buffer, "test");
    }

    #[test]
    fn test_semantic_assertions() {
        let analysis = SemanticAnalysis {
            triples: vec![
                Triple {
                    subject: "System".to_string(),
                    predicate: "is".to_string(),
                    object: "distributed".to_string(),
                    source_line: 0,
                    confidence: 80,
                },
            ],
            holes: vec![
                TypedHole {
                    name: "TODO".to_string(),
                    kind: HoleKind::Incomplete,
                    line: 5,
                    column: 0,
                    context: "TODO: implement".to_string(),
                    suggestions: vec![],
                },
            ],
            entities: [("System".to_string(), 2)].into_iter().collect(),
            relationships: vec![],
        };

        assert_triples_found(&analysis, 1);
        assert_triple_exists(&analysis, "System", "is", "distributed");
        assert_holes_found(&analysis, 1);
        assert_hole_kind_exists(&analysis, HoleKind::Incomplete);
        assert_entities_found(&analysis, &["System"]);
        assert_entity_count(&analysis, "System", 2);
    }
}
