# Incremental Analysis Specification

## Overview

Implement debounced incremental analysis for Tier 2 relational analyzers to avoid re-analyzing unchanged text. This enables efficient real-time updates as users type while maintaining good performance.

## Current State

**What Exists:**
- Cache infrastructure: `RelationalCache` with LRU eviction
- Cache entry types: `CachedResult<T>` with TTL validation
- Stubbed method: `RelationalAnalyzer::schedule_analysis()` at `tier2_relational/mod.rs:85`
- Public API: `SemanticHighlightEngine::schedule_analysis()` forwarding to relational analyzer

**What's Missing:**
- Debouncing mechanism to batch rapid text changes
- Dirty region tracking to identify changed ranges
- Cache invalidation for modified regions
- Integration with text change events
- Background task to execute scheduled analysis

---

## Requirements

### Functional Requirements

**FR-1**: Debounced scheduling must delay analysis until typing pauses
- Input: Text changes with character ranges
- Behavior: Wait for configurable delay (default 250ms) of inactivity
- Output: Single analysis run after delay, not per-keystroke

**FR-2**: Dirty region tracking must identify changed text ranges
- Track which regions have been modified since last analysis
- Merge overlapping dirty regions
- Clear dirty regions after successful analysis

**FR-3**: Cache invalidation must clear stale entries
- Invalidate cache entries overlapping dirty regions
- Preserve cache entries for unchanged regions
- Support partial invalidation (not just full clear)

**FR-4**: Incremental re-analysis must only process changed regions
- Analyze only dirty regions, not entire document
- Use cached results for unchanged regions
- Merge new results with cached results

### Non-Functional Requirements

**NFR-1**: Performance
- Debounce delay: 250ms default (configurable 100ms-1000ms)
- Analysis latency: <200ms for typical text changes
- Memory overhead: <1MB for dirty region tracking

**NFR-2**: Correctness
- No race conditions between typing and analysis
- Cache invalidation must be conservative (invalidate on overlap)
- Results must be consistent with full re-analysis

**NFR-3**: Observability
- Log debounce triggers
- Track cache hit/miss on dirty regions
- Monitor analysis frequency

---

## Design

### Type Definitions

```rust
use std::ops::Range;
use std::time::{Duration, Instant};
use tokio::time::Sleep;
use std::pin::Pin;

/// Dirty region tracker
pub struct DirtyRegions {
    /// Dirty ranges that need re-analysis
    regions: Vec<Range<usize>>,

    /// Last modification timestamp
    last_modified: Instant,
}

impl DirtyRegions {
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
            last_modified: Instant::now(),
        }
    }

    /// Mark a region as dirty
    pub fn mark_dirty(&mut self, range: Range<usize>) {
        self.regions.push(range);
        self.last_modified = Instant::now();
        self.merge_overlapping();
    }

    /// Get all dirty regions
    pub fn get_dirty(&self) -> &[Range<usize>] {
        &self.regions
    }

    /// Clear all dirty regions
    pub fn clear(&mut self) {
        self.regions.clear();
    }

    /// Check if any region overlaps with given range
    pub fn overlaps(&self, range: &Range<usize>) -> bool {
        self.regions.iter().any(|r| ranges_overlap(r, range))
    }

    /// Merge overlapping/adjacent regions
    fn merge_overlapping(&mut self) {
        if self.regions.len() <= 1 {
            return;
        }

        // Sort by start position
        self.regions.sort_by_key(|r| r.start);

        let mut merged = Vec::new();
        let mut current = self.regions[0].clone();

        for region in &self.regions[1..] {
            if region.start <= current.end {
                // Overlapping or adjacent - merge
                current.end = current.end.max(region.end);
            } else {
                // Non-overlapping - save current and start new
                merged.push(current);
                current = region.clone();
            }
        }
        merged.push(current);

        self.regions = merged;
    }
}

fn ranges_overlap(a: &Range<usize>, b: &Range<usize>) -> bool {
    a.start < b.end && b.start < a.end
}

/// Debouncer for scheduling analysis
pub struct Debouncer {
    /// Debounce delay
    delay: Duration,

    /// Pending analysis timer
    timer: Option<Pin<Box<Sleep>>>,
}

impl Debouncer {
    pub fn new(delay_ms: u64) -> Self {
        Self {
            delay: Duration::from_millis(delay_ms),
            timer: None,
        }
    }

    /// Schedule analysis with debounce
    pub fn schedule(&mut self) -> Pin<Box<Sleep>> {
        let sleep = Box::pin(tokio::time::sleep(self.delay));
        self.timer = Some(sleep);
        Box::pin(tokio::time::sleep(self.delay))
    }

    /// Cancel pending analysis
    pub fn cancel(&mut self) {
        self.timer = None;
    }
}
```

### Implementation Plan

#### 1. Extend RelationalAnalyzer with Dirty Tracking

**File**: `src/ics/semantic_highlighter/tier2_relational/mod.rs`

```rust
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub struct RelationalAnalyzer {
    settings: RelationalSettings,
    cache: Arc<SemanticCache>,

    // ... existing fields ...

    /// Dirty region tracker
    dirty_regions: Arc<RwLock<DirtyRegions>>,

    /// Debouncer for scheduling
    debouncer: Arc<RwLock<Debouncer>>,

    /// Channel for triggering analysis
    analysis_tx: mpsc::Sender<AnalysisRequest>,

    /// Background task handle
    analysis_task: Option<JoinHandle<()>>,
}

impl RelationalAnalyzer {
    pub fn new(settings: RelationalSettings, cache: Arc<SemanticCache>) -> Self {
        let (tx, rx) = mpsc::channel(32);

        let dirty_regions = Arc::new(RwLock::new(DirtyRegions::new()));
        let debouncer = Arc::new(RwLock::new(Debouncer::new(settings.debounce_ms)));

        // Spawn background analysis task
        let analysis_task = Some(tokio::spawn({
            let dirty_regions = Arc::clone(&dirty_regions);
            let cache = Arc::clone(&cache);
            Self::analysis_loop(rx, dirty_regions, cache, settings.clone())
        }));

        Self {
            settings,
            cache,
            // ... existing fields ...
            dirty_regions,
            debouncer,
            analysis_tx: tx,
            analysis_task,
        }
    }

    /// Schedule analysis for range (debounced)
    pub fn schedule_analysis(&mut self, text: &str, range: Range<usize>) {
        // Mark region as dirty
        if let Ok(mut dirty) = self.dirty_regions.write() {
            dirty.mark_dirty(range);
        }

        // Invalidate cache for overlapping entries
        self.invalidate_cache_for_region(&range);

        // Schedule debounced analysis
        let tx = self.analysis_tx.clone();
        let text_owned = text.to_string();

        tokio::spawn(async move {
            // Wait for debounce delay
            tokio::time::sleep(Duration::from_millis(250)).await;

            // Trigger analysis
            let _ = tx.send(AnalysisRequest::Incremental(text_owned)).await;
        });
    }

    /// Invalidate cache entries overlapping with region
    fn invalidate_cache_for_region(&self, range: &Range<usize>) {
        // Relational cache uses ranges as keys
        // Need to clear any entries that overlap

        // Note: Current RelationalCache doesn't have range-based invalidation
        // For now, use conservative approach: clear entire cache
        // TODO: Implement range-based invalidation in cache.rs
        self.cache.relational.clear();
    }

    /// Background analysis loop
    async fn analysis_loop(
        mut rx: mpsc::Receiver<AnalysisRequest>,
        dirty_regions: Arc<RwLock<DirtyRegions>>,
        cache: Arc<SemanticCache>,
        settings: RelationalSettings,
    ) {
        while let Some(request) = rx.recv().await {
            match request {
                AnalysisRequest::Incremental(text) => {
                    // Get dirty regions
                    let regions = {
                        let dirty = dirty_regions.read().unwrap();
                        dirty.get_dirty().to_vec()
                    };

                    if regions.is_empty() {
                        continue;
                    }

                    // Analyze each dirty region
                    for region in regions {
                        if region.end > text.len() {
                            continue; // Skip invalid regions
                        }

                        let region_text = &text[region.clone()];

                        // Run analysis on region
                        // Results will be stored in cache
                        // (Implementation details depend on specific analyzers)
                    }

                    // Clear dirty regions after successful analysis
                    if let Ok(mut dirty) = dirty_regions.write() {
                        dirty.clear();
                    }
                }
                _ => {}
            }
        }
    }
}

/// Analysis request types
#[derive(Debug, Clone)]
pub enum AnalysisRequest {
    /// Incremental analysis of changed regions
    Incremental(String),

    /// Full re-analysis
    Full(String),
}
```

#### 2. Add Range-Based Cache Invalidation

**File**: `src/ics/semantic_highlighter/cache.rs`

```rust
impl<T: Clone> RelationalCache<T> {
    /// Invalidate cache entries overlapping with range
    pub fn invalidate_range(&self, range: &Range<usize>) {
        if let Ok(mut cache) = self.cache.write() {
            // Collect keys to remove (to avoid borrowing issues)
            let keys_to_remove: Vec<_> = cache
                .iter()
                .filter(|(key, _)| ranges_overlap(key, range))
                .map(|(key, _)| key.clone())
                .collect();

            // Remove overlapping entries
            for key in keys_to_remove {
                cache.pop(&key);
            }
        }
    }
}
```

#### 3. Update Settings

**File**: `src/ics/semantic_highlighter/settings.rs`

```rust
#[derive(Debug, Clone)]
pub struct RelationalSettings {
    // ... existing fields ...

    /// Debounce delay in milliseconds (default: 250ms)
    pub debounce_ms: u64,

    /// Enable incremental analysis (default: true)
    pub enable_incremental: bool,
}

impl Default for RelationalSettings {
    fn default() -> Self {
        Self {
            // ... existing defaults ...
            debounce_ms: 250,
            enable_incremental: true,
        }
    }
}
```

---

## Testing Strategy

### Unit Tests

**Test 1: Dirty Region Merging**
- Given: Multiple overlapping dirty regions
- When: Calling `DirtyRegions::merge_overlapping()`
- Then: Overlapping regions merged into single range

```rust
#[test]
fn test_dirty_region_merge() {
    let mut dirty = DirtyRegions::new();
    dirty.mark_dirty(0..10);
    dirty.mark_dirty(5..15);
    dirty.mark_dirty(20..30);

    let regions = dirty.get_dirty();
    assert_eq!(regions.len(), 2);  // Two non-overlapping regions
    assert_eq!(regions[0], 0..15); // Merged first two
    assert_eq!(regions[1], 20..30);
}
```

**Test 2: Range Overlap Detection**
- Given: Two ranges
- When: Checking for overlap
- Then: Correctly identifies overlapping vs non-overlapping

```rust
#[test]
fn test_range_overlap() {
    assert!(ranges_overlap(&(0..10), &(5..15)));  // Overlapping
    assert!(ranges_overlap(&(0..10), &(0..10)));  // Identical
    assert!(!ranges_overlap(&(0..10), &(10..20))); // Adjacent (no overlap)
    assert!(!ranges_overlap(&(0..10), &(15..20))); // Separate
}
```

**Test 3: Cache Invalidation**
- Given: Cache with multiple entries
- When: Invalidating specific range
- Then: Only overlapping entries removed

```rust
#[test]
fn test_cache_invalidation() {
    let cache = RelationalCache::new(10, 60);
    cache.insert(0..10, CachedResult::new("data1"));
    cache.insert(20..30, CachedResult::new("data2"));
    cache.insert(40..50, CachedResult::new("data3"));

    // Invalidate middle region
    cache.invalidate_range(&(15..35));

    // First and last should remain
    assert!(cache.get(&(0..10)).is_some());
    assert!(cache.get(&(20..30)).is_none());  // Invalidated
    assert!(cache.get(&(40..50)).is_some());
}
```

**Test 4: Debouncer Delay**
- Given: Debouncer with 100ms delay
- When: Scheduling analysis
- Then: Analysis delayed by 100ms

```rust
#[tokio::test]
async fn test_debouncer() {
    let mut debouncer = Debouncer::new(100);
    let start = Instant::now();

    debouncer.schedule().await;

    let elapsed = start.elapsed().as_millis();
    assert!(elapsed >= 100 && elapsed < 150);
}
```

### Integration Tests

**Test 5: End-to-End Incremental Analysis**
- Given: Engine with incremental analysis enabled
- When: Scheduling analysis for changed region
- Then: Only changed region re-analyzed, cache used for rest

**Test 6: Rapid Text Changes**
- Given: Multiple rapid text changes (< debounce delay)
- When: Scheduling analysis for each change
- Then: Only one analysis runs after final change

**Test 7: Cache Hit After Incremental Update**
- Given: Text with cached analysis
- When: Small change in middle, then query unchanged region
- Then: Unchanged region returns cached result

---

## Acceptance Criteria

- [ ] `schedule_analysis()` implemented with debouncing
- [ ] Dirty region tracking with overlap merging
- [ ] Cache invalidation for changed regions only
- [ ] Background task processes scheduled analysis
- [ ] Debounce delay configurable in settings
- [ ] All unit tests passing
- [ ] Integration tests with real text changes passing
- [ ] No performance regression (still <200ms for incremental)
- [ ] Logging in place for debugging
- [ ] Documentation updated

---

## Estimated Effort

- DirtyRegions implementation: 0.5 days
- Debouncer implementation: 0.5 days
- RelationalAnalyzer integration: 0.5 days
- Cache invalidation: 0.5 days
- Testing: 0.5 days
- Integration & debugging: 0.5 days

**Total: 3 days** (can be split into 2 parallel streams: dirty tracking + debouncing, cache invalidation)

---

## Dependencies

- Existing `RelationalCache` and `SemanticCache`
- `tokio::sync::mpsc` for background task communication
- `tokio::time::sleep` for debouncing
- Settings update to include `debounce_ms`

---

## Risks & Mitigation

**Risk 1**: Race conditions between typing and analysis
- Mitigation: Use Arc<RwLock<DirtyRegions>> for thread-safe access, conservative cache invalidation

**Risk 2**: Cache invalidation too aggressive (clears too much)
- Mitigation: Implement precise range-based invalidation, not full clear

**Risk 3**: Debouncing causes perceived lag
- Mitigation: Make delay configurable (100ms-1000ms), default 250ms

**Risk 4**: Memory growth from many dirty regions
- Mitigation: Merge overlapping regions, clear after analysis, cap max regions

---

## References

- Current stub: `tier2_relational/mod.rs:85`
- Cache implementation: `cache.rs`
- Engine integration: `engine.rs:146-150`
- Settings: `settings.rs`
