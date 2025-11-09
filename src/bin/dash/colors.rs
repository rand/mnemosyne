//! Consistent color palette for the dashboard
//!
//! This module defines a consistent, accessible color scheme inspired by btop.
//! All panels should use these constants for visual consistency.

use ratatui::style::Color;

/// Color palette for dashboard elements
pub struct DashboardColors;

impl DashboardColors {
    // === Status Colors ===

    /// Success, healthy, active state (Green)
    pub const SUCCESS: Color = Color::Green;

    /// Warning, degraded, waiting state (Yellow)
    pub const WARNING: Color = Color::Yellow;

    /// Error, failed, critical state (Red)
    pub const ERROR: Color = Color::Red;

    /// In-progress, active operations (Blue)
    pub const IN_PROGRESS: Color = Color::Blue;

    /// Idle, neutral, system information (Gray)
    pub const IDLE: Color = Color::Gray;

    // === Component Colors ===

    /// Memory operations (Magenta)
    pub const MEMORY: Color = Color::Magenta;

    /// Skills and capabilities (Cyan)
    pub const SKILL: Color = Color::Cyan;

    /// CLI operations (Blue)
    pub const CLI: Color = Color::Blue;

    /// Agent activity (Yellow when active)
    pub const AGENT: Color = Color::Yellow;

    /// Work items (Green when in progress)
    pub const WORK: Color = Color::Green;

    /// Context operations (LightBlue)
    pub const CONTEXT: Color = Color::LightBlue;

    // === UI Elements ===

    /// Panel borders (Cyan)
    pub const BORDER: Color = Color::Cyan;

    /// Headers and labels (Yellow)
    pub const HEADER: Color = Color::Yellow;

    /// Secondary text (DarkGray)
    pub const SECONDARY: Color = Color::DarkGray;

    /// Disabled/muted elements (DarkGray)
    pub const MUTED: Color = Color::DarkGray;

    /// Primary text (White)
    pub const TEXT: Color = Color::White;

    /// Highlighted text (Cyan)
    pub const HIGHLIGHT: Color = Color::Cyan;

    // === Performance Colors ===

    /// Fast operations (< 500ms) (Green)
    pub const PERF_FAST: Color = Color::Green;

    /// Medium operations (500-2000ms) (Yellow)
    pub const PERF_MEDIUM: Color = Color::Yellow;

    /// Slow operations (> 2000ms) (Red)
    pub const PERF_SLOW: Color = Color::Red;

    // === Resource Usage Colors ===

    /// Low usage (< 50%) (Green)
    pub const USAGE_LOW: Color = Color::Green;

    /// Medium usage (50-80%) (Yellow)
    pub const USAGE_MEDIUM: Color = Color::Yellow;

    /// High usage (> 80%) (Red)
    pub const USAGE_HIGH: Color = Color::Red;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_colors() {
        assert_eq!(DashboardColors::SUCCESS, Color::Green);
        assert_eq!(DashboardColors::WARNING, Color::Yellow);
        assert_eq!(DashboardColors::ERROR, Color::Red);
        assert_eq!(DashboardColors::IN_PROGRESS, Color::Blue);
        assert_eq!(DashboardColors::IDLE, Color::Gray);
    }

    #[test]
    fn test_component_colors() {
        assert_eq!(DashboardColors::MEMORY, Color::Magenta);
        assert_eq!(DashboardColors::SKILL, Color::Cyan);
        assert_eq!(DashboardColors::CLI, Color::Blue);
        assert_eq!(DashboardColors::AGENT, Color::Yellow);
        assert_eq!(DashboardColors::WORK, Color::Green);
        assert_eq!(DashboardColors::CONTEXT, Color::LightBlue);
    }
}
