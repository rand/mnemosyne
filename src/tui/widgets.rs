//! Shared widget components

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget},
};

/// Status bar widget
pub struct StatusBar<'a> {
    /// Left-aligned items
    left_items: Vec<(&'a str, &'a str)>,

    /// Right-aligned items
    right_items: Vec<(&'a str, &'a str)>,

    /// Style
    style: Style,
}

impl<'a> StatusBar<'a> {
    /// Create new status bar
    pub fn new() -> Self {
        Self {
            left_items: Vec::new(),
            right_items: Vec::new(),
            style: Style::default().bg(Color::DarkGray).fg(Color::White),
        }
    }

    /// Add left-aligned item
    pub fn left_item(mut self, label: &'a str, value: &'a str) -> Self {
        self.left_items.push((label, value));
        self
    }

    /// Add right-aligned item
    pub fn right_item(mut self, label: &'a str, value: &'a str) -> Self {
        self.right_items.push((label, value));
        self
    }

    /// Set style
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl<'a> Widget for StatusBar<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Build left section
        let mut left_spans = Vec::new();
        for (i, (label, value)) in self.left_items.iter().enumerate() {
            if i > 0 {
                left_spans.push(Span::raw(" | "));
            }
            left_spans.push(Span::styled(*label, Style::default().add_modifier(Modifier::BOLD)));
            left_spans.push(Span::raw(": "));
            left_spans.push(Span::raw(*value));
        }

        // Build right section
        let mut right_spans = Vec::new();
        for (i, (label, value)) in self.right_items.iter().enumerate() {
            if i > 0 {
                right_spans.push(Span::raw(" | "));
            }
            right_spans.push(Span::styled(*label, Style::default().add_modifier(Modifier::BOLD)));
            right_spans.push(Span::raw(": "));
            right_spans.push(Span::raw(*value));
        }

        let right_text: String = right_spans.iter().map(|s| s.content.as_ref()).collect();
        let right_width = right_text.len() as u16;

        // Calculate padding
        let padding_width = area.width.saturating_sub(right_width);

        // Combine sections
        let mut spans = left_spans;
        if right_width > 0 && padding_width > 0 {
            spans.push(Span::raw(" ".repeat((padding_width.saturating_sub(spans.iter().map(|s| s.content.len()).sum::<usize>() as u16)) as usize)));
            spans.extend(right_spans);
        }

        let line = Line::from(spans);
        Paragraph::new(line).style(self.style).render(area, buf);
    }
}

/// Command palette widget (stub for now)
pub struct CommandPalette {
    /// Commands
    commands: Vec<String>,

    /// Selected index
    selected: usize,
}

impl CommandPalette {
    /// Create new command palette
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            selected: 0,
        }
    }

    /// Set commands
    pub fn commands(mut self, commands: Vec<String>) -> Self {
        self.commands = commands;
        self
    }

    /// Select next command
    pub fn select_next(&mut self) {
        if self.selected < self.commands.len().saturating_sub(1) {
            self.selected += 1;
        }
    }

    /// Select previous command
    pub fn select_previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Get selected command
    pub fn selected(&self) -> Option<&str> {
        self.commands.get(self.selected).map(|s| s.as_str())
    }
}

impl Widget for &CommandPalette {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let items: Vec<ListItem> = self
            .commands
            .iter()
            .enumerate()
            .map(|(i, cmd)| {
                let style = if i == self.selected {
                    Style::default().bg(Color::Blue).fg(Color::White)
                } else {
                    Style::default()
                };
                ListItem::new(cmd.as_str()).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().title("Commands").borders(Borders::ALL));

        list.render(area, buf);
    }
}

/// Scrollable list widget
pub struct ScrollableList<'a> {
    /// Items
    items: Vec<String>,

    /// Title
    title: &'a str,

    /// Scroll offset
    scroll: usize,
}

impl<'a> ScrollableList<'a> {
    /// Create new scrollable list
    pub fn new(title: &'a str) -> Self {
        Self {
            items: Vec::new(),
            title,
            scroll: 0,
        }
    }

    /// Set items
    pub fn items(mut self, items: Vec<String>) -> Self {
        self.items = items;
        self
    }

    /// Scroll down
    pub fn scroll_down(&mut self) {
        if self.scroll < self.items.len().saturating_sub(1) {
            self.scroll += 1;
        }
    }

    /// Scroll up
    pub fn scroll_up(&mut self) {
        if self.scroll > 0 {
            self.scroll -= 1;
        }
    }
}

impl<'a> Widget for &ScrollableList<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let visible_height = area.height.saturating_sub(2) as usize; // Account for borders
        let visible_items: Vec<ListItem> = self
            .items
            .iter()
            .skip(self.scroll)
            .take(visible_height)
            .map(|item| ListItem::new(item.as_str()))
            .collect();

        let list = List::new(visible_items)
            .block(Block::default().title(self.title).borders(Borders::ALL));

        list.render(area, buf);
    }
}
