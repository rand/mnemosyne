//! Time-series metrics collection for dashboard visualizations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// Circular buffer for time-series data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircularBuffer<T> {
    data: VecDeque<(DateTime<Utc>, T)>,
    capacity: usize,
}

impl<T> CircularBuffer<T> {
    /// Create new circular buffer with specified capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Push new data point (oldest removed if at capacity)
    pub fn push(&mut self, value: T) {
        let timestamp = Utc::now();
        if self.data.len() >= self.capacity {
            self.data.pop_front();
        }
        self.data.push_back((timestamp, value));
    }

    /// Get all data points
    pub fn data(&self) -> &VecDeque<(DateTime<Utc>, T)> {
        &self.data
    }

    /// Get length of buffer
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get most recent value
    pub fn latest(&self) -> Option<&T> {
        self.data.back().map(|(_, v)| v)
    }

    /// Clear all data
    pub fn clear(&mut self) {
        self.data.clear();
    }
}

impl<T> Default for CircularBuffer<T> {
    fn default() -> Self {
        Self::new(300) // 5 minutes at 1Hz sampling
    }
}

/// Agent state counts snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStateCounts {
    pub active: usize,
    pub idle: usize,
    pub waiting: usize,
    pub completed: usize,
    pub failed: usize,
    pub total: usize,
}

impl Default for AgentStateCounts {
    fn default() -> Self {
        Self {
            active: 0,
            idle: 0,
            waiting: 0,
            completed: 0,
            failed: 0,
            total: 0,
        }
    }
}

/// Memory operation rates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryOpRates {
    pub stores_per_minute: f32,
    pub recalls_per_minute: f32,
    pub evolutions_total: usize,
    pub consolidations_total: usize,
    pub graph_nodes: usize,
}

impl Default for MemoryOpRates {
    fn default() -> Self {
        Self {
            stores_per_minute: 0.0,
            recalls_per_minute: 0.0,
            evolutions_total: 0,
            consolidations_total: 0,
            graph_nodes: 0,
        }
    }
}

/// Skill usage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillUsage {
    pub loaded_skills: Vec<String>,
    pub recently_used: Vec<(String, DateTime<Utc>)>,
    pub usage_counts: HashMap<String, usize>,
}

impl Default for SkillUsage {
    fn default() -> Self {
        Self {
            loaded_skills: Vec::new(),
            recently_used: Vec::new(),
            usage_counts: HashMap::new(),
        }
    }
}

/// Work orchestration progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkProgress {
    pub current_phase: String,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub critical_path_progress: f32,
    pub parallel_streams: Vec<String>,
}

impl Default for WorkProgress {
    fn default() -> Self {
        Self {
            current_phase: "Unknown".to_string(),
            total_tasks: 0,
            completed_tasks: 0,
            critical_path_progress: 0.0,
            parallel_streams: Vec::new(),
        }
    }
}

/// Comprehensive metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: DateTime<Utc>,
    pub agent_states: AgentStateCounts,
    pub memory_ops: MemoryOpRates,
    pub skills: SkillUsage,
    pub work: WorkProgress,
}

/// Metrics collector maintaining time-series data
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    /// Agent state counts over time
    agent_states: CircularBuffer<AgentStateCounts>,
    /// Memory operation rates over time
    memory_ops: CircularBuffer<MemoryOpRates>,
    /// Skill usage over time
    skills: CircularBuffer<SkillUsage>,
    /// Work progress over time
    work: CircularBuffer<WorkProgress>,

    // Raw counters for rate calculation
    stores_count: usize,
    recalls_count: usize,
    last_rate_calc: DateTime<Utc>,
}

impl MetricsCollector {
    /// Create new metrics collector with default buffer size (300 points)
    pub fn new() -> Self {
        Self::with_capacity(300)
    }

    /// Create new metrics collector with specified buffer capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            agent_states: CircularBuffer::new(capacity),
            memory_ops: CircularBuffer::new(capacity),
            skills: CircularBuffer::new(capacity),
            work: CircularBuffer::new(capacity),
            stores_count: 0,
            recalls_count: 0,
            last_rate_calc: Utc::now(),
        }
    }

    /// Update agent state counts
    pub fn update_agent_states(&mut self, states: AgentStateCounts) {
        self.agent_states.push(states);
    }

    /// Record memory store
    pub fn record_memory_store(&mut self) {
        self.stores_count += 1;
    }

    /// Record memory recall
    pub fn record_memory_recall(&mut self) {
        self.recalls_count += 1;
    }

    /// Update memory operation rates (call periodically, e.g., every second)
    pub fn update_memory_rates(&mut self, evolutions: usize, consolidations: usize, graph_nodes: usize) {
        let now = Utc::now();
        let elapsed = (now - self.last_rate_calc).num_seconds() as f32;

        if elapsed >= 60.0 {
            let stores_per_minute = (self.stores_count as f32 / elapsed) * 60.0;
            let recalls_per_minute = (self.recalls_count as f32 / elapsed) * 60.0;

            self.memory_ops.push(MemoryOpRates {
                stores_per_minute,
                recalls_per_minute,
                evolutions_total: evolutions,
                consolidations_total: consolidations,
                graph_nodes,
            });

            self.stores_count = 0;
            self.recalls_count = 0;
            self.last_rate_calc = now;
        }
    }

    /// Update skill usage
    pub fn update_skills(&mut self, usage: SkillUsage) {
        self.skills.push(usage);
    }

    /// Update work progress
    pub fn update_work(&mut self, progress: WorkProgress) {
        self.work.push(progress);
    }

    /// Get current metrics snapshot
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            timestamp: Utc::now(),
            agent_states: self.agent_states.latest().cloned().unwrap_or_default(),
            memory_ops: self.memory_ops.latest().cloned().unwrap_or_default(),
            skills: self.skills.latest().cloned().unwrap_or_default(),
            work: self.work.latest().cloned().unwrap_or_default(),
        }
    }

    /// Get agent state time series
    pub fn agent_states_series(&self) -> &CircularBuffer<AgentStateCounts> {
        &self.agent_states
    }

    /// Get memory ops time series
    pub fn memory_ops_series(&self) -> &CircularBuffer<MemoryOpRates> {
        &self.memory_ops
    }

    /// Get skills time series
    pub fn skills_series(&self) -> &CircularBuffer<SkillUsage> {
        &self.skills
    }

    /// Get work progress time series
    pub fn work_series(&self) -> &CircularBuffer<WorkProgress> {
        &self.work
    }

    /// Clear all metrics
    pub fn clear(&mut self) {
        self.agent_states.clear();
        self.memory_ops.clear();
        self.skills.clear();
        self.work.clear();
        self.stores_count = 0;
        self.recalls_count = 0;
        self.last_rate_calc = Utc::now();
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circular_buffer() {
        let mut buffer = CircularBuffer::new(3);
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        assert_eq!(buffer.len(), 3);

        buffer.push(4); // Should remove 1
        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.latest(), Some(&4));
    }

    #[test]
    fn test_metrics_collector() {
        let mut collector = MetricsCollector::new();

        let states = AgentStateCounts {
            active: 2,
            idle: 1,
            waiting: 0,
            completed: 5,
            failed: 0,
            total: 8,
        };

        collector.update_agent_states(states.clone());

        let snapshot = collector.snapshot();
        assert_eq!(snapshot.agent_states.active, 2);
        assert_eq!(snapshot.agent_states.total, 8);
    }

    #[test]
    fn test_memory_rate_calculation() {
        let mut collector = MetricsCollector::new();

        // Simulate stores
        for _ in 0..10 {
            collector.record_memory_store();
        }

        // Fast-forward time (in real usage, this would be 60 seconds later)
        collector.last_rate_calc = Utc::now() - chrono::Duration::seconds(60);
        collector.update_memory_rates(1, 0, 1000);

        let snapshot = collector.snapshot();
        assert!(snapshot.memory_ops.stores_per_minute > 0.0);
    }
}
