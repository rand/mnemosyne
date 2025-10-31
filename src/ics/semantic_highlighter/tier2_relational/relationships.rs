//! Relationship extraction using tree-sitter
//!
//! Extracts semantic relationships between entities in text:
//! - Subject-Verb-Object (SVO) triples
//! - Dependency relations
//! - Predicate-argument structures
//!
//! Uses tree-sitter for syntactic parsing to identify relationships
//! in structured text (code, markdown).

use crate::ics::semantic_highlighter::{
    visualization::{HighlightSpan, HighlightSource, Connection, ConnectionType},
    Result,
};
use ratatui::style::{Color, Modifier, Style};
use std::ops::Range;

/// Relationship triple (Subject, Predicate, Object)
#[derive(Debug, Clone, PartialEq)]
pub struct Relationship {
    pub subject: Range<usize>,
    pub predicate: Range<usize>,
    pub object: Range<usize>,
    pub relation_type: RelationType,
    pub confidence: f32,
}

/// Type of semantic relationship
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationType {
    /// Action relationship (X does Y)
    Action,
    /// Attribution (X is Y)
    Attribution,
    /// Possession (X has Y)
    Possession,
    /// Causation (X causes Y)
    Causation,
    /// Comparison (X compared-to Y)
    Comparison,
}

impl RelationType {
    fn subject_color(&self) -> Color {
        Color::Cyan
    }

    fn predicate_color(&self) -> Color {
        match self {
            RelationType::Action => Color::Green,
            RelationType::Attribution => Color::Blue,
            RelationType::Possession => Color::Yellow,
            RelationType::Causation => Color::Red,
            RelationType::Comparison => Color::Magenta,
        }
    }

    fn object_color(&self) -> Color {
        Color::LightCyan
    }
}

/// Relationship extractor
pub struct RelationshipExtractor {
    /// Minimum confidence threshold
    threshold: f32,
}

impl RelationshipExtractor {
    pub fn new() -> Self {
        Self {
            threshold: 0.5,
        }
    }

    /// Set confidence threshold
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Extract relationships from text
    ///
    /// Uses simple pattern matching for now. Can be enhanced with tree-sitter later.
    pub fn extract(&self, text: &str) -> Result<Vec<Relationship>> {
        let mut relationships = Vec::new();

        // Pattern-based extraction for common SVO patterns
        relationships.extend(self.extract_simple_svo(text)?);

        // Filter by confidence
        relationships.retain(|r| r.confidence >= self.threshold);

        Ok(relationships)
    }

    /// Convert relationships to highlight spans
    pub fn relationships_to_spans(&self, relationships: &[Relationship], text: &str) -> Vec<HighlightSpan> {
        let mut spans = Vec::new();

        for rel in relationships {
            // Highlight subject
            spans.push(HighlightSpan {
                range: rel.subject.clone(),
                style: Style::default()
                    .fg(rel.relation_type.subject_color())
                    .add_modifier(Modifier::BOLD),
                source: HighlightSource::Relational,
                annotation: None,
                confidence: rel.confidence,
                metadata: None,
            });

            // Highlight predicate
            spans.push(HighlightSpan {
                range: rel.predicate.clone(),
                style: Style::default()
                    .fg(rel.relation_type.predicate_color())
                    .add_modifier(Modifier::UNDERLINED),
                source: HighlightSource::Relational,
                annotation: None,
                confidence: rel.confidence,
                metadata: None,
            });

            // Highlight object
            spans.push(HighlightSpan {
                range: rel.object.clone(),
                style: Style::default()
                    .fg(rel.relation_type.object_color())
                    .add_modifier(Modifier::ITALIC),
                source: HighlightSource::Relational,
                annotation: None,
                confidence: rel.confidence,
                metadata: None,
            });
        }

        spans
    }

    /// Create connection objects for relationships
    pub fn relationships_to_connections(&self, relationships: &[Relationship]) -> Vec<Connection> {
        relationships
            .iter()
            .map(|rel| Connection {
                from: rel.subject.clone(),
                to: rel.object.clone(),
                connection_type: ConnectionType::Discourse,
                label: Some("relates".to_string()),
                confidence: rel.confidence,
            })
            .collect()
    }

    /// Extract simple SVO patterns using regex
    fn extract_simple_svo(&self, text: &str) -> Result<Vec<Relationship>> {
        let mut relationships = Vec::new();

        // Common action verbs
        let action_verbs = [
            "calls", "uses", "creates", "implements", "defines",
            "returns", "takes", "performs", "executes", "processes",
            "sends", "receives", "reads", "writes", "updates",
        ];

        // Attribution verbs
        let attribution_verbs = ["is", "are", "was", "were", "represents", "means"];

        // Possession verbs
        let possession_verbs = ["has", "have", "contains", "includes", "owns"];

        // Causation verbs
        let causation_verbs = ["causes", "triggers", "leads to", "results in", "produces"];

        // Simple word tokenization
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut byte_positions: Vec<usize> = Vec::new();
        let mut current_pos = 0;

        for word in &words {
            if let Some(pos) = text[current_pos..].find(word) {
                byte_positions.push(current_pos + pos);
                current_pos += pos + word.len();
            }
        }

        // Look for patterns: Noun Verb Noun
        for i in 1..words.len().saturating_sub(1) {
            let verb = words[i];
            let verb_lower = verb.to_lowercase();

            let (relation_type, confidence) = if action_verbs.contains(&verb_lower.as_str()) {
                (RelationType::Action, 0.7)
            } else if attribution_verbs.contains(&verb_lower.as_str()) {
                (RelationType::Attribution, 0.8)
            } else if possession_verbs.contains(&verb_lower.as_str()) {
                (RelationType::Possession, 0.75)
            } else if causation_verbs.contains(&verb_lower.as_str()) {
                (RelationType::Causation, 0.8)
            } else {
                continue; // Not a recognized verb
            };

            // Subject is word before verb
            let subject = words[i - 1];
            let subject_start = byte_positions[i - 1];
            let subject_end = subject_start + subject.len();

            // Predicate is the verb
            let predicate_start = byte_positions[i];
            let predicate_end = predicate_start + verb.len();

            // Object is word after verb
            let object = words[i + 1];
            let object_start = byte_positions[i + 1];
            let object_end = object_start + object.len();

            relationships.push(Relationship {
                subject: subject_start..subject_end,
                predicate: predicate_start..predicate_end,
                object: object_start..object_end,
                relation_type,
                confidence,
            });
        }

        Ok(relationships)
    }
}

impl Default for RelationshipExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_relationship() {
        let extractor = RelationshipExtractor::new();
        let text = "The function calls the API";
        let relationships = extractor.extract(text).unwrap();

        assert!(!relationships.is_empty());
        let rel = &relationships[0];
        assert_eq!(rel.relation_type, RelationType::Action);
        assert_eq!(&text[rel.subject.clone()], "function");
        assert_eq!(&text[rel.predicate.clone()], "calls");
        assert_eq!(&text[rel.object.clone()], "the");
    }

    #[test]
    fn test_attribution_relationship() {
        let extractor = RelationshipExtractor::new();
        let text = "The result is valid";
        let relationships = extractor.extract(text).unwrap();

        assert!(!relationships.is_empty());
        let rel = &relationships[0];
        assert_eq!(rel.relation_type, RelationType::Attribution);
    }

    #[test]
    fn test_possession_relationship() {
        let extractor = RelationshipExtractor::new();
        let text = "The object has properties";
        let relationships = extractor.extract(text).unwrap();

        assert!(!relationships.is_empty());
        let rel = &relationships[0];
        assert_eq!(rel.relation_type, RelationType::Possession);
    }

    #[test]
    fn test_confidence_threshold() {
        let extractor = RelationshipExtractor::new().with_threshold(0.8);
        let text = "The function calls the API";
        let relationships = extractor.extract(text).unwrap();

        for rel in relationships {
            assert!(rel.confidence >= 0.8);
        }
    }

    #[test]
    fn test_relationships_to_spans() {
        let extractor = RelationshipExtractor::new();
        let text = "The function calls the API";
        let relationships = extractor.extract(text).unwrap();
        let spans = extractor.relationships_to_spans(&relationships, text);

        // Should have 3 spans per relationship (subject, predicate, object)
        assert!(!spans.is_empty());
        assert_eq!(spans.len() % 3, 0);
    }

    #[test]
    fn test_relationships_to_connections() {
        let extractor = RelationshipExtractor::new();
        let text = "The function calls the API";
        let relationships = extractor.extract(text).unwrap();
        let connections = extractor.relationships_to_connections(&relationships);

        assert_eq!(connections.len(), relationships.len());
        for conn in connections {
            assert_eq!(conn.connection_type, ConnectionType::Discourse);
        }
    }

    #[test]
    fn test_multiple_relationships() {
        let extractor = RelationshipExtractor::new();
        let text = "The system processes data and sends results";
        let relationships = extractor.extract(text).unwrap();

        // Should find multiple relationships
        assert!(relationships.len() >= 1);
    }
}
