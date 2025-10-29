//! Attribution display for ICS
//!
//! Shows who (human or agent) made each change in the document.
//! Uses CRDT attribution from the CrdtBuffer.

// Attribution is exported from editor module
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget, Widget},
};
use std::time::SystemTime;

/// Attribution entry with context
#[derive(Debug, Clone)]
pub struct AttributionEntry {
    /// Who made the change
    pub author: String,
    /// Change type
    pub change_type: ChangeType,
    /// When the change was made
    pub timestamp: SystemTime,
    /// Line number where change occurred
    pub line: usize,
    /// Brief description of the change
    pub description: String,
}

/// Type of change
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    /// Text insertion
    Insert,
    /// Text deletion
    Delete,
    /// Text replacement
    Replace,
}

impl ChangeType {
    /// Get color for change type
    pub fn color(&self) -> Color {
        match self {
            ChangeType::Insert => Color::Rgb(180, 200, 180),
            ChangeType::Delete => Color::Rgb(200, 140, 140),
            ChangeType::Replace => Color::Rgb(200, 180, 160),
        }
    }

    /// Get icon for change type
    pub fn icon(&self) -> &'static str {
        match self {
            ChangeType::Insert => "+",
            ChangeType::Delete => "-",
            ChangeType::Replace => "~",
        }
    }

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            ChangeType::Insert => "Insert",
            ChangeType::Delete => "Delete",
            ChangeType::Replace => "Replace",
        }
    }
}

/// Attribution panel state
#[derive(Debug, Clone)]
pub struct AttributionPanelState {
    /// List state for selection
    list_state: ListState,
    /// Whether panel is visible
    visible: bool,
    /// Filter by author
    author_filter: Option<String>,
}

impl Default for AttributionPanelState {
    fn default() -> Self {
        Self::new()
    }
}

impl AttributionPanelState {
    /// Create new attribution panel state
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
            visible: false,
            author_filter: None,
        }
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Show panel
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide panel
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Set author filter
    pub fn set_author_filter(&mut self, author: Option<String>) {
        self.author_filter = author;
    }

    /// Get author filter
    pub fn author_filter(&self) -> Option<&str> {
        self.author_filter.as_deref()
    }

    /// Select next attribution
    pub fn select_next(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => (i + 1).min(count - 1),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Select previous attribution
    pub fn select_previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Get selected index
    pub fn selected(&self) -> Option<usize> {
        self.list_state.selected()
    }
}

/// Attribution panel widget
pub struct AttributionPanel<'a> {
    /// Attribution entries to display
    entries: &'a [AttributionEntry],
}

impl<'a> AttributionPanel<'a> {
    /// Create new attribution panel
    pub fn new(entries: &'a [AttributionEntry]) -> Self {
        Self { entries }
    }
}

impl<'a> StatefulWidget for AttributionPanel<'a> {
    type State = AttributionPanelState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if !state.visible {
            return;
        }

        // Filter by author if set
        let filtered_entries: Vec<&AttributionEntry> = if let Some(ref filter) = state.author_filter {
            self.entries
                .iter()
                .filter(|e| e.author.contains(filter))
                .collect()
        } else {
            self.entries.iter().collect()
        };

        let title = if let Some(ref filter) = state.author_filter {
            format!(" Attribution - Filter: {} ", filter)
        } else {
            format!(" Attribution ({} changes) ", self.entries.len())
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().fg(Color::Rgb(180, 180, 200)));

        // Create list items
        let items: Vec<ListItem> = filtered_entries
            .iter()
            .map(|entry| {
                let icon = entry.change_type.icon();
                let color = entry.change_type.color();

                // Format author name
                let author_color = if entry.author.starts_with("agent:") {
                    Color::Rgb(180, 160, 200) // Purple for agents
                } else {
                    Color::Rgb(180, 200, 180) // Green for humans
                };

                let line = Line::from(vec![
                    Span::styled(icon, Style::default().fg(color)),
                    Span::raw(" "),
                    Span::styled(
                        &entry.author,
                        Style::default().fg(author_color).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" at Ln "),
                    Span::styled(
                        (entry.line + 1).to_string(),
                        Style::default().fg(Color::Rgb(160, 160, 160)),
                    ),
                    Span::raw(": "),
                    Span::raw(&entry.description),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(40, 40, 50))
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        StatefulWidget::render(list, area, buf, &mut state.list_state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attribution_panel_state() {
        let mut state = AttributionPanelState::new();
        assert!(!state.is_visible());

        state.toggle();
        assert!(state.is_visible());

        state.hide();
        assert!(!state.is_visible());

        state.show();
        assert!(state.is_visible());
    }

    #[test]
    fn test_attribution_selection() {
        let mut state = AttributionPanelState::new();
        assert_eq!(state.selected(), None);

        state.select_next(5);
        assert_eq!(state.selected(), Some(0));

        state.select_next(5);
        assert_eq!(state.selected(), Some(1));

        state.select_previous();
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn test_author_filter() {
        let mut state = AttributionPanelState::new();
        assert_eq!(state.author_filter(), None);

        state.set_author_filter(Some("agent:semantic".to_string()));
        assert_eq!(state.author_filter(), Some("agent:semantic"));

        state.set_author_filter(None);
        assert_eq!(state.author_filter(), None);
    }

    #[test]
    fn test_change_type_properties() {
        for change_type in [ChangeType::Insert, ChangeType::Delete, ChangeType::Replace] {
            let _ = change_type.color();
            assert!(!change_type.icon().is_empty());
            assert!(!change_type.name().is_empty());
        }
    }

    #[test]
    fn test_attribution_entry() {
        let entry = AttributionEntry {
            author: "agent:semantic".to_string(),
            change_type: ChangeType::Insert,
            timestamp: SystemTime::now(),
            line: 5,
            description: "Added documentation".to_string(),
        };

        assert_eq!(entry.author, "agent:semantic");
        assert_eq!(entry.change_type, ChangeType::Insert);
        assert_eq!(entry.line, 5);
    }
}
