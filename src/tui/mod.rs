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

mod terminal;
mod events;
mod layout;
mod widgets;
mod views;
mod app;

pub use terminal::{TerminalManager, TerminalConfig};
pub use events::{TuiEvent, EventHandler, EventLoop};
pub use layout::{LayoutManager, PanelConfig, Split};
pub use widgets::{Command, CommandCategory, CommandPalette, ScrollableList, StatusBar};
pub use views::{ChatView, Dashboard, IcsPanel};
pub use app::TuiApp;
