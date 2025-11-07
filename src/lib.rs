//! Mnemosyne - Project-Aware Agentic Memory System
//!
//! A high-performance Rust-based memory system for Claude Code that provides:
//! - Project-aware namespace isolation
//! - Semantic memory search with hybrid retrieval
//! - LLM-guided note construction and linking
//! - OODA loop integration for human and agent users
//! - Self-organizing knowledge graphs
//!
//! # Architecture
//!
//! The system is organized into several layers:
//! - **Types**: Core data structures (MemoryNote, Namespace, etc.)
//! - **Storage**: Database backends (SQLite, Postgres)
//! - **Services**: LLM integration, embedding generation
//! - **MCP**: Model Context Protocol server interface
//!
//! # Example
//!
//! ```ignore
//! use mnemosyne::{MemoryManager, Namespace, SearchQuery};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = MnemosyneConfig::from_file("config.toml")?;
//!     let manager = MemoryManager::new(config).await?;
//!
//!     // Store a memory
//!     let id = manager.remember(
//!         "Decided to use PostgreSQL for user data".to_string(),
//!         Namespace::Project { name: "myapp".to_string() },
//!         Some(9)
//!     ).await?;
//!
//!     // Recall memories
//!     let results = manager.recall(SearchQuery {
//!         query: "database decisions".to_string(),
//!         ..Default::default()
//!     }).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod agents;
pub mod api; // HTTP API for event streaming
pub mod artifacts; // Specification workflow artifacts
pub mod config;
pub mod coordination; // ICS handoff coordination
pub mod daemon;
pub mod diagnostics; // Memory profiling and resource tracking
pub mod embeddings;
pub mod error;
pub mod evaluation;
pub mod evolution;
pub mod health; // Health check system
pub mod icons; // Nerd Font icons with ASCII fallbacks
pub mod ics; // Integrated Context Studio
pub mod launcher;
pub mod mcp;
pub mod namespace;
pub mod orchestration;
pub mod pty; // PTY wrapper for Claude Code
pub mod secrets;
pub mod services;
pub mod storage;
pub mod tui; // Shared TUI infrastructure
pub mod types;
pub mod update; // Tool update and installation system
pub mod utils; // Utility functions and helpers
pub mod version_check; // Version checking and update system

// Python bindings (PyO3) - only available with "python" feature
#[cfg(feature = "python")]
pub mod python_bindings;

// Re-export commonly used types
pub use agents::{AgentMemoryView, AgentRole, CustomImportanceScorer, MemoryAccessControl};
pub use config::{ConfigManager, EmbeddingConfig, SearchConfig};
pub use diagnostics::{global_memory_tracker, start_memory_monitoring, MemorySnapshot, MemoryStatus};
pub use embeddings::{
    cosine_similarity, EmbeddingService, LocalEmbeddingService, RemoteEmbeddingService,
    VOYAGE_EMBEDDING_DIM,
};
pub use error::{MnemosyneError, Result};
pub use evaluation::{
    ContextEvaluation, FeatureExtractor, FeedbackCollector, ProvidedContext, RelevanceFeatures,
    RelevanceScorer, Scope, WeightSet,
};
pub use evolution::{
    ArchivalJob, BackgroundScheduler, ConsolidationJob, EvolutionConfig, EvolutionJob,
    ImportanceRecalibrator, JobConfig, JobReport, LinkDecayJob,
};
pub use mcp::{EventSink, McpServer, ToolHandler};
pub use namespace::{NamespaceDetector, ProjectMetadata};
pub use orchestration::{AgentEvent, OrchestrationEngine, SupervisionConfig, WorkItem, WorkQueue};
pub use services::{LlmConfig, LlmService};
pub use storage::{
    libsql::{ConnectionMode, LibsqlStorage},
    StorageBackend,
};
pub use types::{
    ConsolidationDecision, LinkType, MemoryId, MemoryLink, MemoryNote, MemoryType, MemoryUpdates,
    Namespace, SearchQuery, SearchResult,
};
pub use update::{prompt_for_install, prompt_for_update, UpdateManager, UpdateResult};
pub use version_check::{Tool, VersionCheckCache, VersionChecker, VersionInfo};
