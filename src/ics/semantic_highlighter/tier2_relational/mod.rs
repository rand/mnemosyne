//! Tier 2: Relational Analysis (Local, Incremental <200ms)
//!
//! Local NLP analysis using heuristics and pattern matching.
//! No external API calls - all processing is local.

use crate::ics::semantic_highlighter::{
    cache::SemanticCache,
    incremental::{DirtyRegions, Debouncer},
    settings::RelationalSettings,
    visualization::HighlightSpan,
    Result,
};
use std::ops::Range;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;
use tracing::{debug, info};

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

/// Analysis request for background processing
#[derive(Debug, Clone)]
enum AnalysisRequest {
    /// Incremental analysis of dirty regions
    Incremental(String),
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

        // Spawn background analysis task
        let cache_clone = Arc::clone(&cache);
        let dirty_clone = Arc::clone(&dirty_regions);
        let settings_clone = settings.clone();

        tokio::spawn(async move {
            Self::analysis_loop(rx, cache_clone, dirty_clone, settings_clone).await
        });

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
            dirty_regions,
            debouncer,
            analysis_tx: Some(tx),
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
    pub fn schedule_analysis(&mut self, text: &str, range: Range<usize>) {
        debug!("Scheduling analysis for range {:?}", range);

        // Mark region as dirty
        if let Ok(mut dirty) = self.dirty_regions.write() {
            dirty.mark_dirty(range.clone());
        }

        // Invalidate cache for overlapping entries
        self.cache.relational.invalidate_range(&range);

        // Send debounced analysis request
        if let Some(ref tx) = self.analysis_tx {
            let tx_clone = tx.clone();
            let text_owned = text.to_string();
            let debounce_delay = self.settings.debounce_ms;

            tokio::spawn(async move {
                // Wait for debounce delay
                tokio::time::sleep(std::time::Duration::from_millis(debounce_delay)).await;

                // Trigger analysis
                let _ = tx_clone.send(AnalysisRequest::Incremental(text_owned)).await;
            });
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
                            debug!("Skipping invalid region {:?} (text len: {})", region, text.len());
                            continue;
                        }

                        let region_text = &text[region.clone()];

                        // Run local analysis on region
                        // Create temporary analyzers with configured thresholds
                        let entity_recognizer = EntityRecognizer::new()
                            .with_threshold(settings.min_entity_confidence);
                        let _relation_extractor = RelationshipExtractor::new()
                            .with_threshold(settings.min_entity_confidence);

                        // Analyze entities in region
                        if let Ok(_entities) = entity_recognizer.recognize(region_text) {
                            // Results would be cached here in production
                            // For now, analysis happens but results aren't stored
                            debug!("Analyzed region {:?}", region);
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
