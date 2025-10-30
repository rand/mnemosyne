//! Subtle syntax highlighting for context documents
//!
//! Provides gentle, non-distracting syntax highlighting using tree-sitter
//! with a calm color palette that recedes into the background.

use super::Language;
use anyhow::Result;
use ratatui::style::Color;
use tree_sitter::{Parser, Tree};

/// Highlight span with color
#[derive(Debug, Clone)]
pub struct HighlightSpan {
    /// Start byte offset
    pub start: usize,
    /// End byte offset
    pub end: usize,
    /// Color for this span
    pub color: Color,
    /// Highlight kind
    pub kind: HighlightKind,
}

/// Types of highlights
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighlightKind {
    /// Markdown heading
    Heading,
    /// Bold text
    Bold,
    /// Italic text
    Italic,
    /// Code inline
    CodeInline,
    /// Code block
    CodeBlock,
    /// Link
    Link,
    /// List marker
    ListMarker,
    /// Quote
    Quote,
    /// Comment
    Comment,
    /// Keyword
    Keyword,
    /// String
    String,
    /// Number
    Number,
    /// Operator
    Operator,
    /// Plain text
    Text,
}

impl HighlightKind {
    /// Get subtle color for highlight kind
    /// Uses calm, muted colors that don't distract
    pub fn color(&self) -> Color {
        match self {
            HighlightKind::Heading => Color::Rgb(180, 180, 220),      // Soft blue
            HighlightKind::Bold => Color::Rgb(200, 200, 200),          // Light gray
            HighlightKind::Italic => Color::Rgb(160, 160, 180),        // Muted purple
            HighlightKind::CodeInline => Color::Rgb(180, 200, 180),    // Soft green
            HighlightKind::CodeBlock => Color::Rgb(170, 190, 170),     // Muted green
            HighlightKind::Link => Color::Rgb(150, 180, 210),          // Soft cyan
            HighlightKind::ListMarker => Color::Rgb(190, 170, 150),    // Soft orange
            HighlightKind::Quote => Color::Rgb(160, 160, 160),         // Gray
            HighlightKind::Comment => Color::Rgb(140, 140, 140),       // Dark gray
            HighlightKind::Keyword => Color::Rgb(180, 160, 200),       // Soft purple
            HighlightKind::String => Color::Rgb(180, 200, 160),        // Soft yellow-green
            HighlightKind::Number => Color::Rgb(200, 180, 160),        // Soft tan
            HighlightKind::Operator => Color::Rgb(170, 170, 190),      // Soft lavender
            HighlightKind::Text => Color::White,
        }
    }
}

/// Syntax highlighter using tree-sitter
pub struct Highlighter {
    /// Tree-sitter parser
    parser: Parser,
    /// Current language
    language: Language,
}

impl Highlighter {
    /// Create new highlighter
    pub fn new(language: Language) -> Result<Self> {
        let mut parser = Parser::new();

        // Set language for parser
        match language {
            Language::Markdown => {
                parser.set_language(tree_sitter_markdown::language())?;
            }
            Language::PlainText => {
                // Plain text doesn't need parsing
            }
            _ => {
                // For other languages, use plain text for now
                // TODO: Add more language support
            }
        }

        Ok(Self { parser, language })
    }

    /// Parse text and return syntax tree
    pub fn parse(&mut self, text: &str) -> Option<Tree> {
        self.parser.parse(text, None)
    }

    /// Get highlight spans for text
    pub fn highlight(&mut self, text: &str) -> Vec<HighlightSpan> {
        match self.language {
            Language::Markdown => self.highlight_markdown(text),
            Language::PlainText => Vec::new(),
            _ => Vec::new(),
        }
    }

    /// Highlight markdown text
    fn highlight_markdown(&mut self, text: &str) -> Vec<HighlightSpan> {
        let tree = match self.parse(text) {
            Some(tree) => tree,
            None => return Vec::new(),
        };

        let mut spans = Vec::new();
        let root_node = tree.root_node();

        // Walk the syntax tree
        self.walk_node(root_node, text, &mut spans);

        // Sort by start position
        spans.sort_by_key(|s| s.start);

        spans
    }

    /// Walk syntax tree node and extract highlights
    fn walk_node(&self, node: tree_sitter::Node, text: &str, spans: &mut Vec<HighlightSpan>) {
        let kind = node.kind();
        let start = node.start_byte();
        let end = node.end_byte();

        // Map node kinds to highlight kinds
        let highlight_kind = match kind {
            "atx_heading" | "setext_heading" => Some(HighlightKind::Heading),
            "strong_emphasis" => Some(HighlightKind::Bold),
            "emphasis" => Some(HighlightKind::Italic),
            "code_span" => Some(HighlightKind::CodeInline),
            "fenced_code_block" | "indented_code_block" => Some(HighlightKind::CodeBlock),
            "link" | "autolink" => Some(HighlightKind::Link),
            "list_marker" => Some(HighlightKind::ListMarker),
            "block_quote" => Some(HighlightKind::Quote),
            _ => None,
        };

        if let Some(kind) = highlight_kind {
            spans.push(HighlightSpan {
                start,
                end,
                color: kind.color(),
                kind,
            });
        }

        // Recurse into children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.walk_node(child, text, spans);
        }
    }

    /// Update language
    pub fn set_language(&mut self, language: Language) -> Result<()> {
        match language {
            Language::Markdown => {
                self.parser.set_language(tree_sitter_markdown::language())?;
            }
            Language::PlainText => {
                // Plain text doesn't need parsing
            }
            _ => {
                // For other languages, use plain text for now
            }
        }
        self.language = language;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlighter_markdown() {
        let mut highlighter = Highlighter::new(Language::Markdown).unwrap();

        let text = "# Heading\n\n**bold** and *italic*\n\n`code`";
        let spans = highlighter.highlight(text);

        // Should have highlights for heading, bold, italic, code
        assert!(!spans.is_empty());

        // Check that we have different highlight kinds
        let kinds: Vec<_> = spans.iter().map(|s| s.kind).collect();
        assert!(kinds.contains(&HighlightKind::Heading));
    }

    #[test]
    fn test_highlight_colors() {
        // All highlight kinds should have colors
        for kind in [
            HighlightKind::Heading,
            HighlightKind::Bold,
            HighlightKind::Italic,
            HighlightKind::CodeInline,
            HighlightKind::Link,
        ] {
            let color = kind.color();
            // Just verify it returns a color
            match color {
                Color::Rgb(_, _, _) | Color::White => {}
                _ => panic!("Unexpected color type"),
            }
        }
    }
}
