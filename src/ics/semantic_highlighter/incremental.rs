//! Incremental analysis utilities
//!
//! Provides debouncing and dirty region tracking for efficient incremental updates.

use std::ops::Range;
use std::time::{Duration, Instant};

/// Dirty region tracker for incremental analysis
#[derive(Debug, Clone)]
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

    /// Get last modification time
    pub fn last_modified(&self) -> Instant {
        self.last_modified
    }

    /// Check if there are any dirty regions
    pub fn is_dirty(&self) -> bool {
        !self.regions.is_empty()
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

impl Default for DirtyRegions {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if two ranges overlap
pub fn ranges_overlap(a: &Range<usize>, b: &Range<usize>) -> bool {
    a.start < b.end && b.start < a.end
}

/// Debouncer for scheduling analysis with configurable delay
#[derive(Debug)]
pub struct Debouncer {
    /// Debounce delay
    delay: Duration,

    /// Last schedule time
    last_schedule: Option<Instant>,
}

impl Debouncer {
    pub fn new(delay_ms: u64) -> Self {
        Self {
            delay: Duration::from_millis(delay_ms),
            last_schedule: None,
        }
    }

    /// Get the debounce delay
    pub fn delay(&self) -> Duration {
        self.delay
    }

    /// Record that a schedule was requested
    pub fn mark_scheduled(&mut self) {
        self.last_schedule = Some(Instant::now());
    }

    /// Check if enough time has elapsed since last schedule
    pub fn should_trigger(&self) -> bool {
        match self.last_schedule {
            Some(last) => last.elapsed() >= self.delay,
            None => true,
        }
    }

    /// Reset the debouncer
    pub fn reset(&mut self) {
        self.last_schedule = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dirty_region_single() {
        let mut dirty = DirtyRegions::new();
        dirty.mark_dirty(0..10);

        let regions = dirty.get_dirty();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0], 0..10);
        assert!(dirty.is_dirty());
    }

    #[test]
    fn test_dirty_region_merge_overlapping() {
        let mut dirty = DirtyRegions::new();
        dirty.mark_dirty(0..10);
        dirty.mark_dirty(5..15);
        dirty.mark_dirty(20..30);

        let regions = dirty.get_dirty();
        assert_eq!(regions.len(), 2); // Two non-overlapping regions
        assert_eq!(regions[0], 0..15); // Merged first two
        assert_eq!(regions[1], 20..30);
    }

    #[test]
    fn test_dirty_region_merge_adjacent() {
        let mut dirty = DirtyRegions::new();
        dirty.mark_dirty(0..10);
        dirty.mark_dirty(10..20);

        let regions = dirty.get_dirty();
        assert_eq!(regions.len(), 1); // Merged adjacent regions
        assert_eq!(regions[0], 0..20);
    }

    #[test]
    fn test_dirty_region_clear() {
        let mut dirty = DirtyRegions::new();
        dirty.mark_dirty(0..10);
        assert!(dirty.is_dirty());

        dirty.clear();
        assert!(!dirty.is_dirty());
        assert_eq!(dirty.get_dirty().len(), 0);
    }

    #[test]
    fn test_range_overlap() {
        assert!(ranges_overlap(&(0..10), &(5..15))); // Overlapping
        assert!(ranges_overlap(&(0..10), &(0..10))); // Identical
        assert!(!ranges_overlap(&(0..10), &(10..20))); // Adjacent (no overlap)
        assert!(!ranges_overlap(&(0..10), &(15..20))); // Separate
    }

    #[test]
    fn test_dirty_region_overlaps() {
        let mut dirty = DirtyRegions::new();
        dirty.mark_dirty(0..10);
        dirty.mark_dirty(20..30);

        assert!(dirty.overlaps(&(5..15)));  // Overlaps first region
        assert!(dirty.overlaps(&(15..25))); // Overlaps second region
        assert!(!dirty.overlaps(&(10..20))); // Between regions
    }

    #[test]
    fn test_debouncer_delay() {
        let debouncer = Debouncer::new(250);
        assert_eq!(debouncer.delay(), Duration::from_millis(250));
        assert!(debouncer.should_trigger()); // No schedule yet
    }

    #[test]
    fn test_debouncer_schedule() {
        let mut debouncer = Debouncer::new(100);
        assert!(debouncer.should_trigger());

        debouncer.mark_scheduled();
        assert!(!debouncer.should_trigger()); // Just scheduled

        // Wait for delay
        std::thread::sleep(Duration::from_millis(150));
        assert!(debouncer.should_trigger()); // Enough time elapsed
    }

    #[test]
    fn test_debouncer_reset() {
        let mut debouncer = Debouncer::new(100);
        debouncer.mark_scheduled();

        debouncer.reset();
        assert!(debouncer.should_trigger()); // Reset to initial state
    }
}
