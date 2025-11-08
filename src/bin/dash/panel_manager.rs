//! Panel management system - Control panel visibility and layout
//!
//! Provides btop-inspired panel toggling and layout management for the redesigned
//! 4-panel dashboard focused on real-time monitoring during active development.

use ratatui::layout::Constraint;
use serde::{Deserialize, Serialize};

/// Panel identifiers for the redesigned 4-panel layout
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelId {
    /// System Overview: At-a-glance health summary (top, 6-8 lines)
    SystemOverview,
    /// Activity Stream: Intelligent event log with filtering (left, 60%)
    ActivityStream,
    /// Agent Details: Deep-dive into agent activity (right-top, 40%)
    AgentDetails,
    /// Operations: CLI command history and stats (right-bottom, 40%)
    Operations,
}

impl PanelId {
    /// Get all panel IDs in display order
    pub fn all() -> Vec<PanelId> {
        vec![
            PanelId::SystemOverview,
            PanelId::ActivityStream,
            PanelId::AgentDetails,
            PanelId::Operations,
        ]
    }

    /// Get keyboard shortcut number (0-3)
    pub fn shortcut_key(&self) -> char {
        match self {
            PanelId::SystemOverview => '0',
            PanelId::ActivityStream => '1',
            PanelId::AgentDetails => '2',
            PanelId::Operations => '3',
        }
    }

    /// Get panel name
    pub fn name(&self) -> &'static str {
        match self {
            PanelId::SystemOverview => "System Overview",
            PanelId::ActivityStream => "Activity Stream",
            PanelId::AgentDetails => "Agent Details",
            PanelId::Operations => "Operations",
        }
    }

    /// Get default height constraint for this panel
    pub fn default_height(&self) -> Constraint {
        match self {
            PanelId::SystemOverview => Constraint::Length(8), // Fixed top panel
            PanelId::ActivityStream => Constraint::Percentage(60), // 60% of remaining
            PanelId::AgentDetails => Constraint::Percentage(50), // 50% of right column
            PanelId::Operations => Constraint::Min(10), // Remaining space
        }
    }

    /// Get minimum height for this panel
    pub fn min_height(&self) -> u16 {
        match self {
            PanelId::SystemOverview => 6,
            PanelId::ActivityStream => 10,
            PanelId::AgentDetails => 8,
            PanelId::Operations => 8,
        }
    }
}

/// Panel visibility configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelVisibility {
    pub system_overview: bool,
    pub activity_stream: bool,
    pub agent_details: bool,
    pub operations: bool,
}

impl PanelVisibility {
    /// Create with all panels visible
    pub fn all_visible() -> Self {
        Self {
            system_overview: true,
            activity_stream: true,
            agent_details: true,
            operations: true,
        }
    }

    /// Create with no panels visible
    pub fn none_visible() -> Self {
        Self {
            system_overview: false,
            activity_stream: false,
            agent_details: false,
            operations: false,
        }
    }

    /// Get visibility for specific panel
    pub fn is_visible(&self, panel: PanelId) -> bool {
        match panel {
            PanelId::SystemOverview => self.system_overview,
            PanelId::ActivityStream => self.activity_stream,
            PanelId::AgentDetails => self.agent_details,
            PanelId::Operations => self.operations,
        }
    }

    /// Set visibility for specific panel
    pub fn set_visible(&mut self, panel: PanelId, visible: bool) {
        match panel {
            PanelId::SystemOverview => self.system_overview = visible,
            PanelId::ActivityStream => self.activity_stream = visible,
            PanelId::AgentDetails => self.agent_details = visible,
            PanelId::Operations => self.operations = visible,
        }
    }

    /// Toggle visibility for specific panel
    pub fn toggle(&mut self, panel: PanelId) {
        let current = self.is_visible(panel);
        self.set_visible(panel, !current);
    }

    /// Count visible panels
    pub fn visible_count(&self) -> usize {
        PanelId::all()
            .iter()
            .filter(|p| self.is_visible(**p))
            .count()
    }

    /// Get list of visible panels
    pub fn visible_panels(&self) -> Vec<PanelId> {
        PanelId::all()
            .into_iter()
            .filter(|p| self.is_visible(*p))
            .collect()
    }
}

impl Default for PanelVisibility {
    fn default() -> Self {
        Self::all_visible()
    }
}

/// Layout preset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutPreset {
    pub name: String,
    pub visibility: PanelVisibility,
}

impl LayoutPreset {
    /// Create new preset
    pub fn new(name: impl Into<String>, visibility: PanelVisibility) -> Self {
        Self {
            name: name.into(),
            visibility,
        }
    }

    /// Default "All" preset
    pub fn preset_all() -> Self {
        Self::new("All Panels", PanelVisibility::all_visible())
    }

    /// "Activity Focus" preset - overview + activity stream
    pub fn preset_activity_focus() -> Self {
        let mut visibility = PanelVisibility::none_visible();
        visibility.system_overview = true;
        visibility.activity_stream = true;
        Self::new("Activity Focus", visibility)
    }

    /// "Agent Monitor" preset - overview + agent details
    pub fn preset_agent_monitor() -> Self {
        let mut visibility = PanelVisibility::none_visible();
        visibility.system_overview = true;
        visibility.agent_details = true;
        visibility.operations = true;
        Self::new("Agent Monitor", visibility)
    }

    /// "Minimal" preset - only activity stream
    pub fn preset_minimal() -> Self {
        let mut visibility = PanelVisibility::none_visible();
        visibility.activity_stream = true;
        Self::new("Minimal", visibility)
    }

    /// Get default presets
    pub fn default_presets() -> Vec<LayoutPreset> {
        vec![
            Self::preset_all(),
            Self::preset_activity_focus(),
            Self::preset_agent_monitor(),
            Self::preset_minimal(),
        ]
    }
}

/// Panel manager - Controls visibility and layout
pub struct PanelManager {
    visibility: PanelVisibility,
    presets: Vec<LayoutPreset>,
    current_preset_index: Option<usize>,
}

impl PanelManager {
    /// Create new panel manager with default visibility
    pub fn new() -> Self {
        Self {
            visibility: PanelVisibility::default(),
            presets: LayoutPreset::default_presets(),
            current_preset_index: Some(0), // Start with "All" preset
        }
    }

    /// Get current visibility configuration
    pub fn visibility(&self) -> &PanelVisibility {
        &self.visibility
    }

    /// Toggle panel visibility
    pub fn toggle_panel(&mut self, panel: PanelId) {
        self.visibility.toggle(panel);
        self.current_preset_index = None; // Custom layout
    }

    /// Set panel visibility
    pub fn set_panel_visible(&mut self, panel: PanelId, visible: bool) {
        self.visibility.set_visible(panel, visible);
        self.current_preset_index = None;
    }

    /// Check if panel is visible
    pub fn is_panel_visible(&self, panel: PanelId) -> bool {
        self.visibility.is_visible(panel)
    }

    /// Show all panels
    pub fn show_all(&mut self) {
        self.visibility = PanelVisibility::all_visible();
        self.current_preset_index = Some(0);
    }

    /// Hide all panels
    pub fn hide_all(&mut self) {
        self.visibility = PanelVisibility::none_visible();
        self.current_preset_index = None;
    }

    /// Apply preset by index
    pub fn apply_preset(&mut self, index: usize) -> Result<(), String> {
        if index >= self.presets.len() {
            return Err(format!("Preset index {} out of bounds", index));
        }
        self.visibility = self.presets[index].visibility.clone();
        self.current_preset_index = Some(index);
        Ok(())
    }

    /// Get current preset name (if any)
    pub fn current_preset_name(&self) -> Option<&str> {
        self.current_preset_index
            .and_then(|idx| self.presets.get(idx))
            .map(|p| p.name.as_str())
    }

    /// Get all presets
    pub fn presets(&self) -> &[LayoutPreset] {
        &self.presets
    }

    /// Get layout constraints for visible panels
    pub fn layout_constraints(&self, available_height: u16) -> Vec<Constraint> {
        let visible = self.visibility.visible_panels();

        if visible.is_empty() {
            return vec![Constraint::Min(0)];
        }

        // Calculate if we need to compress
        let total_default_height: u16 = visible
            .iter()
            .map(|p| match p.default_height() {
                Constraint::Length(h) => h,
                Constraint::Min(h) => h,
                Constraint::Percentage(p) => (available_height * p) / 100,
                _ => 10,
            })
            .sum();

        let header_footer_height = 2; // Header (1) + Footer (1)
        let usable_height = available_height.saturating_sub(header_footer_height);

        if total_default_height <= usable_height {
            // Enough space - use default heights
            visible.iter().map(|p| p.default_height()).collect()
        } else {
            // Need to compress - use minimum heights and distribute remaining
            let min_heights: Vec<u16> = visible.iter().map(|p| p.min_height()).collect();
            let total_min: u16 = min_heights.iter().sum();

            if total_min >= usable_height {
                // Even minimums don't fit - just use minimums
                min_heights.into_iter().map(Constraint::Length).collect()
            } else {
                // Distribute remaining space proportionally
                let remaining = usable_height - total_min;
                let extra_per_panel = remaining / (visible.len() as u16);

                min_heights
                    .into_iter()
                    .enumerate()
                    .map(|(i, min_h)| {
                        if i == visible.len() - 1 {
                            // Last panel gets remaining space
                            Constraint::Min(min_h)
                        } else {
                            Constraint::Length(min_h + extra_per_panel)
                        }
                    })
                    .collect()
            }
        }
    }

    /// Get visible panel count
    pub fn visible_count(&self) -> usize {
        self.visibility.visible_count()
    }
}

impl Default for PanelManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_visibility_default() {
        let vis = PanelVisibility::default();
        assert_eq!(vis.visible_count(), 4);
        assert!(vis.is_visible(PanelId::SystemOverview));
    }

    #[test]
    fn test_panel_visibility_toggle() {
        let mut vis = PanelVisibility::default();
        assert!(vis.is_visible(PanelId::SystemOverview));

        vis.toggle(PanelId::SystemOverview);
        assert!(!vis.is_visible(PanelId::SystemOverview));

        vis.toggle(PanelId::SystemOverview);
        assert!(vis.is_visible(PanelId::SystemOverview));
    }

    #[test]
    fn test_panel_visibility_visible_panels() {
        let mut vis = PanelVisibility::none_visible();
        vis.system_overview = true;
        vis.activity_stream = true;

        let visible = vis.visible_panels();
        assert_eq!(visible.len(), 2);
        assert!(visible.contains(&PanelId::SystemOverview));
        assert!(visible.contains(&PanelId::ActivityStream));
    }

    #[test]
    fn test_panel_manager_creation() {
        let manager = PanelManager::new();
        assert_eq!(manager.visible_count(), 4);
        assert_eq!(manager.current_preset_name(), Some("All Panels"));
    }

    #[test]
    fn test_panel_manager_toggle() {
        let mut manager = PanelManager::new();
        assert!(manager.is_panel_visible(PanelId::SystemOverview));

        manager.toggle_panel(PanelId::SystemOverview);
        assert!(!manager.is_panel_visible(PanelId::SystemOverview));
        assert_eq!(manager.current_preset_name(), None); // Custom layout
    }

    #[test]
    fn test_panel_manager_presets() {
        let mut manager = PanelManager::new();

        // Apply activity focus preset
        manager.apply_preset(1).unwrap();
        assert_eq!(manager.visible_count(), 2); // Overview + activity stream
        assert!(manager.is_panel_visible(PanelId::SystemOverview));
        assert!(manager.is_panel_visible(PanelId::ActivityStream));
        assert!(!manager.is_panel_visible(PanelId::AgentDetails));
    }

    #[test]
    fn test_panel_manager_show_hide_all() {
        let mut manager = PanelManager::new();

        manager.hide_all();
        assert_eq!(manager.visible_count(), 0);

        manager.show_all();
        assert_eq!(manager.visible_count(), 4);
    }

    #[test]
    fn test_layout_constraints_all_visible() {
        let manager = PanelManager::new();
        let constraints = manager.layout_constraints(80);

        assert_eq!(constraints.len(), 4);
    }

    #[test]
    fn test_layout_constraints_limited_space() {
        let manager = PanelManager::new();
        // Very limited space should compress to minimums
        let constraints = manager.layout_constraints(30);

        assert_eq!(constraints.len(), 4);
        // Should use minimum heights or compressed
        for c in constraints {
            match c {
                Constraint::Length(h) | Constraint::Min(h) => {
                    assert!(h >= 6); // At least minimum
                }
                Constraint::Percentage(_) => {
                    // Percentages are okay in default layout
                }
                _ => panic!("Unexpected constraint type"),
            }
        }
    }

    #[test]
    fn test_panel_shortcuts() {
        assert_eq!(PanelId::SystemOverview.shortcut_key(), '0');
        assert_eq!(PanelId::ActivityStream.shortcut_key(), '1');
        assert_eq!(PanelId::AgentDetails.shortcut_key(), '2');
        assert_eq!(PanelId::Operations.shortcut_key(), '3');
    }

    #[test]
    fn test_default_presets() {
        let presets = LayoutPreset::default_presets();
        assert_eq!(presets.len(), 4);
        assert_eq!(presets[0].name, "All Panels");
        assert_eq!(presets[1].name, "Activity Focus");
        assert_eq!(presets[2].name, "Agent Monitor");
        assert_eq!(presets[3].name, "Minimal");
    }
}
