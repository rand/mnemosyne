//! Human-centric workflow tests for ICS
//!
//! Tests covering typical human interaction patterns:
//! - H1: Document creation & basic editing
//! - H2: Panel navigation & discovery
//! - H3: Memory search & integration
//! - H4: Real-time diagnostics
//! - H5: Multi-buffer management
//! - H6: Complex editing operations
//! - H7: Semantic analysis integration
//! - H8: Save/load workflow

use crate::ics_e2e::*;
use mnemosyne_core::ics::editor::Severity;
use mnemosyne_core::ics::HoleKind;

/// H1: Document creation & basic editing
#[tokio::test]
async fn h1_document_creation_and_basic_editing() {
    let mut ctx = TestContext::new();

    // User creates new document (starts with empty buffer)
    assert_buffer_empty(ctx.editor.active_buffer());

    // Types multi-line content with markdown
    ctx.add_text("# My Document\n\n");
    ctx.add_text("This is the first paragraph.\n\n");
    ctx.add_text("## Section 1\n\n");
    ctx.add_text("Content for section 1.\n");

    // Verify content
    let content = ctx.buffer_content();
    assert!(content.contains("# My Document"));
    assert!(content.contains("## Section 1"));
    // Rope adds an implicit final newline, so we have 8 lines including the empty one
    assert_buffer_lines(ctx.editor.active_buffer(), 8);

    // TODO: Add cursor navigation tests
    // TODO: Add undo/redo tests
    // TODO: Add save tests
}

/// H2: Panel navigation & discovery
#[tokio::test]
async fn h2_panel_navigation_and_discovery() {
    let ctx = TestContext::with_fixtures();

    // Start with no visible panels (this is ICS app logic, so we test state)
    // In real implementation, panels would start hidden
    // For now, verify panel state can be managed independently
    assert!(ctx.memories.len() > 0, "Should have test memories");
    assert!(ctx.proposals.len() > 0, "Should have test proposals");
}

/// H3: Memory search & integration
#[tokio::test]
async fn h3_memory_search_and_integration() {
    let ctx = TestContext::with_fixtures();

    // Verify we have memories to search
    assert_eq!(ctx.memories.len(), 3);

    // Test search filtering
    let rust_memories: Vec<_> = ctx
        .memories
        .iter()
        .filter(|m| m.keywords.contains(&"rust".to_string()))
        .collect();
    assert_eq!(rust_memories.len(), 1);

    // Test tag filtering
    let arch_memories: Vec<_> = ctx
        .memories
        .iter()
        .filter(|m| m.tags.contains(&"Architecture".to_string()))
        .collect();
    assert_eq!(arch_memories.len(), 2);

    // Test importance sorting
    let high_importance: Vec<_> = ctx.memories.iter().filter(|m| m.importance >= 8).collect();
    assert_eq!(high_importance.len(), 2);
}

/// H4: Real-time diagnostics
#[tokio::test]
async fn h4_realtime_diagnostics() {
    let diagnostics = fixtures::sample_diagnostics();

    // Verify we have diagnostics
    assert_eq!(diagnostics.len(), 4);

    // Count by severity
    assert_diagnostics_count(&diagnostics, Severity::Error, 3);
    assert_diagnostics_count(&diagnostics, Severity::Warning, 1);

    // Verify specific diagnostics exist
    let has_todo = diagnostics.iter().any(|d| d.message.contains("TODO"));
    assert!(has_todo, "Should have TODO diagnostic");

    let has_contradiction = diagnostics
        .iter()
        .any(|d| d.message.contains("Contradictory"));
    assert!(has_contradiction, "Should have contradiction diagnostic");
}

/// H5: Multi-buffer management
#[tokio::test]
async fn h5_multi_buffer_management() {
    let mut ctx = TestContext::new();

    // Start with buffer 0
    assert_eq!(ctx.editor.active_buffer().id, 0);

    // Create additional buffers
    let buf1 = ctx.create_buffer();
    let buf2 = ctx.create_buffer();
    assert_eq!(buf1, 1);
    assert_eq!(buf2, 2);

    // Add different content to each
    ctx.add_text("Buffer 0 content");

    ctx.switch_buffer(buf1);
    ctx.add_text("Buffer 1 content");

    ctx.switch_buffer(buf2);
    ctx.add_text("Buffer 2 content");

    // Verify each buffer has independent content
    ctx.switch_buffer(0);
    assert_buffer_contains(ctx.editor.active_buffer(), "Buffer 0");

    ctx.switch_buffer(buf1);
    assert_buffer_contains(ctx.editor.active_buffer(), "Buffer 1");

    ctx.switch_buffer(buf2);
    assert_buffer_contains(ctx.editor.active_buffer(), "Buffer 2");
}

/// H6: Complex editing operations
#[tokio::test]
async fn h6_complex_editing_operations() {
    let mut ctx = TestContext::new();

    // Insert text at start
    ctx.add_text("Line 1\n");
    ctx.add_text("Line 2\n");
    ctx.add_text("Line 3\n");

    // Verify content
    let content = ctx.buffer_content();
    assert!(content.contains("Line 1"));
    assert!(content.contains("Line 2"));
    assert!(content.contains("Line 3"));

    // Test undo
    ctx.editor.active_buffer_mut().undo();
    let content_after_undo = ctx.buffer_content();
    assert!(!content_after_undo.contains("Line 3"));
    assert!(content_after_undo.contains("Line 2"));

    // Test redo
    ctx.editor.active_buffer_mut().redo();
    let content_after_redo = ctx.buffer_content();
    assert!(content_after_redo.contains("Line 3"));
}

/// H7: Semantic analysis integration
#[tokio::test]
async fn h7_semantic_analysis_integration() {
    let mut ctx = TestContext::new();

    // Add document with semantic patterns
    ctx.add_text(fixtures::sample_markdown_doc());

    // Trigger analysis
    let analysis = ctx.analyze().await.expect("Analysis should succeed");

    // Verify triples extracted
    assert_triples_found(&analysis, 1);
    assert_triple_exists(&analysis, "system", "is", "distributed");

    // Verify holes detected
    assert_holes_found(&analysis, 1);
    assert_hole_kind_exists(&analysis, HoleKind::Incomplete);

    // Verify entities extracted
    assert_entities_found(&analysis, &["Orchestrator", "Optimizer"]);
}

/// H8: Save/load workflow
#[tokio::test]
async fn h8_save_load_workflow() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("test.md");

    // Create buffer with content
    let mut buffer = TextBuffer::new(0, Some(file_path.clone()));
    buffer.insert("# Test Document\n\n");
    buffer.insert("Content here.\n");

    // Save
    buffer.save_file().expect("Should save");
    assert!(!buffer.dirty, "Buffer should not be dirty after save");

    // Verify file exists
    assert!(file_path.exists());

    // Load content from disk
    let content = fs::read_to_string(&file_path).expect("Should read file");
    assert!(content.contains("# Test Document"));
    assert!(content.contains("Content here"));
}
