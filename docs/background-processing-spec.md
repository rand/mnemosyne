# Background Processing Specification

## Overview

Complete the Tier 3 background processing loop to execute batched LLM analysis requests, store results in cache, and coordinate between the three analytical analyzers (Discourse, Contradiction, Pragmatics).

## Current State

**What Exists:**
- Full batching infrastructure: `RequestBatcher` with priority, rate limiting, deduplication
- `AnalyticalProcessor` structure with request channel
- Background loop skeleton at `tier3_analytical/mod.rs:87-131`
- Batch processing task spawned with empty handler (lines 88-112)
- Main request loop with stubbed handlers (lines 115-130)

**What's Missing:**
- Actual analysis execution inside batch processing task
- Integration with three analyzers (Discourse, Contradiction, Pragmatics)
- Result storage in cache after analysis
- Error handling and retry logic
- Progress tracking and cancellation
- Observability (logging, metrics)

---

## Requirements

### Functional Requirements

**FR-1**: Background loop must process analysis requests asynchronously
- Input: `AnalysisRequest` from channel (Full, Range, ClearCache)
- Behavior: Convert to `BatchRequest`, submit to batcher
- Output: Results stored in cache, non-blocking

**FR-2**: Batch processor must execute LLM calls for batched requests
- Input: Batch of `BatchRequest` from batcher
- Behavior: Group by analysis type, call appropriate analyzer
- Output: Analysis results (segments, contradictions, elements)

**FR-3**: Results must be cached by content hash
- Each analysis result stored with content hash key
- TTL-based expiration (default 1 hour)
- Deduplication: multiple requests for same content share result

**FR-4**: Error handling must be robust
- Retry transient failures (timeouts, network errors) with exponential backoff
- Log persistent failures without crashing loop
- Graceful degradation: continue processing other requests

### Non-Functional Requirements

**NFR-1**: Performance
- Non-blocking: Main loop never waits for analysis completion
- Concurrent processing: Multiple analysis types in parallel
- Resource limits: Max concurrent LLM calls (default 3)

**NFR-2**: Reliability
- Crash recovery: Loop restarts on panic
- Cancellation support: Can stop gracefully
- No request loss: Persistent failures logged, requests marked failed

**NFR-3**: Observability
- Log all batch processing events
- Track success/failure rates
- Monitor queue depth and latency
- Expose metrics via cache stats

---

## Design

### Type Definitions

```rust
/// Analysis result wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalysisResult {
    Discourse(Vec<DiscourseSegment>),
    Contradiction(Vec<Contradiction>),
    Pragmatics(Vec<PragmaticElement>),
}

impl AnalysisResult {
    /// Convert to highlight spans
    pub fn to_spans(&self, analyzer: &AnalyticalProcessor) -> Vec<HighlightSpan> {
        match self {
            AnalysisResult::Discourse(segments) => {
                analyzer._discourse_analyzer.segments_to_spans(segments)
            }
            AnalysisResult::Contradiction(contradictions) => {
                analyzer._contradiction_detector.contradictions_to_spans(contradictions)
            }
            AnalysisResult::Pragmatics(elements) => {
                analyzer._pragmatics_analyzer.elements_to_spans(elements)
            }
        }
    }
}

/// Processing error types
#[derive(Debug, thiserror::Error)]
pub enum ProcessingError {
    #[error("Analysis failed: {0}")]
    AnalysisFailed(String),

    #[error("Timeout after {0}s")]
    Timeout(u64),

    #[error("Rate limited")]
    RateLimited,

    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

/// Request processing status
#[derive(Debug, Clone)]
pub struct ProcessingStatus {
    pub request_id: String,
    pub status: StatusType,
    pub started_at: Instant,
    pub completed_at: Option<Instant>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StatusType {
    Queued,
    Processing,
    Completed,
    Failed,
}
```

### Implementation Plan

#### 1. Complete Main Request Loop

**File**: `src/ics/semantic_highlighter/tier3_analytical/mod.rs`

```rust
impl AnalyticalProcessor {
    /// Background processing loop
    pub async fn run(mut self) -> Result<()> {
        // Spawn batch processing task
        let batcher = Arc::clone(&self.batcher);
        let llm_service = Arc::clone(&self._llm_service);
        let cache = Arc::clone(&self.cache);
        let settings = self._settings.clone();

        // Create analyzers for batch task (they need to be Send)
        let discourse = DiscourseAnalyzer::new(Arc::clone(&llm_service));
        let contradiction = ContradictionDetector::new(Arc::clone(&llm_service))
            .with_threshold(0.7);
        let pragmatics = PragmaticsAnalyzer::new(Arc::clone(&llm_service))
            .with_threshold(0.6);

        tokio::spawn(async move {
            Self::batch_processing_loop(
                batcher,
                discourse,
                contradiction,
                pragmatics,
                cache,
                settings,
            )
            .await
        });

        // Main request processing loop
        while let Some(request) = self.request_rx.recv().await {
            match request {
                AnalysisRequest::Full => {
                    // Get full document text
                    // Would need to pass text along with request
                    // For now, log that full analysis was requested
                    log::info!("Full document analysis requested");
                }
                AnalysisRequest::Range(range) => {
                    // Range-specific analysis
                    // Would need document text + range
                    log::info!("Range analysis requested: {:?}", range);
                }
                AnalysisRequest::ClearCache => {
                    log::info!("Clearing analytical cache");
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
        settings: AnalyticalSettings,
    ) {
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
                    log::error!("Failed to get batch: {}", e);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            if batch.is_empty() {
                continue;
            }

            log::info!("Processing batch of {} requests", batch.len());

            // Group by analysis type
            let mut discourse_requests = Vec::new();
            let mut contradiction_requests = Vec::new();
            let mut pragmatics_requests = Vec::new();

            for req in batch {
                match req.analysis_type {
                    AnalysisType::Discourse => discourse_requests.push(req),
                    AnalysisType::Contradiction => contradiction_requests.push(req),
                    AnalysisType::Pragmatics => pragmatics_requests.push(req),
                    _ => {}
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
            let all_hashes: Vec<_> = discourse_requests
                .iter()
                .chain(contradiction_requests.iter())
                .chain(pragmatics_requests.iter())
                .map(|r| r.content_hash.clone())
                .collect();

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
            match Self::analyze_with_retry(&request.text, |text| async {
                analyzer.analyze(text).await
            })
            .await
            {
                Ok(segments) => {
                    // Store in cache
                    let result = AnalysisResult::Discourse(segments);
                    let cached = CachedResult::new(serde_json::to_value(&result).unwrap());
                    cache
                        .analytical
                        .insert_with_content(&request.text, cached);

                    log::debug!("Discourse analysis completed for request {}", request.id);
                }
                Err(e) => {
                    log::error!("Discourse analysis failed for request {}: {}", request.id, e);
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
            match Self::analyze_with_retry(&request.text, |text| async {
                analyzer.detect(text).await
            })
            .await
            {
                Ok(contradictions) => {
                    let result = AnalysisResult::Contradiction(contradictions);
                    let cached = CachedResult::new(serde_json::to_value(&result).unwrap());
                    cache
                        .analytical
                        .insert_with_content(&request.text, cached);

                    log::debug!(
                        "Contradiction detection completed for request {}",
                        request.id
                    );
                }
                Err(e) => {
                    log::error!(
                        "Contradiction detection failed for request {}: {}",
                        request.id,
                        e
                    );
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
            match Self::analyze_with_retry(&request.text, |text| async {
                analyzer.analyze(text).await
            })
            .await
            {
                Ok(elements) => {
                    let result = AnalysisResult::Pragmatics(elements);
                    let cached = CachedResult::new(serde_json::to_value(&result).unwrap());
                    cache
                        .analytical
                        .insert_with_content(&request.text, cached);

                    log::debug!(
                        "Pragmatics analysis completed for request {}",
                        request.id
                    );
                }
                Err(e) => {
                    log::error!("Pragmatics analysis failed for request {}: {}", request.id, e);
                }
            }
        }
    }

    /// Retry wrapper with exponential backoff
    async fn analyze_with_retry<F, Fut, T>(text: &str, analyze_fn: F) -> Result<T>
    where
        F: Fn(&str) -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut retries = 0;
        let max_retries = 3;

        loop {
            match analyze_fn(text).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    retries += 1;
                    if retries >= max_retries {
                        return Err(e);
                    }

                    let backoff = Duration::from_millis(100 * 2_u64.pow(retries));
                    log::warn!("Analysis failed, retrying in {:?}: {}", backoff, e);
                    tokio::time::sleep(backoff).await;
                }
            }
        }
    }
}
```

#### 2. Update get_cached_highlights() to Return Real Results

```rust
impl AnalyticalProcessor {
    /// Get cached highlights (non-blocking)
    pub fn get_cached_highlights(&self, text: &str) -> Result<Vec<HighlightSpan>> {
        let mut spans = Vec::new();

        // Try to get cached results
        if let Some(cached) = self.cache.analytical.get_by_content(text) {
            // Parse cached result
            if let Ok(result) = serde_json::from_value::<AnalysisResult>(cached.data) {
                spans.extend(result.to_spans(self));
            }
        }

        Ok(spans)
    }
}
```

#### 3. Add AnalysisRequest Text Support

**File**: `src/ics/semantic_highlighter/engine.rs`

Update `AnalysisRequest` to include text:

```rust
/// Request for background analysis
#[derive(Debug, Clone)]
pub enum AnalysisRequest {
    /// Analyze specific range
    Range { text: String, range: std::ops::Range<usize> },

    /// Analyze full document
    Full { text: String },

    /// Clear all caches
    ClearCache,
}
```

Update `SemanticHighlightEngine`:

```rust
impl SemanticHighlightEngine {
    /// Request background analytical processing (Tier 3)
    pub async fn request_analysis(&self, text: String, request_type: AnalysisRequestType) -> Result<()> {
        let request = match request_type {
            AnalysisRequestType::Full => AnalysisRequest::Full { text },
            AnalysisRequestType::Range(range) => AnalysisRequest::Range { text, range },
            AnalysisRequestType::ClearCache => AnalysisRequest::ClearCache,
        };

        if let Some(ref tx) = self.analysis_tx {
            tx.send(request)
                .await
                .map_err(|e| SemanticError::AnalysisFailed(e.to_string()))?;
            Ok(())
        } else {
            Err(SemanticError::AnalysisFailed("Tier 3 not available".to_string()))
        }
    }
}

pub enum AnalysisRequestType {
    Full,
    Range(std::ops::Range<usize>),
    ClearCache,
}
```

---

## Testing Strategy

### Unit Tests

**Test 1: Batch Processing with Mock Analyzer**
- Given: Batch of 3 requests, mock analyzer
- When: Processing batch
- Then: All 3 analyzed, results cached

**Test 2: Retry Logic**
- Given: Analyzer that fails twice then succeeds
- When: Calling `analyze_with_retry()`
- Then: Retries 2 times, eventually succeeds

**Test 3: Error Handling**
- Given: Analyzer that always fails
- When: Processing batch
- Then: Errors logged, other requests continue

**Test 4: Cache Storage**
- Given: Successful analysis
- When: Result stored in cache
- Then: Subsequent calls return cached result

### Integration Tests

**Test 5: End-to-End Background Processing**
- Given: Engine with Tier 3 enabled
- When: Requesting full document analysis
- Then: Request batched, analyzed, cached, retrievable

**Test 6: Concurrent Processing**
- Given: Multiple requests of different types
- When: Processing batch
- Then: All types processed in parallel

**Test 7: Rate Limiting**
- Given: Many requests exceeding rate limit
- When: Processing batches
- Then: Rate limit respected, no API errors

---

## Acceptance Criteria

- [ ] Main request loop converts requests to BatchRequests
- [ ] Batch processing loop executes LLM calls
- [ ] Results stored in cache by content hash
- [ ] Three analyzer types processed in parallel
- [ ] Retry logic with exponential backoff (3 retries)
- [ ] Error handling: logs failures, continues processing
- [ ] `get_cached_highlights()` returns real cached results
- [ ] All unit tests passing
- [ ] Integration tests with mock/real LLM passing
- [ ] Logging throughout (info, warn, error levels)
- [ ] No blocking on main thread

---

## Estimated Effort

- Main loop completion: 0.5 days
- Batch processing with analyzers: 1 day
- Retry and error handling: 0.5 days
- Cache integration: 0.5 days
- Testing: 0.5 days
- Integration & debugging: 0.5 days

**Total: 3.5 days** (can parallelize: loop + batch processor, cache + retry)

---

## Dependencies

- Tier 3 LLM integration must be complete (Phase 2)
- Batching infrastructure (already complete)
- Cache infrastructure (already complete)
- `serde_json` for serializing results
- `log` crate for observability

---

## Risks & Mitigation

**Risk 1**: Batch processing blocks on slow LLM calls
- Mitigation: Spawn separate tasks for each analysis type, use tokio::spawn

**Risk 2**: Memory growth from cached results
- Mitigation: Use TTL-based expiration, periodic cleanup task

**Risk 3**: Loop crashes on panic
- Mitigation: Wrap in catch_unwind, restart loop, log crashes

**Risk 4**: Request loss on channel overflow
- Mitigation: Channel capacity monitoring, back-pressure if needed

---

## References

- Current implementation: `tier3_analytical/mod.rs:87-131`
- Batching system: `tier3_analytical/batching.rs`
- Cache: `cache.rs`
- Engine integration: `engine.rs`
