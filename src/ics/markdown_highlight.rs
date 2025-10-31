//! Markdown Highlighting - Hybrid Syntax + Semantic
//!
//! Combines tree-sitter syntax highlighting with semantic pattern detection
//! for an enhanced ICS editing experience.
//!
//! # Highlighting Priority
//! 1. **Semantic patterns** (ICS-specific): `#file`, `@symbol`, `?hole`
//! 2. **Syntax highlighting** (tree-sitter): headings, code blocks, lists, emphasis
//! 3. **Plain text** (fallback)
//!
//! # Features
//! - Markdown-first: Optimized for normal text and markdown
//! - Real-time: Fast enough for interactive editing
//! - Composable: Semantic and syntax layers work together

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use tree_sitter::Parser;

/// Highlighted span with style and text
#[derive(Debug, Clone)]
pub struct HighlightedSpan {
    /// Text content
    pub text: String,
    /// Style to apply
    pub style: Style,
    /// Source of highlighting (for debugging/priority)
    pub source: HighlightSource,
}

/// Source of highlighting (determines priority)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HighlightSource {
    /// Plain text (lowest priority)
    Plain = 0,
    /// Tree-sitter syntax
    Syntax = 1,
    /// Semantic pattern (highest priority)
    Semantic = 2,
}

/// Markdown highlighter with hybrid syntax + semantic
pub struct MarkdownHighlighter {
    /// Tree-sitter parser for markdown
    parser: Parser,
    /// Whether syntax highlighting is enabled
    syntax_enabled: bool,
    /// Whether semantic highlighting is enabled
    semantic_enabled: bool,
}

impl MarkdownHighlighter {
    /// Create new markdown highlighter
    pub fn new() -> anyhow::Result<Self> {
        let mut parser = Parser::new();

        // Load markdown grammar
        parser
            .set_language(&tree_sitter_md::LANGUAGE.into())
            .map_err(|e| anyhow::anyhow!("Failed to load markdown grammar: {}", e))?;

        Ok(Self {
            parser,
            syntax_enabled: true,
            semantic_enabled: true,
        })
    }

    /// Enable/disable syntax highlighting
    pub fn set_syntax_enabled(&mut self, enabled: bool) {
        self.syntax_enabled = enabled;
    }

    /// Check if syntax highlighting is enabled
    pub fn is_syntax_enabled(&self) -> bool {
        self.syntax_enabled
    }

    /// Enable/disable semantic highlighting
    pub fn set_semantic_enabled(&mut self, enabled: bool) {
        self.semantic_enabled = enabled;
    }

    /// Check if semantic highlighting is enabled
    pub fn is_semantic_enabled(&self) -> bool {
        self.semantic_enabled
    }

    /// Highlight a line of markdown text
    ///
    /// Returns a ratatui Line with styled spans
    pub fn highlight_line(&mut self, text: &str) -> Line<'static> {
        let mut spans = Vec::new();

        // Layer 1: Semantic patterns (highest priority)
        if self.semantic_enabled {
            if let Some(semantic_spans) = self.highlight_semantic_patterns(text) {
                return Line::from(semantic_spans);
            }
        }

        // Layer 2: Tree-sitter syntax highlighting
        if self.syntax_enabled {
            if let Ok(syntax_spans) = self.highlight_syntax(text) {
                if !syntax_spans.is_empty() {
                    return Line::from(syntax_spans);
                }
            }
        }

        // Layer 3: Plain text fallback
        spans.push(Span::raw(text.to_string()));
        Line::from(spans)
    }

    /// Highlight ICS semantic patterns
    ///
    /// Patterns:
    /// - `#path/to/file.rs` - File references (blue)
    /// - `@symbol_name` - Symbol references (green)
    /// - `?interface_name` - Typed holes (yellow)
    fn highlight_semantic_patterns(&self, text: &str) -> Option<Vec<Span<'static>>> {
        let mut spans = Vec::new();
        let mut last_pos = 0;
        let mut found_pattern = false;

        // Scan for patterns
        for (idx, ch) in text.char_indices() {
            match ch {
                '#' | '@' | '?' => {
                    // Add text before pattern
                    if idx > last_pos {
                        spans.push(Span::raw(text[last_pos..idx].to_string()));
                    }

                    // Extract pattern (until whitespace or end)
                    let pattern_start = idx;
                    let pattern_end = text[idx..]
                        .find(|c: char| c.is_whitespace() || c == ',' || c == '.' || c == ')')
                        .map(|pos| idx + pos)
                        .unwrap_or(text.len());

                    let pattern = &text[pattern_start..pattern_end];

                    // Style based on pattern type
                    let style = match ch {
                        '#' => Style::default()
                            .fg(Color::Blue)
                            .add_modifier(Modifier::BOLD),
                        '@' => Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                        '?' => Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                        _ => Style::default(),
                    };

                    spans.push(Span::styled(pattern.to_string(), style));
                    last_pos = pattern_end;
                    found_pattern = true;
                }
                _ => {}
            }
        }

        // Add remaining text
        if last_pos < text.len() {
            spans.push(Span::raw(text[last_pos..].to_string()));
        }

        if found_pattern {
            Some(spans)
        } else {
            None
        }
    }

    /// Highlight using tree-sitter syntax
    fn highlight_syntax(&mut self, text: &str) -> anyhow::Result<Vec<Span<'static>>> {
        let tree = self
            .parser
            .parse(text, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse markdown"))?;

        let root_node = tree.root_node();
        let mut spans = Vec::new();
        let mut last_pos = 0;

        // Walk the tree and style nodes
        self.walk_tree(root_node, text, &mut spans, &mut last_pos)?;

        // Add any remaining text
        if last_pos < text.len() {
            spans.push(Span::raw(text[last_pos..].to_string()));
        }

        Ok(spans)
    }

    /// Walk tree-sitter tree and extract styled spans
    fn walk_tree(
        &self,
        node: tree_sitter::Node,
        text: &str,
        spans: &mut Vec<Span<'static>>,
        last_pos: &mut usize,
    ) -> anyhow::Result<()> {
        let start = node.start_byte();
        let end = node.end_byte();

        // Add text before this node
        if start > *last_pos {
            spans.push(Span::raw(text[*last_pos..start].to_string()));
        }

        // Style based on node type
        let style = self.style_for_node(&node);
        let node_text = &text[start..end];

        // Handle different node types
        match node.kind() {
            // Headings
            "atx_heading" | "setext_heading" => {
                spans.push(Span::styled(
                    node_text.to_string(),
                    style.add_modifier(Modifier::BOLD),
                ));
                *last_pos = end;
                return Ok(());
            }
            // Code blocks
            "fenced_code_block" | "indented_code_block" => {
                spans.push(Span::styled(node_text.to_string(), style));
                *last_pos = end;
                return Ok(());
            }
            // Inline code
            "code_span" => {
                spans.push(Span::styled(node_text.to_string(), style));
                *last_pos = end;
                return Ok(());
            }
            // Emphasis
            "emphasis" => {
                spans.push(Span::styled(
                    node_text.to_string(),
                    style.add_modifier(Modifier::ITALIC),
                ));
                *last_pos = end;
                return Ok(());
            }
            // Strong emphasis
            "strong" | "strong_emphasis" => {
                spans.push(Span::styled(
                    node_text.to_string(),
                    style.add_modifier(Modifier::BOLD),
                ));
                *last_pos = end;
                return Ok(());
            }
            // Lists
            "list_item" | "list_marker" => {
                spans.push(Span::styled(
                    node_text.to_string(),
                    Style::default().fg(Color::Cyan),
                ));
                *last_pos = end;
                return Ok(());
            }
            _ => {
                // Recurse into children for other node types
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.walk_tree(child, text, spans, last_pos)?;
                }
            }
        }

        Ok(())
    }

    /// Get style for a tree-sitter node
    fn style_for_node(&self, node: &tree_sitter::Node) -> Style {
        match node.kind() {
            // Headings
            "atx_heading" | "setext_heading" => {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            }
            // Code
            "fenced_code_block" | "indented_code_block" | "code_span" => {
                Style::default().fg(Color::Green)
            }
            // Emphasis
            "emphasis" => Style::default().add_modifier(Modifier::ITALIC),
            "strong" | "strong_emphasis" => Style::default().add_modifier(Modifier::BOLD),
            // Lists
            "list_item" | "list_marker" => Style::default().fg(Color::Yellow),
            // Links
            "link" | "uri" => Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED),
            // Default
            _ => Style::default(),
        }
    }
}

impl Default for MarkdownHighlighter {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            // Fallback: create highlighter with disabled syntax
            Self {
                parser: Parser::new(),
                syntax_enabled: false,
                semantic_enabled: true,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_pattern_detection() {
        let highlighter = MarkdownHighlighter::new().unwrap();
        let spans = highlighter
            .highlight_semantic_patterns("See #src/main.rs and @function_name")
            .unwrap();

        assert!(spans.len() >= 3); // Before, pattern, after
    }

    #[test]
    fn test_file_reference_highlighting() {
        let highlighter = MarkdownHighlighter::new().unwrap();
        let spans = highlighter.highlight_semantic_patterns("#path/to/file.rs").unwrap();

        // Should have blue color for file reference
        assert!(spans.iter().any(|s| {
            if let ratatui::style::Color::Blue = s.style.fg.unwrap_or(Color::Reset) {
                true
            } else {
                false
            }
        }));
    }

    #[test]
    fn test_symbol_reference_highlighting() {
        let highlighter = MarkdownHighlighter::new().unwrap();
        let spans = highlighter.highlight_semantic_patterns("@my_function").unwrap();

        // Should have green color for symbol reference
        assert!(spans.iter().any(|s| {
            if let ratatui::style::Color::Green = s.style.fg.unwrap_or(Color::Reset) {
                true
            } else {
                false
            }
        }));
    }

    #[test]
    fn test_typed_hole_highlighting() {
        let highlighter = MarkdownHighlighter::new().unwrap();
        let spans = highlighter.highlight_semantic_patterns("?interface_name").unwrap();

        // Should have yellow color for typed hole
        assert!(spans.iter().any(|s| {
            if let ratatui::style::Color::Yellow = s.style.fg.unwrap_or(Color::Reset) {
                true
            } else {
                false
            }
        }));
    }

    #[test]
    fn test_multiple_patterns_in_line() {
        let highlighter = MarkdownHighlighter::new().unwrap();
        let text = "The #config.toml has @settings and ?missing_interface";
        let spans = highlighter.highlight_semantic_patterns(text).unwrap();

        // Should have at least 6 spans: before, #, between, @, between, ?
        assert!(spans.len() >= 6);
    }

    #[test]
    fn test_no_patterns() {
        let highlighter = MarkdownHighlighter::new().unwrap();
        let result = highlighter.highlight_semantic_patterns("plain text with no patterns");

        assert!(result.is_none());
    }

    #[test]
    fn test_highlighter_creation() {
        let highlighter = MarkdownHighlighter::new();
        assert!(highlighter.is_ok());
    }

    #[test]
    fn test_toggle_highlighting() {
        let mut highlighter = MarkdownHighlighter::new().unwrap();

        highlighter.set_syntax_enabled(false);
        assert!(!highlighter.syntax_enabled);

        highlighter.set_semantic_enabled(false);
        assert!(!highlighter.semantic_enabled);
    }
}
