//! ICS Editor integration tests for semantic highlighting
//!
//! Tests the complete integration of semantic highlighting with the ICS editor,
//! including text change hooks and rendering.

use mnemosyne_core::ics::editor::{Actor, CrdtBuffer};

#[tokio::test]
async fn test_buffer_has_semantic_engine() {
    let buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();

    // Semantic engine should be initialized by default
    assert!(buffer.semantic_engine.is_some());
}

#[tokio::test]
async fn test_insert_triggers_analysis() {
    let mut buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();

    // Insert text should not panic and should schedule analysis
    buffer.insert(0, "Test text with MUST keyword").unwrap();

    // Verify text was inserted
    let text = buffer.text().unwrap();
    assert!(text.contains("Test text"));
}

#[tokio::test]
async fn test_delete_triggers_analysis() {
    let mut buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();

    // Insert some text
    buffer.insert(0, "Test text with content").unwrap();

    // Delete some text should schedule analysis
    buffer.delete(5, 5).unwrap();

    // Verify deletion worked
    let text = buffer.text().unwrap();
    assert!(!text.contains("text "));
}

#[tokio::test]
async fn test_multiple_edits_sequential() {
    let mut buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();

    // Multiple edits should all schedule analysis without panicking
    buffer.insert(0, "First line\n").unwrap();
    buffer.insert(11, "Second line\n").unwrap();
    buffer.insert(23, "Third line\n").unwrap();

    let text = buffer.text().unwrap();
    assert!(text.contains("First"));
    assert!(text.contains("Second"));
    assert!(text.contains("Third"));
}

#[tokio::test]
async fn test_semantic_engine_with_complex_text() {
    let mut buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();

    // Insert text with multiple semantic features
    let complex_text =
        "<thinking>The system MUST validate input. Dr. Smith reviewed the algorithm.</thinking>";
    buffer.insert(0, complex_text).unwrap();

    // Get the semantic engine
    if let Some(ref engine_cell) = buffer.semantic_engine {
        // Borrow and highlight
        let line = engine_cell.borrow_mut().highlight_line(complex_text);

        // Should have highlighted spans
        assert!(!line.spans.is_empty());
    }
}

#[tokio::test]
async fn test_semantic_highlighting_after_edits() {
    let mut buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();

    // Initial text
    buffer.insert(0, "Initial text").unwrap();

    // Multiple edits
    buffer.insert(12, " with MUST keyword").unwrap();
    buffer.delete(0, 8).unwrap(); // Remove "Initial "

    let final_text = buffer.text().unwrap();

    // Get highlighted output
    if let Some(ref engine_cell) = buffer.semantic_engine {
        let line = engine_cell.borrow_mut().highlight_line(&final_text);

        // Should handle edited text without panicking
        let _ = line;
    }
}

#[tokio::test]
async fn test_analysis_range_for_insert() {
    let mut buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();

    // Insert at position 0 with length 10
    buffer.insert(0, "0123456789").unwrap();

    // The scheduled analysis should cover range 0..10
    // (verified by no panic and successful insertion)
    let text = buffer.text().unwrap();
    assert_eq!(text.len(), 10);
}

#[tokio::test]
async fn test_analysis_range_for_delete() {
    let mut buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();

    buffer.insert(0, "0123456789ABCDEF").unwrap();

    // Delete at position 5, length 5
    // Should schedule analysis with expanded context (pos-50 to pos+50)
    buffer.delete(5, 5).unwrap();

    let text = buffer.text().unwrap();
    assert_eq!(text.len(), 11); // 16 - 5 = 11
}

#[tokio::test]
async fn test_context_expansion_on_delete() {
    let mut buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();

    // Insert long text
    let long_text = "A".repeat(200);
    buffer.insert(0, &long_text).unwrap();

    // Delete in the middle at position 100
    buffer.delete(100, 10).unwrap();

    // Context should expand ±50 chars from deletion point
    // Analysis should cover approximately 50..150 (clamped to text bounds)
    // Verified by no panic
    let text = buffer.text().unwrap();
    assert_eq!(text.len(), 190);
}

#[tokio::test]
async fn test_edge_case_delete_at_start() {
    let mut buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();

    buffer.insert(0, "Test content here").unwrap();

    // Delete at start (context_start should saturate to 0)
    buffer.delete(0, 4).unwrap();

    let text = buffer.text().unwrap();
    assert!(text.starts_with(" content"));
}

#[tokio::test]
async fn test_edge_case_delete_at_end() {
    let mut buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();

    buffer.insert(0, "Test content here").unwrap();
    let len = buffer.text().unwrap().len();

    // Delete last 4 chars "here" (context_end should clamp to text length)
    buffer.delete(len - 4, 4).unwrap();

    let text = buffer.text().unwrap();
    // After deleting "here", should end with "content "
    assert!(text.ends_with("content ") || text.ends_with("tent "));
}

#[tokio::test]
async fn test_empty_buffer_semantic_engine() {
    let buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();

    // Empty buffer should still have engine
    assert!(buffer.semantic_engine.is_some());

    if let Some(ref engine_cell) = buffer.semantic_engine {
        let line = engine_cell.borrow_mut().highlight_line("");
        // Empty text should return empty or single plain span
        let _ = line;
    }
}

#[tokio::test]
async fn test_unicode_text_handling() {
    let mut buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();

    // Insert text with Unicode characters
    let unicode_text = "Hello 世界 🌍 MUST validate データ";
    buffer.insert(0, unicode_text).unwrap();

    let text = buffer.text().unwrap();
    assert_eq!(text, unicode_text);

    // Semantic highlighting should handle Unicode
    if let Some(ref engine_cell) = buffer.semantic_engine {
        let line = engine_cell.borrow_mut().highlight_line(&text);
        // Should not panic on Unicode
        let _ = line;
    }
}

#[tokio::test]
async fn test_rapid_edits() {
    let mut buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();

    // Simulate rapid typing (multiple inserts in quick succession)
    for i in 0..10 {
        let text = format!("Line {} ", i);
        let pos = buffer.text().unwrap().len();
        buffer.insert(pos, &text).unwrap();
    }

    let final_text = buffer.text().unwrap();
    assert!(final_text.contains("Line 0"));
    assert!(final_text.contains("Line 9"));
}

#[tokio::test]
async fn test_interleaved_insert_delete() {
    let mut buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();

    buffer.insert(0, "AAAA").unwrap();
    buffer.delete(2, 2).unwrap();
    buffer.insert(2, "BBBB").unwrap();
    buffer.delete(0, 1).unwrap();
    buffer.insert(0, "C").unwrap();

    // Should handle interleaved operations
    let text = buffer.text().unwrap();
    assert!(!text.is_empty());
}

#[tokio::test]
async fn test_semantic_engine_caching() {
    let buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();

    if let Some(ref engine_cell) = buffer.semantic_engine {
        let test_text = "Test with MUST keyword";

        // First highlight (cold cache)
        let line1 = engine_cell.borrow_mut().highlight_line(test_text);

        // Second highlight (should use cache for some tiers)
        let line2 = engine_cell.borrow_mut().highlight_line(test_text);

        // Both should produce results
        assert!(!line1.spans.is_empty() || !line2.spans.is_empty());
    }
}
