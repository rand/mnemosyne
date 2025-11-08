//! Operations panel - Shows recent CLI operations and their status

use chrono::{DateTime, Utc};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

/// CLI operation entry
#[derive(Debug, Clone)]
pub struct OperationEntry {
    pub command: String,
    pub args: Vec<String>,
    pub status: OperationStatus,
    pub duration_ms: Option<u64>,
    pub result_summary: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Operation status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationStatus {
    Running,
    Completed,
    Failed(String),
}

impl OperationStatus {
    /// Get color for status
    fn color(&self) -> Color {
        match self {
            OperationStatus::Running => Color::Blue,
            OperationStatus::Completed => Color::Green,
            OperationStatus::Failed(_) => Color::Red,
        }
    }

    /// Get display string
    fn display(&self) -> &str {
        match self {
            OperationStatus::Running => "RUNNING",
            OperationStatus::Completed => "DONE",
            OperationStatus::Failed(_) => "FAIL",
        }
    }
}

/// Operations panel widget
pub struct OperationsPanel {
    operations: Vec<OperationEntry>,
    max_operations: usize,
    scroll_offset: usize,
}

impl OperationsPanel {
    /// Create new operations panel
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            max_operations: 100,
            scroll_offset: 0,
        }
    }

    /// Add a started operation
    pub fn add_started(&mut self, command: String, args: Vec<String>) {
        self.operations.push(OperationEntry {
            command,
            args,
            status: OperationStatus::Running,
            duration_ms: None,
            result_summary: None,
            timestamp: Utc::now(),
        });

        // Trim old operations
        if self.operations.len() > self.max_operations {
            self.operations.drain(0..self.operations.len() - self.max_operations);
        }

        self.scroll_offset = 0;
    }

    /// Update operation to completed
    pub fn update_completed(&mut self, command: &str, duration_ms: u64, result_summary: String) {
        // Find most recent matching command
        if let Some(op) = self.operations.iter_mut().rev().find(|op| op.command == command && matches!(op.status, OperationStatus::Running)) {
            op.status = OperationStatus::Completed;
            op.duration_ms = Some(duration_ms);
            op.result_summary = Some(result_summary);
        }
    }

    /// Update operation to failed
    pub fn update_failed(&mut self, command: &str, error: String, duration_ms: u64) {
        // Find most recent matching command
        if let Some(op) = self.operations.iter_mut().rev().find(|op| op.command == command && matches!(op.status, OperationStatus::Running)) {
            op.status = OperationStatus::Failed(error);
            op.duration_ms = Some(duration_ms);
        }
    }

    /// Clear all operations
    pub fn clear(&mut self) {
        self.operations.clear();
        self.scroll_offset = 0;
    }

    /// Scroll up
    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(amount);
        if self.scroll_offset >= self.operations.len() {
            self.scroll_offset = self.operations.len().saturating_sub(1);
        }
    }

    /// Scroll down
    pub fn scroll_down(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    /// Format duration as human-readable string
    fn format_duration(duration_ms: u64) -> String {
        if duration_ms < 1000 {
            format!("{}ms", duration_ms)
        } else if duration_ms < 60_000 {
            format!("{:.1}s", duration_ms as f64 / 1000.0)
        } else {
            format!("{:.1}m", duration_ms as f64 / 60_000.0)
        }
    }

    /// Render the operations panel
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let available_height = area.height.saturating_sub(3) as usize; // Subtract borders and header

        if self.operations.is_empty() {
            let block = Block::default()
                .title("CLI Operations (0 total)")
                .borders(Borders::ALL);
            frame.render_widget(block, area);

            let empty_text = Span::styled(
                "No CLI operations yet",
                Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC),
            );
            let x = area.x + 2;
            let y = area.y + 1;
            if y < area.bottom() {
                frame.render_widget(Line::from(empty_text), Rect::new(x, y, area.width.saturating_sub(4), 1));
            }
            return;
        }

        // Prepare rows (most recent first)
        let rows: Vec<Row> = self.operations
            .iter()
            .rev()
            .skip(self.scroll_offset)
            .take(available_height)
            .map(|op| {
                let status_color = op.status.color();

                // Format command with args
                let cmd_display = if op.args.is_empty() {
                    op.command.clone()
                } else {
                    format!("{} {}", op.command, op.args.join(" "))
                };

                // Truncate command to fit
                let cmd_display = if cmd_display.len() > 20 {
                    format!("{}...", &cmd_display[..17])
                } else {
                    cmd_display
                };

                // Duration
                let duration_display = op.duration_ms
                    .map(Self::format_duration)
                    .unwrap_or_else(|| "-".to_string());

                // Result/Error
                let result_display = match &op.status {
                    OperationStatus::Running => "...".to_string(),
                    OperationStatus::Completed => {
                        op.result_summary.as_ref()
                            .map(|s| if s.len() > 30 { format!("{}...", &s[..27]) } else { s.clone() })
                            .unwrap_or_else(|| "OK".to_string())
                    },
                    OperationStatus::Failed(err) => {
                        if err.len() > 30 {
                            format!("{}...", &err[..27])
                        } else {
                            err.clone()
                        }
                    }
                };

                // Timestamp (HH:MM:SS)
                let time_display = op.timestamp.format("%H:%M:%S").to_string();

                Row::new(vec![
                    Cell::from(time_display).style(Style::default().fg(Color::DarkGray)),
                    Cell::from(cmd_display),
                    Cell::from(op.status.display()).style(Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
                    Cell::from(duration_display).style(Style::default().fg(Color::Cyan)),
                    Cell::from(result_display).style(Style::default().fg(Color::White)),
                ])
            })
            .collect();

        // Header
        let header = Row::new(vec!["Time", "Command", "Status", "Duration", "Result"])
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .bottom_margin(0);

        // Table
        let title = if self.scroll_offset > 0 {
            format!("CLI Operations (↑ {} hidden, {} total)", self.scroll_offset, self.operations.len())
        } else {
            format!("CLI Operations ({} total)", self.operations.len())
        };

        let widths = [
            Constraint::Length(9),  // Time
            Constraint::Length(22), // Command
            Constraint::Length(10), // Status
            Constraint::Length(10), // Duration
            Constraint::Min(20),    // Result (takes remaining space)
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .block(Block::default().title(title).borders(Borders::ALL))
            .column_spacing(1);

        frame.render_widget(table, area);
    }

    /// Get total operation count
    pub fn operation_count(&self) -> usize {
        self.operations.len()
    }

    /// Get current scroll offset
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }
}

impl Default for OperationsPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operations_panel_creation() {
        let panel = OperationsPanel::new();
        assert_eq!(panel.operation_count(), 0);
        assert_eq!(panel.scroll_offset(), 0);
    }

    #[test]
    fn test_add_started_operation() {
        let mut panel = OperationsPanel::new();
        panel.add_started("remember".to_string(), vec!["--content".to_string(), "test".to_string()]);

        assert_eq!(panel.operation_count(), 1);
    }

    #[test]
    fn test_update_completed() {
        let mut panel = OperationsPanel::new();
        panel.add_started("remember".to_string(), vec![]);
        panel.update_completed("remember", 1500, "Memory stored".to_string());

        assert_eq!(panel.operation_count(), 1);
    }

    #[test]
    fn test_update_failed() {
        let mut panel = OperationsPanel::new();
        panel.add_started("recall".to_string(), vec![]);
        panel.update_failed("recall", "Database error".to_string(), 500);

        assert_eq!(panel.operation_count(), 1);
    }

    #[test]
    fn test_operation_status_color() {
        assert_eq!(OperationStatus::Running.color(), Color::Blue);
        assert_eq!(OperationStatus::Completed.color(), Color::Green);
        assert_eq!(OperationStatus::Failed("error".to_string()).color(), Color::Red);
    }

    #[test]
    fn test_operation_status_display() {
        assert_eq!(OperationStatus::Running.display(), "RUNNING");
        assert_eq!(OperationStatus::Completed.display(), "DONE");
        assert_eq!(OperationStatus::Failed("error".to_string()).display(), "FAIL");
    }

    #[test]
    fn test_max_operations_limit() {
        let mut panel = OperationsPanel::new();

        // Add more than max_operations
        for i in 0..150 {
            panel.add_started(format!("cmd{}", i), vec![]);
        }

        // Should be capped at max_operations
        assert_eq!(panel.operation_count(), 100);
    }

    #[test]
    fn test_clear_operations() {
        let mut panel = OperationsPanel::new();
        panel.add_started("test".to_string(), vec![]);
        panel.add_started("test2".to_string(), vec![]);

        assert_eq!(panel.operation_count(), 2);

        panel.clear();
        assert_eq!(panel.operation_count(), 0);
        assert_eq!(panel.scroll_offset(), 0);
    }

    #[test]
    fn test_scroll_operations() {
        let mut panel = OperationsPanel::new();
        for i in 0..10 {
            panel.add_started(format!("cmd{}", i), vec![]);
        }

        panel.scroll_up(3);
        assert_eq!(panel.scroll_offset(), 3);

        panel.scroll_down(2);
        assert_eq!(panel.scroll_offset(), 1);

        panel.scroll_down(5);
        assert_eq!(panel.scroll_offset(), 0);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(OperationsPanel::format_duration(500), "500ms");
        assert_eq!(OperationsPanel::format_duration(1500), "1.5s");
        assert_eq!(OperationsPanel::format_duration(65000), "1.1m");
    }

    #[test]
    fn test_multiple_running_operations() {
        let mut panel = OperationsPanel::new();
        panel.add_started("remember".to_string(), vec![]);
        panel.add_started("recall".to_string(), vec![]);
        panel.add_started("evolve".to_string(), vec![]);

        assert_eq!(panel.operation_count(), 3);

        // Complete first remember command
        panel.update_completed("remember", 1000, "Success".to_string());

        // Should still have 3 operations
        assert_eq!(panel.operation_count(), 3);
    }
}
