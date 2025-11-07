//! Events panel - Scrollable event log with color-coded entries

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

/// Event log entry
#[derive(Debug, Clone)]
pub struct EventEntry {
    pub text: String,
    pub timestamp: Option<String>,
}

/// Event log panel widget
pub struct EventLogPanel {
    events: Vec<EventEntry>,
    max_events: usize,
    title: String,
    scroll_offset: usize,
}

impl EventLogPanel {
    /// Create new event log panel
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            max_events: 1000,
            title: "Event Log".to_string(),
            scroll_offset: 0,
        }
    }

    /// Set custom title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set maximum events to retain
    pub fn max_events(mut self, max: usize) -> Self {
        self.max_events = max;
        self
    }

    /// Add event to log
    pub fn add_event(&mut self, text: impl Into<String>) {
        self.events.push(EventEntry {
            text: text.into(),
            timestamp: None,
        });

        // Trim old events
        if self.events.len() > self.max_events {
            self.events.drain(0..self.events.len() - self.max_events);
        }

        // Auto-scroll to bottom when new event arrives
        self.scroll_offset = 0;
    }

    /// Add event with timestamp
    pub fn add_event_with_timestamp(&mut self, text: impl Into<String>, timestamp: impl Into<String>) {
        self.events.push(EventEntry {
            text: text.into(),
            timestamp: Some(timestamp.into()),
        });

        if self.events.len() > self.max_events {
            self.events.drain(0..self.events.len() - self.max_events);
        }

        self.scroll_offset = 0;
    }

    /// Clear all events
    pub fn clear(&mut self) {
        self.events.clear();
        self.scroll_offset = 0;
    }

    /// Scroll up (increase offset)
    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(amount);
        // Don't scroll beyond available events
        if self.scroll_offset >= self.events.len() {
            self.scroll_offset = self.events.len().saturating_sub(1);
        }
    }

    /// Scroll down (decrease offset)
    pub fn scroll_down(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    /// Scroll to top
    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = self.events.len().saturating_sub(1);
    }

    /// Scroll to bottom
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    /// Get color for event text based on content (btop-inspired)
    fn event_color(text: &str) -> Color {
        if text.contains("session_started") {
            Color::Green
        } else if text.contains("heartbeat") {
            Color::Blue
        } else if text.contains("deadlock_detected") {
            Color::Red // Critical
        } else if text.contains("review_failed") {
            Color::LightRed // Warning
        } else if text.contains("error") || text.contains("failed") || text.contains("agent_failed") {
            Color::Red
        } else if text.contains("phase_changed") {
            Color::Magenta // Important
        } else if text.contains("work_item_retried") {
            Color::Yellow // Notice
        } else if text.contains("context_checkpointed") {
            Color::Cyan // Info
        } else if text.contains("agent_started") || text.contains("agent_completed") {
            Color::LightGreen // Success
        } else if text.contains("memory_stored") || text.contains("memory_recalled") {
            Color::Blue
        } else if text.contains("skill_loaded") || text.contains("skill_used") {
            Color::Magenta
        } else {
            Color::White
        }
    }

    /// Render the event log panel
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let available_height = area.height.saturating_sub(2) as usize; // Subtract borders

        let items: Vec<ListItem> = if self.events.is_empty() {
            vec![ListItem::new(Line::from(vec![Span::styled(
                "No events yet",
                Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::ITALIC),
            )]))]
        } else {
            // Get visible events (reverse order: most recent at top)
            let start_idx = self.scroll_offset;
            let end_idx = (start_idx + available_height).min(self.events.len());

            self.events
                .iter()
                .rev()
                .skip(start_idx)
                .take(available_height)
                .map(|entry| {
                    let color = Self::event_color(&entry.text);
                    let mut spans = Vec::new();

                    // Add timestamp if available
                    if let Some(ts) = &entry.timestamp {
                        spans.push(Span::styled(
                            format!("[{}] ", ts),
                            Style::default().fg(Color::DarkGray),
                        ));
                    }

                    spans.push(Span::styled(&entry.text, Style::default().fg(color)));

                    ListItem::new(Line::from(spans))
                })
                .collect()
        };

        let title = if self.scroll_offset > 0 {
            format!("{} (↑ {} events hidden)", self.title, self.scroll_offset)
        } else {
            format!("{} ({} total)", self.title, self.events.len())
        };

        let list = List::new(items).block(Block::default().title(title).borders(Borders::ALL));

        frame.render_widget(list, area);
    }

    /// Get total event count
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Get current scroll offset
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }
}

impl Default for EventLogPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_log_creation() {
        let panel = EventLogPanel::new();
        assert_eq!(panel.event_count(), 0);
        assert_eq!(panel.scroll_offset(), 0);
    }

    #[test]
    fn test_add_event() {
        let mut panel = EventLogPanel::new();
        panel.add_event("Test event 1");
        panel.add_event("Test event 2");
        assert_eq!(panel.event_count(), 2);
    }

    #[test]
    fn test_max_events_limit() {
        let mut panel = EventLogPanel::new().max_events(3);

        for i in 0..5 {
            panel.add_event(format!("Event {}", i));
        }

        assert_eq!(panel.event_count(), 3); // Only last 3
    }

    #[test]
    fn test_clear() {
        let mut panel = EventLogPanel::new();
        panel.add_event("Event 1");
        panel.add_event("Event 2");
        assert_eq!(panel.event_count(), 2);

        panel.clear();
        assert_eq!(panel.event_count(), 0);
    }

    #[test]
    fn test_scrolling() {
        let mut panel = EventLogPanel::new();
        for i in 0..10 {
            panel.add_event(format!("Event {}", i));
        }

        assert_eq!(panel.scroll_offset(), 0);

        panel.scroll_up(3);
        assert_eq!(panel.scroll_offset(), 3);

        panel.scroll_down(1);
        assert_eq!(panel.scroll_offset(), 2);

        panel.scroll_to_bottom();
        assert_eq!(panel.scroll_offset(), 0);

        panel.scroll_to_top();
        assert_eq!(panel.scroll_offset(), 9);
    }

    #[test]
    fn test_scroll_bounds() {
        let mut panel = EventLogPanel::new();
        panel.add_event("Event 1");

        // Try to scroll beyond bounds
        panel.scroll_up(100);
        assert_eq!(panel.scroll_offset(), 0); // Capped

        panel.scroll_down(100);
        assert_eq!(panel.scroll_offset(), 0); // Can't go negative
    }

    #[test]
    fn test_event_colors() {
        assert_eq!(EventLogPanel::event_color("session_started"), Color::Green);
        assert_eq!(EventLogPanel::event_color("heartbeat"), Color::Blue);
        assert_eq!(EventLogPanel::event_color("error occurred"), Color::Red);
        assert_eq!(EventLogPanel::event_color("phase_changed"), Color::Magenta);
    }

    #[test]
    fn test_add_event_with_timestamp() {
        let mut panel = EventLogPanel::new();
        panel.add_event_with_timestamp("Test event", "12:34:56");

        assert_eq!(panel.event_count(), 1);
        assert_eq!(panel.events[0].timestamp, Some("12:34:56".to_string()));
    }
}
