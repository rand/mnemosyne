//! Custom assertions for integration tests

use mnemosyne_core::types::{MemoryNote, Namespace};
use mnemosyne_core::ics::SemanticAnalysis;

/// Assert memory exists in list
pub fn assert_memory_exists(memories: &[MemoryNote], content_substring: &str) {
    assert!(
        memories.iter().any(|m| m.content.contains(content_substring)),
        "Expected memory containing '{}' not found. Found {} memories",
        content_substring,
        memories.len()
    );
}

/// Assert memory count
pub fn assert_memory_count(memories: &[MemoryNote], expected: usize) {
    assert_eq!(
        memories.len(),
        expected,
        "Expected {} memories, found {}",
        expected,
        memories.len()
    );
}

/// Assert memory has correct namespace
pub fn assert_memory_namespace(memory: &MemoryNote, expected: &Namespace) {
    assert_eq!(
        &memory.namespace, expected,
        "Expected namespace {:?}, found {:?}",
        expected, memory.namespace
    );
}

/// Assert memory is not archived
pub fn assert_not_archived(memory: &MemoryNote) {
    assert!(
        !memory.is_archived,
        "Memory should not be archived but is: {}",
        memory.id
    );
}

/// Assert memory is archived
pub fn assert_archived(memory: &MemoryNote) {
    assert!(
        memory.is_archived,
        "Memory should be archived but is not: {}",
        memory.id
    );
}

/// Assert semantic analysis has minimum triples
pub fn assert_min_triples(analysis: &SemanticAnalysis, min: usize) {
    assert!(
        analysis.triples.len() >= min,
        "Expected at least {} triples, found {}",
        min,
        analysis.triples.len()
    );
}

/// Assert semantic analysis has entities
pub fn assert_has_entities(analysis: &SemanticAnalysis, expected_entities: &[&str]) {
    for entity in expected_entities {
        assert!(
            analysis.entities.iter().any(|(name, _count)| name.contains(entity)),
            "Expected entity '{}' not found in analysis",
            entity
        );
    }
}

/// Assert memory has embedding
pub fn assert_has_embedding(memory: &MemoryNote) {
    assert!(
        memory.embedding.is_some(),
        "Memory should have embedding but does not: {}",
        memory.id
    );
}

/// Assert memory does not have embedding
pub fn assert_no_embedding(memory: &MemoryNote) {
    assert!(
        memory.embedding.is_none(),
        "Memory should not have embedding but does: {}",
        memory.id
    );
}

/// Assert memories are sorted by importance (descending)
pub fn assert_sorted_by_importance(memories: &[MemoryNote]) {
    for i in 1..memories.len() {
        assert!(
            memories[i - 1].importance >= memories[i].importance,
            "Memories not sorted by importance: mem[{}]={} < mem[{}]={}",
            i - 1,
            memories[i - 1].importance,
            i,
            memories[i].importance
        );
    }
}

/// Assert access count increased
pub fn assert_access_count_increased(before: u32, after: u32) {
    assert!(
        after > before,
        "Access count should increase: before={}, after={}",
        before,
        after
    );
}

/// Assert proposal is accepted
pub fn assert_proposal_accepted(proposal: &mnemosyne_core::ics::ChangeProposal) {
    assert_eq!(
        proposal.status,
        mnemosyne_core::ics::ProposalStatus::Accepted,
        "Proposal should be accepted: {}",
        proposal.id
    );
}
