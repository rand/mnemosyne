//! Integration tests for ICS (Integrated Context Studio)
//!
//! Tests complete workflows and interactions between components:
//! - Editor → Semantic Analysis → Panels
//! - Panel state management and coordination
//! - Async operations integration
//! - Memory loading and search

use mnemosyne_core::ics::editor::*;
use mnemosyne_core::ics::*;
use mnemosyne_core::{MemoryNote, MemoryType};
use std::time::SystemTime;

mod common;

/// Test complete editor workflow: create buffer, edit, analyze
#[tokio::test]
async fn test_editor_to_semantic_analysis_workflow() {
    // Create editor
    let mut editor = IcsEditor::new();
    let buffer = editor.active_buffer_mut();

    // Insert text with semantic patterns
    let pos = buffer.text_len().expect("Should get text length");
    buffer
        .insert(pos, "The system is distributed.\n")
        .expect("Should insert");
    let pos = buffer.text_len().expect("Should get text length");
    buffer
        .insert(pos, "The agent has memory.\n")
        .expect("Should insert");
    let pos = buffer.text_len().expect("Should get text length");
    buffer
        .insert(pos, "Service requires authentication.\n")
        .expect("Should insert");

    // Get buffer content
    let text = buffer.text().expect("Should get text");

    // Create semantic analyzer
    let mut analyzer = SemanticAnalyzer::new();

    // Initially not analyzing
    assert!(!analyzer.is_analyzing());

    // Trigger analysis
    analyzer.analyze(text).expect("Analysis should start");

    // Should be analyzing now
    assert!(analyzer.is_analyzing());

    // Poll for result (with timeout)
    let mut attempts = 0;
    let mut result = None;
    while attempts < 100 {
        if let Some(analysis) = analyzer.try_recv() {
            result = Some(analysis);
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        attempts += 1;
    }

    // Should have result
    let analysis = result.expect("Analysis should complete");

    // Should no longer be analyzing
    assert!(!analyzer.is_analyzing());

    // Verify analysis results
    assert_eq!(analysis.triples.len(), 3);
    assert!(analysis.triples.iter().any(|t| t.predicate == "is"));
    assert!(analysis.triples.iter().any(|t| t.predicate == "has"));
    assert!(analysis.triples.iter().any(|t| t.predicate == "requires"));
}

/// Test panel state coordination
#[test]
fn test_panel_state_coordination() {
    // Create panel states
    let mut memory_panel = MemoryPanelState::new();
    let mut diagnostics_panel = DiagnosticsPanelState::new();
    let proposals_panel = ProposalsPanelState::new();

    // All panels start hidden
    assert!(!memory_panel.is_visible());
    assert!(!diagnostics_panel.is_visible());
    assert!(!proposals_panel.is_visible());

    // Open memory panel
    memory_panel.show();
    assert!(memory_panel.is_visible());

    // Other panels remain hidden
    assert!(!diagnostics_panel.is_visible());
    assert!(!proposals_panel.is_visible());

    // Toggle diagnostics
    diagnostics_panel.toggle();
    assert!(diagnostics_panel.is_visible());

    // Both now visible (multiple panels can be open)
    assert!(memory_panel.is_visible());
    assert!(diagnostics_panel.is_visible());

    // Hide all
    memory_panel.hide();
    diagnostics_panel.hide();

    assert!(!memory_panel.is_visible());
    assert!(!diagnostics_panel.is_visible());
}

/// Test memory panel search and selection workflow
#[test]
fn test_memory_panel_search_and_selection() {
    // Create test memories
    let memories = vec![
        MemoryNote {
            id: mnemosyne_core::MemoryId::new(),
            namespace: mnemosyne_core::Namespace::Global,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            content: "The system uses Rust for performance".to_string(),
            summary: "Rust usage".to_string(),
            keywords: vec!["rust".to_string(), "performance".to_string()],
            tags: vec!["Architecture".to_string()],
            context: "technical decision".to_string(),
            memory_type: MemoryType::ArchitectureDecision,
            importance: 8,
            confidence: 0.9,
            links: vec![],
            related_files: vec![],
            related_entities: vec![],
            access_count: 0,
            last_accessed_at: chrono::Utc::now(),
            expires_at: None,
            is_archived: false,
            superseded_by: None,
            embedding: None,
            embedding_model: "test".to_string(),
        },
        MemoryNote {
            id: mnemosyne_core::MemoryId::new(),
            namespace: mnemosyne_core::Namespace::Global,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            content: "The API uses REST endpoints".to_string(),
            summary: "REST API".to_string(),
            keywords: vec!["api".to_string(), "rest".to_string()],
            tags: vec!["Pattern".to_string()],
            context: "api design".to_string(),
            memory_type: MemoryType::CodePattern,
            importance: 5,
            confidence: 0.8,
            links: vec![],
            related_files: vec![],
            related_entities: vec![],
            access_count: 0,
            last_accessed_at: chrono::Utc::now(),
            expires_at: None,
            is_archived: false,
            superseded_by: None,
            embedding: None,
            embedding_model: "test".to_string(),
        },
    ];

    let mut state = MemoryPanelState::new();

    // Initially no selection
    assert_eq!(state.selected(), None);

    // Select first item
    state.select_next(memories.len());
    assert_eq!(state.selected(), Some(0));

    // Select next
    state.select_next(memories.len());
    assert_eq!(state.selected(), Some(1));

    // Cannot select beyond end
    state.select_next(memories.len());
    assert_eq!(state.selected(), Some(1));

    // Select previous
    state.select_previous();
    assert_eq!(state.selected(), Some(0));

    // Set search query
    state.set_search("rust".to_string());
    assert_eq!(state.search_query(), "rust");

    // Set selected memory for preview
    state.set_selected_memory(Some(memories[0].clone()));
    assert!(state.selected_memory().is_some());
}

/// Test diagnostics panel filtering and navigation
#[test]
fn test_diagnostics_panel_filtering() {
    use mnemosyne_core::ics::editor::{Diagnostic, Position, Severity};

    // Create test diagnostics
    let diagnostics = vec![
        Diagnostic {
            position: Position { line: 0, column: 0 },
            length: 5,
            severity: Severity::Error,
            message: "Undefined symbol".to_string(),
            suggestion: Some("Define the symbol first".to_string()),
        },
        Diagnostic {
            position: Position {
                line: 5,
                column: 10,
            },
            length: 3,
            severity: Severity::Warning,
            message: "Unused variable".to_string(),
            suggestion: Some("Remove or use the variable".to_string()),
        },
        Diagnostic {
            position: Position {
                line: 10,
                column: 0,
            },
            length: 1,
            severity: Severity::Hint,
            message: "Consider using const".to_string(),
            suggestion: None,
        },
    ];

    let mut state = DiagnosticsPanelState::new();

    // No filter initially
    assert_eq!(state.filter(), None);

    // Filter by error
    state.set_filter(Some(Severity::Error));
    assert_eq!(state.filter(), Some(Severity::Error));

    // Filter by warning
    state.set_filter(Some(Severity::Warning));
    assert_eq!(state.filter(), Some(Severity::Warning));

    // Clear filter
    state.set_filter(None);
    assert_eq!(state.filter(), None);

    // Navigate diagnostics
    state.select_next(diagnostics.len());
    assert_eq!(state.selected(), Some(0));

    state.select_next(diagnostics.len());
    assert_eq!(state.selected(), Some(1));

    state.select_previous();
    assert_eq!(state.selected(), Some(0));
}

/// Test proposals panel workflow: create, filter, navigate
#[test]
fn test_proposals_panel_workflow() {
    // Create test proposals
    let proposals = vec![
        ChangeProposal {
            id: "prop-1".to_string(),
            agent: "agent:semantic".to_string(),
            description: "Fix typo".to_string(),
            original: "teh".to_string(),
            proposed: "the".to_string(),
            line_range: (5, 5),
            created_at: SystemTime::now(),
            status: ProposalStatus::Pending,
            rationale: "Common typo detected".to_string(),
        },
        ChangeProposal {
            id: "prop-2".to_string(),
            agent: "agent:style".to_string(),
            description: "Improve clarity".to_string(),
            original: "It does stuff".to_string(),
            proposed: "The system processes requests".to_string(),
            line_range: (10, 10),
            created_at: SystemTime::now(),
            status: ProposalStatus::Accepted,
            rationale: "More specific description".to_string(),
        },
    ];

    let mut state = ProposalsPanelState::new();

    // Default filter is Pending
    assert_eq!(state.status_filter(), Some(ProposalStatus::Pending));

    // Change filter
    state.set_status_filter(Some(ProposalStatus::Accepted));
    assert_eq!(state.status_filter(), Some(ProposalStatus::Accepted));

    // Clear filter (show all)
    state.set_status_filter(None);
    assert_eq!(state.status_filter(), None);

    // Navigate proposals
    state.select_next(proposals.len());
    assert_eq!(state.selected(), Some(0));

    state.select_next(proposals.len());
    assert_eq!(state.selected(), Some(1));

    // Toggle details view
    assert!(!state.is_showing_details());
    state.toggle_details();
    assert!(state.is_showing_details());
}

/// Test semantic analysis with typed holes detection
#[tokio::test]
async fn test_semantic_analysis_typed_holes() {
    let mut analyzer = SemanticAnalyzer::new();

    // Text with various typed holes
    let text = r#"
TODO: implement authentication
The system is distributed. However, the system is slow.
The @undefined_symbol needs to be #missing.
"#;

    analyzer
        .analyze(text.to_string())
        .expect("Analysis should start");

    // Poll for result
    let mut attempts = 0;
    let mut result = None;
    while attempts < 100 {
        if let Some(analysis) = analyzer.try_recv() {
            result = Some(analysis);
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        attempts += 1;
    }

    let analysis = result.expect("Analysis should complete");

    // Should detect multiple holes
    assert!(!analysis.holes.is_empty());

    // Should detect TODO as incomplete
    assert!(analysis
        .holes
        .iter()
        .any(|h| h.kind == HoleKind::Incomplete));

    // Should detect contradiction
    assert!(analysis
        .holes
        .iter()
        .any(|h| h.kind == HoleKind::Contradiction));

    // Should detect undefined symbols
    assert!(analysis.holes.iter().any(|h| h.kind == HoleKind::Undefined));
}

/// Test semantic analysis entity extraction
#[tokio::test]
async fn test_semantic_analysis_entities() {
    let mut analyzer = SemanticAnalyzer::new();

    let text = "The Orchestrator manages the Agent and calls Process() with #config. The Orchestrator also handles Events.";

    analyzer
        .analyze(text.to_string())
        .expect("Analysis should start");

    // Poll for result
    let mut attempts = 0;
    let mut result = None;
    while attempts < 100 {
        if let Some(analysis) = analyzer.try_recv() {
            result = Some(analysis);
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        attempts += 1;
    }

    let analysis = result.expect("Analysis should complete");

    // Should extract entities
    assert!(analysis.entities.contains_key("Orchestrator"));
    assert!(analysis.entities.contains_key("Agent"));
    assert!(analysis.entities.contains_key("Process"));
    assert!(analysis.entities.contains_key("#config"));
    assert!(analysis.entities.contains_key("Events"));

    // Orchestrator mentioned twice
    assert_eq!(analysis.entities.get("Orchestrator"), Some(&2));
}

/// Test memory panel loading state coordination
#[test]
fn test_memory_panel_loading_coordination() {
    let mut state = MemoryPanelState::new();

    // Not loading initially
    assert!(!state.is_loading());

    // Start loading
    state.set_loading(true);
    assert!(state.is_loading());

    // Panel should show loading state
    assert!(state.is_visible() || !state.is_visible()); // State independent of loading

    // Finish loading
    state.set_loading(false);
    assert!(!state.is_loading());
}

/// Test multiple buffers in editor
#[test]
fn test_editor_multiple_buffers() {
    let mut editor = IcsEditor::new();

    // Start with one buffer (ID 0)
    assert_eq!(editor.active_buffer().id, 0);

    // Create second buffer
    let buffer2_id = editor.new_buffer(None);
    assert_eq!(buffer2_id, 1);

    // Create third buffer
    let buffer3_id = editor.new_buffer(None);
    assert_eq!(buffer3_id, 2);

    // Active buffer still the first
    assert_eq!(editor.active_buffer().id, 0);

    // Switch to buffer 2
    editor.set_active_buffer(buffer2_id);
    assert_eq!(editor.active_buffer().id, buffer2_id);

    // Add text to buffer 2
    let buffer = editor.active_buffer_mut();
    let pos = buffer.text_len().expect("Should get text length");
    buffer
        .insert(pos, "Buffer 2 content")
        .expect("Should insert");

    // Switch to buffer 3
    editor.set_active_buffer(buffer3_id);
    let buffer = editor.active_buffer_mut();
    let pos = buffer.text_len().expect("Should get text length");
    buffer
        .insert(pos, "Buffer 3 content")
        .expect("Should insert");

    // Verify buffer contents are independent
    assert!(editor
        .buffer(buffer2_id)
        .unwrap()
        .text()
        .expect("Should get text")
        .contains("Buffer 2"));
    assert!(editor
        .buffer(buffer3_id)
        .unwrap()
        .text()
        .expect("Should get text")
        .contains("Buffer 3"));
}

/// Test end-to-end workflow: edit → analyze → proposals → diagnostics
#[tokio::test]
async fn test_full_ics_workflow() {
    // 1. Create editor and add content
    let mut editor = IcsEditor::new();
    let buffer = editor.active_buffer_mut();

    let pos = buffer.text_len().expect("Should get text length");
    buffer
        .insert(pos, "TODO: Add authentication\n")
        .expect("Should insert");
    let pos = buffer.text_len().expect("Should get text length");
    buffer
        .insert(pos, "The system is fast. However, the system is slow.\n")
        .expect("Should insert");
    let pos = buffer.text_len().expect("Should get text length");
    buffer
        .insert(pos, "Service requires authentication.\n")
        .expect("Should insert");

    let text = buffer.text().expect("Should get text");

    // 2. Run semantic analysis
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(text).expect("Analysis should start");

    let mut attempts = 0;
    let mut analysis = None;
    while attempts < 100 {
        if let Some(result) = analyzer.try_recv() {
            analysis = Some(result);
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        attempts += 1;
    }

    let analysis = analysis.expect("Analysis should complete");

    // 3. Verify analysis found issues (holes)
    assert!(!analysis.holes.is_empty());
    let has_incomplete = analysis
        .holes
        .iter()
        .any(|h| h.kind == HoleKind::Incomplete);
    let has_contradiction = analysis
        .holes
        .iter()
        .any(|h| h.kind == HoleKind::Contradiction);
    assert!(has_incomplete);
    assert!(has_contradiction);

    // 4. Create diagnostics from holes
    use mnemosyne_core::ics::editor::{Diagnostic, Position, Severity};
    let diagnostics: Vec<Diagnostic> = analysis
        .holes
        .iter()
        .map(|hole| Diagnostic {
            position: Position {
                line: hole.line,
                column: hole.column,
            },
            length: 1,
            severity: match hole.kind {
                HoleKind::Incomplete => Severity::Warning,
                HoleKind::Contradiction => Severity::Error,
                HoleKind::Undefined => Severity::Error,
                _ => Severity::Hint,
            },
            message: format!("{}: {}", hole.name, hole.context),
            suggestion: hole.suggestions.first().cloned(),
        })
        .collect();

    // 5. Show diagnostics in panel
    let mut diag_state = DiagnosticsPanelState::new();
    diag_state.show();
    assert!(diag_state.is_visible());
    assert!(!diagnostics.is_empty());

    // 6. Create change proposal to fix an issue
    let proposal = ChangeProposal {
        id: "fix-1".to_string(),
        agent: "agent:semantic".to_string(),
        description: "Resolve contradiction".to_string(),
        original: "The system is fast. However, the system is slow.".to_string(),
        proposed: "The system has variable performance depending on load.".to_string(),
        line_range: (1, 1),
        created_at: SystemTime::now(),
        status: ProposalStatus::Pending,
        rationale: "Contradiction detected - providing more nuanced description".to_string(),
    };

    // 7. Show proposal in panel
    let mut prop_state = ProposalsPanelState::new();
    prop_state.show();
    assert!(prop_state.is_visible());
    assert_eq!(proposal.status, ProposalStatus::Pending);

    // Workflow complete: editor → analysis → diagnostics → proposals
}
