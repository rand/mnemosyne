//! Modality and hedging analyzer
//!
//! Detects epistemic modality markers that indicate the certainty level of statements:
//! - Certain: definitely, clearly, must, will, cannot
//! - Probable: probably, likely, should, usually
//! - Uncertain: maybe, perhaps, might, possibly
//! - Conditional: if, unless, assuming, provided
//!
//! This helps identify hedged language, speculative statements, and certainty levels.

use crate::ics::semantic_highlighter::{
    utils::ModalityDictionaries,
    visualization::{HighlightSource, HighlightSpan},
    Result,
};
use once_cell::sync::Lazy;
use ratatui::style::{Color, Modifier, Style};
use regex::Regex;

/// Modality level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModalityLevel {
    Certain,
    Probable,
    Uncertain,
    Conditional,
}

impl ModalityLevel {
    /// Get color for this modality level
    fn color(&self) -> Color {
        match self {
            ModalityLevel::Certain => Color::Green,
            ModalityLevel::Probable => Color::Yellow,
            ModalityLevel::Uncertain => Color::Magenta,
            ModalityLevel::Conditional => Color::Cyan,
        }
    }

    /// Get underline style for this modality level
    fn underline_style(&self) -> Modifier {
        match self {
            ModalityLevel::Certain => Modifier::BOLD,
            ModalityLevel::Probable => Modifier::UNDERLINED,
            ModalityLevel::Uncertain => Modifier::DIM,
            ModalityLevel::Conditional => Modifier::ITALIC,
        }
    }

    /// Get description for this modality level
    fn _description(&self) -> &'static str {
        match self {
            ModalityLevel::Certain => "High certainty",
            ModalityLevel::Probable => "Probable",
            ModalityLevel::Uncertain => "Uncertain/hedged",
            ModalityLevel::Conditional => "Conditional",
        }
    }
}

/// Modality analyzer
pub struct ModalityAnalyzer {
    /// Word boundary regex for matching
    word_boundary: &'static Regex,
}

impl ModalityAnalyzer {
    pub fn new() -> Self {
        static WORD_BOUNDARY: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"\b\w+\b").expect("Valid word boundary regex"));

        Self {
            word_boundary: &WORD_BOUNDARY,
        }
    }

    /// Analyze text for modality markers
    pub fn analyze(&self, text: &str) -> Result<Vec<HighlightSpan>> {
        let mut spans = Vec::new();
        let text_lower = text.to_lowercase();

        // Find all words in the text
        for mat in self.word_boundary.find_iter(&text_lower) {
            let word = mat.as_str();
            let start = mat.start();
            let end = mat.end();

            // Check against each modality dictionary
            let level = if ModalityDictionaries::certain_markers().contains(word) {
                Some(ModalityLevel::Certain)
            } else if ModalityDictionaries::probable_markers().contains(word) {
                Some(ModalityLevel::Probable)
            } else if ModalityDictionaries::uncertain_markers().contains(word) {
                Some(ModalityLevel::Uncertain)
            } else if ModalityDictionaries::conditional_markers().contains(word) {
                Some(ModalityLevel::Conditional)
            } else {
                None
            };

            if let Some(level) = level {
                let style = Style::default()
                    .fg(level.color())
                    .add_modifier(level.underline_style());

                spans.push(HighlightSpan {
                    range: start..end,
                    style,
                    source: HighlightSource::Structural,
                    annotation: None,
                    confidence: 0.8, // Dictionary-based matching is fairly confident
                    metadata: None,
                });
            }
        }

        // Also detect multi-word modality phrases
        spans.extend(self.detect_phrases(&text_lower)?);

        Ok(spans)
    }

    /// Detect multi-word modality phrases
    fn detect_phrases(&self, text: &str) -> Result<Vec<HighlightSpan>> {
        let mut spans = Vec::new();

        // Common multi-word modality phrases
        let phrases = [
            ("in fact", ModalityLevel::Certain),
            ("in reality", ModalityLevel::Certain),
            ("without doubt", ModalityLevel::Certain),
            ("for certain", ModalityLevel::Certain),
            ("most likely", ModalityLevel::Probable),
            ("more likely", ModalityLevel::Probable),
            ("it seems", ModalityLevel::Probable),
            ("it appears", ModalityLevel::Probable),
            ("might be", ModalityLevel::Uncertain),
            ("could be", ModalityLevel::Uncertain),
            ("may be", ModalityLevel::Uncertain),
            ("sort of", ModalityLevel::Uncertain),
            ("kind of", ModalityLevel::Uncertain),
            ("in case", ModalityLevel::Conditional),
            ("provided that", ModalityLevel::Conditional),
            ("assuming that", ModalityLevel::Conditional),
            ("on condition", ModalityLevel::Conditional),
        ];

        for (phrase, level) in phrases.iter() {
            // Find all occurrences of this phrase
            let mut start = 0;
            while let Some(pos) = text[start..].find(phrase) {
                let abs_pos = start + pos;
                let end = abs_pos + phrase.len();

                let style = Style::default()
                    .fg(level.color())
                    .add_modifier(level.underline_style());

                spans.push(HighlightSpan {
                    range: abs_pos..end,
                    style,
                    source: HighlightSource::Structural,
                    annotation: None,
                    confidence: 0.9, // Phrase matching is more confident than single words
                    metadata: None,
                });

                start = end;
            }
        }

        Ok(spans)
    }
}

impl Default for ModalityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_certain_markers() {
        let analyzer = ModalityAnalyzer::new();
        let text = "This is definitely the correct approach";
        let spans = analyzer.analyze(text).unwrap();

        assert!(spans.len() > 0);
        let definitely_span = spans
            .iter()
            .find(|s| text[s.range.clone()].to_lowercase() == "definitely")
            .expect("Should find 'definitely'");

        assert!(definitely_span.confidence > 0.0);
    }

    #[test]
    fn test_probable_markers() {
        let analyzer = ModalityAnalyzer::new();
        let text = "This will probably work as expected";
        let spans = analyzer.analyze(text).unwrap();

        let probably_span = spans
            .iter()
            .find(|s| text[s.range.clone()].to_lowercase() == "probably")
            .expect("Should find 'probably'");

        assert!(probably_span.confidence > 0.0);
    }

    #[test]
    fn test_uncertain_markers() {
        let analyzer = ModalityAnalyzer::new();
        let text = "This might be the issue";
        let spans = analyzer.analyze(text).unwrap();

        let might_span = spans
            .iter()
            .find(|s| text[s.range.clone()].to_lowercase() == "might")
            .expect("Should find 'might'");

        assert!(might_span.confidence > 0.0);
    }

    #[test]
    fn test_conditional_markers() {
        let analyzer = ModalityAnalyzer::new();
        let text = "If we add caching, the system will be faster";
        let spans = analyzer.analyze(text).unwrap();

        let if_span = spans
            .iter()
            .find(|s| text[s.range.clone()].to_lowercase() == "if")
            .expect("Should find 'if'");

        assert!(if_span.confidence > 0.0);
    }

    #[test]
    fn test_multi_word_phrases() {
        let analyzer = ModalityAnalyzer::new();
        let text = "It seems that in fact the issue might be here";
        let spans = analyzer.analyze(text).unwrap();

        // Should find both "it seems" and "in fact" and "might"
        assert!(spans.len() >= 2);
    }

    #[test]
    fn test_multiple_markers() {
        let analyzer = ModalityAnalyzer::new();
        let text = "This probably works, but maybe we should test it";
        let spans = analyzer.analyze(text).unwrap();

        // Should find: probably, maybe, should
        assert!(spans.len() >= 3);
    }
}
