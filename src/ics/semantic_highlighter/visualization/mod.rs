//! Visualization types and rendering utilities

use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};
use std::ops::Range;

pub mod colors;
pub mod annotations;
pub mod connections;

pub use colors::ColorScheme;
pub use annotations::{AnnotationType, Annotation};
pub use connections::{Connection, ConnectionType};

/// A highlighted span of text with styling and metadata
#[derive(Debug, Clone)]
pub struct HighlightSpan {
    /// The text range this span covers
    pub range: Range<usize>,

    /// The visual style to apply
    pub style: Style,

    /// Source of the highlighting (determines priority)
    pub source: HighlightSource,

    /// Optional annotation
    pub annotation: Option<Annotation>,

    /// Confidence score (0.0-1.0)
    pub confidence: f32,

    /// Metadata for hover tooltips
    pub metadata: Option<SpanMetadata>,
}

impl HighlightSpan {
    /// Create a new highlight span
    pub fn new(range: Range<usize>, style: Style, source: HighlightSource) -> Self {
        Self {
            range,
            style,
            source,
            annotation: None,
            confidence: 1.0,
            metadata: None,
        }
    }

    /// Add an annotation
    pub fn with_annotation(mut self, annotation: Annotation) -> Self {
        self.annotation = Some(annotation);
        self
    }

    /// Set confidence score
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, metadata: SpanMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Source of highlighting (determines priority)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum HighlightSource {
    /// Plain text (lowest priority)
    Plain = 0,

    /// Tree-sitter syntax
    Syntax = 1,

    /// Tier 1: Structural patterns
    Structural = 2,

    /// Tier 2: Relational analysis
    Relational = 3,

    /// Tier 3: Analytical (Claude API)
    Analytical = 4,
}

/// Metadata associated with a highlight span
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanMetadata {
    /// Type of semantic element
    pub element_type: String,

    /// Description for tooltips
    pub description: Option<String>,

    /// Related spans (for navigation)
    pub related_spans: Vec<usize>,

    /// Additional properties
    pub properties: std::collections::HashMap<String, String>,
}

impl SpanMetadata {
    pub fn new(element_type: impl Into<String>) -> Self {
        Self {
            element_type: element_type.into(),
            description: None,
            related_spans: Vec::new(),
            properties: std::collections::HashMap::new(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_related(mut self, related: Vec<usize>) -> Self {
        self.related_spans = related;
        self
    }
}

/// Priority-based span merger
pub struct SpanMerger {
    spans: Vec<HighlightSpan>,
}

impl SpanMerger {
    pub fn new() -> Self {
        Self { spans: Vec::new() }
    }

    /// Add a span to be merged
    pub fn add(&mut self, span: HighlightSpan) {
        self.spans.push(span);
    }

    /// Merge overlapping spans, keeping highest priority
    pub fn merge(mut self) -> Vec<HighlightSpan> {
        if self.spans.is_empty() {
            return Vec::new();
        }

        // Sort by start position, then by priority (descending)
        self.spans.sort_by(|a, b| {
            a.range
                .start
                .cmp(&b.range.start)
                .then_with(|| b.source.cmp(&a.source))
        });

        let mut result = Vec::new();
        let mut current: Option<HighlightSpan> = None;

        for span in self.spans {
            match current.take() {
                None => {
                    current = Some(span);
                }
                Some(prev) => {
                    if span.range.start < prev.range.end {
                        // Overlapping - keep higher priority
                        if span.source > prev.source {
                            // Handle partial overlap
                            if prev.range.start < span.range.start {
                                // Add non-overlapping part of prev
                                result.push(HighlightSpan {
                                    range: prev.range.start..span.range.start,
                                    ..prev.clone()
                                });
                            }
                            current = Some(span);
                        } else {
                            // Keep prev, but may need to split
                            current = Some(prev);
                        }
                    } else {
                        // Non-overlapping
                        result.push(prev);
                        current = Some(span);
                    }
                }
            }
        }

        if let Some(span) = current {
            result.push(span);
        }

        result
    }
}

impl Default for SpanMerger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_creation() {
        let span = HighlightSpan::new(0..10, Style::default(), HighlightSource::Syntax);
        assert_eq!(span.range, 0..10);
        assert_eq!(span.source, HighlightSource::Syntax);
        assert_eq!(span.confidence, 1.0);
    }

    #[test]
    fn test_span_priority() {
        assert!(HighlightSource::Analytical > HighlightSource::Relational);
        assert!(HighlightSource::Relational > HighlightSource::Structural);
        assert!(HighlightSource::Structural > HighlightSource::Syntax);
    }

    #[test]
    fn test_span_merger_non_overlapping() {
        let mut merger = SpanMerger::new();
        merger.add(HighlightSpan::new(
            0..5,
            Style::default(),
            HighlightSource::Syntax,
        ));
        merger.add(HighlightSpan::new(
            10..15,
            Style::default(),
            HighlightSource::Structural,
        ));

        let result = merger.merge();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_span_merger_overlapping_priority() {
        let mut merger = SpanMerger::new();
        merger.add(HighlightSpan::new(
            0..10,
            Style::default(),
            HighlightSource::Syntax,
        ));
        merger.add(HighlightSpan::new(
            5..15,
            Style::default().fg(Color::Red),
            HighlightSource::Relational,
        ));

        let result = merger.merge();
        // Should prefer higher priority (Relational)
        assert!(result.iter().any(|s| s.source == HighlightSource::Relational));
    }
}
