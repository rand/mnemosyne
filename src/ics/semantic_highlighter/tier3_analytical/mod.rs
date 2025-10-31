//! Tier 3: Analytical Processing (Claude API, Optional, 2s+)
//!
//! Advanced semantic analysis using Claude API.
//! Batched, cached, and completely optional.

use crate::{
    ics::semantic_highlighter::{
        cache::SemanticCache,
        settings::AnalyticalSettings,
        visualization::HighlightSpan,
        Result,
    },
    LlmService,
};
use std::sync::Arc;
use tokio::sync::mpsc;

pub mod discourse;
pub mod contradictions;
pub mod pragmatics;
pub mod batching;

pub use discourse::{DiscourseAnalyzer, DiscourseSegment, DiscourseRelation, CoherenceScore};
pub use contradictions::{ContradictionDetector, Contradiction, ContradictionType};
pub use pragmatics::{PragmaticsAnalyzer, PragmaticElement, PragmaticType, SpeechActType};
pub use batching::{RequestBatcher, BatchRequest, BatchConfig, AnalysisType};

use super::engine::AnalysisRequest;

/// Analytical processor using Claude API
///
/// Runs in background, batches requests, caches aggressively.
pub struct AnalyticalProcessor {
    llm_service: Arc<LlmService>,
    settings: AnalyticalSettings,
    cache: Arc<SemanticCache>,
    request_rx: mpsc::Receiver<AnalysisRequest>,

    // Analyzers
    discourse_analyzer: DiscourseAnalyzer,
    contradiction_detector: ContradictionDetector,
    pragmatics_analyzer: PragmaticsAnalyzer,

    // Batching system
    batcher: Arc<RequestBatcher>,
}

impl AnalyticalProcessor {
    pub fn new(
        llm_service: Arc<LlmService>,
        settings: AnalyticalSettings,
        cache: Arc<SemanticCache>,
        request_rx: mpsc::Receiver<AnalysisRequest>,
    ) -> Self {
        let batch_config = BatchConfig {
            max_batch_size: settings.max_batch_size,
            max_wait_duration: std::time::Duration::from_millis(settings.batch_wait_ms),
            rate_limit_rpm: settings.rate_limit_rpm,
            ..Default::default()
        };

        Self {
            discourse_analyzer: DiscourseAnalyzer::new(Arc::clone(&llm_service)),
            contradiction_detector: ContradictionDetector::new(Arc::clone(&llm_service))
                .with_threshold(0.7),
            pragmatics_analyzer: PragmaticsAnalyzer::new(Arc::clone(&llm_service))
                .with_threshold(0.6),
            batcher: Arc::new(RequestBatcher::new(batch_config)),
            llm_service,
            settings,
            cache,
            request_rx,
        }
    }

    /// Get cached highlights (non-blocking)
    pub fn get_cached_highlights(&self, _text: &str) -> Result<Vec<HighlightSpan>> {
        // Check cache only - never blocks on API
        // In full implementation, would check cache for:
        // - Discourse segments
        // - Contradictions
        // - Pragmatic elements
        Ok(Vec::new())
    }

    /// Background processing loop
    pub async fn run(mut self) -> Result<()> {
        // Spawn batch processing task
        let batcher = Arc::clone(&self.batcher);
        tokio::spawn(async move {
            loop {
                // Check if batch is ready
                if batcher.should_process_batch().await {
                    if let Ok(batch) = batcher.get_batch().await {
                        // Process batch here
                        // In full implementation, would:
                        // 1. Group by analysis type
                        // 2. Make API calls
                        // 3. Cache results
                        // 4. Notify waiting requests

                        // Clear dedup cache
                        let hashes: Vec<_> = batch.iter()
                            .map(|r| r.content_hash.clone())
                            .collect();
                        batcher.clear_dedup(&hashes).await;
                    }
                }

                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });

        // Main request processing loop
        while let Some(request) = self.request_rx.recv().await {
            match request {
                AnalysisRequest::Full => {
                    // Submit for batched analysis
                    // Would create BatchRequest and submit to batcher
                }
                AnalysisRequest::Range(_range) => {
                    // Submit range for analysis
                }
                AnalysisRequest::ClearCache => {
                    self.cache.clear_all();
                }
            }
        }

        Ok(())
    }
}
