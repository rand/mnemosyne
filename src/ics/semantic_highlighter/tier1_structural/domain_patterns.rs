//! Domain-specific pattern matcher
//!
//! Detects patterns commonly used in code and technical contexts:
//! - File paths: #/path/to/file.rs or #file.txt
//! - Symbol references: @function_name, @Type::method
//! - Typed holes: ?placeholder, ?to_be_implemented
//! - URLs: https://example.com
//! - Code blocks: ```language
//! - Inline code: `code`
//!
//! These patterns are useful for highlighting references in agentic contexts
//! and technical documentation.

use crate::ics::semantic_highlighter::{
    utils::CommonPatterns,
    visualization::{Annotation, AnnotationType, HighlightSource, HighlightSpan},
    Result,
};
use ratatui::style::{Color, Modifier, Style};

/// Types of domain patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternType {
    /// File path reference (#path)
    FilePath,
    /// Symbol reference (@symbol)
    Symbol,
    /// Typed hole (?hole)
    TypedHole,
    /// URL (http/https)
    Url,
    /// Code fence (```)
    CodeFence,
    /// Inline code (`code`)
    InlineCode,
}

impl PatternType {
    fn color(&self) -> Color {
        match self {
            PatternType::FilePath => Color::Cyan,
            PatternType::Symbol => Color::Yellow,
            PatternType::TypedHole => Color::Magenta,
            PatternType::Url => Color::Blue,
            PatternType::CodeFence => Color::Green,
            PatternType::InlineCode => Color::LightGreen,
        }
    }

    fn modifier(&self) -> Modifier {
        match self {
            PatternType::FilePath => Modifier::UNDERLINED,
            PatternType::Symbol => Modifier::BOLD,
            PatternType::TypedHole => Modifier::ITALIC | Modifier::BOLD,
            PatternType::Url => Modifier::UNDERLINED,
            PatternType::CodeFence => Modifier::BOLD,
            PatternType::InlineCode => Modifier::empty(),
        }
    }

    fn description(&self) -> &'static str {
        match self {
            PatternType::FilePath => "File reference",
            PatternType::Symbol => "Symbol reference",
            PatternType::TypedHole => "Typed hole (to be implemented)",
            PatternType::Url => "URL",
            PatternType::CodeFence => "Code block",
            PatternType::InlineCode => "Inline code",
        }
    }
}

/// Domain pattern matcher
pub struct DomainPatternMatcher {
    /// Whether to show annotations
    show_annotations: bool,
    /// Whether to validate file paths
    validate_paths: bool,
}

impl DomainPatternMatcher {
    pub fn new() -> Self {
        Self {
            show_annotations: false, // Default to false to reduce noise
            validate_paths: false,   // File validation can be expensive
        }
    }

    /// Configure whether to show annotations
    pub fn with_annotations(mut self, show: bool) -> Self {
        self.show_annotations = show;
        self
    }

    /// Configure whether to validate file paths
    pub fn with_path_validation(mut self, validate: bool) -> Self {
        self.validate_paths = validate;
        self
    }

    /// Analyze text for domain patterns
    pub fn analyze(&self, text: &str) -> Result<Vec<HighlightSpan>> {
        let mut spans = Vec::new();

        // Detect file paths
        spans.extend(self.detect_file_paths(text)?);

        // Detect symbol references
        spans.extend(self.detect_symbols(text)?);

        // Detect typed holes
        spans.extend(self.detect_typed_holes(text)?);

        // Detect URLs
        spans.extend(self.detect_urls(text)?);

        // Detect code blocks
        spans.extend(self.detect_code_blocks(text)?);

        // Detect inline code
        spans.extend(self.detect_inline_code(text)?);

        Ok(spans)
    }

    /// Detect file path references
    fn detect_file_paths(&self, text: &str) -> Result<Vec<HighlightSpan>> {
        let mut spans = Vec::new();

        for mat in CommonPatterns::file_path().find_iter(text) {
            let style = Style::default()
                .fg(PatternType::FilePath.color())
                .add_modifier(PatternType::FilePath.modifier());

            let annotation = if self.show_annotations {
                Some(Annotation {
                    annotation_type: AnnotationType::Information,
                    underline: None,
                    tooltip: Some(PatternType::FilePath.description().to_string()),
                })
            } else {
                None
            };

            spans.push(HighlightSpan {
                range: mat.start()..mat.end(),
                style,
                source: HighlightSource::Structural,
                annotation,
                confidence: 0.9,
                metadata: None,
            });
        }

        Ok(spans)
    }

    /// Detect symbol references
    fn detect_symbols(&self, text: &str) -> Result<Vec<HighlightSpan>> {
        let mut spans = Vec::new();

        for mat in CommonPatterns::symbol_reference().find_iter(text) {
            let style = Style::default()
                .fg(PatternType::Symbol.color())
                .add_modifier(PatternType::Symbol.modifier());

            let annotation = if self.show_annotations {
                Some(Annotation {
                    annotation_type: AnnotationType::Information,
                    underline: None,
                    tooltip: Some(PatternType::Symbol.description().to_string()),
                })
            } else {
                None
            };

            spans.push(HighlightSpan {
                range: mat.start()..mat.end(),
                style,
                source: HighlightSource::Structural,
                annotation,
                confidence: 0.9,
                metadata: None,
            });
        }

        Ok(spans)
    }

    /// Detect typed holes
    fn detect_typed_holes(&self, text: &str) -> Result<Vec<HighlightSpan>> {
        let mut spans = Vec::new();

        for mat in CommonPatterns::typed_hole().find_iter(text) {
            let style = Style::default()
                .fg(PatternType::TypedHole.color())
                .add_modifier(PatternType::TypedHole.modifier());

            let annotation = if self.show_annotations {
                Some(Annotation {
                    annotation_type: AnnotationType::Warning,
                    underline: None,
                    tooltip: Some(PatternType::TypedHole.description().to_string()),
                })
            } else {
                None
            };

            spans.push(HighlightSpan {
                range: mat.start()..mat.end(),
                style,
                source: HighlightSource::Structural,
                annotation,
                confidence: 0.95,
                metadata: None,
            });
        }

        Ok(spans)
    }

    /// Detect URLs
    fn detect_urls(&self, text: &str) -> Result<Vec<HighlightSpan>> {
        let mut spans = Vec::new();

        for mat in CommonPatterns::url().find_iter(text) {
            let style = Style::default()
                .fg(PatternType::Url.color())
                .add_modifier(PatternType::Url.modifier());

            spans.push(HighlightSpan {
                range: mat.start()..mat.end(),
                style,
                source: HighlightSource::Structural,
                annotation: None, // URLs are self-explanatory
                confidence: 1.0,
                metadata: None,
            });
        }

        Ok(spans)
    }

    /// Detect code fence markers
    fn detect_code_blocks(&self, text: &str) -> Result<Vec<HighlightSpan>> {
        let mut spans = Vec::new();

        for mat in CommonPatterns::code_fence().find_iter(text) {
            let style = Style::default()
                .fg(PatternType::CodeFence.color())
                .add_modifier(PatternType::CodeFence.modifier());

            spans.push(HighlightSpan {
                range: mat.start()..mat.end(),
                style,
                source: HighlightSource::Structural,
                annotation: None,
                confidence: 1.0,
                metadata: None,
            });
        }

        Ok(spans)
    }

    /// Detect inline code
    fn detect_inline_code(&self, text: &str) -> Result<Vec<HighlightSpan>> {
        let mut spans = Vec::new();

        for mat in CommonPatterns::inline_code().find_iter(text) {
            let style = Style::default()
                .fg(PatternType::InlineCode.color())
                .add_modifier(PatternType::InlineCode.modifier());

            spans.push(HighlightSpan {
                range: mat.start()..mat.end(),
                style,
                source: HighlightSource::Structural,
                annotation: None,
                confidence: 1.0,
                metadata: None,
            });
        }

        Ok(spans)
    }
}

impl Default for DomainPatternMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_path_detection() {
        let matcher = DomainPatternMatcher::new();
        let text = "See #src/main.rs for details";
        let spans = matcher.analyze(text).unwrap();

        assert!(!spans.is_empty());
        let path_span = spans
            .iter()
            .find(|s| text[s.range.clone()].contains("src/main.rs"))
            .expect("Should find file path");

        assert_eq!(path_span.confidence, 0.9);
    }

    #[test]
    fn test_symbol_detection() {
        let matcher = DomainPatternMatcher::new();
        let text = "Call @calculate_total and @User::new";
        let spans = matcher.analyze(text).unwrap();

        // Should find both symbols
        let has_calculate = spans
            .iter()
            .any(|s| text[s.range.clone()].contains("calculate_total"));
        let has_user_new = spans
            .iter()
            .any(|s| text[s.range.clone()].contains("User::new"));

        assert!(has_calculate);
        assert!(has_user_new);
    }

    #[test]
    fn test_typed_hole_detection() {
        let matcher = DomainPatternMatcher::new();
        let text = "Implement ?auth_handler and ?database_connection";
        let spans = matcher.analyze(text).unwrap();

        // Should find both typed holes
        let has_auth = spans
            .iter()
            .any(|s| text[s.range.clone()].contains("auth_handler"));
        let has_db = spans
            .iter()
            .any(|s| text[s.range.clone()].contains("database_connection"));

        assert!(has_auth);
        assert!(has_db);
    }

    #[test]
    fn test_url_detection() {
        let matcher = DomainPatternMatcher::new();
        let text = "Visit https://example.com for more info";
        let spans = matcher.analyze(text).unwrap();

        let url_span = spans
            .iter()
            .find(|s| text[s.range.clone()].contains("https://example.com"))
            .expect("Should find URL");

        assert_eq!(url_span.confidence, 1.0);
    }

    #[test]
    fn test_code_fence_detection() {
        let matcher = DomainPatternMatcher::new();
        let text = "```rust\nfn main() {}\n```";
        let spans = matcher.analyze(text).unwrap();

        let fence_span = spans
            .iter()
            .find(|s| text[s.range.clone()].contains("```rust"))
            .expect("Should find code fence");

        assert_eq!(fence_span.confidence, 1.0);
    }

    #[test]
    fn test_inline_code_detection() {
        let matcher = DomainPatternMatcher::new();
        let text = "Use the `async fn` syntax";
        let spans = matcher.analyze(text).unwrap();

        let code_span = spans
            .iter()
            .find(|s| text[s.range.clone()].contains("`async fn`"))
            .expect("Should find inline code");

        assert_eq!(code_span.confidence, 1.0);
    }

    #[test]
    fn test_multiple_patterns() {
        let matcher = DomainPatternMatcher::new();
        let text = "Check #src/lib.rs at @main function with ?todo implementation";
        let spans = matcher.analyze(text).unwrap();

        // Should find all three pattern types
        assert!(spans.len() >= 3);

        let has_file = spans
            .iter()
            .any(|s| text[s.range.clone()].contains("src/lib.rs"));
        let has_symbol = spans
            .iter()
            .any(|s| text[s.range.clone()].contains("@main"));
        let has_hole = spans
            .iter()
            .any(|s| text[s.range.clone()].contains("?todo"));

        assert!(has_file);
        assert!(has_symbol);
        assert!(has_hole);
    }

    #[test]
    fn test_annotations() {
        let matcher = DomainPatternMatcher::new().with_annotations(true);
        let text = "See #file.txt";
        let spans = matcher.analyze(text).unwrap();

        let has_annotations = spans.iter().any(|s| s.annotation.is_some());
        assert!(has_annotations);
    }
}
