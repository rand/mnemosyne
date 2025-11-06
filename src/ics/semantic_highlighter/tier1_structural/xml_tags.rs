//! XML tag analyzer for agentic context
//!
//! Highlights XML-style tags commonly used in Claude's output and agentic context:
//! - <thinking>, </thinking>
//! - <example>, </example>
//! - <search_quality_reflection>, etc.
//!
//! Also validates nesting and matching of open/close tags.

use crate::ics::semantic_highlighter::{
    utils::CommonPatterns,
    visualization::{Annotation, AnnotationType, HighlightSource, HighlightSpan},
    Result,
};
use ratatui::style::{Color, Modifier, Style};
use std::collections::HashMap;

/// XML tag analyzer
pub struct XmlTagAnalyzer {
    /// Color scheme for different tag types
    colors: HashMap<&'static str, Color>,
}

impl XmlTagAnalyzer {
    pub fn new() -> Self {
        let mut colors = HashMap::new();

        // Cognitive process tags
        colors.insert("thinking", Color::Cyan);
        colors.insert("commentary", Color::Cyan);
        colors.insert("reflection", Color::Cyan);
        colors.insert("analysis", Color::Cyan);

        // Structure tags
        colors.insert("example", Color::Green);
        colors.insert("good-example", Color::LightGreen);
        colors.insert("bad-example", Color::LightRed);
        colors.insert("code", Color::Yellow);

        // Search and quality tags
        colors.insert("search_quality_reflection", Color::Magenta);
        colors.insert("search_quality_score", Color::Magenta);
        colors.insert("result", Color::Blue);

        // Context tags
        colors.insert("context", Color::LightBlue);
        colors.insert("system", Color::Gray);
        colors.insert("user", Color::White);
        colors.insert("assistant", Color::LightYellow);

        // Meta tags
        colors.insert("note", Color::LightMagenta);
        colors.insert("warning", Color::Red);
        colors.insert("error", Color::Red);
        colors.insert("todo", Color::Yellow);

        Self { colors }
    }

    /// Analyze text for XML tags
    pub fn analyze(&self, text: &str) -> Result<Vec<HighlightSpan>> {
        let mut spans = Vec::new();

        // Find all XML tags
        for cap in CommonPatterns::xml_tag().captures_iter(text) {
            let full_match = cap.get(0).unwrap();
            let is_closing = cap.get(1).map(|m| !m.as_str().is_empty()).unwrap_or(false);
            let tag_name = cap.get(2).unwrap().as_str();
            let is_self_closing = cap.get(4).map(|m| !m.as_str().is_empty()).unwrap_or(false);

            let start = full_match.start();
            let end = full_match.end();

            // Determine color based on tag name
            let color = self
                .colors
                .get(tag_name.to_lowercase().as_str())
                .copied()
                .unwrap_or(Color::DarkGray);

            // Style based on tag type
            let mut style = Style::default().fg(color);

            if is_closing {
                // Closing tags are slightly dimmed
                style = style.add_modifier(Modifier::DIM);
            } else if is_self_closing {
                // Self-closing tags are italic
                style = style.add_modifier(Modifier::ITALIC);
            } else {
                // Opening tags are bold
                style = style.add_modifier(Modifier::BOLD);
            }

            spans.push(HighlightSpan {
                range: start..end,
                style,
                source: HighlightSource::Structural,
                annotation: None,
                confidence: 1.0,
                metadata: None,
            });
        }

        // Check for self-closing tags separately
        for cap in CommonPatterns::xml_self_closing().captures_iter(text) {
            let full_match = cap.get(0).unwrap();
            let tag_name = cap.get(1).unwrap().as_str();

            let start = full_match.start();
            let end = full_match.end();

            let color = self
                .colors
                .get(tag_name.to_lowercase().as_str())
                .copied()
                .unwrap_or(Color::DarkGray);

            let style = Style::default().fg(color).add_modifier(Modifier::ITALIC);

            spans.push(HighlightSpan {
                range: start..end,
                style,
                source: HighlightSource::Structural,
                annotation: None,
                confidence: 1.0,
                metadata: None,
            });
        }

        // Validate tag nesting and add warnings for mismatched tags
        let validation_spans = self.validate_tag_nesting(text);
        spans.extend(validation_spans);

        Ok(spans)
    }

    /// Validate tag nesting and return warning spans for issues
    fn validate_tag_nesting(&self, text: &str) -> Vec<HighlightSpan> {
        let mut spans = Vec::new();
        let mut tag_stack: Vec<(String, usize)> = Vec::new();

        // Collect all tags with their positions
        for cap in CommonPatterns::xml_tag().captures_iter(text) {
            let full_match = cap.get(0).unwrap();
            let is_closing = cap.get(1).map(|m| !m.as_str().is_empty()).unwrap_or(false);
            let tag_name = cap.get(2).unwrap().as_str().to_string();
            let is_self_closing = cap.get(4).map(|m| !m.as_str().is_empty()).unwrap_or(false);

            let pos = full_match.start();

            if is_self_closing {
                // Self-closing tags don't affect the stack
                continue;
            }

            if is_closing {
                // Closing tag - check if it matches the top of the stack
                if let Some((open_tag, _)) = tag_stack.last() {
                    if open_tag == &tag_name {
                        tag_stack.pop();
                    } else {
                        // Mismatched closing tag
                        spans.push(HighlightSpan {
                            range: pos..full_match.end(),
                            style: Style::default()
                                .fg(Color::Red)
                                .add_modifier(Modifier::UNDERLINED),
                            source: HighlightSource::Structural,
                            annotation: Some(
                                Annotation::new(AnnotationType::Warning).with_tooltip(format!(
                                    "Mismatched tag: expected </{}>",
                                    open_tag
                                )),
                            ),
                            confidence: 1.0,
                            metadata: None,
                        });
                    }
                } else {
                    // Closing tag with no matching opening tag
                    spans.push(HighlightSpan {
                        range: pos..full_match.end(),
                        style: Style::default()
                            .fg(Color::Red)
                            .add_modifier(Modifier::UNDERLINED),
                        source: HighlightSource::Structural,
                        annotation: Some(
                            Annotation::new(AnnotationType::Warning)
                                .with_tooltip("Closing tag without opening tag"),
                        ),
                        confidence: 1.0,
                        metadata: None,
                    });
                }
            } else {
                // Opening tag - push to stack
                tag_stack.push((tag_name, pos));
            }
        }

        // Check for unclosed tags
        for (tag_name, pos) in tag_stack {
            // Find the end of this opening tag
            if let Some(tag_end) = text[pos..].find('>').map(|offset| pos + offset + 1) {
                spans.push(HighlightSpan {
                    range: pos..tag_end,
                    style: Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::UNDERLINED),
                    source: HighlightSource::Structural,
                    annotation: Some(
                        Annotation::new(AnnotationType::Warning)
                            .with_tooltip(format!("Unclosed tag: <{}>", tag_name)),
                    ),
                    confidence: 1.0,
                    metadata: None,
                });
            }
        }

        spans
    }
}

impl Default for XmlTagAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tags() {
        let analyzer = XmlTagAnalyzer::new();
        let text = "<thinking>Some thought</thinking>";
        let spans = analyzer.analyze(text).unwrap();

        // Should have 2 spans (opening and closing tags)
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].range, 0..10); // <thinking>
        assert_eq!(spans[1].range, 22..33); // </thinking> (starts after "Some thought")
    }

    #[test]
    fn test_self_closing_tags() {
        let analyzer = XmlTagAnalyzer::new();
        let text = "<note/>";
        let spans = analyzer.analyze(text).unwrap();

        // Current implementation treats self-closing as opening + closing
        assert_eq!(spans.len(), 2);
        // Both spans cover the whole tag
        assert!(spans.iter().any(|s| s.range.start == 0 && s.range.end == 7));
    }

    #[test]
    fn test_mismatched_tags() {
        let analyzer = XmlTagAnalyzer::new();
        let text = "<thinking>Some thought</example>";
        let spans = analyzer.analyze(text).unwrap();

        // Should have 2 regular spans + 1 warning span for mismatch
        assert!(spans.len() >= 3);

        // Check that there's a warning annotation
        let has_warning = spans.iter().any(|s| {
            s.annotation
                .as_ref()
                .map(|a| matches!(a.annotation_type, AnnotationType::Warning))
                .unwrap_or(false)
        });
        assert!(has_warning);
    }

    #[test]
    fn test_unclosed_tag() {
        let analyzer = XmlTagAnalyzer::new();
        let text = "<thinking>Some thought";
        let spans = analyzer.analyze(text).unwrap();

        // Should have 1 regular span + 1 warning span for unclosed tag
        assert!(spans.len() >= 2);

        // Check for unclosed tag warning
        let has_warning = spans.iter().any(|s| {
            s.annotation
                .as_ref()
                .and_then(|a| a.tooltip.as_ref())
                .and_then(|t| t.strip_prefix("Unclosed tag"))
                .is_some()
        });
        assert!(has_warning);
    }
}
