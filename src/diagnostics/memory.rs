//! Memory profiling and instrumentation
//!
//! This module provides memory tracking and profiling capabilities to diagnose
//! OOM issues (exit code 143). It instruments hot paths and tracks allocations.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Memory statistics tracker (thread-safe)
#[derive(Clone)]
pub struct MemoryTracker {
    /// Total bytes allocated (cumulative)
    pub total_allocated: Arc<AtomicU64>,

    /// Current bytes in use
    pub current_usage: Arc<AtomicU64>,

    /// Peak memory usage
    pub peak_usage: Arc<AtomicU64>,

    /// Number of active allocations
    pub allocation_count: Arc<AtomicUsize>,

    /// Embeddings cache size (bytes)
    pub embeddings_cache_bytes: Arc<AtomicU64>,

    /// Event queue size (count)
    pub event_queue_size: Arc<AtomicUsize>,

    /// Work queue size (count)
    pub work_queue_size: Arc<AtomicUsize>,

    /// Active database connections
    pub db_connections: Arc<AtomicUsize>,

    /// Active spawned tasks
    pub spawned_tasks: Arc<AtomicUsize>,
}

impl Default for MemoryTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryTracker {
    pub fn new() -> Self {
        Self {
            total_allocated: Arc::new(AtomicU64::new(0)),
            current_usage: Arc::new(AtomicU64::new(0)),
            peak_usage: Arc::new(AtomicU64::new(0)),
            allocation_count: Arc::new(AtomicUsize::new(0)),
            embeddings_cache_bytes: Arc::new(AtomicU64::new(0)),
            event_queue_size: Arc::new(AtomicUsize::new(0)),
            work_queue_size: Arc::new(AtomicUsize::new(0)),
            db_connections: Arc::new(AtomicUsize::new(0)),
            spawned_tasks: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Record an allocation
    pub fn record_allocation(&self, bytes: u64) {
        self.total_allocated.fetch_add(bytes, Ordering::Relaxed);
        let current = self.current_usage.fetch_add(bytes, Ordering::Relaxed) + bytes;
        self.allocation_count.fetch_add(1, Ordering::Relaxed);

        // Update peak if needed
        let mut peak = self.peak_usage.load(Ordering::Relaxed);
        while current > peak {
            match self.peak_usage.compare_exchange_weak(
                peak,
                current,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(p) => peak = p,
            }
        }
    }

    /// Record a deallocation
    pub fn record_deallocation(&self, bytes: u64) {
        self.current_usage.fetch_sub(bytes, Ordering::Relaxed);
        self.allocation_count.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record embeddings cache update
    pub fn set_embeddings_cache_size(&self, bytes: u64) {
        self.embeddings_cache_bytes.store(bytes, Ordering::Relaxed);
    }

    /// Record event queue size
    pub fn set_event_queue_size(&self, count: usize) {
        self.event_queue_size.store(count, Ordering::Relaxed);
    }

    /// Record work queue size
    pub fn set_work_queue_size(&self, count: usize) {
        self.work_queue_size.store(count, Ordering::Relaxed);
    }

    /// Increment database connection count
    pub fn increment_db_connections(&self) {
        self.db_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement database connection count
    pub fn decrement_db_connections(&self) {
        self.db_connections.fetch_sub(1, Ordering::Relaxed);
    }

    /// Increment spawned task count
    pub fn increment_spawned_tasks(&self) {
        self.spawned_tasks.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement spawned task count
    pub fn decrement_spawned_tasks(&self) {
        self.spawned_tasks.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get current statistics snapshot
    pub fn snapshot(&self) -> MemorySnapshot {
        MemorySnapshot {
            total_allocated: self.total_allocated.load(Ordering::Relaxed),
            current_usage: self.current_usage.load(Ordering::Relaxed),
            peak_usage: self.peak_usage.load(Ordering::Relaxed),
            allocation_count: self.allocation_count.load(Ordering::Relaxed),
            embeddings_cache_bytes: self.embeddings_cache_bytes.load(Ordering::Relaxed),
            event_queue_size: self.event_queue_size.load(Ordering::Relaxed),
            work_queue_size: self.work_queue_size.load(Ordering::Relaxed),
            db_connections: self.db_connections.load(Ordering::Relaxed),
            spawned_tasks: self.spawned_tasks.load(Ordering::Relaxed),
        }
    }

    /// Log current memory statistics
    pub fn log_statistics(&self) {
        let snapshot = self.snapshot();
        info!(
            current_mb = snapshot.current_usage / 1_048_576,
            peak_mb = snapshot.peak_usage / 1_048_576,
            allocations = snapshot.allocation_count,
            embeddings_mb = snapshot.embeddings_cache_bytes / 1_048_576,
            event_queue = snapshot.event_queue_size,
            work_queue = snapshot.work_queue_size,
            db_conns = snapshot.db_connections,
            tasks = snapshot.spawned_tasks,
            "Memory statistics"
        );
    }

    /// Check if memory usage is approaching critical levels
    pub fn check_thresholds(&self) -> MemoryStatus {
        let snapshot = self.snapshot();
        let current_mb = snapshot.current_usage / 1_048_576;

        // Get system memory if available
        #[cfg(target_os = "macos")]
        let system_total_mb = {
            use std::process::Command;
            Command::new("sysctl")
                .args(["-n", "hw.memsize"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .and_then(|s| s.trim().parse::<u64>().ok())
                .map(|bytes| bytes / 1_048_576)
                .unwrap_or(8192) // Default to 8GB if unknown
        };

        #[cfg(not(target_os = "macos"))]
        let system_total_mb = 8192u64; // Default assumption

        let usage_pct = (current_mb as f64 / system_total_mb as f64) * 100.0;

        if usage_pct > 80.0 {
            warn!(
                usage_pct = format!("{:.1}%", usage_pct),
                current_mb,
                system_total_mb,
                "Memory usage critical"
            );
            MemoryStatus::Critical
        } else if usage_pct > 60.0 {
            warn!(
                usage_pct = format!("{:.1}%", usage_pct),
                current_mb,
                system_total_mb,
                "Memory usage high"
            );
            MemoryStatus::High
        } else if usage_pct > 40.0 {
            debug!(
                usage_pct = format!("{:.1}%", usage_pct),
                current_mb,
                "Memory usage moderate"
            );
            MemoryStatus::Moderate
        } else {
            MemoryStatus::Normal
        }
    }
}

/// Memory statistics snapshot
#[derive(Debug, Clone)]
pub struct MemorySnapshot {
    pub total_allocated: u64,
    pub current_usage: u64,
    pub peak_usage: u64,
    pub allocation_count: usize,
    pub embeddings_cache_bytes: u64,
    pub event_queue_size: usize,
    pub work_queue_size: usize,
    pub db_connections: usize,
    pub spawned_tasks: usize,
}

impl MemorySnapshot {
    /// Convert to human-readable format
    pub fn to_human_readable(&self) -> String {
        format!(
            "Memory Usage:\n\
             - Current: {:.2} MB\n\
             - Peak: {:.2} MB\n\
             - Total Allocated: {:.2} MB\n\
             - Active Allocations: {}\n\
             - Embeddings Cache: {:.2} MB\n\
             - Event Queue: {} items\n\
             - Work Queue: {} items\n\
             - DB Connections: {}\n\
             - Spawned Tasks: {}",
            self.current_usage as f64 / 1_048_576.0,
            self.peak_usage as f64 / 1_048_576.0,
            self.total_allocated as f64 / 1_048_576.0,
            self.allocation_count,
            self.embeddings_cache_bytes as f64 / 1_048_576.0,
            self.event_queue_size,
            self.work_queue_size,
            self.db_connections,
            self.spawned_tasks,
        )
    }
}

/// Memory usage status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryStatus {
    Normal,    // < 40%
    Moderate,  // 40-60%
    High,      // 60-80%
    Critical,  // > 80%
}

/// Global memory tracker instance
static MEMORY_TRACKER: once_cell::sync::Lazy<MemoryTracker> =
    once_cell::sync::Lazy::new(MemoryTracker::new);

/// Get the global memory tracker
pub fn global_memory_tracker() -> &'static MemoryTracker {
    &MEMORY_TRACKER
}

/// Start periodic memory monitoring (logs every 30 seconds)
pub fn start_memory_monitoring() -> tokio::task::JoinHandle<()> {
    let tracker = global_memory_tracker().clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            tracker.log_statistics();
            let status = tracker.check_thresholds();

            if status == MemoryStatus::Critical {
                warn!("CRITICAL: Memory usage is dangerously high, consider graceful shutdown");
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_tracker_allocation() {
        let tracker = MemoryTracker::new();

        tracker.record_allocation(1024);
        assert_eq!(tracker.current_usage.load(Ordering::Relaxed), 1024);
        assert_eq!(tracker.allocation_count.load(Ordering::Relaxed), 1);

        tracker.record_deallocation(512);
        assert_eq!(tracker.current_usage.load(Ordering::Relaxed), 512);
        assert_eq!(tracker.allocation_count.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_memory_tracker_peak() {
        let tracker = MemoryTracker::new();

        tracker.record_allocation(1000);
        tracker.record_allocation(2000);
        assert_eq!(tracker.peak_usage.load(Ordering::Relaxed), 3000);

        tracker.record_deallocation(2000);
        assert_eq!(tracker.peak_usage.load(Ordering::Relaxed), 3000); // Peak unchanged
    }

    #[test]
    fn test_snapshot() {
        let tracker = MemoryTracker::new();

        tracker.record_allocation(1024);
        tracker.set_work_queue_size(5);
        tracker.increment_db_connections();

        let snapshot = tracker.snapshot();
        assert_eq!(snapshot.current_usage, 1024);
        assert_eq!(snapshot.work_queue_size, 5);
        assert_eq!(snapshot.db_connections, 1);
    }
}
