//! Caching system for semantic analysis results

use lru::LruCache;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::ops::Range;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Content-based hash for caching
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContentHash(u64);

impl ContentHash {
    /// Create hash from content
    pub fn from_content(content: &str) -> Self {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        Self(hasher.finish())
    }
}

/// Cached analysis result
#[derive(Debug, Clone)]
pub struct CachedResult<T> {
    /// The cached data
    pub data: T,

    /// When this was cached
    pub cached_at: Instant,

    /// Confidence score
    pub confidence: f32,
}

impl<T> CachedResult<T> {
    pub fn new(data: T) -> Self {
        Self {
            data,
            cached_at: Instant::now(),
            confidence: 1.0,
        }
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence;
        self
    }

    /// Check if this result is still valid given TTL
    pub fn is_valid(&self, ttl: Duration) -> bool {
        self.cached_at.elapsed() < ttl
    }
}

/// LRU cache for relational analysis results
pub struct RelationalCache<T> {
    cache: RwLock<LruCache<Range<usize>, CachedResult<T>>>,
    ttl: Duration,
}

impl<T: Clone> RelationalCache<T> {
    pub fn new(capacity: usize, ttl_seconds: u64) -> Self {
        Self {
            cache: RwLock::new(LruCache::new(NonZeroUsize::new(capacity).unwrap())),
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    /// Get cached result
    pub fn get(&self, range: &Range<usize>) -> Option<CachedResult<T>> {
        let mut cache = self.cache.write().ok()?;
        cache.get(range).and_then(|result| {
            if result.is_valid(self.ttl) {
                Some(result.clone())
            } else {
                None
            }
        })
    }

    /// Insert result into cache
    pub fn insert(&self, range: Range<usize>, result: CachedResult<T>) {
        if let Ok(mut cache) = self.cache.write() {
            cache.put(range, result);
        }
    }

    /// Clear the cache
    pub fn clear(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        if let Ok(cache) = self.cache.read() {
            CacheStats {
                size: cache.len(),
                capacity: cache.cap().get(),
            }
        } else {
            CacheStats {
                size: 0,
                capacity: 0,
            }
        }
    }
}

/// Content-hash based cache for analytical results
pub struct AnalyticalCache<T> {
    cache: Arc<RwLock<HashMap<ContentHash, CachedResult<T>>>>,
    ttl: Duration,
}

impl<T: Clone> AnalyticalCache<T> {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    /// Get cached result by content hash
    pub fn get(&self, hash: ContentHash) -> Option<CachedResult<T>> {
        let cache = self.cache.read().ok()?;
        cache.get(&hash).and_then(|result| {
            if result.is_valid(self.ttl) {
                Some(result.clone())
            } else {
                None
            }
        })
    }

    /// Get cached result by content (computes hash)
    pub fn get_by_content(&self, content: &str) -> Option<CachedResult<T>> {
        let hash = ContentHash::from_content(content);
        self.get(hash)
    }

    /// Insert result into cache
    pub fn insert(&self, hash: ContentHash, result: CachedResult<T>) {
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(hash, result);
        }
    }

    /// Insert with content (computes hash)
    pub fn insert_with_content(&self, content: &str, result: CachedResult<T>) {
        let hash = ContentHash::from_content(content);
        self.insert(hash, result);
    }

    /// Clear expired entries
    pub fn clear_expired(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.retain(|_, result| result.is_valid(self.ttl));
        }
    }

    /// Clear all entries
    pub fn clear(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        if let Ok(cache) = self.cache.read() {
            let valid_count = cache
                .values()
                .filter(|r| r.is_valid(self.ttl))
                .count();
            CacheStats {
                size: valid_count,
                capacity: cache.len(),
            }
        } else {
            CacheStats {
                size: 0,
                capacity: 0,
            }
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Copy)]
pub struct CacheStats {
    /// Current number of entries
    pub size: usize,

    /// Maximum capacity
    pub capacity: usize,
}

impl CacheStats {
    /// Hit rate estimate (size / capacity)
    pub fn utilization(&self) -> f32 {
        if self.capacity == 0 {
            0.0
        } else {
            self.size as f32 / self.capacity as f32
        }
    }
}

/// Combined cache for all tiers
pub struct SemanticCache {
    /// Cache for Tier 2 relational analysis
    pub relational: RelationalCache<serde_json::Value>,

    /// Cache for Tier 3 analytical results
    pub analytical: AnalyticalCache<serde_json::Value>,
}

impl SemanticCache {
    pub fn new(relational_capacity: usize, ttl_seconds: u64) -> Self {
        Self {
            relational: RelationalCache::new(relational_capacity, ttl_seconds),
            analytical: AnalyticalCache::new(ttl_seconds),
        }
    }

    /// Clear all caches
    pub fn clear_all(&self) {
        self.relational.clear();
        self.analytical.clear();
    }

    /// Get combined statistics
    pub fn stats(&self) -> (CacheStats, CacheStats) {
        (self.relational.stats(), self.analytical.stats())
    }
}

impl Default for SemanticCache {
    fn default() -> Self {
        Self::new(100, 3600)  // 100 entries, 1 hour TTL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_hash() {
        let hash1 = ContentHash::from_content("test");
        let hash2 = ContentHash::from_content("test");
        let hash3 = ContentHash::from_content("different");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_relational_cache() {
        let cache = RelationalCache::new(10, 60);
        let range = 0..10;
        let result = CachedResult::new("test_data".to_string());

        cache.insert(range.clone(), result.clone());
        let retrieved = cache.get(&range);

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().data, "test_data");
    }

    #[test]
    fn test_analytical_cache() {
        let cache = AnalyticalCache::new(60);
        let content = "test content";
        let result = CachedResult::new("analysis result".to_string());

        cache.insert_with_content(content, result.clone());
        let retrieved = cache.get_by_content(content);

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().data, "analysis result");
    }

    #[test]
    fn test_cache_expiration() {
        let cache = AnalyticalCache::new(0);  // 0 second TTL
        let result = CachedResult::new("data".to_string());

        cache.insert_with_content("test", result);

        // Wait a moment
        std::thread::sleep(Duration::from_millis(10));

        // Should be expired
        assert!(cache.get_by_content("test").is_none());
    }

    #[test]
    fn test_cache_stats() {
        let cache = RelationalCache::new(10, 60);
        cache.insert(0..5, CachedResult::new("data1".to_string()));
        cache.insert(5..10, CachedResult::new("data2".to_string()));

        let stats = cache.stats();
        assert_eq!(stats.size, 2);
        assert_eq!(stats.capacity, 10);
        assert_eq!(stats.utilization(), 0.2);
    }
}
