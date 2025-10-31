//! Shared TUI infrastructure for Mnemosyne
//!
//! Provides common terminal UI components used by both:
//! - PTY wrapper mode (wrapping Claude Code)
//! - ICS mode (Integrated Context Studio)
//!
//! This module contains:
//! - Terminal setup and management
//! - Event handling system
//! - Layout management
//! - Shared widget components
//! - View components (Chat, Dashboard, ICS Panel)

mod app;
mod events;
mod layout;
mod terminal;
mod views;
mod widgets;

pub use app::TuiApp;
pub use events::{EventHandler, EventLoop, TuiEvent};
pub use layout::{LayoutManager, PanelConfig, Split};
pub use terminal::{TerminalConfig, TerminalManager};
pub use views::{ChatView, Dashboard, IcsPanel};
pub use widgets::{
    Command, CommandCategory, CommandPalette, ConfirmDialog, Dialog, DialogResult, HelpOverlay,
    InputDialog, PreviewDialog, ScrollableList, StatusBar,
};
