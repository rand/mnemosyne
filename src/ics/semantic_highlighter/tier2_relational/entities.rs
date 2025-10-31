//! Rule-based Named Entity Recognition (NER)
//!
//! Detects and classifies named entities using local heuristics:
//! - PERSON: Names with titles (Dr. Smith), capitalized names
//! - ORGANIZATION: Company names with suffixes (Inc., Corp.), capitalized orgs
//! - LOCATION: Place names, geographical terms
//! - TEMPORAL: Dates, times, temporal references
//! - CONCEPT: Abstract technical concepts, methodologies
//!
//! Uses dictionary-based matching, capitalization patterns, and context clues.

use crate::ics::semantic_highlighter::{
    visualization::{HighlightSpan, HighlightSource, SpanMetadata},
    utils::EntityDictionaries,
    Result,
};
use ratatui::style::{Color, Modifier, Style};
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Entity type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityType {
    Person,
    Organization,
    Location,
    Temporal,
    Concept,
}

impl EntityType {
    /// Get color for this entity type
    pub fn color(&self) -> Color {
        match self {
            EntityType::Person => Color::Rgb(255, 215, 0),      // Warm yellow
            EntityType::Organization => Color::Rgb(65, 105, 225), // Corporate blue
            EntityType::Location => Color::Rgb(46, 139, 87),     // Earth green
            EntityType::Temporal => Color::Rgb(255, 140, 0),     // Clock orange
            EntityType::Concept => Color::Rgb(147, 112, 219),    // Abstract purple
        }
    }

    /// Get description for this entity type
    pub fn description(&self) -> &'static str {
        match self {
            EntityType::Person => "Person",
            EntityType::Organization => "Organization",
            EntityType::Location => "Location",
            EntityType::Temporal => "Temporal",
            EntityType::Concept => "Concept",
        }
    }
}

/// Recognized entity with metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entity {
    pub entity_type: EntityType,
    pub text: String,
    pub range: std::ops::Range<usize>,
    pub confidence: f32,
}

/// Rule-based entity recognizer
pub struct EntityRecognizer {
    /// Minimum confidence threshold
    threshold: f32,
    /// Patterns for entity detection
    patterns: EntityPatterns,
}

/// Compiled regex patterns for entity detection
struct EntityPatterns {
    /// Capitalized words (potential names/places)
    capitalized: Regex,
    /// Person name patterns (Title + Name)
    person_with_title: Regex,
    /// Multi-word capitalized (potential org/location)
    multi_cap: Regex,
    /// Date patterns
    date_pattern: Regex,
    /// Time patterns
    time_pattern: Regex,
}

impl EntityPatterns {
    fn new() -> Self {
        Self {
            capitalized: Regex::new(r"\b[A-Z][a-z]+\b").unwrap(),
            person_with_title: Regex::new(r"\b(Dr|Prof|Mr|Mrs|Ms|Miss|Sir|Lord|Lady|Captain|President|Senator)\.\s+([A-Z][a-z]+(?:\s+[A-Z][a-z]+)*)\b").unwrap(),
            multi_cap: Regex::new(r"\b([A-Z][a-z]+(?:\s+[A-Z][a-z]+)+)\b").unwrap(),
            date_pattern: Regex::new(r"\b(\d{1,2}[/-]\d{1,2}[/-]\d{2,4}|\d{4}[/-]\d{1,2}[/-]\d{1,2}|(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[a-z]*\s+\d{1,2},?\s+\d{4})\b").unwrap(),
            time_pattern: Regex::new(r"\b(\d{1,2}:\d{2}(?::\d{2})?(?:\s*(?:AM|PM|am|pm))?)\b").unwrap(),
        }
    }
}

impl EntityRecognizer {
    pub fn new() -> Self {
        Self {
            threshold: 0.5,
            patterns: EntityPatterns::new(),
        }
    }

    /// Set confidence threshold
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Recognize entities in text
    pub fn recognize(&self, text: &str) -> Result<Vec<Entity>> {
        let mut entities = Vec::new();

        // Detect person names with titles
        entities.extend(self.detect_persons_with_titles(text)?);

        // Detect temporal entities
        entities.extend(self.detect_temporal(text)?);

        // Detect organizations
        entities.extend(self.detect_organizations(text)?);

        // Detect locations
        entities.extend(self.detect_locations(text)?);

        // Detect concepts
        entities.extend(self.detect_concepts(text)?);

        // Filter by confidence threshold
        entities.retain(|e| e.confidence >= self.threshold);

        // Resolve overlaps (keep highest confidence)
        entities = self.resolve_overlaps(entities);

        Ok(entities)
    }

    /// Convert entities to highlight spans
    pub fn entities_to_spans(&self, entities: &[Entity]) -> Vec<HighlightSpan> {
        entities
            .iter()
            .map(|entity| {
                let style = Style::default()
                    .fg(entity.entity_type.color())
                    .add_modifier(Modifier::BOLD);

                HighlightSpan {
                    range: entity.range.clone(),
                    style,
                    source: HighlightSource::Relational,
                    annotation: None,
                    confidence: entity.confidence,
                    metadata: Some(SpanMetadata {
                        entity_id: None,
                        entity_type: Some(entity.entity_type.description().to_string()),
                        relations: Vec::new(),
                        properties: std::collections::HashMap::new(),
                    }),
                }
            })
            .collect()
    }

    /// Detect person names with titles
    fn detect_persons_with_titles(&self, text: &str) -> Result<Vec<Entity>> {
        let mut entities = Vec::new();

        for cap in self.patterns.person_with_title.captures_iter(text) {
            let full_match = cap.get(0).unwrap();
            entities.push(Entity {
                entity_type: EntityType::Person,
                text: full_match.as_str().to_string(),
                range: full_match.start()..full_match.end(),
                confidence: 0.9, // High confidence for title + name pattern
            });
        }

        Ok(entities)
    }

    /// Detect temporal entities (dates, times)
    fn detect_temporal(&self, text: &str) -> Result<Vec<Entity>> {
        let mut entities = Vec::new();
        let text_lower = text.to_lowercase();

        // Detect dates
        for mat in self.patterns.date_pattern.find_iter(text) {
            entities.push(Entity {
                entity_type: EntityType::Temporal,
                text: mat.as_str().to_string(),
                range: mat.start()..mat.end(),
                confidence: 0.95,
            });
        }

        // Detect times
        for mat in self.patterns.time_pattern.find_iter(text) {
            entities.push(Entity {
                entity_type: EntityType::Temporal,
                text: mat.as_str().to_string(),
                range: mat.start()..mat.end(),
                confidence: 0.95,
            });
        }

        // Detect temporal words from dictionary
        for word in EntityDictionaries::temporal_indicators().iter() {
            let mut start = 0;
            while let Some(pos) = text_lower[start..].find(word) {
                let abs_pos = start + pos;
                entities.push(Entity {
                    entity_type: EntityType::Temporal,
                    text: text[abs_pos..abs_pos + word.len()].to_string(),
                    range: abs_pos..abs_pos + word.len(),
                    confidence: 0.7,
                });
                start = abs_pos + word.len();
            }
        }

        Ok(entities)
    }

    /// Detect organizations using multi-word capitalized patterns and suffixes
    fn detect_organizations(&self, text: &str) -> Result<Vec<Entity>> {
        let mut entities = Vec::new();
        let text_lower = text.to_lowercase();

        // Multi-word capitalized phrases followed by org indicators
        for cap in self.patterns.multi_cap.captures_iter(text) {
            let phrase = cap.get(1).unwrap();
            let phrase_text = phrase.as_str();

            // Check if followed by organization indicator
            let end_pos = phrase.end();
            let remaining = &text_lower[end_pos..];

            let has_org_indicator = EntityDictionaries::organization_indicators()
                .iter()
                .any(|indicator| remaining.starts_with(&format!(" {}", indicator)));

            if has_org_indicator {
                entities.push(Entity {
                    entity_type: EntityType::Organization,
                    text: phrase_text.to_string(),
                    range: phrase.start()..phrase.end(),
                    confidence: 0.8,
                });
            } else {
                // Lower confidence without explicit indicator
                entities.push(Entity {
                    entity_type: EntityType::Organization,
                    text: phrase_text.to_string(),
                    range: phrase.start()..phrase.end(),
                    confidence: 0.5,
                });
            }
        }

        Ok(entities)
    }

    /// Detect locations using patterns and indicators
    fn detect_locations(&self, text: &str) -> Result<Vec<Entity>> {
        let mut entities = Vec::new();
        let text_lower = text.to_lowercase();

        // Look for capitalized words near location indicators
        for indicator in EntityDictionaries::location_indicators().iter() {
            let mut start = 0;
            while let Some(pos) = text_lower[start..].find(indicator) {
                let abs_pos = start + pos;

                // Look for capitalized word before the indicator
                if abs_pos > 0 {
                    let before = &text[..abs_pos];
                    if let Some(cap_match) = self.patterns.capitalized.find_iter(before).last() {
                        entities.push(Entity {
                            entity_type: EntityType::Location,
                            text: cap_match.as_str().to_string(),
                            range: cap_match.start()..cap_match.end(),
                            confidence: 0.7,
                        });
                    }
                }

                start = abs_pos + indicator.len();
            }
        }

        Ok(entities)
    }

    /// Detect abstract concepts
    fn detect_concepts(&self, text: &str) -> Result<Vec<Entity>> {
        let mut entities = Vec::new();
        let text_lower = text.to_lowercase();

        // Match against concept dictionary
        for concept in EntityDictionaries::concept_indicators().iter() {
            let mut start = 0;
            while let Some(pos) = text_lower[start..].find(concept) {
                let abs_pos = start + pos;
                entities.push(Entity {
                    entity_type: EntityType::Concept,
                    text: text[abs_pos..abs_pos + concept.len()].to_string(),
                    range: abs_pos..abs_pos + concept.len(),
                    confidence: 0.8,
                });
                start = abs_pos + concept.len();
            }
        }

        Ok(entities)
    }

    /// Resolve overlapping entities (keep highest confidence)
    fn resolve_overlaps(&self, mut entities: Vec<Entity>) -> Vec<Entity> {
        if entities.is_empty() {
            return entities;
        }

        // Sort by start position, then by confidence (descending)
        entities.sort_by(|a, b| {
            a.range.start.cmp(&b.range.start)
                .then_with(|| b.confidence.partial_cmp(&a.confidence).unwrap())
        });

        let mut resolved = Vec::new();
        let mut last_end = 0;

        for entity in entities {
            if entity.range.start >= last_end {
                last_end = entity.range.end;
                resolved.push(entity);
            }
        }

        resolved
    }
}

impl Default for EntityRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_person_with_title() {
        let recognizer = EntityRecognizer::new();
        let text = "Dr. Smith presented the research to Prof. Johnson";
        let entities = recognizer.recognize(text).unwrap();

        let persons: Vec<_> = entities.iter()
            .filter(|e| e.entity_type == EntityType::Person)
            .collect();

        assert!(persons.len() >= 2);
        assert!(persons.iter().any(|e| e.text.contains("Smith")));
        assert!(persons.iter().any(|e| e.text.contains("Johnson")));
    }

    #[test]
    fn test_temporal_detection() {
        let recognizer = EntityRecognizer::new();
        let text = "The meeting is on 2024-01-15 at 3:30 PM";
        let entities = recognizer.recognize(text).unwrap();

        let temporal: Vec<_> = entities.iter()
            .filter(|e| e.entity_type == EntityType::Temporal)
            .collect();

        assert!(!temporal.is_empty());
    }

    #[test]
    fn test_organization_detection() {
        let recognizer = EntityRecognizer::new();
        let text = "Apple Inc and Google Corporation are competitors";
        let entities = recognizer.recognize(text).unwrap();

        let orgs: Vec<_> = entities.iter()
            .filter(|e| e.entity_type == EntityType::Organization)
            .collect();

        assert!(!orgs.is_empty());
    }

    #[test]
    fn test_concept_detection() {
        let recognizer = EntityRecognizer::new();
        let text = "The algorithm uses a tree-based approach with caching";
        let entities = recognizer.recognize(text).unwrap();

        let concepts: Vec<_> = entities.iter()
            .filter(|e| e.entity_type == EntityType::Concept)
            .collect();

        assert!(!concepts.is_empty());
        assert!(concepts.iter().any(|e| e.text == "algorithm"));
    }

    #[test]
    fn test_confidence_threshold() {
        let recognizer = EntityRecognizer::new().with_threshold(0.8);
        let text = "Some text with algorithm";
        let entities = recognizer.recognize(text).unwrap();

        // Should only include high-confidence entities
        for entity in entities {
            assert!(entity.confidence >= 0.8);
        }
    }

    #[test]
    fn test_overlap_resolution() {
        let recognizer = EntityRecognizer::new();

        // Create overlapping entities
        let entities = vec![
            Entity {
                entity_type: EntityType::Person,
                text: "John".to_string(),
                range: 0..4,
                confidence: 0.7,
            },
            Entity {
                entity_type: EntityType::Person,
                text: "John Smith".to_string(),
                range: 0..10,
                confidence: 0.9,
            },
        ];

        let resolved = recognizer.resolve_overlaps(entities);

        // Should keep only the higher confidence "John Smith"
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].text, "John Smith");
    }

    #[test]
    fn test_entities_to_spans() {
        let recognizer = EntityRecognizer::new();
        let entities = vec![
            Entity {
                entity_type: EntityType::Person,
                text: "Smith".to_string(),
                range: 0..5,
                confidence: 0.9,
            },
        ];

        let spans = recognizer.entities_to_spans(&entities);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].range, 0..5);
        assert_eq!(spans[0].source, HighlightSource::Relational);
    }
}
