//! Tier 2: Relational Analysis (Local, Incremental <200ms)
//!
//! Local NLP analysis using heuristics and pattern matching.
//! No external API calls - all processing is local.

use crate::ics::semantic_highlighter::{
    cache::{CachedResult, SemanticCache},
    incremental::{Debouncer, DirtyRegions},
    settings::RelationalSettings,
    visualization::HighlightSpan,
    Result,
};
use serde::{Deserialize, Serialize};
use std::ops::Range;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;
use tracing::{debug, info};

pub mod anaphora;
pub mod coreference;
pub mod entities;
pub mod relationships;
pub mod semantic_roles;

pub use anaphora::{Anaphor, AnaphoraResolution, AnaphoraResolver};
pub use coreference::{CorefChain, CoreferenceResolver, Mention};
pub use entities::{Entity, EntityRecognizer, EntityType};
pub use relationships::{RelationType, Relationship, RelationshipExtractor};
pub use semantic_roles::{RoleAssignment, SemanticRole, SemanticRoleLabeler};

/// Analysis request for background processing
#[derive(Debug, Clone)]
enum AnalysisRequest {
    /// Incremental analysis of dirty regions
    Incremental(String),
}

/// Analysis result wrapper for caching (similar to Tier 3)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Tier2AnalysisResult {
    Entities(Vec<Entity>),
    Relationships(Vec<Relationship>),
    Roles(Vec<RoleAssignment>),
    Combined {
        entities: Vec<Entity>,
        relationships: Vec<Relationship>,
        roles: Vec<RoleAssignment>,
    },
}

impl Tier2AnalysisResult {
    /// Convert cached analysis results to highlight spans
    pub fn to_spans(
        &self,
        entity_recognizer: &EntityRecognizer,
        relation_extractor: &RelationshipExtractor,
        role_labeler: &SemanticRoleLabeler,
        text: &str,
    ) -> Vec<HighlightSpan> {
        match self {
            Tier2AnalysisResult::Entities(entities) => {
                entity_recognizer.entities_to_spans(entities)
            }
            Tier2AnalysisResult::Relationships(relationships) => {
                relation_extractor.relationships_to_spans(relationships, text)
            }
            Tier2AnalysisResult::Roles(roles) => role_labeler.roles_to_spans(roles),
            Tier2AnalysisResult::Combined {
                entities,
                relationships,
                roles,
            } => {
                let mut spans = Vec::new();
                spans.extend(entity_recognizer.entities_to_spans(entities));
                spans.extend(relation_extractor.relationships_to_spans(relationships, text));
                spans.extend(role_labeler.roles_to_spans(roles));
                spans
            }
        }
    }
}

/// Relational analyzer
///
/// Performs local NLP analysis with caching and debouncing.
pub struct RelationalAnalyzer {
    settings: RelationalSettings,
    cache: Arc<SemanticCache>,

    // Analyzers
    entity_recognizer: EntityRecognizer,
    coref_resolver: CoreferenceResolver,
    relation_extractor: RelationshipExtractor,
    role_labeler: SemanticRoleLabeler,
    anaphora_resolver: AnaphoraResolver,

    // Incremental analysis support
    dirty_regions: Arc<RwLock<DirtyRegions>>,
    debouncer: Arc<RwLock<Debouncer>>,
    analysis_tx: Option<mpsc::Sender<AnalysisRequest>>,
}

impl RelationalAnalyzer {
    pub fn new(settings: RelationalSettings, cache: Arc<SemanticCache>) -> Self {
        let dirty_regions = Arc::new(RwLock::new(DirtyRegions::new()));
        let debouncer = Arc::new(RwLock::new(Debouncer::new(settings.debounce_ms)));

        // Create background analysis channel
        let (tx, rx) = mpsc::channel(32);

        // Spawn background analysis task only if tokio runtime is available
        // This allows tests to run without a runtime
        if tokio::runtime::Handle::try_current().is_ok() {
            let cache_clone = Arc::clone(&cache);
            let dirty_clone = Arc::clone(&dirty_regions);
            let settings_clone = settings.clone();

            tokio::spawn(async move {
                Self::analysis_loop(rx, cache_clone, dirty_clone, settings_clone).await
            });
        }

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
            role_labeler: SemanticRoleLabeler::new().with_threshold(settings.min_entity_confidence),
            anaphora_resolver: AnaphoraResolver::new()
                .with_max_lookback(settings.max_coref_distance)
                .with_threshold(settings.min_entity_confidence),
            dirty_regions,
            debouncer,
            analysis_tx: Some(tx),
        }
    }

    /// Get cached highlights for range (non-blocking)
    pub fn get_cached_highlights(
        &self,
        range: &Range<usize>,
        text: &str,
    ) -> Result<Vec<HighlightSpan>> {
        if let Some(cached) = self.cache.relational.get(range) {
            // Deserialize cached result
            if let Ok(result) = serde_json::from_value::<Tier2AnalysisResult>(cached.data) {
                debug!("Cache hit for range {:?}", range);
                return Ok(result.to_spans(
                    &self.entity_recognizer,
                    &self.relation_extractor,
                    &self.role_labeler,
                    text,
                ));
            }
        }
        Ok(Vec::new())
    }

    /// Get cached highlights for text
    pub fn highlight_cached(&self, text: &str) -> Result<Vec<HighlightSpan>> {
        let mut spans = Vec::new();

        // Try cache for full text range first
        let full_range = 0..text.len();
        if let Some(cached) = self.cache.relational.get(&full_range) {
            if let Ok(result) = serde_json::from_value::<Tier2AnalysisResult>(cached.data) {
                debug!("Cache hit for full text");
                return Ok(result.to_spans(
                    &self.entity_recognizer,
                    &self.relation_extractor,
                    &self.role_labeler,
                    text,
                ));
            }
        }

        // Cache miss - run analysis synchronously (still fast <200ms)
        debug!("Cache miss - running synchronous analysis");

        // Run all analyzers
        if let Ok(entities) = self.entity_recognizer.recognize(text) {
            spans.extend(self.entity_recognizer.entities_to_spans(&entities));
        }

        if let Ok(relationships) = self.relation_extractor.extract(text) {
            spans.extend(
                self.relation_extractor
                    .relationships_to_spans(&relationships, text),
            );
        }

        if let Ok(roles) = self.role_labeler.label(text) {
            spans.extend(self.role_labeler.roles_to_spans(&roles));
        }

        // Coreference and anaphora resolution create connections, not spans directly
        // These would be used for visual connection rendering

        Ok(spans)
    }

    /// Schedule analysis for range (debounced)
    pub fn schedule_analysis(&mut self, text: &str, range: Range<usize>) {
        debug!("Scheduling analysis for range {:?}", range);

        // Mark region as dirty
        if let Ok(mut dirty) = self.dirty_regions.write() {
            dirty.mark_dirty(range.clone());
        }

        // Invalidate cache for overlapping entries
        self.cache.relational.invalidate_range(&range);

        // Send debounced analysis request only if tokio runtime is available
        if let Some(ref tx) = self.analysis_tx {
            if tokio::runtime::Handle::try_current().is_ok() {
                let tx_clone = tx.clone();
                let text_owned = text.to_string();
                let debounce_delay = self.settings.debounce_ms;

                tokio::spawn(async move {
                    // Wait for debounce delay
                    tokio::time::sleep(std::time::Duration::from_millis(debounce_delay)).await;

                    // Trigger analysis
                    let _ = tx_clone
                        .send(AnalysisRequest::Incremental(text_owned))
                        .await;
                });
            }
        }
    }

    /// Background analysis loop
    async fn analysis_loop(
        mut rx: mpsc::Receiver<AnalysisRequest>,
        _cache: Arc<SemanticCache>,
        dirty_regions: Arc<RwLock<DirtyRegions>>,
        settings: RelationalSettings,
    ) {
        info!("Starting relational analysis loop");

        while let Some(request) = rx.recv().await {
            match request {
                AnalysisRequest::Incremental(text) => {
                    // Get dirty regions
                    let regions = {
                        if let Ok(dirty) = dirty_regions.read() {
                            dirty.get_dirty().to_vec()
                        } else {
                            continue;
                        }
                    };

                    if regions.is_empty() {
                        debug!("No dirty regions to analyze");
                        continue;
                    }

                    debug!("Analyzing {} dirty regions", regions.len());

                    // Analyze each dirty region
                    for region in regions {
                        if region.end > text.len() {
                            debug!(
                                "Skipping invalid region {:?} (text len: {})",
                                region,
                                text.len()
                            );
                            continue;
                        }

                        let region_text = &text[region.clone()];

                        // Run local analysis on region
                        // Create temporary analyzers with configured thresholds
                        let entity_recognizer =
                            EntityRecognizer::new().with_threshold(settings.min_entity_confidence);
                        let relation_extractor = RelationshipExtractor::new()
                            .with_threshold(settings.min_entity_confidence);
                        let role_labeler = SemanticRoleLabeler::new()
                            .with_threshold(settings.min_entity_confidence);

                        // Analyze all components
                        let entities = entity_recognizer.recognize(region_text).unwrap_or_default();
                        let relationships =
                            relation_extractor.extract(region_text).unwrap_or_default();
                        let roles = role_labeler.label(region_text).unwrap_or_default();

                        debug!(
                            "Analyzed region {:?}: {} entities, {} relationships, {} roles",
                            region,
                            entities.len(),
                            relationships.len(),
                            roles.len()
                        );

                        // Cache combined results
                        if !entities.is_empty() || !relationships.is_empty() || !roles.is_empty() {
                            let result = Tier2AnalysisResult::Combined {
                                entities,
                                relationships,
                                roles,
                            };

                            if let Ok(json_value) = serde_json::to_value(&result) {
                                let cached = CachedResult::new(json_value)
                                    .with_confidence(settings.min_entity_confidence);
                                _cache.relational.insert(region.clone(), cached);
                                debug!("Cached Tier 2 results for region {:?}", region);
                            } else {
                                debug!(
                                    "Failed to serialize Tier 2 results for region {:?}",
                                    region
                                );
                            }
                        }
                    }

                    // Clear dirty regions after successful analysis
                    if let Ok(mut dirty) = dirty_regions.write() {
                        dirty.clear();
                        debug!("Cleared dirty regions");
                    }
                }
            }
        }

        info!("Relational analysis loop terminated");
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

        // Update debouncer delay
        if let Ok(mut debouncer) = self.debouncer.write() {
            *debouncer = Debouncer::new(settings.debounce_ms);
        }
    }
}
