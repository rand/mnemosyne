//! Memory panel for viewing and managing memories from ICS
//!
//! Provides inline access to Mnemosyne memories with:
//! - Memory list with search
//! - Quick preview
//! - Create/edit actions
//! - Progressive disclosure (hidden by default)

use crate::types::MemoryNote;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, StatefulWidget, Widget},
};

/// Memory panel state
pub struct MemoryPanelState {
    /// List state for memory selection
    list_state: ListState,
    /// Search query
    search_query: String,
    /// Whether panel is visible
    visible: bool,
    /// Selected memory for preview
    selected_memory: Option<MemoryNote>,
    /// Whether memories are currently loading
    loading: bool,
}

impl Default for MemoryPanelState {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryPanelState {
    /// Create new memory panel state
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
            search_query: String::new(),
            visible: false,
            selected_memory: None,
            loading: false,
        }
    }

    /// Toggle panel visibility
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

    /// Check if panel is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Set search query
    pub fn set_search(&mut self, query: String) {
        self.search_query = query;
    }

    /// Get search query
    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    /// Select next memory
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

    /// Select previous memory
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

    /// Set selected memory for preview
    pub fn set_selected_memory(&mut self, memory: Option<MemoryNote>) {
        self.selected_memory = memory;
    }

    /// Get selected memory
    pub fn selected_memory(&self) -> Option<&MemoryNote> {
        self.selected_memory.as_ref()
    }

    /// Set loading state
    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }

    /// Check if loading
    pub fn is_loading(&self) -> bool {
        self.loading
    }
}

/// Memory panel widget
pub struct MemoryPanel<'a> {
    /// Memories to display
    memories: &'a [MemoryNote],
    /// Whether to show preview
    show_preview: bool,
}

impl<'a> MemoryPanel<'a> {
    /// Create new memory panel
    pub fn new(memories: &'a [MemoryNote]) -> Self {
        Self {
            memories,
            show_preview: true,
        }
    }

    /// Set whether to show preview
    pub fn show_preview(mut self, show: bool) -> Self {
        self.show_preview = show;
        self
    }
}

impl<'a> StatefulWidget for MemoryPanel<'a> {
    type State = MemoryPanelState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if !state.visible {
            return;
        }

        // Create layout
        let chunks = if self.show_preview && state.selected_memory.is_some() {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(60), // Memory list
                    Constraint::Percentage(40), // Preview
                ])
                .split(area)
                .to_vec()
        } else {
            vec![area]
        };

        // Render memory list
        self.render_memory_list(chunks[0], buf, state);

        // Render preview if enabled
        if self.show_preview && chunks.len() > 1 {
            if let Some(memory) = &state.selected_memory {
                self.render_preview(chunks[1], buf, memory);
            }
        }
    }
}

impl<'a> MemoryPanel<'a> {
    /// Render memory list
    fn render_memory_list(&self, area: Rect, buf: &mut Buffer, state: &mut MemoryPanelState) {
        let title = if state.search_query.is_empty() {
            format!(" Memories ({}) ", self.memories.len())
        } else {
            format!(" Memories ({}) - Search: {} ", self.memories.len(), state.search_query)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().fg(Color::Rgb(180, 180, 200)));

        // Show loading state if loading
        if state.loading {
            let inner = block.inner(area);
            block.render(area, buf);

            let loading_text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "Loading memories...",
                    Style::default().fg(Color::Rgb(160, 160, 180)),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "◐",
                    Style::default().fg(Color::Rgb(140, 140, 160)),
                )),
            ];

            let paragraph = Paragraph::new(loading_text).alignment(ratatui::layout::Alignment::Center);
            paragraph.render(inner, buf);
            return;
        }

        // Filter memories based on search
        let filtered_memories: Vec<&MemoryNote> = if state.search_query.is_empty() {
            self.memories.iter().collect()
        } else {
            self.memories
                .iter()
                .filter(|m| {
                    m.content.to_lowercase().contains(&state.search_query.to_lowercase())
                        || m.tags.iter().any(|tag| tag.to_lowercase().contains(&state.search_query.to_lowercase()))
                        || format!("{:?}", m.memory_type).to_lowercase().contains(&state.search_query.to_lowercase())
                })
                .collect()
        };

        // Show empty state if no memories
        if filtered_memories.is_empty() {
            let inner = block.inner(area);
            block.render(area, buf);

            let empty_msg = if !state.search_query.is_empty() {
                "No memories match your search"
            } else if self.memories.is_empty() {
                "No memories loaded yet"
            } else {
                "No results"
            };

            let empty_text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    empty_msg,
                    Style::default().fg(Color::Rgb(140, 140, 160)),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Relevant memories will appear here",
                    Style::default().fg(Color::Rgb(120, 120, 140)),
                )),
            ];

            let paragraph = Paragraph::new(empty_text).alignment(ratatui::layout::Alignment::Center);
            paragraph.render(inner, buf);
            return;
        }

        // Create list items
        let items: Vec<ListItem> = filtered_memories
            .iter()
            .map(|memory| {
                let importance_indicator = "●".repeat(memory.importance as usize);
                let category_style = Style::default().fg(Color::Rgb(160, 180, 160));
                let importance_style = Style::default().fg(match memory.importance {
                    0..=3 => Color::Rgb(140, 140, 140),
                    4..=6 => Color::Rgb(180, 180, 120),
                    7..=8 => Color::Rgb(200, 160, 100),
                    _ => Color::Rgb(200, 120, 120),
                });

                let content_preview = if memory.content.len() > 60 {
                    format!("{}...", &memory.content[..57])
                } else {
                    memory.content.clone()
                };

                let category = memory.tags.first()
                    .map(|s| s.as_str())
                    .unwrap_or_else(|| match memory.memory_type {
                        crate::types::MemoryType::ArchitectureDecision => "Architecture",
                        crate::types::MemoryType::CodePattern => "Pattern",
                        crate::types::MemoryType::BugFix => "BugFix",
                        crate::types::MemoryType::Configuration => "Config",
                        crate::types::MemoryType::Constraint => "Constraint",
                        crate::types::MemoryType::Entity => "Entity",
                        crate::types::MemoryType::Insight => "Insight",
                        crate::types::MemoryType::Reference => "Reference",
                        crate::types::MemoryType::Preference => "Preference",
                        crate::types::MemoryType::AgentEvent => "AgentEvent",
                    });

                let line = Line::from(vec![
                    Span::styled(category, category_style),
                    Span::raw(" "),
                    Span::styled(importance_indicator, importance_style),
                    Span::raw(" "),
                    Span::raw(content_preview),
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

    /// Render memory preview
    fn render_preview(&self, area: Rect, buf: &mut Buffer, memory: &MemoryNote) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Preview ")
            .style(Style::default().fg(Color::Rgb(180, 180, 200)));

        let inner = block.inner(area);
        block.render(area, buf);

        let category = memory.tags.first()
            .map(|s| s.as_str())
            .unwrap_or_else(|| match memory.memory_type {
                crate::types::MemoryType::ArchitectureDecision => "Architecture",
                crate::types::MemoryType::CodePattern => "Pattern",
                crate::types::MemoryType::BugFix => "BugFix",
                crate::types::MemoryType::Configuration => "Config",
                crate::types::MemoryType::Constraint => "Constraint",
                crate::types::MemoryType::Entity => "Entity",
                crate::types::MemoryType::Insight => "Insight",
                crate::types::MemoryType::Reference => "Reference",
                crate::types::MemoryType::Preference => "Preference",
                crate::types::MemoryType::AgentEvent => "AgentEvent",
            });

        // Format memory details
        let lines = vec![
            Line::from(vec![
                Span::styled("Type: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(category),
            ]),
            Line::from(vec![
                Span::styled("Importance: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}/10", memory.importance)),
            ]),
            Line::from(""),
            Line::from(Span::styled("Content:", Style::default().add_modifier(Modifier::BOLD))),
            Line::from(""),
        ];

        // Split content into lines
        let content_lines: Vec<Line> = memory
            .content
            .lines()
            .map(|line| Line::from(line))
            .collect();

        let all_lines: Vec<Line> = lines.into_iter().chain(content_lines).collect();

        let paragraph = Paragraph::new(all_lines).style(Style::default());
        paragraph.render(inner, buf);
    }
}

/// Quick actions for memories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryAction {
    /// Create new memory from selection
    CreateFromSelection,
    /// Edit selected memory
    Edit,
    /// Delete selected memory
    Delete,
    /// Insert memory reference
    InsertReference,
    /// Search memories
    Search,
}

impl MemoryAction {
    /// Get keyboard shortcut
    pub fn shortcut(&self) -> &'static str {
        match self {
            MemoryAction::CreateFromSelection => "Ctrl+M",
            MemoryAction::Edit => "Enter",
            MemoryAction::Delete => "Del",
            MemoryAction::InsertReference => "Ctrl+I",
            MemoryAction::Search => "Ctrl+F",
        }
    }

    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            MemoryAction::CreateFromSelection => "Create memory from selection",
            MemoryAction::Edit => "Edit memory",
            MemoryAction::Delete => "Delete memory",
            MemoryAction::InsertReference => "Insert reference",
            MemoryAction::Search => "Search memories",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_panel_state() {
        let mut state = MemoryPanelState::new();

        assert!(!state.is_visible());

        state.toggle();
        assert!(state.is_visible());

        state.hide();
        assert!(!state.is_visible());

        state.show();
        assert!(state.is_visible());
    }

    #[test]
    fn test_memory_panel_selection() {
        let mut state = MemoryPanelState::new();

        assert_eq!(state.selected(), None);

        state.select_next(5);
        assert_eq!(state.selected(), Some(0));

        state.select_next(5);
        assert_eq!(state.selected(), Some(1));

        state.select_previous();
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn test_memory_panel_search() {
        let mut state = MemoryPanelState::new();

        assert_eq!(state.search_query(), "");

        state.set_search("test query".to_string());
        assert_eq!(state.search_query(), "test query");
    }

    #[test]
    fn test_memory_action_shortcuts() {
        let action = MemoryAction::CreateFromSelection;
        assert_eq!(action.shortcut(), "Ctrl+M");
        assert!(!action.description().is_empty());
    }

    #[test]
    fn test_memory_panel_loading_state() {
        let mut state = MemoryPanelState::new();

        // Initially not loading
        assert!(!state.is_loading());

        // Set loading
        state.set_loading(true);
        assert!(state.is_loading());

        // Clear loading
        state.set_loading(false);
        assert!(!state.is_loading());
    }
}
