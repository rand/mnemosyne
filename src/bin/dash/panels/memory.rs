//! Memory panel - Display memory operations and evolution metrics

use crate::widgets::{StateIndicator, StateType};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};
use serde::Deserialize;

/// Memory operations metrics from API
#[derive(Debug, Clone, Deserialize, Default)]
pub struct MemoryOpsMetrics {
    pub stores_per_minute: f32,
    pub recalls_per_minute: f32,
    pub evolutions_total: usize,
    pub consolidations_total: usize,
    pub graph_nodes: usize,
}

/// Memory panel widget
pub struct MemoryPanel {
    metrics: MemoryOpsMetrics,
    title: String,
}

impl MemoryPanel {
    /// Create new memory panel
    pub fn new() -> Self {
        Self {
            metrics: MemoryOpsMetrics::default(),
            title: "Memory Operations".to_string(),
        }
    }

    /// Update memory metrics
    pub fn update(&mut self, metrics: MemoryOpsMetrics) {
        self.metrics = metrics;
    }

    /// Set custom title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Render the memory panel
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let items = vec![
            // Stores per minute
            {
                let indicator = StateIndicator::new(
                    StateType::MemoryStore,
                    format!("Stores: {:.1}/min", self.metrics.stores_per_minute),
                );
                ListItem::new(Line::from(vec![indicator.render()]))
            },
            // Recalls per minute
            {
                let indicator = StateIndicator::new(
                    StateType::MemoryRecall,
                    format!("Recalls: {:.1}/min", self.metrics.recalls_per_minute),
                );
                ListItem::new(Line::from(vec![indicator.render()]))
            },
            // Evolution events
            {
                let indicator = StateIndicator::new(
                    StateType::MemoryEvolution,
                    format!("Evolutions: {}", self.metrics.evolutions_total),
                );
                ListItem::new(Line::from(vec![indicator.render()]))
            },
            // Consolidations
            {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("  Consolidations: {}", self.metrics.consolidations_total),
                        Style::default().fg(ratatui::style::Color::Cyan),
                    ),
                ]))
            },
            // Graph size
            {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("  Graph Nodes: {}", self.metrics.graph_nodes),
                        Style::default()
                            .fg(ratatui::style::Color::Blue)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]))
            },
        ];

        let list = List::new(items).block(Block::default().title(self.title.as_str()).borders(Borders::ALL));

        frame.render_widget(list, area);
    }

    /// Get current metrics
    pub fn metrics(&self) -> &MemoryOpsMetrics {
        &self.metrics
    }
}

impl Default for MemoryPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_panel_creation() {
        let panel = MemoryPanel::new();
        assert_eq!(panel.metrics().stores_per_minute, 0.0);
        assert_eq!(panel.metrics().recalls_per_minute, 0.0);
    }

    #[test]
    fn test_memory_panel_update() {
        let mut panel = MemoryPanel::new();

        let metrics = MemoryOpsMetrics {
            stores_per_minute: 5.5,
            recalls_per_minute: 10.2,
            evolutions_total: 3,
            consolidations_total: 2,
            graph_nodes: 1000,
        };

        panel.update(metrics.clone());
        assert_eq!(panel.metrics().stores_per_minute, 5.5);
        assert_eq!(panel.metrics().graph_nodes, 1000);
    }

    #[test]
    fn test_custom_title() {
        let panel = MemoryPanel::new().title("Custom Memory");
        assert_eq!(panel.title, "Custom Memory");
    }
}
