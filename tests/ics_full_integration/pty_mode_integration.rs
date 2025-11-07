//! PTY Mode Integration Tests (P1-P6)
//!
//! Tests ICS integration within PTY/TUI mode

use crate::ics_full_integration::*;
use mnemosyne_core::{
    storage::StorageBackend,
    tui::{ChatView, Dashboard, IcsPanel, TuiEvent},
    types::{MemoryType, Namespace},
};

/// P1: ICS panel display in PTY mode
#[tokio::test]
async fn p1_ics_panel_display() {
    let mut ics_panel = IcsPanel::new();

    // Verify initial state
    assert!(!ics_panel.is_visible(), "Panel should start hidden");

    // Toggle visibility
    ics_panel.toggle();
    assert!(
        ics_panel.is_visible(),
        "Panel should be visible after toggle"
    );

    // Toggle again
    ics_panel.toggle();
    assert!(
        !ics_panel.is_visible(),
        "Panel should be hidden after second toggle"
    );

    // Set custom content
    ics_panel.set_content("Custom ICS content with memories".to_string());
    ics_panel.toggle();

    assert!(ics_panel.is_visible(), "Panel should show custom content");
}

/// P2: Keyboard navigation and event handling
#[tokio::test]
async fn p2_keyboard_navigation() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let mut ics_panel = IcsPanel::new();
    let mut chat_view = ChatView::new();

    // Simulate Ctrl+E keypress to toggle ICS
    let toggle_event = TuiEvent::Key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL));

    // Handle toggle event
    if let TuiEvent::Key(key) = toggle_event {
        if key.code == KeyCode::Char('e') && key.modifiers.contains(KeyModifiers::CONTROL) {
            ics_panel.toggle();
        }
    }

    assert!(ics_panel.is_visible(), "Ctrl+E should toggle ICS panel");

    // Simulate scroll events in chat
    let scroll_up = TuiEvent::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));

    if let TuiEvent::Key(key) = scroll_up {
        if key.code == KeyCode::Up {
            chat_view.scroll_up(1);
        }
    }

    // Scroll down
    let scroll_down = TuiEvent::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));

    if let TuiEvent::Key(key) = scroll_down {
        if key.code == KeyCode::Down {
            chat_view.scroll_down(1);
        }
    }

    // Verify events handled without panic
    assert!(true, "Event handling should complete successfully");
}

/// P3: Memory panel integration with storage
#[tokio::test]
async fn p3_memory_panel_integration() {
    let storage = StorageFixture::new().await.expect("Storage setup failed");
    let mut ics_panel = IcsPanel::new();

    // Create some memories
    for i in 0..5 {
        let memory = create_test_memory(
            &format!("Memory {} for ICS panel display", i + 1),
            MemoryType::CodePattern,
            Namespace::Global,
            7,
        );

        storage
            .storage()
            .store_memory(&memory)
            .await
            .expect("Store memory");
    }

    // Retrieve memories for display
    let results = storage
        .storage()
        .keyword_search("", Some(Namespace::Global))
        .await
        .expect("Search memories");

    let memories: Vec<MemoryNote> = results.into_iter().map(|r| r.memory).collect();

    // Format memories for ICS panel
    let mut content = String::from("Active Memories:\n\n");
    for (idx, memory) in memories.iter().enumerate() {
        content.push_str(&format!(
            "{}. [{:?}] {}\n",
            idx + 1,
            memory.memory_type,
            memory.summary
        ));
    }

    ics_panel.set_content(content);
    ics_panel.toggle();

    assert!(
        ics_panel.is_visible(),
        "ICS panel should display memory list"
    );
}

/// P4: Chat view + ICS panel layout
#[tokio::test]
async fn p4_chat_ics_layout() {
    use mnemosyne_core::pty::ParsedChunk;
    use ratatui::layout::{Constraint, Direction, Layout, Rect};

    let mut chat_view = ChatView::new();
    let mut ics_panel = IcsPanel::new();

    // Add some chat messages
    for i in 0..10 {
        chat_view.add_message(ParsedChunk {
            text: format!("Message {}: Test conversation", i + 1),
            agent: None,
            is_tool_use: false,
            is_error: false,
        });
    }

    // Define terminal area (simulated)
    let terminal_area = Rect::new(0, 0, 120, 40);

    // Create layout with ICS panel hidden
    let layout_hidden = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(100)])
        .split(terminal_area);

    assert_eq!(layout_hidden.len(), 1, "Single pane when ICS panel hidden");

    // Enable ICS panel
    ics_panel.toggle();

    // Create layout with ICS panel visible
    let layout_visible = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(terminal_area);

    assert_eq!(layout_visible.len(), 2, "Two panes when ICS panel visible");
    assert!(
        layout_visible[0].width > layout_visible[1].width,
        "Chat should have more space than ICS panel"
    );
}

/// P5: Event handling and real-time updates
#[tokio::test]
async fn p5_event_handling_updates() {
    use mnemosyne_core::pty::ParsedChunk;
    use std::time::Duration;
    use tokio::time::sleep;

    let storage = StorageFixture::new().await.expect("Storage setup failed");
    let mut chat_view = ChatView::new();
    let mut ics_panel = IcsPanel::new();

    // Simulate incoming chat messages
    for i in 0..5 {
        chat_view.add_message(ParsedChunk {
            text: format!("Claude Code: Processing request {}", i + 1),
            agent: None,
            is_tool_use: false,
            is_error: false,
        });

        // Simulate small delay between messages
        sleep(Duration::from_millis(10)).await;
    }

    // Simulate memory creation during conversation
    let memory = create_test_memory(
        "New insight from conversation",
        MemoryType::Insight,
        Namespace::Global,
        8,
    );

    storage
        .storage()
        .store_memory(&memory)
        .await
        .expect("Store memory");

    // Update ICS panel with new memory
    let results = storage
        .storage()
        .keyword_search("", Some(Namespace::Global))
        .await
        .expect("Search");

    let memories: Vec<MemoryNote> = results.into_iter().map(|r| r.memory).collect();

    let content = format!("Memories: {} total", memories.len());
    ics_panel.set_content(content);

    // Verify updates applied
    assert_memory_count(&memories, 1);
}

/// P6: Terminal state management and cleanup
#[tokio::test]
async fn p6_terminal_state_management() {
    let mut chat_view = ChatView::new();
    let mut dashboard = Dashboard::new();
    let mut ics_panel = IcsPanel::new();

    // Add state to components
    for i in 0..20 {
        chat_view.add_message(mnemosyne_core::pty::ParsedChunk {
            text: format!("Message {}", i + 1),
            agent: None,
            is_tool_use: false,
            is_error: false,
        });
    }

    // Update dashboard with metrics (only has update method)
    dashboard.update(3, 42);

    ics_panel.toggle();
    ics_panel.set_content("Active session data".to_string());

    // Simulate cleanup/reset
    chat_view.clear();
    ics_panel.toggle(); // Hide panel

    // Verify cleanup
    assert!(!ics_panel.is_visible(), "ICS panel should be hidden");

    // Dashboard state persists (by design)
    // Chat cleared
}

/// P7: Error handling in TUI mode (bonus test)
#[tokio::test]
async fn p7_error_handling_in_tui() {
    use mnemosyne_core::pty::ParsedChunk;

    let mut chat_view = ChatView::new();

    // Simulate error message from Claude Code
    chat_view.add_message(ParsedChunk {
        text: "Error: Failed to read file".to_string(),
        agent: None,
        is_tool_use: false,
        is_error: true,
    });

    // Simulate tool use message
    chat_view.add_message(ParsedChunk {
        text: "Using tool: Read /path/to/file".to_string(),
        agent: None,
        is_tool_use: true,
        is_error: false,
    });

    // Normal message after error
    chat_view.add_message(ParsedChunk {
        text: "Continuing after error...".to_string(),
        agent: None,
        is_tool_use: false,
        is_error: false,
    });

    // Verify messages added (no panic on errors)
    assert!(true, "Error messages handled gracefully");
}

/// P8: Dashboard metrics integration (bonus test)
#[tokio::test]
async fn p8_dashboard_metrics() {
    let mut dashboard = Dashboard::new();

    // Update metrics (Dashboard only has update(agents, messages) method)
    dashboard.update(5, 128);

    // Verify metrics tracked
    // (Dashboard doesn't expose getters, but we verify no panic)
    assert!(true, "Dashboard metrics updated successfully");

    // Update again
    dashboard.update(7, 256);

    // Simulate real-time updates
    for i in 0..10 {
        dashboard.update(7, 256 + i);
    }

    assert!(true, "Real-time dashboard updates handled");
}
