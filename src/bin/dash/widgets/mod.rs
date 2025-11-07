//! Reusable dashboard widgets
//!
//! This module provides common UI components for the btop-inspired dashboard:
//! - State indicators (color-coded status badges)
//! - Enhanced progress bars with color zones
//! - Time-series sparklines for inline trend visualization

pub mod progress_bar;
pub mod sparkline;
pub mod state_indicator;

pub use progress_bar::ProgressBar;
pub use sparkline::Sparkline;
pub use state_indicator::{StateIndicator, StateType};
