//! Shared widget components

pub mod dialogs;

pub use dialogs::{ConfirmDialog, Dialog, DialogResult, InputDialog, PreviewDialog};

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

impl<'a> Default for StatusBar<'a> {
    fn default() -> Self {
        Self::new()
    }
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
            left_spans.push(Span::styled(
                *label,
                Style::default().add_modifier(Modifier::BOLD),
            ));
            left_spans.push(Span::raw(": "));
            left_spans.push(Span::raw(*value));
        }

        // Build right section
        let mut right_spans = Vec::new();
        for (i, (label, value)) in self.right_items.iter().enumerate() {
            if i > 0 {
                right_spans.push(Span::raw(" | "));
            }
            right_spans.push(Span::styled(
                *label,
                Style::default().add_modifier(Modifier::BOLD),
            ));
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
            spans.push(
                Span::raw(
                    " ".repeat(
                        (padding_width.saturating_sub(
                            spans.iter().map(|s| s.content.len()).sum::<usize>() as u16,
                        )) as usize,
                    ),
                ),
            );
            spans.extend(right_spans);
        }

        let line = Line::from(spans);
        Paragraph::new(line).style(self.style).render(area, buf);
    }
}

/// Command in the palette
#[derive(Debug, Clone)]
pub struct Command {
    /// Command identifier
    pub id: String,

    /// Display name
    pub name: String,

    /// Description
    pub description: String,

    /// Category
    pub category: CommandCategory,

    /// Keyboard shortcut (if any)
    pub shortcut: Option<String>,
}

impl Command {
    /// Create new command
    pub fn new(id: String, name: String, description: String, category: CommandCategory) -> Self {
        Self {
            id,
            name,
            description,
            category,
            shortcut: None,
        }
    }

    /// Set keyboard shortcut
    pub fn with_shortcut(mut self, shortcut: String) -> Self {
        self.shortcut = Some(shortcut);
        self
    }
}

/// Command category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCategory {
    /// File operations
    File,
    /// Editor operations
    Edit,
    /// Navigation
    Navigation,
    /// View/display
    View,
    /// Tools and utilities
    Tools,
    /// System operations
    System,
    /// ICS (Integrated Context Studio) operations
    Ics,
}

impl CommandCategory {
    /// Get category name
    pub fn name(&self) -> &str {
        match self {
            CommandCategory::File => "File",
            CommandCategory::Edit => "Edit",
            CommandCategory::Navigation => "Navigation",
            CommandCategory::View => "View",
            CommandCategory::Tools => "Tools",
            CommandCategory::System => "System",
            CommandCategory::Ics => "ICS",
        }
    }

    /// Get category color
    pub fn color(&self) -> Color {
        match self {
            CommandCategory::File => Color::Blue,
            CommandCategory::Edit => Color::Green,
            CommandCategory::Navigation => Color::Cyan,
            CommandCategory::View => Color::Magenta,
            CommandCategory::Tools => Color::Yellow,
            CommandCategory::System => Color::Red,
            CommandCategory::Ics => Color::LightCyan,
        }
    }
}

/// Enhanced command palette widget
pub struct CommandPalette {
    /// All available commands
    commands: Vec<Command>,

    /// Filtered commands (based on search)
    filtered: Vec<usize>,

    /// Search query
    query: String,

    /// Selected index in filtered list
    selected: usize,

    /// Whether palette is visible
    visible: bool,

    /// Recent commands (most recent first)
    recent: Vec<String>,

    /// Maximum recent commands to track
    max_recent: usize,
}

impl CommandPalette {
    /// Create new command palette
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            filtered: Vec::new(),
            query: String::new(),
            selected: 0,
            visible: false,
            recent: Vec::new(),
            max_recent: 10,
        }
    }

    /// Set commands
    pub fn with_commands(mut self, commands: Vec<Command>) -> Self {
        self.commands = commands;
        self.update_filter();
        self
    }

    /// Add command
    pub fn add_command(&mut self, command: Command) {
        self.commands.push(command);
        self.update_filter();
    }

    /// Show palette
    pub fn show(&mut self) {
        self.visible = true;
        self.query.clear();
        self.selected = 0;
        self.update_filter();
    }

    /// Hide palette
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        if self.visible {
            self.hide();
        } else {
            self.show();
        }
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Update search query
    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.selected = 0;
        self.update_filter();
    }

    /// Append to search query
    pub fn append_query(&mut self, ch: char) {
        self.query.push(ch);
        self.selected = 0;
        self.update_filter();
    }

    /// Backspace in search query
    pub fn backspace_query(&mut self) {
        self.query.pop();
        self.selected = 0;
        self.update_filter();
    }

    /// Clear search query
    pub fn clear_query(&mut self) {
        self.query.clear();
        self.selected = 0;
        self.update_filter();
    }

    /// Update filtered commands based on query
    fn update_filter(&mut self) {
        if self.query.is_empty() {
            // Show all commands
            self.filtered = (0..self.commands.len()).collect();
        } else {
            // Simple fuzzy matching - check if query chars appear in order
            let query_lower = self.query.to_lowercase();
            self.filtered = self
                .commands
                .iter()
                .enumerate()
                .filter(|(_, cmd)| {
                    let name_lower = cmd.name.to_lowercase();
                    let desc_lower = cmd.description.to_lowercase();

                    // Check if all query characters appear in order
                    let mut query_chars = query_lower.chars();
                    let mut current_char = query_chars.next();

                    for ch in name_lower.chars().chain(desc_lower.chars()) {
                        if Some(ch) == current_char {
                            current_char = query_chars.next();
                            if current_char.is_none() {
                                return true;
                            }
                        }
                    }

                    false
                })
                .map(|(i, _)| i)
                .collect();
        }

        // Clamp selected index
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
    }

    /// Select next command
    pub fn select_next(&mut self) {
        if self.selected < self.filtered.len().saturating_sub(1) {
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
    pub fn selected(&self) -> Option<&Command> {
        self.filtered
            .get(self.selected)
            .and_then(|&idx| self.commands.get(idx))
    }

    /// Execute selected command (add to recent)
    pub fn execute_selected(&mut self) -> Option<String> {
        if let Some(cmd) = self.selected() {
            let cmd_id = cmd.id.clone();

            // Add to recent (remove if already present, then add to front)
            self.recent.retain(|id| id != &cmd_id);
            self.recent.insert(0, cmd_id.clone());

            // Trim recent list
            if self.recent.len() > self.max_recent {
                self.recent.truncate(self.max_recent);
            }

            self.hide();
            Some(cmd_id)
        } else {
            None
        }
    }

    /// Get recent commands
    pub fn recent_commands(&self) -> &[String] {
        &self.recent
    }
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for &CommandPalette {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }

        // Create block with search query in title
        let title = if self.query.is_empty() {
            " Command Palette ".to_string()
        } else {
            format!(" Command Palette: {} ", self.query)
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );

        let inner = block.inner(area);
        block.render(area, buf);

        // Show filtered commands
        if self.filtered.is_empty() {
            let msg = if self.query.is_empty() {
                "No commands available"
            } else {
                "No matching commands"
            };
            let paragraph = Paragraph::new(msg).style(Style::default().fg(Color::DarkGray));
            paragraph.render(inner, buf);
            return;
        }

        // Build list items (Helix-style: simple and clean)
        let items: Vec<ListItem> = self
            .filtered
            .iter()
            .enumerate()
            .map(|(i, &cmd_idx)| {
                let cmd = &self.commands[cmd_idx];

                let is_selected = i == self.selected;
                let style = if is_selected {
                    Style::default()
                        .bg(Color::Blue)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                // Helix-style format: > command-name  Description
                let mut spans = Vec::new();

                // Selection indicator
                if is_selected {
                    spans.push(Span::styled(
                        "> ",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ));
                } else {
                    spans.push(Span::raw("  "));
                }

                // Command name
                spans.push(Span::styled(
                    &cmd.name,
                    Style::default().add_modifier(Modifier::BOLD),
                ));

                // Description (separated by two spaces for visual clarity)
                if !cmd.description.is_empty() {
                    spans.push(Span::raw("  "));
                    spans.push(Span::styled(
                        &cmd.description,
                        Style::default().fg(Color::Gray),
                    ));
                }

                ListItem::new(Line::from(spans)).style(style)
            })
            .collect();

        let list = List::new(items);
        list.render(inner, buf);
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

/// Help overlay widget - shows keybindings
pub struct HelpOverlay {
    /// Whether overlay is visible
    visible: bool,
    /// Context mode (for context-aware help)
    ics_mode: bool,
}

impl HelpOverlay {
    /// Create new help overlay
    pub fn new() -> Self {
        Self {
            visible: false,
            ics_mode: false,
        }
    }

    /// Show overlay
    pub fn show(&mut self, ics_mode: bool) {
        self.visible = true;
        self.ics_mode = ics_mode;
    }

    /// Hide overlay
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Toggle visibility
    pub fn toggle(&mut self, ics_mode: bool) {
        if self.visible {
            self.hide();
        } else {
            self.show(ics_mode);
        }
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

impl Default for HelpOverlay {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for &HelpOverlay {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }

        // Create centered help panel
        let help_width = 60.min(area.width.saturating_sub(4));
        let help_height = 20.min(area.height.saturating_sub(4));
        let help_x = (area.width.saturating_sub(help_width)) / 2;
        let help_y = (area.height.saturating_sub(help_height)) / 2;

        let help_area = Rect {
            x: help_x,
            y: help_y,
            width: help_width,
            height: help_height,
        };

        // Build help content based on mode
        let mut help_text = vec![
            Line::from(vec![Span::styled(
                "Keyboard Shortcuts",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
        ];

        if self.ics_mode {
            // ICS-specific help
            help_text.extend(vec![
                Line::from(vec![Span::styled(
                    "ICS Mode",
                    Style::default().add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  Ctrl+Enter", Style::default().fg(Color::Yellow)),
                    Span::raw("  Submit refined context to Claude"),
                ]),
                Line::from(vec![
                    Span::styled("  Ctrl+S", Style::default().fg(Color::Yellow)),
                    Span::raw("      Save edited document"),
                ]),
                Line::from(vec![
                    Span::styled("  Ctrl+E", Style::default().fg(Color::Yellow)),
                    Span::raw("      Focus/toggle ICS editor"),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "Patterns",
                    Style::default().add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  #file.rs", Style::default().fg(Color::Blue)),
                    Span::raw("    File reference (blue)"),
                ]),
                Line::from(vec![
                    Span::styled("  @symbol", Style::default().fg(Color::Green)),
                    Span::raw("     Symbol reference (green)"),
                ]),
                Line::from(vec![
                    Span::styled("  ?hole", Style::default().fg(Color::Yellow)),
                    Span::raw("       Typed hole (yellow)"),
                ]),
            ]);
        } else {
            // General TUI help
            help_text.extend(vec![
                Line::from(vec![Span::styled(
                    "Navigation",
                    Style::default().add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  Ctrl+P", Style::default().fg(Color::Yellow)),
                    Span::raw("      Open command palette"),
                ]),
                Line::from(vec![
                    Span::styled("  Ctrl+E", Style::default().fg(Color::Yellow)),
                    Span::raw("      Toggle ICS panel"),
                ]),
                Line::from(vec![
                    Span::styled("  Ctrl+D", Style::default().fg(Color::Yellow)),
                    Span::raw("      Toggle dashboard"),
                ]),
                Line::from(vec![
                    Span::styled("  Ctrl+Q", Style::default().fg(Color::Yellow)),
                    Span::raw("      Quit application"),
                ]),
            ]);
        }

        // Add common footer
        help_text.extend(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  ?/Esc", Style::default().fg(Color::Yellow)),
                Span::raw("       Close this help"),
            ]),
        ]);

        // Create paragraph with help text
        let help_block = Block::default()
            .title(" Help ")
            .borders(Borders::ALL)
            .border_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().bg(Color::Black));

        let help_paragraph = Paragraph::new(help_text)
            .block(help_block)
            .style(Style::default().fg(Color::White));

        // Render with background overlay
        let clear = Block::default().style(Style::default().bg(Color::Black));
        clear.render(area, buf);

        help_paragraph.render(help_area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_creation() {
        let cmd = Command::new(
            "open_file".to_string(),
            "Open File".to_string(),
            "Open a file for editing".to_string(),
            CommandCategory::File,
        )
        .with_shortcut("Ctrl+O".to_string());

        assert_eq!(cmd.id, "open_file");
        assert_eq!(cmd.name, "Open File");
        assert_eq!(cmd.shortcut, Some("Ctrl+O".to_string()));
    }

    #[test]
    fn test_command_category() {
        assert_eq!(CommandCategory::File.name(), "File");
        assert_eq!(CommandCategory::Edit.name(), "Edit");
        assert_eq!(CommandCategory::Navigation.name(), "Navigation");
        assert_eq!(CommandCategory::View.name(), "View");
        assert_eq!(CommandCategory::Tools.name(), "Tools");
        assert_eq!(CommandCategory::System.name(), "System");
    }

    #[test]
    fn test_palette_creation() {
        let palette = CommandPalette::new();
        assert!(!palette.is_visible());
        assert_eq!(palette.query, "");
    }

    #[test]
    fn test_palette_toggle() {
        let mut palette = CommandPalette::new();

        palette.toggle();
        assert!(palette.is_visible());

        palette.toggle();
        assert!(!palette.is_visible());
    }

    #[test]
    fn test_palette_add_command() {
        let mut palette = CommandPalette::new();

        let cmd = Command::new(
            "test".to_string(),
            "Test".to_string(),
            "Test command".to_string(),
            CommandCategory::Tools,
        );

        palette.add_command(cmd);
        assert_eq!(palette.commands.len(), 1);
        assert_eq!(palette.filtered.len(), 1);
    }

    #[test]
    fn test_palette_fuzzy_search() {
        let mut palette = CommandPalette::new();

        palette.add_command(Command::new(
            "open_file".to_string(),
            "Open File".to_string(),
            "Open a file".to_string(),
            CommandCategory::File,
        ));

        palette.add_command(Command::new(
            "save_file".to_string(),
            "Save File".to_string(),
            "Save current file".to_string(),
            CommandCategory::File,
        ));

        palette.add_command(Command::new(
            "close_window".to_string(),
            "Close Window".to_string(),
            "Close current window".to_string(),
            CommandCategory::View,
        ));

        // Show palette
        palette.show();
        assert_eq!(palette.filtered.len(), 3);

        // Filter by "file"
        palette.set_query("file".to_string());
        assert_eq!(palette.filtered.len(), 2); // open_file and save_file

        // Filter by "sf" (should match "save file")
        palette.set_query("sf".to_string());
        assert_eq!(palette.filtered.len(), 1);
        assert_eq!(palette.selected().unwrap().id, "save_file");

        // Clear query
        palette.clear_query();
        assert_eq!(palette.filtered.len(), 3);
    }

    #[test]
    fn test_palette_navigation() {
        let mut palette = CommandPalette::new();

        for i in 0..5 {
            palette.add_command(Command::new(
                format!("cmd{}", i),
                format!("Command {}", i),
                format!("Description {}", i),
                CommandCategory::Tools,
            ));
        }

        palette.show();
        assert_eq!(palette.selected, 0);

        palette.select_next();
        assert_eq!(palette.selected, 1);

        palette.select_next();
        palette.select_next();
        assert_eq!(palette.selected, 3);

        palette.select_previous();
        assert_eq!(palette.selected, 2);

        // Can't go beyond bounds
        for _ in 0..10 {
            palette.select_next();
        }
        assert_eq!(palette.selected, 4);

        for _ in 0..10 {
            palette.select_previous();
        }
        assert_eq!(palette.selected, 0);
    }

    #[test]
    fn test_palette_execute_command() {
        let mut palette = CommandPalette::new();

        palette.add_command(Command::new(
            "test_cmd".to_string(),
            "Test Command".to_string(),
            "Test".to_string(),
            CommandCategory::Tools,
        ));

        palette.show();
        assert!(palette.is_visible());

        let executed = palette.execute_selected();
        assert_eq!(executed, Some("test_cmd".to_string()));
        assert!(!palette.is_visible()); // Should hide after execution

        // Should be in recent commands
        assert_eq!(palette.recent_commands().len(), 1);
        assert_eq!(palette.recent_commands()[0], "test_cmd");
    }

    #[test]
    fn test_palette_recent_commands() {
        let mut palette = CommandPalette::new();
        palette.max_recent = 3;

        for i in 0..5 {
            palette.add_command(Command::new(
                format!("cmd{}", i),
                format!("Command {}", i),
                "Test".to_string(),
                CommandCategory::Tools,
            ));
        }

        // Execute commands
        palette.show();
        palette.selected = 0;
        palette.execute_selected();

        palette.show();
        palette.selected = 1;
        palette.execute_selected();

        palette.show();
        palette.selected = 2;
        palette.execute_selected();

        palette.show();
        palette.selected = 3;
        palette.execute_selected();

        // Should only keep last 3
        assert_eq!(palette.recent_commands().len(), 3);
        assert_eq!(palette.recent_commands()[0], "cmd3"); // Most recent first
        assert_eq!(palette.recent_commands()[1], "cmd2");
        assert_eq!(palette.recent_commands()[2], "cmd1");
    }

    #[test]
    fn test_palette_query_manipulation() {
        let mut palette = CommandPalette::new();

        palette.append_query('t');
        palette.append_query('e');
        palette.append_query('s');
        palette.append_query('t');
        assert_eq!(palette.query, "test");

        palette.backspace_query();
        assert_eq!(palette.query, "tes");

        palette.backspace_query();
        palette.backspace_query();
        assert_eq!(palette.query, "t");

        palette.clear_query();
        assert_eq!(palette.query, "");
    }
}
