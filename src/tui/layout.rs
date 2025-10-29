//! Layout management for TUI applications

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Split direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Split {
    /// Horizontal split (left/right)
    Horizontal,
    /// Vertical split (top/bottom)
    Vertical,
}

/// Panel configuration
#[derive(Debug, Clone)]
pub struct PanelConfig {
    /// Panel identifier
    pub id: String,

    /// Panel title
    pub title: String,

    /// Whether panel is visible
    pub visible: bool,

    /// Panel size constraint
    pub size: Constraint,
}

/// Layout manager for organizing panels
pub struct LayoutManager {
    /// Root layout area
    area: Rect,

    /// Panels in the layout
    panels: Vec<PanelConfig>,
}

impl LayoutManager {
    /// Create new layout manager
    pub fn new(area: Rect) -> Self {
        Self {
            area,
            panels: Vec::new(),
        }
    }

    /// Update layout area
    pub fn set_area(&mut self, area: Rect) {
        self.area = area;
    }

    /// Add panel to layout
    pub fn add_panel(&mut self, panel: PanelConfig) {
        self.panels.push(panel);
    }

    /// Get panel by ID
    pub fn panel(&self, id: &str) -> Option<&PanelConfig> {
        self.panels.iter().find(|p| p.id == id)
    }

    /// Toggle panel visibility
    pub fn toggle_panel(&mut self, id: &str) {
        if let Some(panel) = self.panels.iter_mut().find(|p| p.id == id) {
            panel.visible = !panel.visible;
        }
    }

    /// Split area horizontally
    pub fn split_horizontal(&self, constraints: &[Constraint]) -> Vec<Rect> {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(self.area)
            .to_vec()
    }

    /// Split area vertically
    pub fn split_vertical(&self, constraints: &[Constraint]) -> Vec<Rect> {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(self.area)
            .to_vec()
    }

    /// Split area with direction
    pub fn split(&self, direction: Split, constraints: &[Constraint]) -> Vec<Rect> {
        match direction {
            Split::Horizontal => self.split_horizontal(constraints),
            Split::Vertical => self.split_vertical(constraints),
        }
    }

    /// Get visible panels
    pub fn visible_panels(&self) -> impl Iterator<Item = &PanelConfig> {
        self.panels.iter().filter(|p| p.visible)
    }

    /// Get visible panel count
    pub fn visible_count(&self) -> usize {
        self.panels.iter().filter(|p| p.visible).count()
    }
}
