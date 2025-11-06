//! Request batching and coordination for Tier 3 analysis
//!
//! Handles:
//! - Request aggregation and batching
//! - Rate limiting to respect API limits
//! - Priority-based scheduling
//! - Deduplication of identical requests
//! - Background processing coordination

use crate::ics::semantic_highlighter::{Result, SemanticError};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

/// Analysis request type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AnalysisType {
    Discourse,
    Contradiction,
    Pragmatics,
    Coherence,
}

/// Batched analysis request
#[derive(Debug, Clone)]
pub struct BatchRequest {
    /// Unique request ID
    pub id: String,
    /// Text to analyze
    pub text: String,
    /// Content hash for deduplication
    pub content_hash: String,
    /// Type of analysis requested
    pub analysis_type: AnalysisType,
    /// Priority (higher = more urgent)
    pub priority: u8,
    /// When request was submitted
    pub submitted_at: Instant,
}

/// Batch configuration
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Maximum batch size
    pub max_batch_size: usize,
    /// Maximum wait time before processing batch
    pub max_wait_duration: Duration,
    /// Maximum requests per minute (rate limit)
    pub rate_limit_rpm: usize,
    /// Minimum interval between batches
    pub min_batch_interval: Duration,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 5,
            max_wait_duration: Duration::from_secs(2),
            rate_limit_rpm: 50, // Conservative default
            min_batch_interval: Duration::from_millis(100),
        }
    }
}

/// Request batcher for Tier 3 analysis
pub struct RequestBatcher {
    config: BatchConfig,
    /// Pending requests queue
    pending: Arc<Mutex<VecDeque<BatchRequest>>>,
    /// Deduplication cache (content hash -> request IDs)
    dedup_cache: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Rate limiting state
    rate_limiter: Arc<Mutex<RateLimiter>>,
    /// Last batch processing time
    last_batch: Arc<Mutex<Option<Instant>>>,
}

/// Simple rate limiter using token bucket
struct RateLimiter {
    tokens: f64,
    last_refill: Instant,
    tokens_per_second: f64,
    max_tokens: f64,
}

impl RateLimiter {
    fn new(rpm: usize) -> Self {
        let tokens_per_second = rpm as f64 / 60.0;
        Self {
            tokens: tokens_per_second,
            last_refill: Instant::now(),
            tokens_per_second,
            max_tokens: tokens_per_second * 2.0, // Allow some burst
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.tokens_per_second).min(self.max_tokens);
        self.last_refill = now;
    }

    fn try_consume(&mut self, count: usize) -> bool {
        self.refill();
        if self.tokens >= count as f64 {
            self.tokens -= count as f64;
            true
        } else {
            false
        }
    }

    fn wait_time(&mut self, count: usize) -> Duration {
        self.refill();
        if self.tokens >= count as f64 {
            Duration::ZERO
        } else {
            let needed = count as f64 - self.tokens;
            Duration::from_secs_f64(needed / self.tokens_per_second)
        }
    }
}

impl RequestBatcher {
    pub fn new(config: BatchConfig) -> Self {
        Self {
            rate_limiter: Arc::new(Mutex::new(RateLimiter::new(config.rate_limit_rpm))),
            config,
            pending: Arc::new(Mutex::new(VecDeque::new())),
            dedup_cache: Arc::new(RwLock::new(HashMap::new())),
            last_batch: Arc::new(Mutex::new(None)),
        }
    }

    /// Submit a request for batching
    pub async fn submit(&self, request: BatchRequest) -> Result<()> {
        // Check deduplication
        let mut cache = self.dedup_cache.write().await;
        if let Some(_existing) = cache.get(&request.content_hash) {
            // Already have this request, just add ID to waiting list
            cache
                .get_mut(&request.content_hash)
                .unwrap()
                .push(request.id.clone());
            return Ok(());
        }

        // New request - add to cache and queue
        cache.insert(request.content_hash.clone(), vec![request.id.clone()]);
        drop(cache);

        let mut pending = self.pending.lock().await;
        pending.push_back(request);

        Ok(())
    }

    /// Check if batch is ready to process
    pub async fn should_process_batch(&self) -> bool {
        let pending = self.pending.lock().await;

        if pending.is_empty() {
            return false;
        }

        // Check if batch is full
        if pending.len() >= self.config.max_batch_size {
            return true;
        }

        // Check if oldest request has waited long enough
        if let Some(oldest) = pending.front() {
            if oldest.submitted_at.elapsed() >= self.config.max_wait_duration {
                return true;
            }
        }

        // Check minimum batch interval
        let last = self.last_batch.lock().await;
        if let Some(last_time) = *last {
            if last_time.elapsed() >= self.config.min_batch_interval {
                // Has been long enough since last batch
                return !pending.is_empty();
            }
        } else {
            // Never processed a batch, go ahead if we have requests
            return !pending.is_empty();
        }

        false
    }

    /// Get next batch to process
    pub async fn get_batch(&self) -> Result<Vec<BatchRequest>> {
        // Check rate limit
        {
            let mut limiter = self.rate_limiter.lock().await;
            if !limiter.try_consume(1) {
                let wait = limiter.wait_time(1);
                drop(limiter);

                if wait > Duration::from_secs(5) {
                    return Err(SemanticError::AnalysisFailed(
                        "Rate limit exceeded, wait time too long".to_string(),
                    ));
                }

                tokio::time::sleep(wait).await;
            }
        }

        // Get pending requests
        let mut pending = self.pending.lock().await;
        let batch_size = pending.len().min(self.config.max_batch_size);

        let mut batch = Vec::new();
        for _ in 0..batch_size {
            if let Some(req) = pending.pop_front() {
                batch.push(req);
            }
        }

        // Sort by priority (higher priority first)
        batch.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Update last batch time
        let mut last = self.last_batch.lock().await;
        *last = Some(Instant::now());

        Ok(batch)
    }

    /// Clear deduplication cache for processed requests
    pub async fn clear_dedup(&self, content_hashes: &[String]) {
        let mut cache = self.dedup_cache.write().await;
        for hash in content_hashes {
            cache.remove(hash);
        }
    }

    /// Get current queue size
    pub async fn queue_size(&self) -> usize {
        self.pending.lock().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_config_default() {
        let config = BatchConfig::default();
        assert_eq!(config.max_batch_size, 5);
        assert_eq!(config.rate_limit_rpm, 50);
    }

    #[test]
    fn test_rate_limiter_creation() {
        let limiter = RateLimiter::new(60); // 60 RPM = 1 per second
        assert!(limiter.tokens > 0.0);
        assert_eq!(limiter.tokens_per_second, 1.0);
    }

    #[test]
    fn test_rate_limiter_consume() {
        let mut limiter = RateLimiter::new(120); // 2 tokens/sec, max 4 tokens
        limiter.tokens = 3.5;
        limiter.last_refill = std::time::Instant::now(); // Prevent refill during test

        // Can consume 2 tokens
        assert!(limiter.try_consume(2));
        // Check approximately equal (refill may add tiny amount due to time elapsed)
        assert!((limiter.tokens - 1.5).abs() < 0.01);

        // Cannot consume 3 tokens (only ~1.5 available)
        assert!(!limiter.try_consume(3));
        assert!((limiter.tokens - 1.5).abs() < 0.01); // Still approximately 1.5
    }

    #[tokio::test]
    async fn test_request_submission() {
        let batcher = RequestBatcher::new(BatchConfig::default());

        let request = BatchRequest {
            id: "test-1".to_string(),
            text: "test text".to_string(),
            content_hash: "hash1".to_string(),
            analysis_type: AnalysisType::Discourse,
            priority: 5,
            submitted_at: Instant::now(),
        };

        assert!(batcher.submit(request).await.is_ok());
        assert_eq!(batcher.queue_size().await, 1);
    }

    #[tokio::test]
    async fn test_deduplication() {
        let batcher = RequestBatcher::new(BatchConfig::default());

        let request1 = BatchRequest {
            id: "test-1".to_string(),
            text: "test text".to_string(),
            content_hash: "hash1".to_string(),
            analysis_type: AnalysisType::Discourse,
            priority: 5,
            submitted_at: Instant::now(),
        };

        let request2 = BatchRequest {
            id: "test-2".to_string(),
            text: "test text".to_string(),
            content_hash: "hash1".to_string(), // Same hash
            analysis_type: AnalysisType::Discourse,
            priority: 5,
            submitted_at: Instant::now(),
        };

        assert!(batcher.submit(request1).await.is_ok());
        assert!(batcher.submit(request2).await.is_ok());

        // Should only have 1 in queue due to deduplication
        assert_eq!(batcher.queue_size().await, 1);
    }

    #[tokio::test]
    async fn test_batch_ready_full() {
        let config = BatchConfig {
            max_batch_size: 3,
            ..Default::default()
        };
        let batcher = RequestBatcher::new(config);

        // Submit 3 requests to fill batch
        for i in 0..3 {
            let request = BatchRequest {
                id: format!("test-{}", i),
                text: format!("text {}", i),
                content_hash: format!("hash{}", i),
                analysis_type: AnalysisType::Discourse,
                priority: 5,
                submitted_at: Instant::now(),
            };
            batcher.submit(request).await.unwrap();
        }

        assert!(batcher.should_process_batch().await);
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let batcher = RequestBatcher::new(BatchConfig::default());

        let low_priority = BatchRequest {
            id: "low".to_string(),
            text: "low".to_string(),
            content_hash: "hash-low".to_string(),
            analysis_type: AnalysisType::Discourse,
            priority: 1,
            submitted_at: Instant::now(),
        };

        let high_priority = BatchRequest {
            id: "high".to_string(),
            text: "high".to_string(),
            content_hash: "hash-high".to_string(),
            analysis_type: AnalysisType::Discourse,
            priority: 10,
            submitted_at: Instant::now(),
        };

        batcher.submit(low_priority).await.unwrap();
        batcher.submit(high_priority).await.unwrap();

        let batch = batcher.get_batch().await.unwrap();
        assert_eq!(batch.len(), 2);
        assert_eq!(batch[0].id, "high"); // High priority first
        assert_eq!(batch[1].id, "low");
    }
}
