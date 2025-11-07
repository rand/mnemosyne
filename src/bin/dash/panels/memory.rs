//! Memory panel - Display memory operations and evolution metrics

use crate::time_series::TimeSeriesBuffer;
use crate::widgets::{Sparkline, StateIndicator, StateType};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
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
    stores_history: TimeSeriesBuffer<f32>,
    recalls_history: TimeSeriesBuffer<f32>,
}

impl MemoryPanel {
    /// Create new memory panel
    pub fn new() -> Self {
        Self {
            metrics: MemoryOpsMetrics::default(),
            title: "Memory Operations".to_string(),
            stores_history: TimeSeriesBuffer::new(50),
            recalls_history: TimeSeriesBuffer::new(50),
        }
    }

    /// Update memory metrics
    pub fn update(&mut self, metrics: MemoryOpsMetrics) {
        // Push new values to history buffers
        self.stores_history.push(metrics.stores_per_minute);
        self.recalls_history.push(metrics.recalls_per_minute);
        self.metrics = metrics;
    }

    /// Set custom title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Render the memory panel
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Prepare data for sparklines (must live for entire function)
        let stores_data = self.stores_history.to_vec();
        let recalls_data = self.recalls_history.to_vec();

        let items = vec![
            // Stores per minute with sparkline
            {
                let indicator = StateIndicator::new(
                    StateType::MemoryStore,
                    format!("Stores: {:.1}/min", self.metrics.stores_per_minute),
                );
                let sparkline = Sparkline::new(&stores_data)
                    .width(10)
                    .style(Style::default().fg(Color::Green));

                let mut spans = vec![indicator.render(), Span::raw("  ")];
                spans.extend(sparkline.render().spans);

                ListItem::new(Line::from(spans))
            },
            // Recalls per minute with sparkline
            {
                let indicator = StateIndicator::new(
                    StateType::MemoryRecall,
                    format!("Recalls: {:.1}/min", self.metrics.recalls_per_minute),
                );
                let sparkline = Sparkline::new(&recalls_data)
                    .width(10)
                    .style(Style::default().fg(Color::Cyan));

                let mut spans = vec![indicator.render(), Span::raw("  ")];
                spans.extend(sparkline.render().spans);

                ListItem::new(Line::from(spans))
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
