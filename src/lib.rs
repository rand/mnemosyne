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

pub mod error;
pub mod services;
pub mod storage;
pub mod types;

// Re-export commonly used types
pub use error::{MnemosyneError, Result};
pub use services::LlmService;
pub use storage::{sqlite::SqliteStorage, StorageBackend};
pub use types::{
    ConsolidationDecision, LinkType, MemoryId, MemoryLink, MemoryNote, MemoryType, MemoryUpdates,
    Namespace, SearchQuery, SearchResult,
};
