//! Storage layer for Mnemosyne memory system
//!
//! Provides abstractions and implementations for persistent storage of memories,
//! embeddings, links, and audit logs.

pub mod libsql;
pub mod vectors;

use crate::error::Result;
use crate::types::{MemoryId, MemoryNote, Namespace, SearchResult};
use async_trait::async_trait;

/// Storage backend trait defining all required operations
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Store a new memory
    async fn store_memory(&self, memory: &MemoryNote) -> Result<()>;

    /// Retrieve a memory by ID
    async fn get_memory(&self, id: MemoryId) -> Result<MemoryNote>;

    /// Update an existing memory
    async fn update_memory(&self, memory: &MemoryNote) -> Result<()>;

    /// Archive a memory (soft delete)
    async fn archive_memory(&self, id: MemoryId) -> Result<()>;

    /// Vector similarity search
    async fn vector_search(
        &self,
        embedding: &[f32],
        limit: usize,
        namespace: Option<Namespace>,
    ) -> Result<Vec<SearchResult>>;

    /// Keyword search using FTS5
    async fn keyword_search(
        &self,
        query: &str,
        namespace: Option<Namespace>,
    ) -> Result<Vec<SearchResult>>;

    /// Graph traversal from seed memories
    async fn graph_traverse(
        &self,
        seed_ids: &[MemoryId],
        max_hops: usize,
    ) -> Result<Vec<MemoryNote>>;

    /// Find consolidation candidates (similar memories)
    async fn find_consolidation_candidates(
        &self,
        namespace: Option<Namespace>,
    ) -> Result<Vec<(MemoryNote, MemoryNote)>>;

    /// Increment access counter
    async fn increment_access(&self, id: MemoryId) -> Result<()>;

    /// Get memory count by namespace
    async fn count_memories(&self, namespace: Option<Namespace>) -> Result<usize>;

    /// Hybrid search combining keyword + graph traversal
    /// (vector similarity deferred to v2.0)
    async fn hybrid_search(
        &self,
        query: &str,
        namespace: Option<Namespace>,
        max_results: usize,
        expand_graph: bool,
    ) -> Result<Vec<SearchResult>>;

    /// List recent or important memories
    async fn list_memories(
        &self,
        namespace: Option<Namespace>,
        limit: usize,
        sort_by: MemorySortOrder,
    ) -> Result<Vec<MemoryNote>>;
}

/// Sort order for listing memories
#[derive(Debug, Clone, Copy)]
pub enum MemorySortOrder {
    Recent,
    Importance,
    AccessCount,
}
