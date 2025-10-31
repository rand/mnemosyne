//! Tier 2: Relational Analysis (Local, Incremental <200ms)
//!
//! Local NLP analysis using heuristics and pattern matching.
//! No external API calls - all processing is local.

use crate::ics::semantic_highlighter::{
    cache::SemanticCache,
    settings::RelationalSettings,
    visualization::HighlightSpan,
    Result,
};
use std::ops::Range;
use std::sync::Arc;

pub mod entities;
pub mod coreference;
pub mod relationships;
pub mod semantic_roles;
pub mod anaphora;

pub use entities::{EntityRecognizer, EntityType, Entity};
pub use coreference::{CoreferenceResolver, CorefChain, Mention};
pub use relationships::{RelationshipExtractor, Relationship, RelationType};
pub use semantic_roles::{SemanticRoleLabeler, RoleAssignment, SemanticRole};
pub use anaphora::{AnaphoraResolver, AnaphoraResolution, Anaphor};

/// Relational analyzer
///
/// Performs local NLP analysis with caching and debouncing.
pub struct RelationalAnalyzer {
    settings: RelationalSettings,
    cache: Arc<SemanticCache>,
    entity_recognizer: EntityRecognizer,
    coref_resolver: CoreferenceResolver,
    relation_extractor: RelationshipExtractor,
    role_labeler: SemanticRoleLabeler,
    anaphora_resolver: AnaphoraResolver,
}

impl RelationalAnalyzer {
    pub fn new(settings: RelationalSettings, cache: Arc<SemanticCache>) -> Self {
        Self {
            settings: settings.clone(),
            cache,
            entity_recognizer: EntityRecognizer::new()
                .with_threshold(settings.min_entity_confidence),
            coref_resolver: CoreferenceResolver::new()
                .with_max_distance(settings.max_coref_distance)
                .with_threshold(settings.min_entity_confidence),
            relation_extractor: RelationshipExtractor::new()
                .with_threshold(settings.min_entity_confidence),
            role_labeler: SemanticRoleLabeler::new()
                .with_threshold(settings.min_entity_confidence),
            anaphora_resolver: AnaphoraResolver::new()
                .with_max_lookback(settings.max_coref_distance)
                .with_threshold(settings.min_entity_confidence),
        }
    }

    /// Get cached highlights for text
    pub fn highlight_cached(&self, text: &str) -> Result<Vec<HighlightSpan>> {
        let mut spans = Vec::new();

        // Run all analyzers
        if let Ok(entities) = self.entity_recognizer.recognize(text) {
            spans.extend(self.entity_recognizer.entities_to_spans(&entities));
        }

        if let Ok(relationships) = self.relation_extractor.extract(text) {
            spans.extend(self.relation_extractor.relationships_to_spans(&relationships, text));
        }

        if let Ok(roles) = self.role_labeler.label(text) {
            spans.extend(self.role_labeler.roles_to_spans(&roles));
        }

        // Coreference and anaphora resolution create connections, not spans directly
        // These would be used for visual connection rendering

        Ok(spans)
    }

    /// Schedule analysis for range (debounced)
    pub fn schedule_analysis(&mut self, _text: &str, _range: Range<usize>) {
        // TODO: Implement debounced analysis scheduling
        // This would be used for incremental analysis as the user types
    }

    pub fn update_settings(&mut self, settings: RelationalSettings) {
        self.settings = settings.clone();

        // Update threshold for all analyzers
        let threshold = settings.min_entity_confidence;
        self.entity_recognizer = EntityRecognizer::new().with_threshold(threshold);
        self.coref_resolver = CoreferenceResolver::new()
            .with_max_distance(settings.max_coref_distance)
            .with_threshold(threshold);
        self.relation_extractor = RelationshipExtractor::new().with_threshold(threshold);
        self.role_labeler = SemanticRoleLabeler::new().with_threshold(threshold);
        self.anaphora_resolver = AnaphoraResolver::new()
            .with_max_lookback(settings.max_coref_distance)
            .with_threshold(threshold);
    }
}
