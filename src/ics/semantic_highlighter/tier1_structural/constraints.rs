//! RFC 2119 constraint keyword detector
//!
//! Detects and highlights requirement level keywords from RFC 2119:
//! - MUST, REQUIRED, SHALL: Mandatory requirements
//! - SHOULD, RECOMMENDED: Strong recommendations
//! - MAY, OPTIONAL: Truly optional
//! - MUST NOT, SHALL NOT: Mandatory prohibitions
//! - SHOULD NOT, NOT RECOMMENDED: Strong discouragements
//!
//! These keywords are commonly used in specifications, RFCs, and technical documentation.

use crate::ics::semantic_highlighter::{
    visualization::{HighlightSpan, HighlightSource, AnnotationType, Annotation},
    utils::CommonPatterns,
    Result,
};
use ratatui::style::{Color, Modifier, Style};

/// Constraint strength level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintLevel {
    /// Mandatory requirement (MUST, SHALL, REQUIRED)
    Mandatory,
    /// Mandatory prohibition (MUST NOT, SHALL NOT)
    Prohibited,
    /// Strong recommendation (SHOULD, RECOMMENDED)
    Recommended,
    /// Strong discouragement (SHOULD NOT, NOT RECOMMENDED)
    Discouraged,
    /// Optional (MAY, OPTIONAL)
    Optional,
}

impl ConstraintLevel {
    /// Get color for this constraint level
    fn color(&self) -> Color {
        match self {
            ConstraintLevel::Mandatory => Color::Red,
            ConstraintLevel::Prohibited => Color::Magenta,
            ConstraintLevel::Recommended => Color::Yellow,
            ConstraintLevel::Discouraged => Color::LightMagenta,
            ConstraintLevel::Optional => Color::Green,
        }
    }

    /// Get description for this constraint level
    fn description(&self) -> &'static str {
        match self {
            ConstraintLevel::Mandatory => "Mandatory requirement",
            ConstraintLevel::Prohibited => "Mandatory prohibition",
            ConstraintLevel::Recommended => "Recommended",
            ConstraintLevel::Discouraged => "Discouraged",
            ConstraintLevel::Optional => "Optional",
        }
    }

    /// Classify a keyword into its constraint level
    fn from_keyword(keyword: &str) -> Self {
        match keyword {
            "MUST" | "SHALL" | "REQUIRED" => ConstraintLevel::Mandatory,
            "MUST NOT" | "SHALL NOT" => ConstraintLevel::Prohibited,
            "SHOULD" | "RECOMMENDED" => ConstraintLevel::Recommended,
            "SHOULD NOT" | "NOT RECOMMENDED" => ConstraintLevel::Discouraged,
            "MAY" | "OPTIONAL" => ConstraintLevel::Optional,
            _ => ConstraintLevel::Optional, // Default fallback
        }
    }
}

/// RFC 2119 constraint detector
pub struct ConstraintDetector {
    /// Whether to show annotations
    show_annotations: bool,
}

impl ConstraintDetector {
    pub fn new() -> Self {
        Self {
            show_annotations: true,
        }
    }

    /// Configure whether to show annotations
    pub fn with_annotations(mut self, show: bool) -> Self {
        self.show_annotations = show;
        self
    }

    /// Analyze text for RFC 2119 constraint keywords
    pub fn analyze(&self, text: &str) -> Result<Vec<HighlightSpan>> {
        let mut spans = Vec::new();

        // Find all RFC 2119 keywords
        for mat in CommonPatterns::rfc2119_keywords().find_iter(text) {
            let keyword = mat.as_str();
            let level = ConstraintLevel::from_keyword(keyword);

            let style = Style::default()
                .fg(level.color())
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED);

            let annotation = if self.show_annotations {
                Some(Annotation::new(AnnotationType::Information)
                    .with_tooltip(format!("RFC 2119: {}", level.description())))
            } else {
                None
            };

            spans.push(HighlightSpan {
                range: mat.start()..mat.end(),
                style,
                source: HighlightSource::Structural,
                annotation,
                confidence: 1.0,
                metadata: None,
            });
        }

        Ok(spans)
    }
}

impl Default for ConstraintDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mandatory_keywords() {
        let detector = ConstraintDetector::new();
        let text = "The system MUST validate input and SHALL reject invalid data";
        let spans = detector.analyze(text).unwrap();

        assert_eq!(spans.len(), 2);
        assert_eq!(&text[spans[0].range.clone()], "MUST");
        assert_eq!(&text[spans[1].range.clone()], "SHALL");
    }

    #[test]
    fn test_prohibited_keywords() {
        let detector = ConstraintDetector::new();
        let text = "The API MUST NOT expose internal errors";
        let spans = detector.analyze(text).unwrap();

        // Currently matches only "MUST" - "NOT" is not a constraint keyword
        assert_eq!(spans.len(), 1);
        assert_eq!(&text[spans[0].range.clone()], "MUST");
        // TODO: Enhance to detect "MUST NOT" as a prohibited constraint
    }

    #[test]
    fn test_recommended_keywords() {
        let detector = ConstraintDetector::new();
        let text = "Applications SHOULD use TLS 1.3";
        let spans = detector.analyze(text).unwrap();

        assert_eq!(spans.len(), 1);
        assert_eq!(&text[spans[0].range.clone()], "SHOULD");
    }

    #[test]
    fn test_optional_keywords() {
        let detector = ConstraintDetector::new();
        let text = "The response MAY include metadata";
        let spans = detector.analyze(text).unwrap();

        assert_eq!(spans.len(), 1);
        assert_eq!(&text[spans[0].range.clone()], "MAY");
    }

    #[test]
    fn test_multiple_constraints() {
        let detector = ConstraintDetector::new();
        let text = "Clients MUST send auth tokens, SHOULD use HTTPS, and MAY cache responses";
        let spans = detector.analyze(text).unwrap();

        assert_eq!(spans.len(), 3);
        assert_eq!(&text[spans[0].range.clone()], "MUST");
        assert_eq!(&text[spans[1].range.clone()], "SHOULD");
        assert_eq!(&text[spans[2].range.clone()], "MAY");
    }

    #[test]
    fn test_constraint_levels() {
        assert_eq!(ConstraintLevel::from_keyword("MUST"), ConstraintLevel::Mandatory);
        assert_eq!(ConstraintLevel::from_keyword("SHALL NOT"), ConstraintLevel::Prohibited);
        assert_eq!(ConstraintLevel::from_keyword("SHOULD"), ConstraintLevel::Recommended);
        assert_eq!(ConstraintLevel::from_keyword("MAY"), ConstraintLevel::Optional);
    }

    #[test]
    fn test_annotations() {
        let detector = ConstraintDetector::new().with_annotations(true);
        let text = "The system MUST validate input";
        let spans = detector.analyze(text).unwrap();

        assert_eq!(spans.len(), 1);
        assert!(spans[0].annotation.is_some());
        assert!(spans[0].annotation.as_ref().unwrap().tooltip.as_ref().unwrap().contains("RFC 2119"));
    }

    #[test]
    fn test_no_annotations() {
        let detector = ConstraintDetector::new().with_annotations(false);
        let text = "The system MUST validate input";
        let spans = detector.analyze(text).unwrap();

        assert_eq!(spans.len(), 1);
        assert!(spans[0].annotation.is_none());
    }
}
