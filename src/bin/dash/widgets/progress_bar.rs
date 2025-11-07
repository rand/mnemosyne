//! Enhanced progress bar widget with color zones and labels

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Gauge},
    Frame,
};

/// Progress bar with color zones (btop-inspired)
pub struct ProgressBar {
    progress: f64,
    label: Option<String>,
    show_percentage: bool,
}

impl ProgressBar {
    /// Create new progress bar (progress: 0.0-100.0)
    pub fn new(progress: f64) -> Self {
        Self {
            progress: progress.clamp(0.0, 100.0),
            label: None,
            show_percentage: true,
        }
    }

    /// Set custom label
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set whether to show percentage
    pub fn show_percentage(mut self, show: bool) -> Self {
        self.show_percentage = show;
        self
    }

    /// Get color based on progress (green → yellow → red zones)
    fn color(&self) -> Color {
        if self.progress < 60.0 {
            Color::Green // Safe zone
        } else if self.progress < 75.0 {
            Color::Yellow // Moderate zone
        } else if self.progress < 90.0 {
            Color::LightRed // High zone
        } else {
            Color::Red // Critical zone
        }
    }

    /// Render the progress bar
    pub fn render(&self, frame: &mut Frame, area: Rect, block: Block) {
        let label = if self.show_percentage {
            if let Some(custom_label) = &self.label {
                format!("{} {:.1}%", custom_label, self.progress)
            } else {
                format!("{:.1}%", self.progress)
            }
        } else {
            self.label.clone().unwrap_or_default()
        };

        let gauge = Gauge::default()
            .block(block)
            .gauge_style(Style::default().fg(self.color()))
            .label(label)
            .ratio(self.progress / 100.0);

        frame.render_widget(gauge, area);
    }

    /// Get current progress value
    pub fn progress(&self) -> f64 {
        self.progress
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_bar_creation() {
        let bar = ProgressBar::new(50.0);
        assert_eq!(bar.progress(), 50.0);
    }

    #[test]
    fn test_progress_clamping() {
        let bar = ProgressBar::new(150.0);
        assert_eq!(bar.progress(), 100.0);

        let bar = ProgressBar::new(-10.0);
        assert_eq!(bar.progress(), 0.0);
    }

    #[test]
    fn test_color_zones() {
        assert_eq!(ProgressBar::new(30.0).color(), Color::Green);
        assert_eq!(ProgressBar::new(65.0).color(), Color::Yellow);
        assert_eq!(ProgressBar::new(80.0).color(), Color::LightRed);
        assert_eq!(ProgressBar::new(95.0).color(), Color::Red);
    }

    #[test]
    fn test_custom_label() {
        let bar = ProgressBar::new(75.0).label("Context");
        assert_eq!(bar.label, Some("Context".to_string()));
    }
}
