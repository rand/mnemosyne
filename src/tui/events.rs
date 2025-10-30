//! Event handling system

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use std::time::Duration;

/// TUI events
#[derive(Debug, Clone)]
pub enum TuiEvent {
    /// Key press event
    Key(KeyEvent),

    /// Mouse event
    Mouse(MouseEvent),

    /// Window resize event
    Resize(u16, u16),

    /// Tick event (periodic update)
    Tick,

    /// Quit event
    Quit,
}

/// Event handler trait
pub trait EventHandler {
    /// Handle TUI event
    fn handle_event(&mut self, event: TuiEvent) -> Result<bool>;
}

/// Event loop for TUI applications
pub struct EventLoop {
    /// Tick rate in milliseconds
    tick_rate: u64,
}

impl EventLoop {
    /// Create new event loop
    pub fn new(tick_rate: u64) -> Self {
        Self { tick_rate }
    }

    /// Poll for next event
    pub fn poll_event(&self) -> Result<Option<TuiEvent>> {
        // Check for crossterm events
        if event::poll(Duration::from_millis(self.tick_rate))? {
            match event::read()? {
                Event::Key(key) => {
                    // Check for quit shortcuts
                    if Self::is_quit_key(&key) {
                        return Ok(Some(TuiEvent::Quit));
                    }
                    return Ok(Some(TuiEvent::Key(key)));
                }
                Event::Mouse(mouse) => {
                    return Ok(Some(TuiEvent::Mouse(mouse)));
                }
                Event::Resize(w, h) => {
                    return Ok(Some(TuiEvent::Resize(w, h)));
                }
                _ => {}
            }
        }

        Ok(Some(TuiEvent::Tick))
    }

    /// Check if key event is a quit shortcut
    fn is_quit_key(key: &KeyEvent) -> bool {
        // Ctrl+C or Ctrl+Q
        matches!(
            (key.code, key.modifiers),
            (KeyCode::Char('c'), KeyModifiers::CONTROL)
                | (KeyCode::Char('q'), KeyModifiers::CONTROL)
        )
    }
}

impl Default for EventLoop {
    fn default() -> Self {
        Self::new(100) // 100ms tick rate
    }
}
