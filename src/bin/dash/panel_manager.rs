//! Panel management system - Control panel visibility and layout
//!
//! Provides btop-inspired panel toggling and layout management.

use ratatui::layout::Constraint;
use serde::{Deserialize, Serialize};

/// Panel identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelId {
    Agents,
    Memory,
    Skills,
    Work,
    Context,
    Events,
    Operations,
}

impl PanelId {
    /// Get all panel IDs in display order
    pub fn all() -> Vec<PanelId> {
        vec![
            PanelId::Agents,
            PanelId::Memory,
            PanelId::Skills,
            PanelId::Work,
            PanelId::Context,
            PanelId::Operations,
            PanelId::Events,
        ]
    }

    /// Get keyboard shortcut number (1-7)
    pub fn shortcut_key(&self) -> char {
        match self {
            PanelId::Agents => '1',
            PanelId::Memory => '2',
            PanelId::Skills => '3',
            PanelId::Work => '4',
            PanelId::Context => '5',
            PanelId::Operations => '6',
            PanelId::Events => '7',
        }
    }

    /// Get panel name
    pub fn name(&self) -> &'static str {
        match self {
            PanelId::Agents => "Agents",
            PanelId::Memory => "Memory",
            PanelId::Skills => "Skills",
            PanelId::Work => "Work",
            PanelId::Context => "Context",
            PanelId::Operations => "Operations",
            PanelId::Events => "Events",
        }
    }

    /// Get default height constraint for this panel
    pub fn default_height(&self) -> Constraint {
        match self {
            PanelId::Agents => Constraint::Length(8),
            PanelId::Memory => Constraint::Length(7),
            PanelId::Skills => Constraint::Length(7),
            PanelId::Work => Constraint::Length(8),
            PanelId::Context => Constraint::Length(8),
            PanelId::Operations => Constraint::Length(10),
            PanelId::Events => Constraint::Min(10), // Events gets remaining space
        }
    }

    /// Get minimum height for this panel
    pub fn min_height(&self) -> u16 {
        match self {
            PanelId::Agents => 5,
            PanelId::Memory => 5,
            PanelId::Skills => 5,
            PanelId::Work => 6,
            PanelId::Context => 6,
            PanelId::Operations => 8,
            PanelId::Events => 8,
        }
    }
}

/// Panel visibility configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelVisibility {
    pub agents: bool,
    pub memory: bool,
    pub skills: bool,
    pub work: bool,
    pub context: bool,
    pub operations: bool,
    pub events: bool,
}

impl PanelVisibility {
    /// Create with all panels visible
    pub fn all_visible() -> Self {
        Self {
            agents: true,
            memory: true,
            skills: true,
            work: true,
            context: true,
            operations: true,
            events: true,
        }
    }

    /// Create with no panels visible
    pub fn none_visible() -> Self {
        Self {
            agents: false,
            memory: false,
            skills: false,
            work: false,
            context: false,
            operations: false,
            events: false,
        }
    }

    /// Get visibility for specific panel
    pub fn is_visible(&self, panel: PanelId) -> bool {
        match panel {
            PanelId::Agents => self.agents,
            PanelId::Memory => self.memory,
            PanelId::Skills => self.skills,
            PanelId::Work => self.work,
            PanelId::Context => self.context,
            PanelId::Operations => self.operations,
            PanelId::Events => self.events,
        }
    }

    /// Set visibility for specific panel
    pub fn set_visible(&mut self, panel: PanelId, visible: bool) {
        match panel {
            PanelId::Agents => self.agents = visible,
            PanelId::Memory => self.memory = visible,
            PanelId::Skills => self.skills = visible,
            PanelId::Work => self.work = visible,
            PanelId::Context => self.context = visible,
            PanelId::Operations => self.operations = visible,
            PanelId::Events => self.events = visible,
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

    /// "Minimal" preset - only agents and events
    pub fn preset_minimal() -> Self {
        let mut visibility = PanelVisibility::none_visible();
        visibility.agents = true;
        visibility.events = true;
        Self::new("Minimal", visibility)
    }

    /// "Work Focus" preset - work, agents, events
    pub fn preset_work_focus() -> Self {
        let mut visibility = PanelVisibility::none_visible();
        visibility.agents = true;
        visibility.work = true;
        visibility.events = true;
        Self::new("Work Focus", visibility)
    }

    /// "System Monitor" preset - agents, memory, context
    pub fn preset_system_monitor() -> Self {
        let mut visibility = PanelVisibility::none_visible();
        visibility.agents = true;
        visibility.memory = true;
        visibility.context = true;
        visibility.events = true;
        Self::new("System Monitor", visibility)
    }

    /// "Development" preset - agents, skills, events
    pub fn preset_development() -> Self {
        let mut visibility = PanelVisibility::none_visible();
        visibility.agents = true;
        visibility.skills = true;
        visibility.events = true;
        Self::new("Development", visibility)
    }

    /// Get default presets
    pub fn default_presets() -> Vec<LayoutPreset> {
        vec![
            Self::preset_all(),
            Self::preset_minimal(),
            Self::preset_work_focus(),
            Self::preset_system_monitor(),
            Self::preset_development(),
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
            .map(|p| {
                match p.default_height() {
                    Constraint::Length(h) => h,
                    Constraint::Min(h) => h,
                    _ => 10,
                }
            })
            .sum();

        let header_footer_height = 4; // Header (3) + Footer (1)
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
        assert_eq!(vis.visible_count(), 7);
        assert!(vis.is_visible(PanelId::Agents));
    }

    #[test]
    fn test_panel_visibility_toggle() {
        let mut vis = PanelVisibility::default();
        assert!(vis.is_visible(PanelId::Agents));

        vis.toggle(PanelId::Agents);
        assert!(!vis.is_visible(PanelId::Agents));

        vis.toggle(PanelId::Agents);
        assert!(vis.is_visible(PanelId::Agents));
    }

    #[test]
    fn test_panel_visibility_visible_panels() {
        let mut vis = PanelVisibility::none_visible();
        vis.agents = true;
        vis.memory = true;

        let visible = vis.visible_panels();
        assert_eq!(visible.len(), 2);
        assert!(visible.contains(&PanelId::Agents));
        assert!(visible.contains(&PanelId::Memory));
    }

    #[test]
    fn test_panel_manager_creation() {
        let manager = PanelManager::new();
        assert_eq!(manager.visible_count(), 7);
        assert_eq!(manager.current_preset_name(), Some("All Panels"));
    }

    #[test]
    fn test_panel_manager_toggle() {
        let mut manager = PanelManager::new();
        assert!(manager.is_panel_visible(PanelId::Agents));

        manager.toggle_panel(PanelId::Agents);
        assert!(!manager.is_panel_visible(PanelId::Agents));
        assert_eq!(manager.current_preset_name(), None); // Custom layout
    }

    #[test]
    fn test_panel_manager_presets() {
        let mut manager = PanelManager::new();

        // Apply minimal preset
        manager.apply_preset(1).unwrap();
        assert_eq!(manager.visible_count(), 2); // Only agents and events
        assert!(manager.is_panel_visible(PanelId::Agents));
        assert!(manager.is_panel_visible(PanelId::Events));
        assert!(!manager.is_panel_visible(PanelId::Memory));
    }

    #[test]
    fn test_panel_manager_show_hide_all() {
        let mut manager = PanelManager::new();

        manager.hide_all();
        assert_eq!(manager.visible_count(), 0);

        manager.show_all();
        assert_eq!(manager.visible_count(), 7);
    }

    #[test]
    fn test_layout_constraints_all_visible() {
        let manager = PanelManager::new();
        let constraints = manager.layout_constraints(80);

        assert_eq!(constraints.len(), 7);
    }

    #[test]
    fn test_layout_constraints_limited_space() {
        let manager = PanelManager::new();
        // Very limited space should compress to minimums
        let constraints = manager.layout_constraints(30);

        assert_eq!(constraints.len(), 7);
        // Should use minimum heights or compressed
        for c in constraints {
            match c {
                Constraint::Length(h) | Constraint::Min(h) => {
                    assert!(h >= 5); // At least minimum
                }
                _ => panic!("Unexpected constraint type"),
            }
        }
    }

    #[test]
    fn test_panel_shortcuts() {
        assert_eq!(PanelId::Agents.shortcut_key(), '1');
        assert_eq!(PanelId::Memory.shortcut_key(), '2');
        assert_eq!(PanelId::Skills.shortcut_key(), '3');
        assert_eq!(PanelId::Work.shortcut_key(), '4');
        assert_eq!(PanelId::Context.shortcut_key(), '5');
        assert_eq!(PanelId::Operations.shortcut_key(), '6');
        assert_eq!(PanelId::Events.shortcut_key(), '7');
    }

    #[test]
    fn test_default_presets() {
        let presets = LayoutPreset::default_presets();
        assert_eq!(presets.len(), 5);
        assert_eq!(presets[0].name, "All Panels");
        assert_eq!(presets[1].name, "Minimal");
        assert_eq!(presets[2].name, "Work Focus");
    }
}
