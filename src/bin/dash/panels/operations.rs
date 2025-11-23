//! Operations panel - Shows recent CLI operations and their status

use crate::colors::DashboardColors;
use chrono::{DateTime, Duration, Utc};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};
use std::collections::HashMap;

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
            OperationStatus::Running => DashboardColors::IN_PROGRESS,
            OperationStatus::Completed => DashboardColors::SUCCESS,
            OperationStatus::Failed(_) => DashboardColors::ERROR,
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

/// View mode for operations panel
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewMode {
    /// Show all operations in chronological order
    List,
    /// Group operations by command name
    Grouped,
    /// Show statistics summary
    Statistics,
}

/// Filter options for operations
#[derive(Debug, Clone, Default)]
pub struct OperationFilter {
    /// Filter by status
    pub status: Option<OperationStatus>,
    /// Filter by command name
    pub command: Option<String>,
    /// Filter by minimum duration (ms)
    pub min_duration_ms: Option<u64>,
    /// Filter by time range (last N minutes)
    pub last_minutes: Option<i64>,
}

impl OperationFilter {
    /// Check if an operation matches the filter
    fn matches(&self, op: &OperationEntry) -> bool {
        // Status filter
        if let Some(ref status_filter) = self.status {
            if !matches!(
                (&op.status, status_filter),
                (OperationStatus::Running, OperationStatus::Running)
                    | (OperationStatus::Completed, OperationStatus::Completed)
                    | (OperationStatus::Failed(_), OperationStatus::Failed(_))
            ) {
                return false;
            }
        }

        // Command filter
        if let Some(ref cmd_filter) = self.command {
            if !op.command.contains(cmd_filter) {
                return false;
            }
        }

        // Duration filter
        if let Some(min_dur) = self.min_duration_ms {
            if op.duration_ms.unwrap_or(0) < min_dur {
                return false;
            }
        }

        // Time range filter
        if let Some(minutes) = self.last_minutes {
            let threshold = Utc::now() - Duration::minutes(minutes);
            if op.timestamp < threshold {
                return false;
            }
        }

        true
    }
}

/// Statistics about operations
#[derive(Debug, Clone, Default)]
pub struct OperationStats {
    pub total_count: usize,
    pub completed_count: usize,
    pub failed_count: usize,
    pub running_count: usize,
    pub success_rate: f64,
    pub avg_duration_ms: u64,
    pub slowest_command: Option<(String, u64)>,
    pub most_frequent_command: Option<(String, usize)>,
    pub operations_per_minute: f64,
    pub command_stats: HashMap<String, CommandStats>,
}

/// Per-command statistics
#[derive(Debug, Clone, Default)]
pub struct CommandStats {
    pub count: usize,
    pub avg_duration_ms: u64,
    pub success_count: usize,
    pub failure_count: usize,
}

/// Operations panel widget
pub struct OperationsPanel {
    operations: Vec<OperationEntry>,
    max_operations: usize,
    scroll_offset: usize,
    view_mode: ViewMode,
    filter: OperationFilter,
    collapsed_groups: HashMap<String, bool>,
}

impl OperationsPanel {
    /// Create new operations panel
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            max_operations: 100,
            scroll_offset: 0,
            view_mode: ViewMode::List,
            filter: OperationFilter::default(),
            collapsed_groups: HashMap::new(),
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
            self.operations
                .drain(0..self.operations.len() - self.max_operations);
        }

        self.scroll_offset = 0;
    }

    /// Update operation to completed
    pub fn update_completed(&mut self, command: &str, duration_ms: u64, result_summary: String) {
        // Find most recent matching command
        if let Some(op) = self
            .operations
            .iter_mut()
            .rev()
            .find(|op| op.command == command && matches!(op.status, OperationStatus::Running))
        {
            op.status = OperationStatus::Completed;
            op.duration_ms = Some(duration_ms);
            op.result_summary = Some(result_summary);
        }
    }

    /// Update operation to failed
    pub fn update_failed(&mut self, command: &str, error: String, duration_ms: u64) {
        // Find most recent matching command
        if let Some(op) = self
            .operations
            .iter_mut()
            .rev()
            .find(|op| op.command == command && matches!(op.status, OperationStatus::Running))
        {
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
        match self.view_mode {
            ViewMode::List => self.render_list_view(frame, area),
            ViewMode::Grouped => self.render_grouped_view(frame, area),
            ViewMode::Statistics => self.render_statistics_view(frame, area),
        }
    }

    /// Render list view (original view with enhancements)
    fn render_list_view(&self, frame: &mut Frame, area: Rect) {
        let available_height = area.height.saturating_sub(3) as usize;

        let filtered = self.filtered_operations();
        if filtered.is_empty() {
            let block = Block::default()
                .title("CLI Operations (0 total)")
                .borders(Borders::ALL);
            frame.render_widget(block, area);

            let empty_text = Span::styled(
                "No operations match current filter",
                Style::default()
                    .fg(DashboardColors::MUTED)
                    .add_modifier(Modifier::ITALIC),
            );
            let x = area.x + 2;
            let y = area.y + 1;
            if y < area.bottom() {
                frame.render_widget(
                    Line::from(empty_text),
                    Rect::new(x, y, area.width.saturating_sub(4), 1),
                );
            }
            return;
        }

        // Prepare rows (most recent first)
        let rows: Vec<Row> = filtered
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
                let cmd_display = Self::truncate(&cmd_display, 20);

                // Duration with color coding
                let duration_display = op
                    .duration_ms
                    .map(Self::format_duration)
                    .unwrap_or_else(|| "-".to_string());
                let duration_color = op
                    .duration_ms
                    .map(Self::duration_color)
                    .unwrap_or(Color::Gray);

                // Result/Error
                let result_display = match &op.status {
                    OperationStatus::Running => "...".to_string(),
                    OperationStatus::Completed => op
                        .result_summary
                        .as_ref()
                        .map(|s| Self::truncate(s, 30))
                        .unwrap_or_else(|| "OK".to_string()),
                    OperationStatus::Failed(err) => Self::truncate(err, 30),
                };

                // Relative timestamp
                let time_display = Self::format_relative_time(op.timestamp);

                // Highlight slow operations
                let slow_indicator = if op.duration_ms.unwrap_or(0) > 1000 {
                    "⚠"
                } else {
                    ""
                };

                Row::new(vec![
                    Cell::from(time_display).style(Style::default().fg(DashboardColors::SECONDARY)),
                    Cell::from(cmd_display),
                    Cell::from(op.status.display()).style(
                        Style::default()
                            .fg(status_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Cell::from(format!("{}{}", slow_indicator, duration_display))
                        .style(Style::default().fg(duration_color)),
                    Cell::from(result_display).style(Style::default().fg(DashboardColors::TEXT)),
                ])
            })
            .collect();

        // Header
        let header = Row::new(vec!["Time", "Command", "Status", "Duration", "Result"])
            .style(
                Style::default()
                    .fg(DashboardColors::HEADER)
                    .add_modifier(Modifier::BOLD),
            )
            .bottom_margin(0);

        // Table
        let title = if self.scroll_offset > 0 {
            format!(
                "CLI Operations (↑ {} hidden, {} filtered)",
                self.scroll_offset,
                filtered.len()
            )
        } else {
            format!("CLI Operations ({} filtered)", filtered.len())
        };

        let widths = [
            Constraint::Length(9),
            Constraint::Length(22),
            Constraint::Length(10),
            Constraint::Length(11),
            Constraint::Min(20),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .block(Block::default().title(title).borders(Borders::ALL))
            .column_spacing(1);

        frame.render_widget(table, area);
    }

    /// Render grouped view
    fn render_grouped_view(&self, frame: &mut Frame, area: Rect) {
        let available_height = area.height.saturating_sub(3) as usize;
        let groups = self.group_operations();

        if groups.is_empty() {
            let block = Block::default()
                .title("CLI Operations - Grouped (0 groups)")
                .borders(Borders::ALL);
            frame.render_widget(block, area);
            return;
        }

        let mut rows = Vec::new();
        let mut visible_count = 0;

        for (cmd, ops) in &groups {
            if visible_count >= self.scroll_offset {
                let is_collapsed = self.collapsed_groups.get(cmd).copied().unwrap_or(false);
                let collapse_indicator = if is_collapsed { "▶" } else { "▼" };

                // Group header row
                let header_text = format!("{} {} ({} ops)", collapse_indicator, cmd, ops.len());
                rows.push(Row::new(vec![Cell::from(header_text).style(
                    Style::default()
                        .fg(DashboardColors::HIGHLIGHT)
                        .add_modifier(Modifier::BOLD),
                )]));
                visible_count += 1;

                // Show operations if expanded
                if !is_collapsed {
                    for op in ops.iter().take(5) {
                        if visible_count >= available_height {
                            break;
                        }

                        let time = Self::format_relative_time(op.timestamp);
                        let status = op.status.display();
                        let duration = op
                            .duration_ms
                            .map(Self::format_duration)
                            .unwrap_or_else(|| "-".to_string());

                        rows.push(Row::new(vec![Cell::from(format!(
                            "  {} | {} | {}",
                            time, status, duration
                        ))
                        .style(Style::default().fg(DashboardColors::TEXT))]));
                        visible_count += 1;
                    }

                    if ops.len() > 5 {
                        rows.push(Row::new(vec![Cell::from(format!(
                            "  ... {} more",
                            ops.len() - 5
                        ))
                        .style(
                            Style::default()
                                .fg(DashboardColors::MUTED)
                                .add_modifier(Modifier::ITALIC),
                        )]));
                        visible_count += 1;
                    }
                }
            }
        }

        let table = Table::new(rows, [Constraint::Min(40)]).block(
            Block::default()
                .title(format!(
                    "CLI Operations - Grouped ({} groups)",
                    groups.len()
                ))
                .borders(Borders::ALL),
        );

        frame.render_widget(table, area);
    }

    /// Render statistics view
    fn render_statistics_view(&self, frame: &mut Frame, area: Rect) {
        let stats = self.calculate_stats();

        let mut rows = vec![
            Row::new(vec![
                Cell::from("Total Operations:").style(
                    Style::default()
                        .fg(DashboardColors::HEADER)
                        .add_modifier(Modifier::BOLD),
                ),
                Cell::from(format!("{}", stats.total_count)),
            ]),
            Row::new(vec![
                Cell::from("  Completed:").style(Style::default().fg(DashboardColors::SUCCESS)),
                Cell::from(format!("{}", stats.completed_count)),
            ]),
            Row::new(vec![
                Cell::from("  Failed:").style(Style::default().fg(DashboardColors::ERROR)),
                Cell::from(format!("{}", stats.failed_count)),
            ]),
            Row::new(vec![
                Cell::from("  Running:").style(Style::default().fg(DashboardColors::IN_PROGRESS)),
                Cell::from(format!("{}", stats.running_count)),
            ]),
            Row::new(vec![
                Cell::from("Success Rate:").style(
                    Style::default()
                        .fg(DashboardColors::HEADER)
                        .add_modifier(Modifier::BOLD),
                ),
                Cell::from(format!("{:.1}%", stats.success_rate)),
            ]),
            Row::new(vec![
                Cell::from("Average Duration:").style(
                    Style::default()
                        .fg(DashboardColors::HEADER)
                        .add_modifier(Modifier::BOLD),
                ),
                Cell::from(Self::format_duration(stats.avg_duration_ms)),
            ]),
        ];

        if let Some((cmd, dur)) = &stats.slowest_command {
            rows.push(Row::new(vec![
                Cell::from("Slowest Command:").style(
                    Style::default()
                        .fg(DashboardColors::HEADER)
                        .add_modifier(Modifier::BOLD),
                ),
                Cell::from(format!("{} ({})", cmd, Self::format_duration(*dur))),
            ]));
        }

        if let Some((cmd, count)) = &stats.most_frequent_command {
            rows.push(Row::new(vec![
                Cell::from("Most Frequent:").style(
                    Style::default()
                        .fg(DashboardColors::HEADER)
                        .add_modifier(Modifier::BOLD),
                ),
                Cell::from(format!("{} ({} times)", cmd, count)),
            ]));
        }

        rows.push(Row::new(vec![
            Cell::from("Ops/Minute:").style(
                Style::default()
                    .fg(DashboardColors::HEADER)
                    .add_modifier(Modifier::BOLD),
            ),
            Cell::from(format!("{:.2}", stats.operations_per_minute)),
        ]));

        let table = Table::new(rows, [Constraint::Length(20), Constraint::Min(30)])
            .block(
                Block::default()
                    .title("CLI Operations - Statistics")
                    .borders(Borders::ALL),
            )
            .column_spacing(2);

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

    /// Set view mode
    pub fn set_view_mode(&mut self, mode: ViewMode) {
        self.view_mode = mode;
        self.scroll_offset = 0;
    }

    /// Get current view mode
    pub fn view_mode(&self) -> &ViewMode {
        &self.view_mode
    }

    /// Set filter
    pub fn set_filter(&mut self, filter: OperationFilter) {
        self.filter = filter;
        self.scroll_offset = 0;
    }

    /// Get current filter
    pub fn filter(&self) -> &OperationFilter {
        &self.filter
    }

    /// Clear filter
    pub fn clear_filter(&mut self) {
        self.filter = OperationFilter::default();
        self.scroll_offset = 0;
    }

    /// Toggle group collapsed state
    pub fn toggle_group(&mut self, command: &str) {
        let is_collapsed = self.collapsed_groups.get(command).copied().unwrap_or(false);
        self.collapsed_groups
            .insert(command.to_string(), !is_collapsed);
    }

    /// Get filtered operations
    fn filtered_operations(&self) -> Vec<&OperationEntry> {
        self.operations
            .iter()
            .filter(|op| self.filter.matches(op))
            .collect()
    }

    /// Calculate statistics for operations
    pub fn calculate_stats(&self) -> OperationStats {
        let filtered = self.filtered_operations();
        let total_count = filtered.len();

        if total_count == 0 {
            return OperationStats::default();
        }

        let mut completed_count = 0;
        let mut failed_count = 0;
        let mut running_count = 0;
        let mut total_duration_ms: u64 = 0;
        let mut duration_count = 0;
        let mut slowest_command: Option<(String, u64)> = None;
        let mut command_counts: HashMap<String, usize> = HashMap::new();
        let mut command_stats: HashMap<String, CommandStats> = HashMap::new();

        // First pass: collect basic stats
        for op in &filtered {
            match &op.status {
                OperationStatus::Running => running_count += 1,
                OperationStatus::Completed => completed_count += 1,
                OperationStatus::Failed(_) => failed_count += 1,
            }

            if let Some(duration) = op.duration_ms {
                total_duration_ms += duration;
                duration_count += 1;

                // Track slowest
                if slowest_command.is_none() || duration > slowest_command.as_ref().unwrap().1 {
                    slowest_command = Some((op.command.clone(), duration));
                }
            }

            // Count commands
            *command_counts.entry(op.command.clone()).or_insert(0) += 1;

            // Per-command stats
            let stats = command_stats.entry(op.command.clone()).or_default();
            stats.count += 1;
            if let Some(duration) = op.duration_ms {
                stats.avg_duration_ms = ((stats.avg_duration_ms * (stats.count - 1) as u64)
                    + duration)
                    / stats.count as u64;
            }
            match &op.status {
                OperationStatus::Completed => stats.success_count += 1,
                OperationStatus::Failed(_) => stats.failure_count += 1,
                _ => {}
            }
        }

        // Calculate derived stats
        let success_rate = if completed_count + failed_count > 0 {
            (completed_count as f64 / (completed_count + failed_count) as f64) * 100.0
        } else {
            0.0
        };

        let avg_duration_ms = if duration_count > 0 {
            total_duration_ms / duration_count
        } else {
            0
        };

        let most_frequent_command = command_counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(cmd, count)| (cmd.clone(), *count));

        // Calculate operations per minute
        let operations_per_minute = if let Some(oldest) = filtered.last() {
            let time_span = Utc::now() - oldest.timestamp;
            let minutes = time_span.num_minutes() as f64;
            if minutes > 0.0 {
                total_count as f64 / minutes
            } else {
                0.0
            }
        } else {
            0.0
        };

        OperationStats {
            total_count,
            completed_count,
            failed_count,
            running_count,
            success_rate,
            avg_duration_ms,
            slowest_command,
            most_frequent_command,
            operations_per_minute,
            command_stats,
        }
    }

    /// Group operations by command
    fn group_operations(&self) -> Vec<(String, Vec<&OperationEntry>)> {
        let filtered = self.filtered_operations();
        let mut groups: HashMap<String, Vec<&OperationEntry>> = HashMap::new();

        for op in filtered {
            groups.entry(op.command.clone()).or_default().push(op);
        }

        let mut grouped: Vec<(String, Vec<&OperationEntry>)> = groups
            .into_iter()
            .map(|(cmd, mut ops)| {
                // Sort by timestamp (most recent first)
                ops.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                (cmd, ops)
            })
            .collect();

        // Sort groups by most recent operation
        grouped.sort_by(|a, b| {
            let a_recent =
                a.1.first()
                    .map(|op| op.timestamp)
                    .unwrap_or(DateTime::<Utc>::MIN_UTC);
            let b_recent =
                b.1.first()
                    .map(|op| op.timestamp)
                    .unwrap_or(DateTime::<Utc>::MIN_UTC);
            b_recent.cmp(&a_recent)
        });

        grouped
    }

    /// Format relative timestamp (e.g., "2m ago")
    fn format_relative_time(timestamp: DateTime<Utc>) -> String {
        let now = Utc::now();
        let diff = now - timestamp;

        if diff.num_seconds() < 60 {
            format!("{}s ago", diff.num_seconds())
        } else if diff.num_minutes() < 60 {
            format!("{}m ago", diff.num_minutes())
        } else if diff.num_hours() < 24 {
            format!("{}h ago", diff.num_hours())
        } else {
            format!("{}d ago", diff.num_days())
        }
    }

    /// Get color for duration (green=fast, yellow=medium, red=slow)
    fn duration_color(duration_ms: u64) -> Color {
        if duration_ms < 500 {
            DashboardColors::PERF_FAST
        } else if duration_ms < 2000 {
            DashboardColors::PERF_MEDIUM
        } else {
            DashboardColors::PERF_SLOW
        }
    }

    /// Truncate string with ellipsis
    fn truncate(s: &str, max_len: usize) -> String {
        if s.len() > max_len {
            format!("{}...", &s[..max_len.saturating_sub(3)])
        } else {
            s.to_string()
        }
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

    // ===== Basic Operations Tests =====

    #[test]
    fn test_operations_panel_creation() {
        let panel = OperationsPanel::new();
        assert_eq!(panel.operation_count(), 0);
        assert_eq!(panel.scroll_offset(), 0);
        assert_eq!(panel.view_mode(), &ViewMode::List);
    }

    #[test]
    fn test_add_started_operation() {
        let mut panel = OperationsPanel::new();
        panel.add_started(
            "remember".to_string(),
            vec!["--content".to_string(), "test".to_string()],
        );

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
        assert_eq!(
            OperationStatus::Running.color(),
            DashboardColors::IN_PROGRESS
        );
        assert_eq!(OperationStatus::Completed.color(), DashboardColors::SUCCESS);
        assert_eq!(
            OperationStatus::Failed("error".to_string()).color(),
            DashboardColors::ERROR
        );
    }

    #[test]
    fn test_operation_status_display() {
        assert_eq!(OperationStatus::Running.display(), "RUNNING");
        assert_eq!(OperationStatus::Completed.display(), "DONE");
        assert_eq!(
            OperationStatus::Failed("error".to_string()).display(),
            "FAIL"
        );
    }

    #[test]
    fn test_max_operations_limit() {
        let mut panel = OperationsPanel::new();

        for i in 0..150 {
            panel.add_started(format!("cmd{}", i), vec![]);
        }

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

        panel.update_completed("remember", 1000, "Success".to_string());

        assert_eq!(panel.operation_count(), 3);
    }

    // ===== View Mode Tests =====

    #[test]
    fn test_view_mode_switching() {
        let mut panel = OperationsPanel::new();
        assert_eq!(panel.view_mode(), &ViewMode::List);

        panel.set_view_mode(ViewMode::Grouped);
        assert_eq!(panel.view_mode(), &ViewMode::Grouped);

        panel.set_view_mode(ViewMode::Statistics);
        assert_eq!(panel.view_mode(), &ViewMode::Statistics);
    }

    #[test]
    fn test_view_mode_resets_scroll() {
        let mut panel = OperationsPanel::new();
        for i in 0..10 {
            panel.add_started(format!("cmd{}", i), vec![]);
        }

        panel.scroll_up(5);
        assert_eq!(panel.scroll_offset(), 5);

        panel.set_view_mode(ViewMode::Grouped);
        assert_eq!(panel.scroll_offset(), 0);
    }

    // ===== Filter Tests =====

    #[test]
    fn test_filter_by_status() {
        let mut panel = OperationsPanel::new();
        panel.add_started("remember".to_string(), vec![]);
        panel.add_started("recall".to_string(), vec![]);
        panel.update_completed("remember", 1000, "OK".to_string());

        let filter = OperationFilter {
            status: Some(OperationStatus::Completed),
            ..Default::default()
        };
        panel.set_filter(filter);

        let filtered = panel.filtered_operations();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].command, "remember");
    }

    #[test]
    fn test_filter_by_command() {
        let mut panel = OperationsPanel::new();
        panel.add_started("remember".to_string(), vec![]);
        panel.add_started("recall".to_string(), vec![]);
        panel.add_started("evolve".to_string(), vec![]);

        let filter = OperationFilter {
            command: Some("re".to_string()),
            ..Default::default()
        };
        panel.set_filter(filter);

        let filtered = panel.filtered_operations();
        assert_eq!(filtered.len(), 2); // remember and recall contain "re"
    }

    #[test]
    fn test_filter_by_duration() {
        let mut panel = OperationsPanel::new();
        panel.add_started("cmd1".to_string(), vec![]);
        panel.add_started("cmd2".to_string(), vec![]);
        panel.add_started("cmd3".to_string(), vec![]);

        panel.update_completed("cmd1", 500, "OK".to_string());
        panel.update_completed("cmd2", 1500, "OK".to_string());
        panel.update_completed("cmd3", 2500, "OK".to_string());

        let filter = OperationFilter {
            min_duration_ms: Some(1000),
            ..Default::default()
        };
        panel.set_filter(filter);

        let filtered = panel.filtered_operations();
        assert_eq!(filtered.len(), 2); // cmd2 and cmd3
    }

    #[test]
    fn test_clear_filter() {
        let mut panel = OperationsPanel::new();
        panel.add_started("test".to_string(), vec![]);

        let filter = OperationFilter {
            command: Some("nonexistent".to_string()),
            ..Default::default()
        };
        panel.set_filter(filter);

        assert_eq!(panel.filtered_operations().len(), 0);

        panel.clear_filter();
        assert_eq!(panel.filtered_operations().len(), 1);
    }

    #[test]
    fn test_combined_filters() {
        let mut panel = OperationsPanel::new();

        panel.add_started("remember".to_string(), vec![]);
        panel.add_started("recall".to_string(), vec![]);
        panel.add_started("remember".to_string(), vec![]);

        panel.update_completed("remember", 1500, "OK".to_string());
        panel.update_failed("recall", "Error".to_string(), 500);
        panel.update_completed("remember", 2500, "OK".to_string());

        let filter = OperationFilter {
            command: Some("remember".to_string()),
            status: Some(OperationStatus::Completed),
            min_duration_ms: Some(2000),
            ..Default::default()
        };
        panel.set_filter(filter);

        let filtered = panel.filtered_operations();
        assert_eq!(filtered.len(), 1); // Only the second remember (2500ms, completed)
    }

    // ===== Grouping Tests =====

    #[test]
    fn test_group_operations_empty() {
        let panel = OperationsPanel::new();
        let groups = panel.group_operations();
        assert_eq!(groups.len(), 0);
    }

    #[test]
    fn test_group_operations_single_command() {
        let mut panel = OperationsPanel::new();
        panel.add_started("remember".to_string(), vec![]);
        panel.add_started("remember".to_string(), vec![]);
        panel.add_started("remember".to_string(), vec![]);

        let groups = panel.group_operations();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].0, "remember");
        assert_eq!(groups[0].1.len(), 3);
    }

    #[test]
    fn test_group_operations_multiple_commands() {
        let mut panel = OperationsPanel::new();
        panel.add_started("remember".to_string(), vec![]);
        panel.add_started("recall".to_string(), vec![]);
        panel.add_started("remember".to_string(), vec![]);
        panel.add_started("evolve".to_string(), vec![]);

        let groups = panel.group_operations();
        assert_eq!(groups.len(), 3);

        // Check counts
        let remember_group = groups.iter().find(|(cmd, _)| cmd == "remember").unwrap();
        assert_eq!(remember_group.1.len(), 2);

        let recall_group = groups.iter().find(|(cmd, _)| cmd == "recall").unwrap();
        assert_eq!(recall_group.1.len(), 1);

        let evolve_group = groups.iter().find(|(cmd, _)| cmd == "evolve").unwrap();
        assert_eq!(evolve_group.1.len(), 1);
    }

    #[test]
    fn test_group_operations_sorted_by_recent() {
        let mut panel = OperationsPanel::new();
        panel.add_started("old".to_string(), vec![]);
        std::thread::sleep(std::time::Duration::from_millis(10));
        panel.add_started("new".to_string(), vec![]);

        let groups = panel.group_operations();
        // Most recent group should be first
        assert_eq!(groups[0].0, "new");
        assert_eq!(groups[1].0, "old");
    }

    #[test]
    fn test_toggle_group_collapse() {
        let mut panel = OperationsPanel::new();

        panel.toggle_group("remember");
        assert_eq!(panel.collapsed_groups.get("remember"), Some(&true));

        panel.toggle_group("remember");
        assert_eq!(panel.collapsed_groups.get("remember"), Some(&false));
    }

    // ===== Statistics Tests =====

    #[test]
    fn test_calculate_stats_empty() {
        let panel = OperationsPanel::new();
        let stats = panel.calculate_stats();

        assert_eq!(stats.total_count, 0);
        assert_eq!(stats.completed_count, 0);
        assert_eq!(stats.failed_count, 0);
        assert_eq!(stats.running_count, 0);
        assert_eq!(stats.success_rate, 0.0);
    }

    #[test]
    fn test_calculate_stats_all_completed() {
        let mut panel = OperationsPanel::new();

        for i in 0..5 {
            panel.add_started(format!("cmd{}", i), vec![]);
            panel.update_completed(
                &format!("cmd{}", i),
                1000 + (i as u64 * 100),
                "OK".to_string(),
            );
        }

        let stats = panel.calculate_stats();
        assert_eq!(stats.total_count, 5);
        assert_eq!(stats.completed_count, 5);
        assert_eq!(stats.failed_count, 0);
        assert_eq!(stats.running_count, 0);
        assert_eq!(stats.success_rate, 100.0);
        assert_eq!(stats.avg_duration_ms, 1200); // (1000+1100+1200+1300+1400)/5
    }

    #[test]
    fn test_calculate_stats_mixed_status() {
        let mut panel = OperationsPanel::new();

        panel.add_started("cmd1".to_string(), vec![]);
        panel.update_completed("cmd1", 1000, "OK".to_string());

        panel.add_started("cmd2".to_string(), vec![]);
        panel.update_failed("cmd2", "Error".to_string(), 500);

        panel.add_started("cmd3".to_string(), vec![]);
        panel.update_completed("cmd3", 1500, "OK".to_string());

        panel.add_started("cmd4".to_string(), vec![]);
        // Leave cmd4 running

        let stats = panel.calculate_stats();
        assert_eq!(stats.total_count, 4);
        assert_eq!(stats.completed_count, 2);
        assert_eq!(stats.failed_count, 1);
        assert_eq!(stats.running_count, 1);
        assert_eq!(stats.success_rate, 66.66666666666666); // 2/3 * 100
        assert_eq!(stats.avg_duration_ms, 1000); // (1000+500+1500)/3
    }

    #[test]
    fn test_calculate_stats_slowest_command() {
        let mut panel = OperationsPanel::new();

        panel.add_started("fast".to_string(), vec![]);
        panel.update_completed("fast", 500, "OK".to_string());

        panel.add_started("slow".to_string(), vec![]);
        panel.update_completed("slow", 5000, "OK".to_string());

        panel.add_started("medium".to_string(), vec![]);
        panel.update_completed("medium", 1500, "OK".to_string());

        let stats = panel.calculate_stats();
        assert_eq!(stats.slowest_command, Some(("slow".to_string(), 5000)));
    }

    #[test]
    fn test_calculate_stats_most_frequent() {
        let mut panel = OperationsPanel::new();

        panel.add_started("remember".to_string(), vec![]);
        panel.add_started("remember".to_string(), vec![]);
        panel.add_started("remember".to_string(), vec![]);
        panel.add_started("recall".to_string(), vec![]);
        panel.add_started("recall".to_string(), vec![]);
        panel.add_started("evolve".to_string(), vec![]);

        let stats = panel.calculate_stats();
        assert_eq!(
            stats.most_frequent_command,
            Some(("remember".to_string(), 3))
        );
    }

    #[test]
    fn test_calculate_stats_per_command() {
        let mut panel = OperationsPanel::new();

        // Add multiple remember operations with different outcomes
        panel.add_started("remember".to_string(), vec![]);
        panel.update_completed("remember", 1000, "OK".to_string());

        panel.add_started("remember".to_string(), vec![]);
        panel.update_completed("remember", 2000, "OK".to_string());

        panel.add_started("remember".to_string(), vec![]);
        panel.update_failed("remember", "Error".to_string(), 500);

        let stats = panel.calculate_stats();
        let remember_stats = stats.command_stats.get("remember").unwrap();

        assert_eq!(remember_stats.count, 3);
        assert_eq!(remember_stats.success_count, 2);
        assert_eq!(remember_stats.failure_count, 1);
        assert_eq!(remember_stats.avg_duration_ms, 1166); // (1000+2000+500)/3 with integer division
    }

    // ===== Utility Function Tests =====

    #[test]
    fn test_format_relative_time() {
        let now = Utc::now();

        let two_sec_ago = now - Duration::seconds(2);
        assert!(OperationsPanel::format_relative_time(two_sec_ago).contains("s ago"));

        let five_min_ago = now - Duration::minutes(5);
        assert!(OperationsPanel::format_relative_time(five_min_ago).contains("m ago"));

        let two_hours_ago = now - Duration::hours(2);
        assert!(OperationsPanel::format_relative_time(two_hours_ago).contains("h ago"));

        let three_days_ago = now - Duration::days(3);
        assert!(OperationsPanel::format_relative_time(three_days_ago).contains("d ago"));
    }

    #[test]
    fn test_duration_color() {
        assert_eq!(
            OperationsPanel::duration_color(300),
            DashboardColors::PERF_FAST
        ); // Fast
        assert_eq!(
            OperationsPanel::duration_color(1000),
            DashboardColors::PERF_MEDIUM
        ); // Medium
        assert_eq!(
            OperationsPanel::duration_color(3000),
            DashboardColors::PERF_SLOW
        ); // Slow
    }

    #[test]
    fn test_truncate() {
        assert_eq!(OperationsPanel::truncate("short", 10), "short");
        assert_eq!(
            OperationsPanel::truncate("this is a very long string", 10),
            "this is..."
        );
        assert_eq!(OperationsPanel::truncate("exact", 5), "exact");
    }

    // ===== Edge Case Tests =====

    #[test]
    fn test_all_operations_same_command() {
        let mut panel = OperationsPanel::new();
        for _ in 0..10 {
            panel.add_started("remember".to_string(), vec![]);
        }

        let groups = panel.group_operations();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].1.len(), 10);

        let stats = panel.calculate_stats();
        assert_eq!(stats.total_count, 10);
        assert_eq!(
            stats.most_frequent_command,
            Some(("remember".to_string(), 10))
        );
    }

    #[test]
    fn test_operations_with_no_duration() {
        let mut panel = OperationsPanel::new();
        panel.add_started("running".to_string(), vec![]);

        let stats = panel.calculate_stats();
        assert_eq!(stats.avg_duration_ms, 0);
        assert_eq!(stats.slowest_command, None);
    }

    #[test]
    fn test_filter_results_in_empty_set() {
        let mut panel = OperationsPanel::new();
        panel.add_started("remember".to_string(), vec![]);

        let filter = OperationFilter {
            command: Some("nonexistent".to_string()),
            ..Default::default()
        };
        panel.set_filter(filter);

        let filtered = panel.filtered_operations();
        assert_eq!(filtered.len(), 0);

        let stats = panel.calculate_stats();
        assert_eq!(stats.total_count, 0);
    }

    #[test]
    fn test_performance_with_many_operations() {
        let mut panel = OperationsPanel::new();

        // Add 100 operations
        for i in 0..100 {
            let cmd = format!("cmd{}", i % 10); // 10 different commands
            panel.add_started(cmd.clone(), vec![]);
            panel.update_completed(&cmd, 1000 + (i * 10), "OK".to_string());
        }

        // Test grouping performance
        let start = std::time::Instant::now();
        let groups = panel.group_operations();
        let grouping_time = start.elapsed();

        assert_eq!(groups.len(), 10);
        assert!(
            grouping_time.as_millis() < 100,
            "Grouping should be fast even with 100 ops"
        );

        // Test statistics performance
        let start = std::time::Instant::now();
        let stats = panel.calculate_stats();
        let stats_time = start.elapsed();

        assert_eq!(stats.total_count, 100);
        assert!(
            stats_time.as_millis() < 100,
            "Stats calculation should be fast even with 100 ops"
        );
    }
}
