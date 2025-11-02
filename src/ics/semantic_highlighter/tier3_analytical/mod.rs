//! Tier 3: Analytical Processing (Claude API, Optional, 2s+)
//!
//! Advanced semantic analysis using Claude API.
//! Batched, cached, and completely optional.

use crate::{
    ics::semantic_highlighter::{
        cache::{SemanticCache, CachedResult, ContentHash},
        settings::AnalyticalSettings,
        visualization::HighlightSpan,
        Result,
    },
    LlmService,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

pub mod discourse;
pub mod contradictions;
pub mod pragmatics;
pub mod batching;

#[cfg(feature = "python")]
pub mod dspy_integration;

pub use discourse::{DiscourseAnalyzer, DiscourseSegment, DiscourseRelation, CoherenceScore};
pub use contradictions::{ContradictionDetector, Contradiction, ContradictionType};
pub use pragmatics::{PragmaticsAnalyzer, PragmaticElement, PragmaticType, SpeechActType};
pub use batching::{RequestBatcher, BatchRequest, BatchConfig, AnalysisType};

#[cfg(feature = "python")]
pub use dspy_integration::DSpySemanticBridge;

use super::engine::AnalysisRequest;

/// Analysis result wrapper for caching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalysisResult {
    Discourse(Vec<DiscourseSegment>),
    Contradiction(Vec<Contradiction>),
    Pragmatics(Vec<PragmaticElement>),
}

impl AnalysisResult {
    /// Convert analysis results to highlight spans
    pub fn to_spans(
        &self,
        discourse: &DiscourseAnalyzer,
        contradiction: &ContradictionDetector,
        pragmatics: &PragmaticsAnalyzer,
    ) -> Vec<HighlightSpan> {
        match self {
            AnalysisResult::Discourse(segments) => discourse.segments_to_spans(segments),
            AnalysisResult::Contradiction(contradictions) => {
                contradiction.contradictions_to_spans(contradictions)
            }
            AnalysisResult::Pragmatics(elements) => pragmatics.elements_to_spans(elements),
        }
    }
}

/// Analytical processor using Claude API
///
/// Runs in background, batches requests, caches aggressively.
pub struct AnalyticalProcessor {
    _llm_service: Arc<LlmService>,
    _settings: AnalyticalSettings,
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
            max_batch_size: 5,
            max_wait_duration: std::time::Duration::from_millis(settings.debounce_ms),
            rate_limit_rpm: settings.max_api_calls_per_minute as usize,
            ..Default::default()
        };

        Self {
            discourse_analyzer: DiscourseAnalyzer::new(Arc::clone(&llm_service)),
            contradiction_detector: ContradictionDetector::new(Arc::clone(&llm_service))
                .with_threshold(0.7),
            pragmatics_analyzer: PragmaticsAnalyzer::new(Arc::clone(&llm_service))
                .with_threshold(0.6),
            batcher: Arc::new(RequestBatcher::new(batch_config)),
            _llm_service: llm_service,
            _settings: settings,
            cache,
            request_rx,
        }
    }

    /// Get cached highlights (non-blocking)
    pub fn get_cached_highlights(&self, text: &str) -> Result<Vec<HighlightSpan>> {
        let mut spans = Vec::new();

        // Try to get cached results by content hash
        if let Some(cached) = self.cache.analytical.get_by_content(text) {
            // Parse cached result
            if let Ok(result) = serde_json::from_value::<AnalysisResult>(cached.data) {
                spans.extend(result.to_spans(
                    &self.discourse_analyzer,
                    &self.contradiction_detector,
                    &self.pragmatics_analyzer,
                ));
            }
        }

        Ok(spans)
    }

    /// Background processing loop
    pub async fn run(mut self) -> Result<()> {
        // Clone resources for batch processing task
        let batcher = Arc::clone(&self.batcher);
        let cache = Arc::clone(&self.cache);
        let discourse = self.discourse_analyzer.clone();
        let contradiction = self.contradiction_detector.clone();
        let pragmatics = self.pragmatics_analyzer.clone();

        // Spawn batch processing task
        tokio::spawn(async move {
            Self::batch_processing_loop(
                batcher,
                discourse,
                contradiction,
                pragmatics,
                cache,
            )
            .await
        });

        // Main request processing loop
        while let Some(request) = self.request_rx.recv().await {
            match request {
                AnalysisRequest::Full { text } => {
                    info!("Full document analysis requested (len: {})", text.len());

                    // Create batch requests for all analysis types
                    let hash = ContentHash::from_content(&text);
                    let timestamp = Instant::now();

                    for analysis_type in [
                        AnalysisType::Discourse,
                        AnalysisType::Contradiction,
                        AnalysisType::Pragmatics,
                    ] {
                        let request = BatchRequest {
                            id: format!("{:?}-{:?}", hash, analysis_type),
                            text: text.clone(),
                            content_hash: format!("{:?}", hash),
                            analysis_type,
                            priority: 5,
                            submitted_at: timestamp,
                        };

                        if let Err(e) = self.batcher.submit(request).await {
                            warn!("Failed to submit batch request: {}", e);
                        }
                    }
                }
                AnalysisRequest::Range { text, range } => {
                    info!("Range analysis requested: {:?} (len: {})", range, text.len());

                    // Extract the text range for analysis
                    let range_text = if range.end <= text.len() {
                        &text[range.clone()]
                    } else {
                        warn!("Range {:?} exceeds text length {}", range, text.len());
                        continue;
                    };

                    let hash = ContentHash::from_content(range_text);
                    let timestamp = Instant::now();

                    for analysis_type in [
                        AnalysisType::Discourse,
                        AnalysisType::Contradiction,
                        AnalysisType::Pragmatics,
                    ] {
                        let request = BatchRequest {
                            id: format!("{:?}-{:?}-{:?}", hash, range, analysis_type),
                            text: range_text.to_string(),
                            content_hash: format!("{:?}", hash),
                            analysis_type,
                            priority: 7, // Higher priority for range requests
                            submitted_at: timestamp,
                        };

                        if let Err(e) = self.batcher.submit(request).await {
                            warn!("Failed to submit batch request: {}", e);
                        }
                    }
                }
                AnalysisRequest::ClearCache => {
                    info!("Clearing analytical cache");
                    self.cache.clear_all();
                }
            }
        }

        Ok(())
    }

    /// Batch processing loop
    async fn batch_processing_loop(
        batcher: Arc<RequestBatcher>,
        discourse: DiscourseAnalyzer,
        contradiction: ContradictionDetector,
        pragmatics: PragmaticsAnalyzer,
        cache: Arc<SemanticCache>,
    ) {
        info!("Starting batch processing loop");

        loop {
            // Check if batch is ready
            if !batcher.should_process_batch().await {
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }

            // Get batch
            let batch = match batcher.get_batch().await {
                Ok(batch) => batch,
                Err(e) => {
                    error!("Failed to get batch: {}", e);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            if batch.is_empty() {
                continue;
            }

            info!("Processing batch of {} requests", batch.len());

            // Group by analysis type
            let mut discourse_requests = Vec::new();
            let mut contradiction_requests = Vec::new();
            let mut pragmatics_requests = Vec::new();

            for req in batch {
                match req.analysis_type {
                    AnalysisType::Discourse => discourse_requests.push(req),
                    AnalysisType::Contradiction => contradiction_requests.push(req),
                    AnalysisType::Pragmatics => pragmatics_requests.push(req),
                    AnalysisType::Coherence => {
                        // Coherence analysis not yet fully implemented
                        warn!("Coherence analysis requested but not implemented");
                    }
                }
            }

            // Process each type in parallel
            let discourse_handle = {
                let cache = Arc::clone(&cache);
                let analyzer = discourse.clone();
                tokio::spawn(async move {
                    Self::process_discourse_batch(discourse_requests, analyzer, cache).await
                })
            };

            let contradiction_handle = {
                let cache = Arc::clone(&cache);
                let analyzer = contradiction.clone();
                tokio::spawn(async move {
                    Self::process_contradiction_batch(contradiction_requests, analyzer, cache).await
                })
            };

            let pragmatics_handle = {
                let cache = Arc::clone(&cache);
                let analyzer = pragmatics.clone();
                tokio::spawn(async move {
                    Self::process_pragmatics_batch(pragmatics_requests, analyzer, cache).await
                })
            };

            // Wait for all to complete
            let _ = tokio::join!(discourse_handle, contradiction_handle, pragmatics_handle);

            // Clear dedup cache for processed requests
            let all_hashes: Vec<String> = Vec::new(); // Will be populated from requests
            batcher.clear_dedup(&all_hashes).await;
        }
    }

    /// Process discourse analysis batch
    async fn process_discourse_batch(
        requests: Vec<BatchRequest>,
        analyzer: DiscourseAnalyzer,
        cache: Arc<SemanticCache>,
    ) {
        for request in requests {
            debug!("Processing discourse request: {}", request.id);

            // Retry with exponential backoff
            let mut retries = 0;
            let max_retries = 3;

            loop {
                match analyzer.analyze(&request.text).await {
                    Ok(segments) => {
                        debug!("Discourse analysis completed: {} segments", segments.len());

                        // Store in cache
                        let result = AnalysisResult::Discourse(segments);
                        if let Ok(value) = serde_json::to_value(&result) {
                            let cached = CachedResult::new(value);
                            cache.analytical.insert_with_content(&request.text, cached);
                        }
                        break;
                    }
                    Err(e) => {
                        retries += 1;
                        if retries >= max_retries {
                            error!("Discourse analysis failed for request {} after {} attempts: {}",
                                   request.id, max_retries, e);
                            break;
                        }

                        let backoff = Duration::from_millis(100 * 2_u64.pow(retries));
                        warn!("Discourse analysis failed (attempt {}/{}), retrying in {:?}: {}",
                              retries, max_retries, backoff, e);
                        tokio::time::sleep(backoff).await;
                    }
                }
            }
        }
    }

    /// Process contradiction detection batch
    async fn process_contradiction_batch(
        requests: Vec<BatchRequest>,
        analyzer: ContradictionDetector,
        cache: Arc<SemanticCache>,
    ) {
        for request in requests {
            debug!("Processing contradiction request: {}", request.id);

            // Retry with exponential backoff
            let mut retries = 0;
            let max_retries = 3;

            loop {
                match analyzer.detect(&request.text).await {
                    Ok(contradictions) => {
                        debug!("Contradiction detection completed: {} found", contradictions.len());

                        // Store in cache
                        let result = AnalysisResult::Contradiction(contradictions);
                        if let Ok(value) = serde_json::to_value(&result) {
                            let cached = CachedResult::new(value);
                            cache.analytical.insert_with_content(&request.text, cached);
                        }
                        break;
                    }
                    Err(e) => {
                        retries += 1;
                        if retries >= max_retries {
                            error!("Contradiction detection failed for request {} after {} attempts: {}",
                                   request.id, max_retries, e);
                            break;
                        }

                        let backoff = Duration::from_millis(100 * 2_u64.pow(retries));
                        warn!("Contradiction detection failed (attempt {}/{}), retrying in {:?}: {}",
                              retries, max_retries, backoff, e);
                        tokio::time::sleep(backoff).await;
                    }
                }
            }
        }
    }

    /// Process pragmatics analysis batch
    async fn process_pragmatics_batch(
        requests: Vec<BatchRequest>,
        analyzer: PragmaticsAnalyzer,
        cache: Arc<SemanticCache>,
    ) {
        for request in requests {
            debug!("Processing pragmatics request: {}", request.id);

            // Retry with exponential backoff
            let mut retries = 0;
            let max_retries = 3;

            loop {
                match analyzer.analyze(&request.text).await {
                    Ok(elements) => {
                        debug!("Pragmatics analysis completed: {} elements", elements.len());

                        // Store in cache
                        let result = AnalysisResult::Pragmatics(elements);
                        if let Ok(value) = serde_json::to_value(&result) {
                            let cached = CachedResult::new(value);
                            cache.analytical.insert_with_content(&request.text, cached);
                        }
                        break;
                    }
                    Err(e) => {
                        retries += 1;
                        if retries >= max_retries {
                            error!("Pragmatics analysis failed for request {} after {} attempts: {}",
                                   request.id, max_retries, e);
                            break;
                        }

                        let backoff = Duration::from_millis(100 * 2_u64.pow(retries));
                        warn!("Pragmatics analysis failed (attempt {}/{}), retrying in {:?}: {}",
                              retries, max_retries, backoff, e);
                        tokio::time::sleep(backoff).await;
                    }
                }
            }
        }
    }
}
