//! Ambiguity and vague language detector
//!
//! Detects patterns that indicate ambiguous or vague language:
//! - Vague quantifiers: "some", "several", "many", "few"
//! - Vague adjectives: "various", "certain", "numerous"
//! - Hedging phrases: "kind of", "sort of", "somewhat"
//! - Approximations: "about", "around", "roughly", "approximately"
//! - Unclear references: "this", "that", "it" without clear antecedent
//!
//! This helps identify areas where more precision might be needed.

use crate::ics::semantic_highlighter::{
    utils::CommonPatterns,
    visualization::{Annotation, AnnotationType, HighlightSource, HighlightSpan},
    Result,
};
use ratatui::style::{Color, Modifier, Style};

/// Types of ambiguity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmbiguityType {
    /// Vague quantifier (some, several, many)
    VagueQuantifier,
    /// Approximation (about, around, roughly)
    Approximation,
    /// Hedging language (kind of, sort of)
    Hedging,
    /// Unclear reference (this, that without antecedent)
    UnclearReference,
    /// General vagueness
    Vague,
}

impl AmbiguityType {
    fn color(&self) -> Color {
        match self {
            AmbiguityType::VagueQuantifier => Color::Yellow,
            AmbiguityType::Approximation => Color::LightYellow,
            AmbiguityType::Hedging => Color::Magenta,
            AmbiguityType::UnclearReference => Color::Red,
            AmbiguityType::Vague => Color::Gray,
        }
    }

    fn description(&self) -> &'static str {
        match self {
            AmbiguityType::VagueQuantifier => "Vague quantity",
            AmbiguityType::Approximation => "Approximation",
            AmbiguityType::Hedging => "Hedging language",
            AmbiguityType::UnclearReference => "Unclear reference",
            AmbiguityType::Vague => "Vague language",
        }
    }
}

/// Ambiguity detector
pub struct AmbiguityDetector {
    /// Severity threshold (0.0-1.0)
    threshold: f32,
    /// Whether to show annotations
    show_annotations: bool,
}

impl AmbiguityDetector {
    pub fn new() -> Self {
        Self {
            threshold: 0.3,
            show_annotations: true,
        }
    }

    /// Set severity threshold
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Configure whether to show annotations
    pub fn with_annotations(mut self, show: bool) -> Self {
        self.show_annotations = show;
        self
    }

    /// Analyze text for ambiguity
    pub fn analyze(&self, text: &str) -> Result<Vec<HighlightSpan>> {
        let mut spans = Vec::new();

        // Detect vague quantifiers
        spans.extend(self.detect_vague_quantifiers(text)?);

        // Detect ambiguous phrases
        spans.extend(self.detect_ambiguous_phrases(text)?);

        // Detect unclear references
        spans.extend(self.detect_unclear_references(text)?);

        Ok(spans)
    }

    /// Detect vague quantifiers
    fn detect_vague_quantifiers(&self, text: &str) -> Result<Vec<HighlightSpan>> {
        let mut spans = Vec::new();

        for mat in CommonPatterns::vague_quantifiers().find_iter(text) {
            let confidence = 0.7; // Fairly confident these are vague

            if confidence < self.threshold {
                continue;
            }

            let style = Style::default()
                .fg(AmbiguityType::VagueQuantifier.color())
                .add_modifier(Modifier::UNDERLINED);

            let annotation = if self.show_annotations {
                Some(Annotation {
                    annotation_type: AnnotationType::Warning,
                    underline: None,
                    tooltip: Some(format!(
                        "{}: Consider being more specific",
                        AmbiguityType::VagueQuantifier.description()
                    )),
                })
            } else {
                None
            };

            spans.push(HighlightSpan {
                range: mat.start()..mat.end(),
                style,
                source: HighlightSource::Structural,
                annotation,
                confidence,
                metadata: None,
            });
        }

        Ok(spans)
    }

    /// Detect ambiguous phrases
    fn detect_ambiguous_phrases(&self, text: &str) -> Result<Vec<HighlightSpan>> {
        let mut spans = Vec::new();

        for mat in CommonPatterns::ambiguous_phrases().find_iter(text) {
            let word = mat.as_str();

            // Classify the type of ambiguity
            let (ambiguity_type, confidence) = match word {
                "about" | "around" | "roughly" | "approximately" => {
                    (AmbiguityType::Approximation, 0.6)
                }
                "unclear" | "ambiguous" | "vague" => {
                    (AmbiguityType::Vague, 0.9) // These words explicitly indicate vagueness
                }
                _ => (AmbiguityType::Vague, 0.5),
            };

            if confidence < self.threshold {
                continue;
            }

            let style = Style::default()
                .fg(ambiguity_type.color())
                .add_modifier(Modifier::DIM);

            let annotation = if self.show_annotations && confidence > 0.6 {
                Some(Annotation {
                    annotation_type: AnnotationType::Information,
                    underline: None,
                    tooltip: Some(ambiguity_type.description().to_string()),
                })
            } else {
                None
            };

            spans.push(HighlightSpan {
                range: mat.start()..mat.end(),
                style,
                source: HighlightSource::Structural,
                annotation,
                confidence,
                metadata: None,
            });
        }

        Ok(spans)
    }

    /// Detect unclear references (simplified heuristic)
    fn detect_unclear_references(&self, text: &str) -> Result<Vec<HighlightSpan>> {
        let mut spans = Vec::new();

        // Look for demonstrative pronouns at sentence start (often unclear)
        let text_lower = text.to_lowercase();

        // Sentence-initial "this", "that", "these", "those" are often unclear
        let patterns = [
            (r"(?:^|\. )(this) ", 0.5),
            (r"(?:^|\. )(that) ", 0.5),
            (r"(?:^|\. )(these) ", 0.4),
            (r"(?:^|\. )(those) ", 0.4),
            (r"(?:^|\. )(it) ", 0.3),
        ];

        for (pattern_str, base_confidence) in patterns.iter() {
            if *base_confidence < self.threshold {
                continue;
            }

            let pattern = regex::Regex::new(pattern_str).unwrap();
            for cap in pattern.captures_iter(&text_lower) {
                if let Some(pronoun_match) = cap.get(1) {
                    let start = pronoun_match.start();
                    let end = pronoun_match.end();

                    let style = Style::default()
                        .fg(AmbiguityType::UnclearReference.color())
                        .add_modifier(Modifier::UNDERLINED);

                    let annotation = if self.show_annotations {
                        Some(Annotation {
                            annotation_type: AnnotationType::Warning,
                            underline: None,
                            tooltip: Some("Potentially unclear reference".to_string()),
                        })
                    } else {
                        None
                    };

                    spans.push(HighlightSpan {
                        range: start..end,
                        style,
                        source: HighlightSource::Structural,
                        annotation,
                        confidence: *base_confidence,
                        metadata: None,
                    });
                }
            }
        }

        Ok(spans)
    }
}

impl Default for AmbiguityDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vague_quantifiers() {
        let detector = AmbiguityDetector::new();
        let text = "There are several issues with many components";
        let spans = detector.analyze(text).unwrap();

        // Should find "several" and "many"
        assert!(spans.len() >= 2);

        let has_several = spans
            .iter()
            .any(|s| text[s.range.clone()].to_lowercase().contains("several"));
        let has_many = spans
            .iter()
            .any(|s| text[s.range.clone()].to_lowercase().contains("many"));

        assert!(has_several);
        assert!(has_many);
    }

    #[test]
    fn test_approximations() {
        let detector = AmbiguityDetector::new();
        let text = "It takes about 10 seconds, roughly speaking";
        let spans = detector.analyze(text).unwrap();

        // Should find "about" and "roughly"
        let has_about = spans
            .iter()
            .any(|s| text[s.range.clone()].to_lowercase().contains("about"));
        let has_roughly = spans
            .iter()
            .any(|s| text[s.range.clone()].to_lowercase().contains("roughly"));

        assert!(has_about);
        assert!(has_roughly);
    }

    #[test]
    fn test_unclear_references() {
        let detector = AmbiguityDetector::new();
        let text = "The system failed. This caused problems.";
        let spans = detector.analyze(text).unwrap();

        // Should find sentence-initial "this"
        let has_this = spans
            .iter()
            .any(|s| text[s.range.clone()].to_lowercase() == "this");

        assert!(has_this);
    }

    #[test]
    fn test_threshold_filtering() {
        let detector = AmbiguityDetector::new().with_threshold(0.9);
        let text = "There are several issues";
        let spans = detector.analyze(text).unwrap();

        // With high threshold, should filter out most detections
        // Only explicit vagueness markers (confidence 0.9) should remain
        assert!(spans.len() < 2);
    }

    #[test]
    fn test_annotations() {
        let detector = AmbiguityDetector::new().with_annotations(true);
        let text = "There are several components";
        let spans = detector.analyze(text).unwrap();

        // Should have annotations
        let has_annotations = spans.iter().any(|s| s.annotation.is_some());
        assert!(has_annotations);
    }

    #[test]
    fn test_no_annotations() {
        let detector = AmbiguityDetector::new().with_annotations(false);
        let text = "There are several components";
        let spans = detector.analyze(text).unwrap();

        // Should not have annotations
        let has_annotations = spans.iter().any(|s| s.annotation.is_some());
        assert!(!has_annotations);
    }
}
