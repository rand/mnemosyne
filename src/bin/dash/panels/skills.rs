//! Skills panel - Display loaded skills and usage statistics

use crate::widgets::{StateIndicator, StateType};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};
use serde::Deserialize;
use std::collections::HashMap;

/// Skills usage metrics from API
#[derive(Debug, Clone, Deserialize, Default)]
pub struct SkillsMetrics {
    pub loaded_skills: Vec<String>,
    pub usage_counts: HashMap<String, usize>,
}

/// Skills panel widget
pub struct SkillsPanel {
    metrics: SkillsMetrics,
    title: String,
    max_display: usize,
}

impl SkillsPanel {
    /// Create new skills panel
    pub fn new() -> Self {
        Self {
            metrics: SkillsMetrics::default(),
            title: "Skills".to_string(),
            max_display: 10,
        }
    }

    /// Update skills metrics
    pub fn update(&mut self, metrics: SkillsMetrics) {
        self.metrics = metrics;
    }

    /// Set custom title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set maximum skills to display
    pub fn max_display(mut self, max: usize) -> Self {
        self.max_display = max;
        self
    }

    /// Get top N skills by usage count
    fn top_skills(&self) -> Vec<(&String, usize)> {
        let mut skills: Vec<_> = self
            .metrics
            .usage_counts
            .iter()
            .map(|(name, count)| (name, *count))
            .collect();

        // Sort by usage count (descending)
        skills.sort_by(|a, b| b.1.cmp(&a.1));

        skills.into_iter().take(self.max_display).collect()
    }

    /// Render the skills panel
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = if self.metrics.loaded_skills.is_empty() {
            vec![ListItem::new(Line::from(vec![Span::styled(
                "No skills loaded",
                Style::default()
                    .fg(ratatui::style::Color::Gray)
                    .add_modifier(Modifier::ITALIC),
            )]))]
        } else {
            // Show top skills by usage
            let top_skills = self.top_skills();

            if top_skills.is_empty() {
                // No usage data yet, just show loaded count
                vec![ListItem::new(Line::from(vec![
                    StateIndicator::new(
                        StateType::SkillLoaded,
                        format!("{} skills loaded", self.metrics.loaded_skills.len()),
                    )
                    .render(),
                ]))]
            } else {
                top_skills
                    .into_iter()
                    .map(|(skill_name, count)| {
                        let indicator = StateIndicator::new(
                            StateType::SkillUsed,
                            format!("{:20} × {}", skill_name, count),
                        );
                        ListItem::new(Line::from(vec![indicator.render()]))
                    })
                    .collect()
            }
        };

        let title = format!(
            "{} ({} loaded)",
            self.title,
            self.metrics.loaded_skills.len()
        );

        let list = List::new(items).block(Block::default().title(title).borders(Borders::ALL));

        frame.render_widget(list, area);
    }

    /// Get number of loaded skills
    pub fn loaded_count(&self) -> usize {
        self.metrics.loaded_skills.len()
    }

    /// Get total usage count across all skills
    pub fn total_usage(&self) -> usize {
        self.metrics.usage_counts.values().sum()
    }
}

impl Default for SkillsPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skills_panel_creation() {
        let panel = SkillsPanel::new();
        assert_eq!(panel.loaded_count(), 0);
        assert_eq!(panel.total_usage(), 0);
    }

    #[test]
    fn test_skills_panel_update() {
        let mut panel = SkillsPanel::new();

        let mut usage_counts = HashMap::new();
        usage_counts.insert("api-design".to_string(), 5);
        usage_counts.insert("database".to_string(), 3);

        let metrics = SkillsMetrics {
            loaded_skills: vec!["api-design".to_string(), "database".to_string()],
            usage_counts,
        };

        panel.update(metrics);
        assert_eq!(panel.loaded_count(), 2);
        assert_eq!(panel.total_usage(), 8);
    }

    #[test]
    fn test_top_skills_sorting() {
        let mut panel = SkillsPanel::new();

        let mut usage_counts = HashMap::new();
        usage_counts.insert("skill-a".to_string(), 10);
        usage_counts.insert("skill-b".to_string(), 50);
        usage_counts.insert("skill-c".to_string(), 25);

        let metrics = SkillsMetrics {
            loaded_skills: vec![
                "skill-a".to_string(),
                "skill-b".to_string(),
                "skill-c".to_string(),
            ],
            usage_counts,
        };

        panel.update(metrics);

        let top = panel.top_skills();
        assert_eq!(top[0].0, "skill-b"); // Highest usage
        assert_eq!(top[0].1, 50);
        assert_eq!(top[1].0, "skill-c");
        assert_eq!(top[1].1, 25);
        assert_eq!(top[2].0, "skill-a"); // Lowest usage
        assert_eq!(top[2].1, 10);
    }

    #[test]
    fn test_max_display() {
        let mut panel = SkillsPanel::new().max_display(2);

        let mut usage_counts = HashMap::new();
        for i in 0..5 {
            usage_counts.insert(format!("skill-{}", i), i);
        }

        let metrics = SkillsMetrics {
            loaded_skills: (0..5).map(|i| format!("skill-{}", i)).collect(),
            usage_counts,
        };

        panel.update(metrics);

        let top = panel.top_skills();
        assert_eq!(top.len(), 2); // Only top 2
    }
}
