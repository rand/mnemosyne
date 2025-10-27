// Evolution Module - Background memory optimization jobs
//
// This module implements autonomous background jobs that continuously optimize
// the memory system without user intervention.
//
// Components:
// - scheduler: Job scheduling with idle detection
// - importance: Importance recalibration based on usage
// - links: Link strength decay for untraversed connections
// - archival: Automatic archival of unused memories
// - consolidation: Duplicate detection and merging (requires vector search)

pub mod config;
pub mod scheduler;
pub mod importance;
pub mod links;
pub mod archival;
// consolidation will be added after Stream 1 completes

pub use config::{EvolutionConfig, JobConfig};
pub use scheduler::{BackgroundScheduler, EvolutionJob, JobReport, JobError};
