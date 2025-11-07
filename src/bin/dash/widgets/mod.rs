//! Reusable dashboard widgets
//!
//! This module provides common UI components for the btop-inspired dashboard:
//! - State indicators (color-coded status badges)
//! - Enhanced progress bars with color zones
//! - Time-series sparklines (using ratatui::widgets::Sparkline)

pub mod progress_bar;
pub mod state_indicator;

pub use progress_bar::ProgressBar;
pub use state_indicator::{StateIndicator, StateType};
