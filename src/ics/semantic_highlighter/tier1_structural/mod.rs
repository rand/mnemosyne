//! Tier 1: Structural Pattern Highlighting (Local, Real-time <5ms)
//!
//! Pure pattern-matching based highlighting with no external dependencies.
//! All analyzers in this tier use regex, tree-sitter, or dictionary lookups.

use crate::ics::semantic_highlighter::{visualization::HighlightSpan, Result};

pub mod ambiguity;
pub mod constraints;
pub mod domain_patterns;
pub mod modality;
pub mod xml_tags;

pub use ambiguity::AmbiguityDetector;
pub use constraints::ConstraintDetector;
pub use domain_patterns::DomainPatternMatcher;
pub use modality::ModalityAnalyzer;
pub use xml_tags::XmlTagAnalyzer;

/// Structural pattern highlighter
///
/// Combines multiple pattern-based analyzers for real-time highlighting.
pub struct StructuralHighlighter {
    xml_tags: XmlTagAnalyzer,
    constraints: ConstraintDetector,
    modality: ModalityAnalyzer,
    ambiguity: AmbiguityDetector,
    domain_patterns: DomainPatternMatcher,
    enabled: bool,
}

impl StructuralHighlighter {
    pub fn new() -> Self {
        Self {
            xml_tags: XmlTagAnalyzer::new(),
            constraints: ConstraintDetector::new(),
            modality: ModalityAnalyzer::new(),
            ambiguity: AmbiguityDetector::new(),
            domain_patterns: DomainPatternMatcher::new(),
            enabled: true,
        }
    }

    /// Highlight text using all structural analyzers
    pub fn highlight(&mut self, text: &str) -> Result<Vec<HighlightSpan>> {
        if !self.enabled {
            return Ok(Vec::new());
        }

        let mut spans = Vec::new();

        // Run all analyzers
        spans.extend(self.xml_tags.analyze(text)?);
        spans.extend(self.constraints.analyze(text)?);
        spans.extend(self.modality.analyze(text)?);
        spans.extend(self.ambiguity.analyze(text)?);
        spans.extend(self.domain_patterns.analyze(text)?);

        Ok(spans)
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

impl Default for StructuralHighlighter {
    fn default() -> Self {
        Self::new()
    }
}
