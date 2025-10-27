//! Agent-specific memory features for the 4-agent architecture
//!
//! This module provides specialized memory access for Mnemosyne's multi-agent system:
//! - **Orchestrator**: Central coordinator managing execution state
//! - **Optimizer**: Context and resource optimization specialist
//! - **Reviewer**: Quality assurance and validation specialist
//! - **Executor**: Primary work agent and sub-agent manager
//!
//! # Features
//!
//! - **Role-Based Views**: Agent-specific memory filtering by type
//! - **Access Control**: Ownership tracking and permission checks
//! - **Custom Scoring**: Role-specific importance calculations
//! - **Prefetching**: LRU cache with intelligent preloading
//!
//! # Example
//!
//! ```ignore
//! use mnemosyne::agents::{AgentRole, AgentMemoryView};
//!
//! let view = AgentMemoryView::new(AgentRole::Executor, storage);
//! let memories = view.search("implementation pattern", 10).await?;
//! // Returns only Implementation and Pattern memories relevant to Executor
//! ```

pub mod access_control;
pub mod importance_scorer;
pub mod memory_view;
pub mod prefetcher;

// Re-export commonly used types
pub use access_control::{MemoryAccessControl, ModificationLog, ModificationType};
pub use importance_scorer::{CustomImportanceScorer, ImportanceWeights};
pub use memory_view::{AgentMemoryView, AgentRole};
pub use prefetcher::{MemoryPrefetcher, PrefetchMetrics, PrefetchTrigger};
