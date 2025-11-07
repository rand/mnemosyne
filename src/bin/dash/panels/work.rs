//! Work panel - Display work orchestration progress

use crate::time_series::TimeSeriesBuffer;
use crate::widgets::{Sparkline, StateIndicator, StateType};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem},
    Frame,
};
use serde::Deserialize;

/// Work progress metrics from API
#[derive(Debug, Clone, Deserialize, Default)]
pub struct WorkMetrics {
    pub current_phase: String,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub critical_path_progress: f32,
    pub parallel_streams: Vec<String>,
}

/// Work panel widget
pub struct WorkPanel {
    metrics: WorkMetrics,
    title: String,
    critical_path_history: TimeSeriesBuffer<f32>,
    completion_history: TimeSeriesBuffer<f32>,
}

impl WorkPanel {
    /// Create new work panel
    pub fn new() -> Self {
        Self {
            metrics: WorkMetrics::default(),
            title: "Work Progress".to_string(),
            critical_path_history: TimeSeriesBuffer::new(50),
            completion_history: TimeSeriesBuffer::new(50),
        }
    }

    /// Update work metrics
    pub fn update(&mut self, metrics: WorkMetrics) {
        // Collect history before updating metrics
        self.critical_path_history.push(metrics.critical_path_progress);
        let completion_pct = if metrics.total_tasks == 0 {
            0.0
        } else {
            (metrics.completed_tasks as f32 / metrics.total_tasks as f32) * 100.0
        };
        self.completion_history.push(completion_pct);

        self.metrics = metrics;
    }

    /// Set custom title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Calculate completion percentage
    fn completion_percentage(&self) -> f32 {
        if self.metrics.total_tasks == 0 {
            0.0
        } else {
            (self.metrics.completed_tasks as f32 / self.metrics.total_tasks as f32) * 100.0
        }
    }

    /// Get color based on completion status
    fn progress_color(&self) -> ratatui::style::Color {
        let pct = self.completion_percentage();
        if pct < 33.0 {
            ratatui::style::Color::Red
        } else if pct < 66.0 {
            ratatui::style::Color::Yellow
        } else {
            ratatui::style::Color::Green
        }
    }

    /// Render the work panel
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Prepare data for sparklines (must live for entire function)
        let critical_path_data = self.critical_path_history.to_vec();
        let completion_data = self.completion_history.to_vec();

        // Split area: progress bar on top, details below
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(3), // Progress bar
                ratatui::layout::Constraint::Min(0),     // Details
            ])
            .split(area);

        // Progress bar
        let completion = self.completion_percentage();
        let gauge = Gauge::default()
            .block(
                Block::default()
                    .title(format!(
                        "{} - Phase: {}",
                        self.title, self.metrics.current_phase
                    ))
                    .borders(Borders::ALL),
            )
            .gauge_style(Style::default().fg(self.progress_color()))
            .label(format!(
                "{}/{} tasks ({:.0}%)",
                self.metrics.completed_tasks, self.metrics.total_tasks, completion
            ))
            .ratio(((completion / 100.0) as f64).clamp(0.0, 1.0));

        frame.render_widget(gauge, chunks[0]);

        // Details
        let mut items = Vec::new();

        // Overall completion trend sparkline
        if !completion_data.is_empty() {
            let sparkline = Sparkline::new(&completion_data)
                .width(12)
                .style(Style::default().fg(self.progress_color()));

            let mut spans = vec![
                Span::styled(
                    "Completion: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ];
            spans.extend(sparkline.render().spans);

            items.push(ListItem::new(Line::from(spans)));
        }

        // Critical path progress with sparkline
        if self.metrics.critical_path_progress > 0.0 && !critical_path_data.is_empty() {
            let indicator = StateIndicator::new(
                if self.metrics.critical_path_progress >= 100.0 {
                    StateType::WorkCompleted
                } else {
                    StateType::WorkInProgress
                },
                format!(
                    "Critical Path: {:.0}%",
                    self.metrics.critical_path_progress
                ),
            );

            let sparkline = Sparkline::new(&critical_path_data)
                .width(10)
                .style(Style::default().fg(ratatui::style::Color::Cyan));

            let mut spans = vec![indicator.render(), Span::raw("  ")];
            spans.extend(sparkline.render().spans);

            items.push(ListItem::new(Line::from(spans)));
        }

        // Parallel streams
        if !self.metrics.parallel_streams.is_empty() {
            items.push(ListItem::new(Line::from(vec![Span::styled(
                format!("Parallel Streams: {}", self.metrics.parallel_streams.len()),
                Style::default()
                    .fg(ratatui::style::Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )])));

            // Show stream names (up to 3)
            for stream in self.metrics.parallel_streams.iter().take(3) {
                items.push(ListItem::new(Line::from(vec![Span::styled(
                    format!("  • {}", stream),
                    Style::default().fg(ratatui::style::Color::Gray),
                )])));
            }

            if self.metrics.parallel_streams.len() > 3 {
                items.push(ListItem::new(Line::from(vec![Span::styled(
                    format!("  ... and {} more", self.metrics.parallel_streams.len() - 3),
                    Style::default()
                        .fg(ratatui::style::Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                )])));
            }
        }

        // If no details, show idle message
        if items.is_empty() {
            items.push(ListItem::new(Line::from(vec![Span::styled(
                "No active work",
                Style::default()
                    .fg(ratatui::style::Color::Gray)
                    .add_modifier(Modifier::ITALIC),
            )])));
        }

        let list = List::new(items).block(Block::default().borders(Borders::ALL));

        frame.render_widget(list, chunks[1]);
    }

    /// Get current metrics
    pub fn metrics(&self) -> &WorkMetrics {
        &self.metrics
    }
}

impl Default for WorkPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_work_panel_creation() {
        let panel = WorkPanel::new();
        assert_eq!(panel.metrics().total_tasks, 0);
        assert_eq!(panel.metrics().completed_tasks, 0);
    }

    #[test]
    fn test_work_panel_update() {
        let mut panel = WorkPanel::new();

        let metrics = WorkMetrics {
            current_phase: "SpecToFullSpec".to_string(),
            total_tasks: 10,
            completed_tasks: 5,
            critical_path_progress: 50.0,
            parallel_streams: vec!["stream-1".to_string(), "stream-2".to_string()],
        };

        panel.update(metrics);
        assert_eq!(panel.metrics().total_tasks, 10);
        assert_eq!(panel.metrics().completed_tasks, 5);
        assert_eq!(panel.completion_percentage(), 50.0);
    }

    #[test]
    fn test_completion_percentage() {
        let mut panel = WorkPanel::new();

        let metrics = WorkMetrics {
            current_phase: "Test".to_string(),
            total_tasks: 4,
            completed_tasks: 1,
            critical_path_progress: 0.0,
            parallel_streams: vec![],
        };

        panel.update(metrics);
        assert_eq!(panel.completion_percentage(), 25.0);
    }

    #[test]
    fn test_completion_percentage_zero_tasks() {
        let panel = WorkPanel::new();
        assert_eq!(panel.completion_percentage(), 0.0);
    }

    #[test]
    fn test_progress_color() {
        let mut panel = WorkPanel::new();

        // Red zone
        panel.update(WorkMetrics {
            current_phase: "Test".to_string(),
            total_tasks: 10,
            completed_tasks: 2, // 20%
            critical_path_progress: 0.0,
            parallel_streams: vec![],
        });
        assert_eq!(panel.progress_color(), ratatui::style::Color::Red);

        // Yellow zone
        panel.update(WorkMetrics {
            current_phase: "Test".to_string(),
            total_tasks: 10,
            completed_tasks: 5, // 50%
            critical_path_progress: 0.0,
            parallel_streams: vec![],
        });
        assert_eq!(panel.progress_color(), ratatui::style::Color::Yellow);

        // Green zone
        panel.update(WorkMetrics {
            current_phase: "Test".to_string(),
            total_tasks: 10,
            completed_tasks: 8, // 80%
            critical_path_progress: 0.0,
            parallel_streams: vec![],
        });
        assert_eq!(panel.progress_color(), ratatui::style::Color::Green);
    }
}
