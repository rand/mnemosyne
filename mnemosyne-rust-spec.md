# Mnemosyne: Rust-Based Agentic Memory System Specification

**Version**: 1.0.0  
**Date**: 2025-10-26  
**Status**: READY FOR IMPLEMENTATION  
**Target**: Claude Code with Rust toolchain

---

## Executive Summary

Mnemosyne is a high-performance, project-aware agentic memory system implemented in Rust that provides Claude Code with persistent semantic memory. The system achieves <200ms retrieval latency, 70-80% accuracy, and 85-95% context compression through hybrid storage (vector + graph + structured) and self-organizing knowledge graphs.

**Why Rust**: Memory safety, zero-cost abstractions, SIMD-optimized vector operations, native async/await, excellent SQLite bindings, and alignment with formal verification principles.

**Core Innovation**: Automatic semantic note construction with LLM-guided linking and evolution, creating a Zettelkasten-inspired knowledge graph that mirrors human understanding building.

---

## Table of Contents

1. [System Architecture](#1-system-architecture)
2. [Technology Stack](#2-technology-stack)
3. [Data Structures](#3-data-structures)
4. [Core Components](#4-core-components)
5. [MCP Server Interface](#5-mcp-server-interface)
6. [Storage Layer](#6-storage-layer)
7. [Memory Operations](#7-memory-operations)
8. [Project-Aware Context Management](#8-project-aware-context-management)
9. [Installation & Configuration](#9-installation--configuration)
10. [Testing Strategy](#10-testing-strategy)
11. [Implementation Roadmap](#11-implementation-roadmap)
12. [Performance Requirements](#12-performance-requirements)
13. [Integration Points](#13-integration-points)
14. [Typed Holes & Extension Points](#14-typed-holes--extension-points)
15. [Security & Privacy](#15-security--privacy)

---

## 1. System Architecture

### 1.1 Component Hierarchy

```
┌─────────────────────────────────────────────────────────────┐
│                    MCP Server (stdio)                       │
│  ┌────────────────────────────────────────────────────┐    │
│  │         Request Handler & Tool Router              │    │
│  └─────────────────┬──────────────────────────────────┘    │
└────────────────────┼─────────────────────────────────────────┘
                     │
          ┌──────────┴──────────┐
          │                     │
┌─────────▼──────────┐ ┌───────▼──────────┐
│  Memory Manager    │ │  LLM Services    │
│                    │ │                  │
│ • Lifecycle        │ │ • Note Builder   │
│ • Orchestration    │ │ • Link Analyzer  │
│ • Namespaces       │ │ • Consolidator   │
│ • Background Tasks │ │ • Embeddings     │
└─────────┬──────────┘ └───────┬──────────┘
          │                    │
          └──────────┬─────────┘
                     │
          ┌──────────▼──────────┐
          │   Storage Layer     │
          │                     │
          │ • Vector Store      │
          │ • Graph Store       │
          │ • Metadata Store    │
          │ • Audit Log         │
          └─────────────────────┘
```

### 1.2 Layer Responsibilities

**Layer 1: MCP Server**
- Protocol handling (JSON-RPC over stdio)
- Request validation and routing
- Response formatting
- Error handling and logging

**Layer 2: Orchestration**
- Memory lifecycle management
- Namespace resolution
- Background task scheduling
- Transaction coordination

**Layer 3: Intelligence**
- LLM-guided note construction
- Semantic link generation
- Memory evolution
- Consolidation decisions

**Layer 4: Storage**
- Vector similarity search
- Graph traversal
- Structured queries
- Atomic transactions

### 1.3 Design Principles

1. **Zero-Copy**: Minimize allocations, use references
2. **Type Safety**: Leverage Rust's type system for correctness
3. **Async-First**: Non-blocking I/O throughout
4. **Fail-Fast**: Explicit error handling with `Result<T, E>`
5. **Immutable Audit Trail**: Never delete, only supersede
6. **Incremental Complexity**: Start simple, add features progressively

---

## 2. Technology Stack

### 2.1 Core Dependencies

```toml
[package]
name = "mnemosyne"
version = "0.1.0"
edition = "2021"

[dependencies]
# Async runtime
tokio = { version = "1.35", features = ["full", "tracing"] }
tokio-util = { version = "0.7", features = ["codec"] }

# Database
sqlx = { version = "0.7", features = [
    "runtime-tokio",
    "sqlite",
    "postgres",
    "json",
    "chrono",
    "uuid"
] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# LLM Integration
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }
anthropic-sdk = "0.1"  # If available, else manual HTTP

# Embeddings
fastembed = "3.0"  # Local embeddings
ndarray = "0.15"   # Vector operations

# MCP Protocol
# Note: May need to implement manually if no Rust SDK exists
serde_jsonrpc = "0.4"

# Utilities
anyhow = "1.0"      # Error handling
thiserror = "1.0"   # Custom errors
tracing = "0.1"     # Structured logging
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.6", features = ["v4", "serde"] }
config = "0.13"     # Configuration management
clap = { version = "4.4", features = ["derive"] }

# Testing
[dev-dependencies]
mockall = "0.12"
proptest = "1.4"
criterion = "0.5"
tempfile = "3.8"
```

### 2.2 External Tools

- **SQLite 3.43+**: With `sqlite-vec` extension for vector operations
- **PostgreSQL 16+**: Optional for production (with pgvector)
- **Anthropic API**: For LLM operations (Claude Haiku for memory ops)
- **Claude Code**: Development environment and primary user

### 2.3 Development Tools

```bash
# Required
rustc 1.75+
cargo
sqlite3

# Optional
postgresql 16+
docker (for testing with postgres)
```

---

## 3. Data Structures

### 3.1 Core Types

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for memories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MemoryId(Uuid);

impl MemoryId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// Namespace hierarchy: Global > Project > Session
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Namespace {
    Global,
    Project { name: String },
    Session { project: String, session_id: String },
}

impl Namespace {
    pub fn priority(&self) -> u8 {
        match self {
            Namespace::Session { .. } => 3,
            Namespace::Project { .. } => 2,
            Namespace::Global => 1,
        }
    }
}

/// Memory type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    ArchitectureDecision,
    CodePattern,
    BugFix,
    Configuration,
    Constraint,
    Entity,
    Insight,
    Reference,
    Preference,
}

/// Relationship types between memories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkType {
    Extends,      // B builds on A
    Contradicts,  // B contradicts A
    Implements,   // B implements A
    References,   // B references A
    Supersedes,   // B replaces A
}

/// Memory link with typed relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLink {
    pub target_id: MemoryId,
    pub link_type: LinkType,
    pub strength: f32,  // 0.0 - 1.0
    pub reason: String,
    pub created_at: DateTime<Utc>,
}

/// Complete memory note structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryNote {
    // Identity
    pub id: MemoryId,
    pub namespace: Namespace,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // Content (human-readable)
    pub content: String,
    pub summary: String,
    pub keywords: Vec<String>,
    pub tags: Vec<String>,
    pub context: String,

    // Classification
    pub memory_type: MemoryType,
    pub importance: u8,    // 1-10
    pub confidence: f32,   // 0.0-1.0

    // Relationships
    pub links: Vec<MemoryLink>,
    pub related_files: Vec<String>,
    pub related_entities: Vec<String>,

    // Lifecycle
    pub access_count: u32,
    pub last_accessed_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_archived: bool,
    pub superseded_by: Option<MemoryId>,

    // Computational
    #[serde(skip)]
    pub embedding: Option<Vec<f32>>,
    pub embedding_model: String,
}

impl MemoryNote {
    /// Calculate decayed importance based on age and access
    pub fn decayed_importance(&self) -> f32 {
        let base = self.importance as f32;
        let recency_factor = self.recency_factor();
        let type_factor = self.type_factor();
        let access_bonus = (self.access_count as f32).ln().max(0.0) * 0.1;
        
        base * recency_factor * type_factor * (1.0 + access_bonus)
    }

    fn recency_factor(&self) -> f32 {
        let age_days = (Utc::now() - self.updated_at).num_days() as f32;
        (-age_days / 180.0).exp() // Half-life of 6 months
    }

    fn type_factor(&self) -> f32 {
        match self.memory_type {
            MemoryType::ArchitectureDecision => 1.2,
            MemoryType::Constraint => 1.1,
            MemoryType::CodePattern => 1.0,
            MemoryType::BugFix => 0.9,
            MemoryType::Insight => 0.9,
            _ => 0.8,
        }
    }
}

/// Embedding vector with dimension type safety
#[derive(Debug, Clone)]
pub struct Embedding<const DIM: usize>(pub [f32; DIM]);

impl<const DIM: usize> Embedding<DIM> {
    /// Compute cosine similarity (SIMD-optimized by compiler)
    pub fn cosine_similarity(&self, other: &Self) -> f32 {
        let dot: f32 = self.0.iter()
            .zip(other.0.iter())
            .map(|(a, b)| a * b)
            .sum();
        
        let norm_a: f32 = self.0.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = other.0.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        dot / (norm_a * norm_b)
    }
}

/// Search query with filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub namespace: Option<Namespace>,
    pub memory_types: Vec<MemoryType>,
    pub tags: Vec<String>,
    pub min_importance: Option<u8>,
    pub max_results: usize,
    pub include_archived: bool,
}

/// Search result with relevance score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub memory: MemoryNote,
    pub score: f32,
    pub match_reason: String,
}
```

### 3.2 Configuration Types

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct MnemosyneConfig {
    /// Database configuration
    pub database: DatabaseConfig,
    
    /// LLM configuration
    pub llm: LlmConfig,
    
    /// Embedding configuration
    pub embeddings: EmbeddingConfig,
    
    /// Memory lifecycle configuration
    pub lifecycle: LifecycleConfig,
    
    /// MCP server configuration
    pub server: ServerConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub backend: DatabaseBackend,
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseBackend {
    Sqlite,
    Postgres,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfig {
    pub api_key: Option<String>,  // Optional, uses env var
    pub model: String,             // "claude-haiku-4-20250514"
    pub max_tokens: u32,
    pub temperature: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingConfig {
    pub provider: EmbeddingProvider,
    pub model: String,
    pub dimension: usize,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddingProvider {
    Local,   // fastembed
    OpenAI,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LifecycleConfig {
    pub archival_threshold_days: u32,
    pub archival_min_importance: f32,
    pub consolidation_interval_hours: u32,
    pub max_memories_per_namespace: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub log_level: String,
    pub enable_metrics: bool,
}
```

---

## 4. Core Components

### 4.1 Memory Manager

```rust
use anyhow::Result;
use tokio::sync::RwLock;
use std::sync::Arc;

/// Central orchestrator for memory operations
pub struct MemoryManager {
    storage: Arc<StorageBackend>,
    llm_service: Arc<LlmService>,
    embedding_service: Arc<EmbeddingService>,
    config: MnemosyneConfig,
    background_tasks: BackgroundTaskManager,
}

impl MemoryManager {
    pub async fn new(config: MnemosyneConfig) -> Result<Self> {
        let storage = Arc::new(StorageBackend::new(&config.database).await?);
        let llm_service = Arc::new(LlmService::new(config.llm.clone())?);
        let embedding_service = Arc::new(
            EmbeddingService::new(config.embeddings.clone()).await?
        );
        
        let background_tasks = BackgroundTaskManager::new(
            Arc::clone(&storage),
            Arc::clone(&llm_service),
            config.lifecycle.clone(),
        );
        
        Ok(Self {
            storage,
            llm_service,
            embedding_service,
            config,
            background_tasks,
        })
    }

    /// Store a new memory with automatic note construction and linking
    pub async fn remember(
        &self,
        content: String,
        namespace: Namespace,
        importance: Option<u8>,
    ) -> Result<MemoryId> {
        // Phase 1: Construct structured note with LLM
        let note_metadata = self.llm_service
            .construct_note(&content, namespace.clone())
            .await?;
        
        // Phase 2: Generate embedding
        let embedding = self.embedding_service
            .embed(&content)
            .await?;
        
        // Phase 3: Create memory note
        let mut memory = MemoryNote {
            id: MemoryId::new(),
            namespace,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            content: content.clone(),
            summary: note_metadata.summary,
            keywords: note_metadata.keywords,
            tags: note_metadata.tags,
            context: note_metadata.context,
            memory_type: note_metadata.memory_type,
            importance: importance.unwrap_or(note_metadata.importance),
            confidence: note_metadata.confidence,
            links: Vec::new(),
            related_files: note_metadata.related_files,
            related_entities: note_metadata.related_entities,
            access_count: 0,
            last_accessed_at: Utc::now(),
            expires_at: None,
            is_archived: false,
            superseded_by: None,
            embedding: Some(embedding.clone()),
            embedding_model: self.config.embeddings.model.clone(),
        };
        
        // Phase 4: Find candidate memories for linking
        let candidates = self.storage
            .vector_search(&embedding, 20, Some(memory.namespace.clone()))
            .await?;
        
        // Phase 5: Generate semantic links with LLM
        if !candidates.is_empty() {
            let links = self.llm_service
                .generate_links(&memory, &candidates)
                .await?;
            memory.links = links;
        }
        
        // Phase 6: Store memory
        self.storage.store_memory(&memory).await?;
        
        // Phase 7: Schedule background tasks
        self.background_tasks
            .schedule_evolution(memory.id)
            .await?;
        
        Ok(memory.id)
    }

    /// Retrieve memories matching query
    pub async fn recall(&self, query: SearchQuery) -> Result<Vec<SearchResult>> {
        // Multi-strategy retrieval
        let embedding = self.embedding_service.embed(&query.query).await?;
        
        // Strategy 1: Vector similarity search
        let vector_results = self.storage
            .vector_search(&embedding, query.max_results * 2, query.namespace.clone())
            .await?;
        
        // Strategy 2: Keyword search
        let keyword_results = self.storage
            .keyword_search(&query.query, query.namespace.clone())
            .await?;
        
        // Strategy 3: Graph traversal (if vector results exist)
        let mut graph_results = Vec::new();
        if !vector_results.is_empty() {
            let seed_ids: Vec<MemoryId> = vector_results
                .iter()
                .take(5)
                .map(|r| r.memory.id)
                .collect();
            graph_results = self.storage
                .graph_traverse(&seed_ids, 2)  // 2-hop traversal
                .await?;
        }
        
        // Fuse results with reciprocal rank fusion
        let fused = self.fuse_search_results(
            vector_results,
            keyword_results,
            graph_results,
        )?;
        
        // Apply filters
        let filtered = self.apply_filters(fused, &query)?;
        
        // Update access statistics
        for result in &filtered {
            self.storage.increment_access(result.memory.id).await?;
        }
        
        Ok(filtered.into_iter().take(query.max_results).collect())
    }

    /// Update existing memory
    pub async fn update_memory(
        &self,
        id: MemoryId,
        updates: MemoryUpdates,
    ) -> Result<()> {
        let mut memory = self.storage.get_memory(id).await?;
        
        // Apply updates
        if let Some(content) = updates.content {
            memory.content = content;
            memory.updated_at = Utc::now();
            
            // Regenerate embedding if content changed
            let embedding = self.embedding_service.embed(&memory.content).await?;
            memory.embedding = Some(embedding);
        }
        
        if let Some(importance) = updates.importance {
            memory.importance = importance;
        }
        
        if let Some(tags) = updates.tags {
            memory.tags = tags;
        }
        
        // Store updated memory
        self.storage.update_memory(&memory).await?;
        
        Ok(())
    }

    /// Delete (archive) memory
    pub async fn delete_memory(&self, id: MemoryId) -> Result<()> {
        self.storage.archive_memory(id).await
    }

    /// Consolidate redundant memories
    pub async fn consolidate_memories(&self, namespace: Option<Namespace>) -> Result<usize> {
        let candidates = self.storage
            .find_consolidation_candidates(namespace)
            .await?;
        
        let mut consolidated = 0;
        
        for (mem1, mem2) in candidates {
            let decision = self.llm_service
                .decide_consolidation(&mem1, &mem2)
                .await?;
            
            match decision {
                ConsolidationDecision::Merge { into, content } => {
                    self.merge_memories(mem1.id, mem2.id, into, content).await?;
                    consolidated += 1;
                }
                ConsolidationDecision::Supersede { kept, superseded } => {
                    self.supersede_memory(superseded, kept).await?;
                    consolidated += 1;
                }
                ConsolidationDecision::KeepBoth => {
                    // No action needed
                }
            }
        }
        
        Ok(consolidated)
    }

    // Private helper methods
    
    fn fuse_search_results(
        &self,
        vector: Vec<SearchResult>,
        keyword: Vec<SearchResult>,
        graph: Vec<MemoryNote>,
    ) -> Result<Vec<SearchResult>> {
        // TODO: Implement reciprocal rank fusion
        todo!("Implement result fusion algorithm")
    }

    fn apply_filters(
        &self,
        results: Vec<SearchResult>,
        query: &SearchQuery,
    ) -> Result<Vec<SearchResult>> {
        // TODO: Implement filtering logic
        todo!("Implement filter application")
    }

    async fn merge_memories(
        &self,
        id1: MemoryId,
        id2: MemoryId,
        into: MemoryId,
        content: String,
    ) -> Result<()> {
        // TODO: Implement memory merging
        todo!("Implement memory merge logic")
    }

    async fn supersede_memory(
        &self,
        old: MemoryId,
        new: MemoryId,
    ) -> Result<()> {
        // TODO: Implement supersession
        todo!("Implement supersession logic")
    }
}

#[derive(Debug, Default)]
pub struct MemoryUpdates {
    pub content: Option<String>,
    pub importance: Option<u8>,
    pub tags: Option<Vec<String>>,
}

pub enum ConsolidationDecision {
    Merge { into: MemoryId, content: String },
    Supersede { kept: MemoryId, superseded: MemoryId },
    KeepBoth,
}
```

### 4.2 LLM Service

```rust
use serde_json::json;

/// LLM operations for memory intelligence
pub struct LlmService {
    client: reqwest::Client,
    config: LlmConfig,
}

impl LlmService {
    pub fn new(config: LlmConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        
        Ok(Self { client, config })
    }

    /// Construct structured note from raw content
    pub async fn construct_note(
        &self,
        content: &str,
        namespace: Namespace,
    ) -> Result<NoteMetadata> {
        let prompt = format!(
            r#"Given this information, extract structured metadata:

Content: {content}
Namespace: {namespace:?}

Respond with JSON:
{{
  "summary": "concise 1-2 sentence summary",
  "keywords": ["key", "terms"],
  "tags": ["categorization", "tags"],
  "context": "when/why this is relevant",
  "memory_type": "architecture_decision|code_pattern|bug_fix|etc",
  "importance": 1-10,
  "confidence": 0.0-1.0,
  "related_files": ["file/paths"],
  "related_entities": ["ComponentName", "ServiceName"]
}}"#
        );
        
        let response = self.call_claude(&prompt).await?;
        let metadata: NoteMetadata = serde_json::from_str(&response)?;
        
        Ok(metadata)
    }

    /// Generate semantic links between memories
    pub async fn generate_links(
        &self,
        memory: &MemoryNote,
        candidates: &[SearchResult],
    ) -> Result<Vec<MemoryLink>> {
        let prompt = format!(
            r#"Analyze relationships between this new memory and existing memories.

New Memory:
Summary: {}
Content: {}

Candidates:
{}

For each relationship, respond with JSON array:
[
  {{
    "target_id": "uuid",
    "link_type": "extends|contradicts|implements|references|supersedes",
    "strength": 0.0-1.0,
    "reason": "why they're related"
  }}
]

Only include strong relationships (strength > 0.5)."#,
            memory.summary,
            memory.content,
            Self::format_candidates(candidates)
        );
        
        let response = self.call_claude(&prompt).await?;
        let links: Vec<MemoryLink> = serde_json::from_str(&response)?;
        
        Ok(links)
    }

    /// Decide how to consolidate two memories
    pub async fn decide_consolidation(
        &self,
        mem1: &MemoryNote,
        mem2: &MemoryNote,
    ) -> Result<ConsolidationDecision> {
        let prompt = format!(
            r#"Should these memories be consolidated?

Memory 1:
Summary: {}
Content: {}

Memory 2:
Summary: {}
Content: {}

Respond with JSON:
{{
  "decision": "merge|supersede|keep_both",
  "kept_id": "uuid if supersede",
  "merged_content": "merged text if merge",
  "reason": "explanation"
}}"#,
            mem1.summary, mem1.content,
            mem2.summary, mem2.content
        );
        
        let response = self.call_claude(&prompt).await?;
        let decision = Self::parse_consolidation_decision(&response)?;
        
        Ok(decision)
    }

    /// Call Claude API with retry logic
    async fn call_claude(&self, prompt: &str) -> Result<String> {
        let api_key = self.config.api_key
            .as_ref()
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok().as_ref())
            .ok_or_else(|| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;
        
        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&json!({
                "model": self.config.model,
                "max_tokens": self.config.max_tokens,
                "temperature": self.config.temperature,
                "messages": [{
                    "role": "user",
                    "content": prompt
                }]
            }))
            .send()
            .await?;
        
        let body: serde_json::Value = response.json().await?;
        let content = body["content"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid response format"))?;
        
        Ok(content.to_string())
    }

    fn format_candidates(candidates: &[SearchResult]) -> String {
        candidates
            .iter()
            .map(|r| format!("- {} (score: {:.2})", r.memory.summary, r.score))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn parse_consolidation_decision(response: &str) -> Result<ConsolidationDecision> {
        // TODO: Implement parsing
        todo!("Parse consolidation decision from LLM response")
    }
}

#[derive(Debug, Deserialize)]
pub struct NoteMetadata {
    pub summary: String,
    pub keywords: Vec<String>,
    pub tags: Vec<String>,
    pub context: String,
    pub memory_type: MemoryType,
    pub importance: u8,
    pub confidence: f32,
    pub related_files: Vec<String>,
    pub related_entities: Vec<String>,
}
```

### 4.3 Embedding Service

```rust
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};

/// Embedding generation service
pub struct EmbeddingService {
    provider: EmbeddingProvider,
    model: Box<dyn EmbeddingBackend>,
}

impl EmbeddingService {
    pub async fn new(config: EmbeddingConfig) -> Result<Self> {
        let model: Box<dyn EmbeddingBackend> = match config.provider {
            EmbeddingProvider::Local => {
                let model = TextEmbedding::try_new(InitOptions {
                    model_name: EmbeddingModel::AllMiniLML6V2,
                    show_download_progress: true,
                    ..Default::default()
                })?;
                Box::new(LocalEmbeddings { model })
            }
            EmbeddingProvider::OpenAI => {
                Box::new(OpenAIEmbeddings::new(config.model)?)
            }
        };
        
        Ok(Self {
            provider: config.provider,
            model,
        })
    }

    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        self.model.embed(text).await
    }

    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        self.model.embed_batch(texts).await
    }
}

#[async_trait::async_trait]
trait EmbeddingBackend: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
}

struct LocalEmbeddings {
    model: TextEmbedding,
}

#[async_trait::async_trait]
impl EmbeddingBackend for LocalEmbeddings {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self.model.embed(vec![text], None)?;
        Ok(embeddings.into_iter().next().unwrap())
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        let embeddings = self.model.embed(text_refs, None)?;
        Ok(embeddings)
    }
}

struct OpenAIEmbeddings {
    client: reqwest::Client,
    model: String,
}

impl OpenAIEmbeddings {
    fn new(model: String) -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::new(),
            model,
        })
    }
}

#[async_trait::async_trait]
impl EmbeddingBackend for OpenAIEmbeddings {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // TODO: Implement OpenAI API call
        todo!("Implement OpenAI embeddings")
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        // TODO: Implement batch OpenAI API call
        todo!("Implement OpenAI batch embeddings")
    }
}
```

---

## 5. MCP Server Interface

### 5.1 MCP Protocol Implementation

```rust
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// MCP server handling JSON-RPC over stdio
pub struct McpServer {
    memory_manager: Arc<MemoryManager>,
}

impl McpServer {
    pub fn new(memory_manager: Arc<MemoryManager>) -> Self {
        Self { memory_manager }
    }

    pub async fn run(&self) -> Result<()> {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                break; // EOF
            }

            let request: Value = serde_json::from_str(&line)?;
            let response = self.handle_request(request).await?;
            
            let response_json = serde_json::to_string(&response)?;
            stdout.write_all(response_json.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }

        Ok(())
    }

    async fn handle_request(&self, request: Value) -> Result<Value> {
        let method = request["method"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing method"))?;
        
        match method {
            "initialize" => self.handle_initialize(request).await,
            "tools/list" => self.handle_list_tools(request).await,
            "tools/call" => self.handle_call_tool(request).await,
            _ => Ok(json!({
                "jsonrpc": "2.0",
                "id": request["id"],
                "error": {
                    "code": -32601,
                    "message": "Method not found"
                }
            })),
        }
    }

    async fn handle_initialize(&self, request: Value) -> Result<Value> {
        Ok(json!({
            "jsonrpc": "2.0",
            "id": request["id"],
            "result": {
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": "mnemosyne",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": {
                    "tools": {}
                }
            }
        }))
    }

    async fn handle_list_tools(&self, request: Value) -> Result<Value> {
        Ok(json!({
            "jsonrpc": "2.0",
            "id": request["id"],
            "result": {
                "tools": [
                    {
                        "name": "remember",
                        "description": "Store important information in long-term memory",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "content": {
                                    "type": "string",
                                    "description": "Information to remember"
                                },
                                "importance": {
                                    "type": "integer",
                                    "description": "Importance level (1-10)",
                                    "minimum": 1,
                                    "maximum": 10
                                },
                                "tags": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Optional tags for categorization"
                                }
                            },
                            "required": ["content"]
                        }
                    },
                    {
                        "name": "recall",
                        "description": "Search long-term memory",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": {
                                    "type": "string",
                                    "description": "Search query"
                                },
                                "max_results": {
                                    "type": "integer",
                                    "description": "Maximum results to return",
                                    "default": 5
                                },
                                "memory_types": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Filter by memory types"
                                }
                            },
                            "required": ["query"]
                        }
                    },
                    {
                        "name": "list_memories",
                        "description": "List all memories with optional filters",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "namespace": {
                                    "type": "string",
                                    "description": "Filter by namespace (global/project/session)"
                                },
                                "tags": {
                                    "type": "array",
                                    "items": { "type": "string" }
                                }
                            }
                        }
                    },
                    {
                        "name": "update_memory",
                        "description": "Update an existing memory",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "id": {
                                    "type": "string",
                                    "description": "Memory ID (UUID)"
                                },
                                "content": { "type": "string" },
                                "importance": { "type": "integer" },
                                "tags": {
                                    "type": "array",
                                    "items": { "type": "string" }
                                }
                            },
                            "required": ["id"]
                        }
                    },
                    {
                        "name": "delete_memory",
                        "description": "Delete (archive) a memory",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "id": {
                                    "type": "string",
                                    "description": "Memory ID (UUID)"
                                }
                            },
                            "required": ["id"]
                        }
                    },
                    {
                        "name": "consolidate_memories",
                        "description": "Consolidate redundant memories",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "namespace": {
                                    "type": "string",
                                    "description": "Namespace to consolidate (optional)"
                                }
                            }
                        }
                    },
                    {
                        "name": "switch_context",
                        "description": "Switch project or session context",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "project": { "type": "string" },
                                "session": { "type": "string" }
                            }
                        }
                    },
                    {
                        "name": "export_memories",
                        "description": "Export memories to Markdown",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "namespace": { "type": "string" },
                                "output_path": { "type": "string" }
                            },
                            "required": ["output_path"]
                        }
                    }
                ]
            }
        }))
    }

    async fn handle_call_tool(&self, request: Value) -> Result<Value> {
        let tool_name = request["params"]["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing tool name"))?;
        
        let arguments = &request["params"]["arguments"];
        
        let result = match tool_name {
            "remember" => self.tool_remember(arguments).await?,
            "recall" => self.tool_recall(arguments).await?,
            "list_memories" => self.tool_list_memories(arguments).await?,
            "update_memory" => self.tool_update_memory(arguments).await?,
            "delete_memory" => self.tool_delete_memory(arguments).await?,
            "consolidate_memories" => self.tool_consolidate(arguments).await?,
            "switch_context" => self.tool_switch_context(arguments).await?,
            "export_memories" => self.tool_export(arguments).await?,
            _ => return Ok(json!({
                "jsonrpc": "2.0",
                "id": request["id"],
                "error": {
                    "code": -32602,
                    "message": "Unknown tool"
                }
            })),
        };
        
        Ok(json!({
            "jsonrpc": "2.0",
            "id": request["id"],
            "result": {
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&result)?
                }]
            }
        }))
    }

    async fn tool_remember(&self, args: &Value) -> Result<Value> {
        let content = args["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing content"))?;
        
        let importance = args["importance"].as_u64().map(|v| v as u8);
        
        // Detect namespace from current context
        let namespace = self.detect_namespace().await?;
        
        let memory_id = self.memory_manager
            .remember(content.to_string(), namespace, importance)
            .await?;
        
        Ok(json!({
            "success": true,
            "memory_id": memory_id.0.to_string(),
            "message": "Memory stored successfully"
        }))
    }

    async fn tool_recall(&self, args: &Value) -> Result<Value> {
        let query_str = args["query"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing query"))?;
        
        let max_results = args["max_results"]
            .as_u64()
            .unwrap_or(5) as usize;
        
        let namespace = self.detect_namespace().await?;
        
        let query = SearchQuery {
            query: query_str.to_string(),
            namespace: Some(namespace),
            memory_types: vec![],
            tags: vec![],
            min_importance: None,
            max_results,
            include_archived: false,
        };
        
        let results = self.memory_manager.recall(query).await?;
        
        Ok(json!({
            "results": results,
            "count": results.len()
        }))
    }

    async fn tool_list_memories(&self, _args: &Value) -> Result<Value> {
        // TODO: Implement list memories
        todo!("Implement list_memories tool")
    }

    async fn tool_update_memory(&self, args: &Value) -> Result<Value> {
        // TODO: Implement update memory
        todo!("Implement update_memory tool")
    }

    async fn tool_delete_memory(&self, args: &Value) -> Result<Value> {
        // TODO: Implement delete memory
        todo!("Implement delete_memory tool")
    }

    async fn tool_consolidate(&self, args: &Value) -> Result<Value> {
        // TODO: Implement consolidation
        todo!("Implement consolidate_memories tool")
    }

    async fn tool_switch_context(&self, args: &Value) -> Result<Value> {
        // TODO: Implement context switching
        todo!("Implement switch_context tool")
    }

    async fn tool_export(&self, args: &Value) -> Result<Value> {
        // TODO: Implement export
        todo!("Implement export_memories tool")
    }

    async fn detect_namespace(&self) -> Result<Namespace> {
        // TODO: Detect namespace from environment
        // Check for git root, CLAUDE.md, current directory
        Ok(Namespace::Global)
    }
}
```

---

## 6. Storage Layer

### 6.1 Storage Backend

```rust
use sqlx::{SqlitePool, PgPool, Row};

/// Unified storage interface
pub struct StorageBackend {
    backend: StorageImpl,
}

enum StorageImpl {
    Sqlite(SqliteStorage),
    Postgres(PostgresStorage),
}

impl StorageBackend {
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        let backend = match config.backend {
            DatabaseBackend::Sqlite => {
                StorageImpl::Sqlite(SqliteStorage::new(&config.url).await?)
            }
            DatabaseBackend::Postgres => {
                StorageImpl::Postgres(PostgresStorage::new(&config.url).await?)
            }
        };
        
        Ok(Self { backend })
    }

    pub async fn store_memory(&self, memory: &MemoryNote) -> Result<()> {
        match &self.backend {
            StorageImpl::Sqlite(s) => s.store_memory(memory).await,
            StorageImpl::Postgres(s) => s.store_memory(memory).await,
        }
    }

    pub async fn get_memory(&self, id: MemoryId) -> Result<MemoryNote> {
        match &self.backend {
            StorageImpl::Sqlite(s) => s.get_memory(id).await,
            StorageImpl::Postgres(s) => s.get_memory(id).await,
        }
    }

    pub async fn update_memory(&self, memory: &MemoryNote) -> Result<()> {
        match &self.backend {
            StorageImpl::Sqlite(s) => s.update_memory(memory).await,
            StorageImpl::Postgres(s) => s.update_memory(memory).await,
        }
    }

    pub async fn archive_memory(&self, id: MemoryId) -> Result<()> {
        match &self.backend {
            StorageImpl::Sqlite(s) => s.archive_memory(id).await,
            StorageImpl::Postgres(s) => s.archive_memory(id).await,
        }
    }

    pub async fn vector_search(
        &self,
        embedding: &[f32],
        limit: usize,
        namespace: Option<Namespace>,
    ) -> Result<Vec<SearchResult>> {
        match &self.backend {
            StorageImpl::Sqlite(s) => s.vector_search(embedding, limit, namespace).await,
            StorageImpl::Postgres(s) => s.vector_search(embedding, limit, namespace).await,
        }
    }

    pub async fn keyword_search(
        &self,
        query: &str,
        namespace: Option<Namespace>,
    ) -> Result<Vec<SearchResult>> {
        match &self.backend {
            StorageImpl::Sqlite(s) => s.keyword_search(query, namespace).await,
            StorageImpl::Postgres(s) => s.keyword_search(query, namespace).await,
        }
    }

    pub async fn graph_traverse(
        &self,
        seed_ids: &[MemoryId],
        max_hops: usize,
    ) -> Result<Vec<MemoryNote>> {
        match &self.backend {
            StorageImpl::Sqlite(s) => s.graph_traverse(seed_ids, max_hops).await,
            StorageImpl::Postgres(s) => s.graph_traverse(seed_ids, max_hops).await,
        }
    }

    pub async fn find_consolidation_candidates(
        &self,
        namespace: Option<Namespace>,
    ) -> Result<Vec<(MemoryNote, MemoryNote)>> {
        match &self.backend {
            StorageImpl::Sqlite(s) => s.find_consolidation_candidates(namespace).await,
            StorageImpl::Postgres(s) => s.find_consolidation_candidates(namespace).await,
        }
    }

    pub async fn increment_access(&self, id: MemoryId) -> Result<()> {
        match &self.backend {
            StorageImpl::Sqlite(s) => s.increment_access(id).await,
            StorageImpl::Postgres(s) => s.increment_access(id).await,
        }
    }
}
```

### 6.2 SQLite Implementation

```rust
struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    async fn new(url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(url).await?;
        
        // Run migrations
        sqlx::migrate!("./migrations/sqlite").run(&pool).await?;
        
        // Load sqlite-vec extension
        sqlx::query("SELECT load_extension('vec0')")
            .execute(&pool)
            .await?;
        
        Ok(Self { pool })
    }

    async fn store_memory(&self, memory: &MemoryNote) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        
        // Insert memory metadata
        sqlx::query(
            r#"
            INSERT INTO memories (
                id, namespace, created_at, updated_at,
                content, summary, keywords, tags, context,
                memory_type, importance, confidence,
                related_files, related_entities,
                access_count, last_accessed_at, expires_at,
                is_archived, superseded_by, embedding_model
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(memory.id.0.to_string())
        .bind(serde_json::to_string(&memory.namespace)?)
        .bind(memory.created_at)
        .bind(memory.updated_at)
        .bind(&memory.content)
        .bind(&memory.summary)
        .bind(serde_json::to_string(&memory.keywords)?)
        .bind(serde_json::to_string(&memory.tags)?)
        .bind(&memory.context)
        .bind(serde_json::to_string(&memory.memory_type)?)
        .bind(memory.importance)
        .bind(memory.confidence)
        .bind(serde_json::to_string(&memory.related_files)?)
        .bind(serde_json::to_string(&memory.related_entities)?)
        .bind(memory.access_count)
        .bind(memory.last_accessed_at)
        .bind(memory.expires_at)
        .bind(memory.is_archived)
        .bind(memory.superseded_by.map(|id| id.0.to_string()))
        .bind(&memory.embedding_model)
        .execute(&mut *tx)
        .await?;
        
        // Insert embedding (using sqlite-vec)
        if let Some(embedding) = &memory.embedding {
            let embedding_blob = Self::serialize_f32_slice(embedding);
            sqlx::query(
                r#"
                INSERT INTO memory_embeddings (memory_id, embedding)
                VALUES (?, vec_f32(?))
                "#
            )
            .bind(memory.id.0.to_string())
            .bind(embedding_blob)
            .execute(&mut *tx)
            .await?;
        }
        
        // Insert links
        for link in &memory.links {
            sqlx::query(
                r#"
                INSERT INTO memory_links (
                    source_id, target_id, link_type, strength, reason, created_at
                ) VALUES (?, ?, ?, ?, ?, ?)
                "#
            )
            .bind(memory.id.0.to_string())
            .bind(link.target_id.0.to_string())
            .bind(serde_json::to_string(&link.link_type)?)
            .bind(link.strength)
            .bind(&link.reason)
            .bind(link.created_at)
            .execute(&mut *tx)
            .await?;
        }
        
        tx.commit().await?;
        Ok(())
    }

    async fn get_memory(&self, id: MemoryId) -> Result<MemoryNote> {
        let row = sqlx::query(
            r#"
            SELECT * FROM memories WHERE id = ?
            "#
        )
        .bind(id.0.to_string())
        .fetch_one(&self.pool)
        .await?;
        
        Self::row_to_memory(row).await
    }

    async fn vector_search(
        &self,
        embedding: &[f32],
        limit: usize,
        namespace: Option<Namespace>,
    ) -> Result<Vec<SearchResult>> {
        let embedding_blob = Self::serialize_f32_slice(embedding);
        
        let query = if let Some(ns) = namespace {
            sqlx::query(
                r#"
                SELECT m.*, vec_distance_cosine(e.embedding, vec_f32(?)) as distance
                FROM memories m
                JOIN memory_embeddings e ON m.id = e.memory_id
                WHERE m.namespace = ? AND m.is_archived = 0
                ORDER BY distance ASC
                LIMIT ?
                "#
            )
            .bind(embedding_blob)
            .bind(serde_json::to_string(&ns)?)
            .bind(limit as i64)
        } else {
            sqlx::query(
                r#"
                SELECT m.*, vec_distance_cosine(e.embedding, vec_f32(?)) as distance
                FROM memories m
                JOIN memory_embeddings e ON m.id = e.memory_id
                WHERE m.is_archived = 0
                ORDER BY distance ASC
                LIMIT ?
                "#
            )
            .bind(embedding_blob)
            .bind(limit as i64)
        };
        
        let rows = query.fetch_all(&self.pool).await?;
        
        let mut results = Vec::new();
        for row in rows {
            let memory = Self::row_to_memory(row).await?;
            let distance: f32 = row.try_get("distance")?;
            let score = 1.0 - distance; // Convert distance to similarity
            
            results.push(SearchResult {
                memory,
                score,
                match_reason: "vector_similarity".to_string(),
            });
        }
        
        Ok(results)
    }

    async fn keyword_search(
        &self,
        query: &str,
        namespace: Option<Namespace>,
    ) -> Result<Vec<SearchResult>> {
        // TODO: Implement FTS5 keyword search
        todo!("Implement SQLite FTS5 keyword search")
    }

    async fn graph_traverse(
        &self,
        seed_ids: &[MemoryId],
        max_hops: usize,
    ) -> Result<Vec<MemoryNote>> {
        // TODO: Implement recursive CTE for graph traversal
        todo!("Implement SQLite recursive graph traversal")
    }

    async fn find_consolidation_candidates(
        &self,
        namespace: Option<Namespace>,
    ) -> Result<Vec<(MemoryNote, MemoryNote)>> {
        // TODO: Find similar memories for consolidation
        todo!("Implement consolidation candidate search")
    }

    async fn increment_access(&self, id: MemoryId) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE memories
            SET access_count = access_count + 1,
                last_accessed_at = ?
            WHERE id = ?
            "#
        )
        .bind(Utc::now())
        .bind(id.0.to_string())
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    async fn update_memory(&self, memory: &MemoryNote) -> Result<()> {
        // TODO: Implement memory update
        todo!("Implement memory update")
    }

    async fn archive_memory(&self, id: MemoryId) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE memories SET is_archived = 1 WHERE id = ?
            "#
        )
        .bind(id.0.to_string())
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    // Helper methods
    
    fn serialize_f32_slice(slice: &[f32]) -> Vec<u8> {
        slice.iter()
            .flat_map(|f| f.to_le_bytes())
            .collect()
    }

    async fn row_to_memory(row: sqlx::sqlite::SqliteRow) -> Result<MemoryNote> {
        // TODO: Implement row to MemoryNote conversion
        todo!("Implement SQLite row to MemoryNote conversion")
    }
}
```

### 6.3 Database Migrations

```sql
-- migrations/sqlite/001_initial_schema.sql

-- Memories table
CREATE TABLE IF NOT EXISTS memories (
    id TEXT PRIMARY KEY,
    namespace TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    
    content TEXT NOT NULL,
    summary TEXT NOT NULL,
    keywords TEXT NOT NULL,  -- JSON array
    tags TEXT NOT NULL,      -- JSON array
    context TEXT NOT NULL,
    
    memory_type TEXT NOT NULL,
    importance INTEGER NOT NULL CHECK(importance BETWEEN 1 AND 10),
    confidence REAL NOT NULL CHECK(confidence BETWEEN 0 AND 1),
    
    related_files TEXT NOT NULL,    -- JSON array
    related_entities TEXT NOT NULL, -- JSON array
    
    access_count INTEGER NOT NULL DEFAULT 0,
    last_accessed_at TIMESTAMP NOT NULL,
    expires_at TIMESTAMP,
    is_archived INTEGER NOT NULL DEFAULT 0,
    superseded_by TEXT,
    
    embedding_model TEXT NOT NULL,
    
    FOREIGN KEY (superseded_by) REFERENCES memories(id)
);

CREATE INDEX idx_memories_namespace ON memories(namespace);
CREATE INDEX idx_memories_created_at ON memories(created_at);
CREATE INDEX idx_memories_memory_type ON memories(memory_type);
CREATE INDEX idx_memories_importance ON memories(importance);
CREATE INDEX idx_memories_is_archived ON memories(is_archived);

-- Memory embeddings (using sqlite-vec)
CREATE VIRTUAL TABLE IF NOT EXISTS memory_embeddings USING vec0(
    memory_id TEXT PRIMARY KEY,
    embedding FLOAT[384]  -- Dimension depends on model
);

-- Memory links (knowledge graph)
CREATE TABLE IF NOT EXISTS memory_links (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    link_type TEXT NOT NULL,
    strength REAL NOT NULL CHECK(strength BETWEEN 0 AND 1),
    reason TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    
    FOREIGN KEY (source_id) REFERENCES memories(id),
    FOREIGN KEY (target_id) REFERENCES memories(id),
    UNIQUE (source_id, target_id, link_type)
);

CREATE INDEX idx_links_source ON memory_links(source_id);
CREATE INDEX idx_links_target ON memory_links(target_id);
CREATE INDEX idx_links_type ON memory_links(link_type);

-- Audit log (immutable)
CREATE TABLE IF NOT EXISTS audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TIMESTAMP NOT NULL,
    operation TEXT NOT NULL,
    memory_id TEXT,
    details TEXT NOT NULL,  -- JSON
    
    FOREIGN KEY (memory_id) REFERENCES memories(id)
);

CREATE INDEX idx_audit_timestamp ON audit_log(timestamp);
CREATE INDEX idx_audit_memory_id ON audit_log(memory_id);

-- Full-text search
CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
    id UNINDEXED,
    content,
    summary,
    keywords,
    tags
);
```

---

## 7. Memory Operations

### 7.1 Memory Lifecycle

```
[Create] → [Active] → [Consolidate] → [Archive] → [Export]
              ↓           ↓
           [Update]    [Supersede]
              ↓           ↓
           [Evolve]    [Merge]
```

### 7.2 Background Tasks

```rust
use tokio::time::{interval, Duration};

pub struct BackgroundTaskManager {
    storage: Arc<StorageBackend>,
    llm_service: Arc<LlmService>,
    config: LifecycleConfig,
}

impl BackgroundTaskManager {
    pub fn new(
        storage: Arc<StorageBackend>,
        llm_service: Arc<LlmService>,
        config: LifecycleConfig,
    ) -> Self {
        Self {
            storage,
            llm_service,
            config,
        }
    }

    pub async fn start(&self) {
        // Spawn background tasks
        let storage = Arc::clone(&self.storage);
        let config = self.config.clone();
        
        tokio::spawn(async move {
            Self::consolidation_task(storage, config).await;
        });
        
        let storage = Arc::clone(&self.storage);
        let config = self.config.clone();
        
        tokio::spawn(async move {
            Self::archival_task(storage, config).await;
        });
    }

    async fn consolidation_task(
        storage: Arc<StorageBackend>,
        config: LifecycleConfig,
    ) {
        let mut interval = interval(Duration::from_secs(
            config.consolidation_interval_hours as u64 * 3600
        ));
        
        loop {
            interval.tick().await;
            
            tracing::info!("Running consolidation task");
            
            // TODO: Run consolidation
        }
    }

    async fn archival_task(
        storage: Arc<StorageBackend>,
        config: LifecycleConfig,
    ) {
        let mut interval = interval(Duration::from_secs(86400)); // Daily
        
        loop {
            interval.tick().await;
            
            tracing::info!("Running archival task");
            
            // TODO: Archive old, low-importance memories
        }
    }

    pub async fn schedule_evolution(&self, memory_id: MemoryId) -> Result<()> {
        // TODO: Schedule memory evolution task
        todo!("Implement evolution scheduling")
    }
}
```

---

## 8. Project-Aware Context Management

### 8.1 Namespace Detection

```rust
use std::path::{Path, PathBuf};

pub struct ContextDetector;

impl ContextDetector {
    /// Detect current project and namespace
    pub async fn detect() -> Result<Namespace> {
        let cwd = std::env::current_dir()?;
        
        // Try to find git root
        if let Some(git_root) = Self::find_git_root(&cwd).await? {
            let project_name = git_root
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid project name"))?
                .to_string();
            
            // Check if in a session (temporary work)
            if Self::is_session_context(&cwd).await? {
                let session_id = Self::get_session_id().await?;
                return Ok(Namespace::Session {
                    project: project_name,
                    session_id,
                });
            }
            
            return Ok(Namespace::Project { name: project_name });
        }
        
        // Fallback to global
        Ok(Namespace::Global)
    }

    async fn find_git_root(path: &Path) -> Result<Option<PathBuf>> {
        let mut current = path.to_path_buf();
        
        loop {
            let git_dir = current.join(".git");
            if git_dir.exists() {
                return Ok(Some(current));
            }
            
            if !current.pop() {
                break;
            }
        }
        
        Ok(None)
    }

    async fn is_session_context(path: &Path) -> Result<bool> {
        // Check for session markers
        Ok(std::env::var("MNEMOSYNE_SESSION").is_ok())
    }

    async fn get_session_id() -> Result<String> {
        Ok(std::env::var("MNEMOSYNE_SESSION")
            .unwrap_or_else(|_| Uuid::new_v4().to_string()))
    }

    /// Check if global memory access is permitted
    pub fn global_access_permitted() -> bool {
        std::env::var("MNEMOSYNE_GLOBAL").is_ok()
    }
}
```

### 8.2 Namespace Priority

```rust
impl MemoryManager {
    /// Search with namespace priority (session > project > global)
    async fn prioritized_search(&self, query: &str) -> Result<Vec<SearchResult>> {
        let namespace = ContextDetector::detect().await?;
        
        // Start with current namespace
        let mut results = self.search_namespace(query, namespace.clone()).await?;
        
        // If insufficient results, expand search
        if results.len() < 5 {
            match namespace {
                Namespace::Session { project, .. } => {
                    // Add project-level results
                    let project_ns = Namespace::Project { name: project.clone() };
                    let mut project_results = self.search_namespace(query, project_ns).await?;
                    results.append(&mut project_results);
                    
                    // Add global if permitted
                    if ContextDetector::global_access_permitted() {
                        let mut global_results = self.search_namespace(
                            query,
                            Namespace::Global
                        ).await?;
                        results.append(&mut global_results);
                    }
                }
                Namespace::Project { .. } => {
                    if ContextDetector::global_access_permitted() {
                        let mut global_results = self.search_namespace(
                            query,
                            Namespace::Global
                        ).await?;
                        results.append(&mut global_results);
                    }
                }
                Namespace::Global => {
                    // Already at global, no expansion
                }
            }
        }
        
        // Re-rank combined results
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(10);
        
        Ok(results)
    }

    async fn search_namespace(
        &self,
        query: &str,
        namespace: Namespace,
    ) -> Result<Vec<SearchResult>> {
        let search_query = SearchQuery {
            query: query.to_string(),
            namespace: Some(namespace),
            memory_types: vec![],
            tags: vec![],
            min_importance: None,
            max_results: 10,
            include_archived: false,
        };
        
        self.recall(search_query).await
    }
}
```

---

## 9. Installation & Configuration

### 9.1 Installation Script

```bash
#!/bin/bash
# install-mnemosyne.sh

set -e

echo "🧠 Installing Mnemosyne..."

# Detect project root
if [ -d ".git" ]; then
    PROJECT_ROOT=$(pwd)
    echo "✓ Detected git repository: $PROJECT_ROOT"
else
    echo "⚠ No git repository found. Installing globally..."
    PROJECT_ROOT="$HOME"
fi

# Create directory structure
INSTALL_DIR="$PROJECT_ROOT/.claude/mcp-servers/mnemosyne"
mkdir -p "$INSTALL_DIR"

echo "📦 Building mnemosyne..."
cd "$(dirname "$0")"
cargo build --release

echo "📋 Installing binary..."
cp target/release/mnemosyne "$INSTALL_DIR/"

echo "⚙️ Creating configuration..."
cat > "$INSTALL_DIR/config.toml" <<EOF
[database]
backend = "sqlite"
url = "sqlite:$INSTALL_DIR/mnemosyne.db"
max_connections = 10

[llm]
model = "claude-haiku-4-20250514"
max_tokens = 4096
temperature = 0.0

[embeddings]
provider = "local"
model = "all-minilm-l6-v2"
dimension = 384

[lifecycle]
archival_threshold_days = 90
archival_min_importance = 2.0
consolidation_interval_hours = 24
max_memories_per_namespace = 10000

[server]
log_level = "info"
enable_metrics = true
EOF

echo "🗄️ Initializing database..."
cd "$INSTALL_DIR"
./mnemosyne init

echo "🔧 Configuring MCP..."
if [ -f "$PROJECT_ROOT/.claude/mcp.json" ]; then
    # Append to existing config
    echo "  Updating existing MCP configuration..."
else
    # Create new config
    cat > "$PROJECT_ROOT/.claude/mcp.json" <<EOF
{
  "mcpServers": {
    "mnemosyne": {
      "command": "$INSTALL_DIR/mnemosyne",
      "args": ["serve"],
      "env": {
        "MNEMOSYNE_CONFIG": "$INSTALL_DIR/config.toml"
      }
    }
  }
}
EOF
fi

echo "📚 Installing skills..."
mkdir -p "$PROJECT_ROOT/.claude/skills/memory-management"
cat > "$PROJECT_ROOT/.claude/skills/memory-management/SKILL.md" <<'EOF'
# Memory Management Skill

## When to Use Memory

### Always Remember
- Architecture decisions and rationale
- Project-specific patterns and conventions
- Bug fixes and their root causes
- Important constraints and invariants
- Configuration and setup steps

### Never Remember
- Temporary debugging output
- Sensitive information (keys, passwords)
- Implementation details (code remembers itself)
- Obvious information already in documentation

## How to Use Memory

### Creating Memories
Use the `remember` tool with clear, concise content:
- Set importance (1-10) based on long-term value
- Add tags for categorization
- Include context about when/why this matters

### Retrieving Memories
Use the `recall` tool before starting work:
- Search for relevant context
- Review retrieved memories
- Build on existing knowledge

### Memory Lifecycle
- Update memories when information changes
- Consolidate redundant memories periodically
- Archive outdated information
EOF

echo "📝 Updating .gitignore..."
cat >> "$PROJECT_ROOT/.gitignore" <<EOF

# Mnemosyne
.claude/mcp-servers/mnemosyne/mnemosyne.db*
.claude/memory/sessions/
EOF

echo "✨ Installation complete!"
echo ""
echo "Next steps:"
echo "1. Restart Claude Code to load the MCP server"
echo "2. Test with: \"Remember that this project uses Rust\""
echo "3. Verify with: \"Recall what you know about this project\""
echo ""
echo "Configuration: $INSTALL_DIR/config.toml"
echo "Database: $INSTALL_DIR/mnemosyne.db"
```

### 9.2 Uninstallation Script

```bash
#!/bin/bash
# uninstall-mnemosyne.sh

set -e

echo "🗑️ Uninstalling Mnemosyne..."

# Detect project root
if [ -d ".git" ]; then
    PROJECT_ROOT=$(pwd)
else
    PROJECT_ROOT="$HOME"
fi

INSTALL_DIR="$PROJECT_ROOT/.claude/mcp-servers/mnemosyne"

# Optional: Backup before removal
read -p "Backup memory data before uninstalling? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    BACKUP_DIR="$HOME/.mnemosyne-backup-$(date +%Y%m%d_%H%M%S)"
    mkdir -p "$BACKUP_DIR"
    cp -r "$INSTALL_DIR" "$BACKUP_DIR/"
    echo "✓ Backup saved to: $BACKUP_DIR"
fi

# Remove installation
rm -rf "$INSTALL_DIR"
echo "✓ Removed installation directory"

# Remove MCP configuration
if [ -f "$PROJECT_ROOT/.claude/mcp.json" ]; then
    # TODO: Remove mnemosyne entry from JSON
    echo "⚠ Please manually remove mnemosyne from .claude/mcp.json"
fi

# Remove skill
rm -rf "$PROJECT_ROOT/.claude/skills/memory-management"
echo "✓ Removed memory management skill"

echo "✨ Uninstallation complete!"
```

---

## 10. Testing Strategy

### 10.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_creation() {
        let config = test_config();
        let manager = MemoryManager::new(config).await.unwrap();
        
        let id = manager.remember(
            "Test memory content".to_string(),
            Namespace::Global,
            Some(5),
        ).await.unwrap();
        
        assert!(id.0.as_bytes().len() > 0);
    }

    #[tokio::test]
    async fn test_memory_retrieval() {
        let config = test_config();
        let manager = MemoryManager::new(config).await.unwrap();
        
        // Create memory
        let content = "Rust uses ownership for memory safety";
        manager.remember(
            content.to_string(),
            Namespace::Global,
            Some(7),
        ).await.unwrap();
        
        // Retrieve
        let query = SearchQuery {
            query: "memory safety".to_string(),
            namespace: Some(Namespace::Global),
            memory_types: vec![],
            tags: vec![],
            min_importance: None,
            max_results: 5,
            include_archived: false,
        };
        
        let results = manager.recall(query).await.unwrap();
        assert!(results.len() > 0);
        assert!(results[0].memory.content.contains("ownership"));
    }

    #[test]
    fn test_cosine_similarity() {
        let a = Embedding([1.0, 0.0, 0.0, 0.0]);
        let b = Embedding([1.0, 0.0, 0.0, 0.0]);
        let c = Embedding([0.0, 1.0, 0.0, 0.0]);
        
        assert!((a.cosine_similarity(&b) - 1.0).abs() < 0.001);
        assert!(a.cosine_similarity(&c).abs() < 0.001);
    }

    #[test]
    fn test_namespace_priority() {
        let global = Namespace::Global;
        let project = Namespace::Project {
            name: "test".to_string()
        };
        let session = Namespace::Session {
            project: "test".to_string(),
            session_id: "abc".to_string()
        };
        
        assert!(session.priority() > project.priority());
        assert!(project.priority() > global.priority());
    }

    fn test_config() -> MnemosyneConfig {
        // TODO: Return test configuration
        todo!()
    }
}
```

### 10.2 Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_end_to_end_memory_flow() {
        let temp_dir = TempDir::new().unwrap();
        let db_url = format!("sqlite:{}/test.db", temp_dir.path().display());
        
        let config = MnemosyneConfig {
            database: DatabaseConfig {
                backend: DatabaseBackend::Sqlite,
                url: db_url,
                max_connections: 5,
            },
            // ... rest of config
        };
        
        let manager = MemoryManager::new(config).await.unwrap();
        
        // Test complete workflow
        // 1. Create memory
        let id = manager.remember(
            "Integration test memory".to_string(),
            Namespace::Global,
            Some(8),
        ).await.unwrap();
        
        // 2. Retrieve memory
        let query = SearchQuery {
            query: "integration test".to_string(),
            namespace: Some(Namespace::Global),
            memory_types: vec![],
            tags: vec![],
            min_importance: None,
            max_results: 5,
            include_archived: false,
        };
        
        let results = manager.recall(query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].memory.id, id);
        
        // 3. Update memory
        let updates = MemoryUpdates {
            importance: Some(9),
            ..Default::default()
        };
        manager.update_memory(id, updates).await.unwrap();
        
        // 4. Verify update
        let memory = manager.storage.get_memory(id).await.unwrap();
        assert_eq!(memory.importance, 9);
        
        // 5. Archive memory
        manager.delete_memory(id).await.unwrap();
        let memory = manager.storage.get_memory(id).await.unwrap();
        assert!(memory.is_archived);
    }
}
```

### 10.3 Property Tests

```rust
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_decayed_importance_bounds(
            importance in 1u8..=10,
            access_count in 0u32..1000,
        ) {
            let memory = MemoryNote {
                importance,
                access_count,
                created_at: Utc::now() - chrono::Duration::days(30),
                updated_at: Utc::now(),
                ..Default::default()
            };
            
            let decayed = memory.decayed_importance();
            assert!(decayed >= 0.0);
            assert!(decayed <= importance as f32 * 2.0);
        }

        #[test]
        fn test_embedding_similarity_properties(
            vec_a in prop::array::uniform4(0.0f32..1.0f32),
            vec_b in prop::array::uniform4(0.0f32..1.0f32),
        ) {
            let a = Embedding(vec_a);
            let b = Embedding(vec_b);
            
            let sim = a.cosine_similarity(&b);
            
            // Similarity should be in [-1, 1]
            assert!(sim >= -1.0);
            assert!(sim <= 1.0);
            
            // Should be commutative
            assert!((sim - b.cosine_similarity(&a)).abs() < 0.001);
            
            // Self-similarity should be 1.0
            assert!((a.cosine_similarity(&a) - 1.0).abs() < 0.001);
        }
    }
}
```

### 10.4 Performance Benchmarks

```rust
#[cfg(test)]
mod benches {
    use super::*;
    use criterion::{black_box, criterion_group, criterion_main, Criterion};

    fn benchmark_embedding_similarity(c: &mut Criterion) {
        let a = Embedding([1.0f32; 384]);
        let b = Embedding([0.9f32; 384]);
        
        c.bench_function("cosine_similarity", |bencher| {
            bencher.iter(|| {
                black_box(a.cosine_similarity(&b))
            });
        });
    }

    fn benchmark_memory_storage(c: &mut Criterion) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let config = test_config();
        let manager = rt.block_on(MemoryManager::new(config)).unwrap();
        
        c.bench_function("store_memory", |bencher| {
            bencher.iter(|| {
                rt.block_on(async {
                    manager.remember(
                        "Benchmark memory".to_string(),
                        Namespace::Global,
                        Some(5),
                    ).await.unwrap()
                })
            });
        });
    }

    criterion_group!(benches, benchmark_embedding_similarity, benchmark_memory_storage);
    criterion_main!(benches);
}
```

---

## 11. Implementation Roadmap

### Phase 0: Foundation (Week 1-2)
**Goal**: Basic working system with core functionality

Tasks:
- [ ] Set up Rust project structure with `cargo new`
- [ ] Define all data structures in `src/types.rs`
- [ ] Implement SQLite storage backend
- [ ] Create database migrations
- [ ] Implement basic embedding service (local only)
- [ ] Write unit tests for core types
- [ ] Create installation script

**Exit Criteria**:
- Can create and store memories
- Can retrieve memories by ID
- All unit tests passing
- Installation script works

**Estimated Effort**: 40-60 hours

### Phase 1: MCP Integration (Week 3-4)
**Goal**: Full MCP server with all tools

Tasks:
- [ ] Implement MCP protocol handler
- [ ] Implement all 8 MCP tools
- [ ] Add namespace detection
- [ ] Implement vector search
- [ ] Add keyword search with FTS5
- [ ] Write integration tests
- [ ] Create memory management skill

**Exit Criteria**:
- MCP server responds to all tool calls
- Can remember and recall through Claude Code
- Namespace detection working
- Integration tests passing

**Estimated Effort**: 60-80 hours

### Phase 2: Intelligence Layer (Week 5-6)
**Goal**: LLM-powered note construction and linking

Tasks:
- [ ] Implement LLM service with Anthropic API
- [ ] Add automatic note construction
- [ ] Implement semantic link generation
- [ ] Add memory evolution (backward propagation)
- [ ] Implement consolidation logic
- [ ] Add background task manager
- [ ] Write property tests

**Exit Criteria**:
- Automatic note metadata extraction
- Semantic links created automatically
- Consolidation removes duplicates
- Background tasks running

**Estimated Effort**: 50-70 hours

### Phase 3: Graph & Advanced Search (Week 7-8)
**Goal**: Full knowledge graph and multi-strategy retrieval

Tasks:
- [ ] Implement graph traversal
- [ ] Add reciprocal rank fusion
- [ ] Implement importance decay algorithm
- [ ] Add archival task
- [ ] Implement memory evolution task
- [ ] Add metrics and observability
- [ ] Performance optimization

**Exit Criteria**:
- Graph traversal working
- Multi-strategy search accurate
- Lifecycle management automatic
- <200ms retrieval latency

**Estimated Effort**: 60-80 hours

### Phase 4: Polish & Documentation (Week 9-10)
**Goal**: Production-ready release

Tasks:
- [ ] Write comprehensive documentation
- [ ] Add PostgreSQL support
- [ ] Implement export functionality
- [ ] Add migration tools
- [ ] Security audit (secret detection, PII filtering)
- [ ] Performance benchmarks
- [ ] Create example workflows
- [ ] Package for distribution

**Exit Criteria**:
- Documentation complete
- All features working
- Security validated
- Performance targets met
- Ready for release

**Estimated Effort**: 40-60 hours

**Total Timeline**: 10 weeks, 250-350 hours

---

## 12. Performance Requirements

### 12.1 Latency Targets

| Operation | Target | Acceptable | Unacceptable |
|-----------|--------|------------|--------------|
| Memory creation | <1s | <2s | >3s |
| Vector search | <100ms | <200ms | >500ms |
| Keyword search | <50ms | <100ms | >200ms |
| Graph traversal | <150ms | <300ms | >500ms |
| Full recall | <200ms | <400ms | >1s |

### 12.2 Throughput Targets

- **Concurrent requests**: 10+ simultaneous
- **Memories per second**: 100+ writes, 1000+ reads
- **Database size**: 10,000+ memories without degradation

### 12.3 Resource Limits

- **Memory footprint**: <50MB baseline, <200MB under load
- **Disk space**: <100MB for 10,000 memories
- **CPU**: <5% baseline, <50% under load

### 12.4 Accuracy Targets

- **Vector search recall@10**: >70%
- **Multi-strategy fusion recall@10**: >80%
- **Consolidation precision**: >90%

---

## 13. Integration Points

### 13.1 CLAUDE.md Integration

Add to project's `CLAUDE.md`:

```markdown
## Mnemosyne Memory System

This project uses mnemosyne for persistent, project-aware memory.

**Before Each Task**:
1. Use `recall` tool to load relevant context
2. Review retrieved memories
3. Proceed with full project knowledge

**After Significant Work**:
1. Use `remember` tool to capture key learnings
2. Tag appropriately for future retrieval
3. Set importance based on long-term value

**Memory Philosophy**:
- Mnemosyne stores evolving project knowledge
- CLAUDE.md contains unchanging project rules
- Stabilized information promotes to CLAUDE.md
```

### 13.2 Slash Commands

Create `.claude/commands/`:

**`remember.md`**:
```markdown
# Quick Memory Capture

Quickly save the current context to memory.

Usage: /remember [importance] [content]

Example: /remember 8 This project uses actix-web for HTTP server
```

**`context.md`**:
```markdown
# Load Project Context

Load relevant memories before starting work.

Usage: /context [query]

Example: /context authentication flow
```

### 13.3 Skills System

The memory management skill at `.claude/skills/memory-management/SKILL.md` should be loaded automatically by Claude Code when working in projects with mnemosyne installed.

---

## 14. Typed Holes & Extension Points

### 14.1 Typed Holes for Incremental Development

```rust
// Storage layer typed holes
impl StorageBackend {
    pub async fn keyword_search(
        &self,
        query: &str,
        namespace: Option<Namespace>,
    ) -> Result<Vec<SearchResult>> {
        todo!("HOLE: Implement FTS5 keyword search with namespace filtering")
    }

    pub async fn graph_traverse(
        &self,
        seed_ids: &[MemoryId],
        max_hops: usize,
    ) -> Result<Vec<MemoryNote>> {
        todo!("HOLE: Implement recursive CTE for graph traversal up to N hops")
    }
}

// Result fusion typed hole
impl MemoryManager {
    fn fuse_search_results(
        &self,
        vector: Vec<SearchResult>,
        keyword: Vec<SearchResult>,
        graph: Vec<MemoryNote>,
    ) -> Result<Vec<SearchResult>> {
        todo!("HOLE: Implement reciprocal rank fusion algorithm")
    }
}

// OpenAI embeddings typed hole
impl OpenAIEmbeddings {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        todo!("HOLE: Implement OpenAI embedding API call with retry logic")
    }
}
```

### 14.2 Extension Points

**Custom Embedding Providers**:
```rust
trait EmbeddingBackend {
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
}

// Add new providers by implementing this trait
```

**Custom Storage Backends**:
```rust
trait StorageBackend {
    async fn store_memory(&self, memory: &MemoryNote) -> Result<()>;
    async fn get_memory(&self, id: MemoryId) -> Result<MemoryNote>;
    // ... other methods
}

// Add Redis, DynamoDB, etc. by implementing this trait
```

**Custom Link Types**:
```rust
// Extend LinkType enum:
pub enum LinkType {
    // ... existing types
    Custom(String),  // User-defined link types
}
```

---

## 15. Security & Privacy

### 15.1 Secret Detection

```rust
use regex::Regex;

pub struct SecretDetector {
    patterns: Vec<Regex>,
}

impl SecretDetector {
    pub fn new() -> Self {
        Self {
            patterns: vec![
                Regex::new(r"(?i)api[_-]?key[\s:=]+['\"]?([a-zA-Z0-9_-]{20,})").unwrap(),
                Regex::new(r"(?i)password[\s:=]+['\"]?([^\s'\"]+)").unwrap(),
                Regex::new(r"(?i)token[\s:=]+['\"]?([a-zA-Z0-9_-]{20,})").unwrap(),
                // Add more patterns
            ],
        }
    }

    pub fn scan(&self, text: &str) -> Vec<SecretMatch> {
        let mut matches = Vec::new();
        
        for pattern in &self.patterns {
            for capture in pattern.captures_iter(text) {
                matches.push(SecretMatch {
                    pattern: pattern.as_str().to_string(),
                    match_text: capture.get(0).unwrap().as_str().to_string(),
                    position: capture.get(0).unwrap().start(),
                });
            }
        }
        
        matches
    }
}

pub struct SecretMatch {
    pub pattern: String,
    pub match_text: String,
    pub position: usize,
}
```

### 15.2 PII Filtering

```rust
pub struct PiiFilter;

impl PiiFilter {
    pub fn redact(text: &str) -> String {
        let mut redacted = text.to_string();
        
        // Email addresses
        let email_regex = Regex::new(r"[\w\.-]+@[\w\.-]+\.\w+").unwrap();
        redacted = email_regex.replace_all(&redacted, "[EMAIL]").to_string();
        
        // Phone numbers (US format)
        let phone_regex = Regex::new(r"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b").unwrap();
        redacted = phone_regex.replace_all(&redacted, "[PHONE]").to_string();
        
        // SSN (US format)
        let ssn_regex = Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap();
        redacted = ssn_regex.replace_all(&redacted, "[SSN]").to_string();
        
        redacted
    }
}
```

### 15.3 Security Guidelines

**Data Protection**:
- Never store secrets or credentials in memory
- Automatically redact PII before storage
- Use encrypted connections for LLM API calls
- Implement access controls for sensitive projects

**Privacy**:
- Memory stays on local machine (no cloud sync)
- User controls what goes into global vs. project memory
- Clear data export and deletion capabilities
- Audit log tracks all memory operations

---

## Appendix A: Configuration Reference

### Complete Configuration File

```toml
# ~/.config/mnemosyne/config.toml

[database]
# Backend: "sqlite" or "postgres"
backend = "sqlite"
url = "sqlite:~/.claude/mcp-servers/mnemosyne/mnemosyne.db"
max_connections = 10

[llm]
# API key (optional, reads from ANTHROPIC_API_KEY env var)
api_key = ""
model = "claude-haiku-4-20250514"
max_tokens = 4096
temperature = 0.0

[embeddings]
# Provider: "local" or "openai"
provider = "local"
model = "all-minilm-l6-v2"
dimension = 384

[lifecycle]
# Archive memories older than N days with low importance
archival_threshold_days = 90
archival_min_importance = 2.0

# Run consolidation every N hours
consolidation_interval_hours = 24

# Maximum memories per namespace
max_memories_per_namespace = 10000

[server]
log_level = "info"  # debug, info, warn, error
enable_metrics = true

[security]
scan_for_secrets = true
redact_pii = true
```

---

## Appendix B: File Structure

```
mnemosyne/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── LICENSE
├── install.sh
├── uninstall.sh
├── config.toml.example
│
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library root
│   │
│   ├── types.rs             # Core data structures
│   ├── config.rs            # Configuration loading
│   ├── error.rs             # Error types
│   │
│   ├── mcp/
│   │   ├── mod.rs
│   │   ├── server.rs        # MCP server implementation
│   │   └── protocol.rs      # JSON-RPC handling
│   │
│   ├── memory/
│   │   ├── mod.rs
│   │   ├── manager.rs       # MemoryManager
│   │   ├── lifecycle.rs     # Lifecycle management
│   │   └── namespace.rs     # Namespace detection
│   │
│   ├── llm/
│   │   ├── mod.rs
│   │   ├── service.rs       # LLM service
│   │   └── prompts.rs       # Prompt templates
│   │
│   ├── embeddings/
│   │   ├── mod.rs
│   │   ├── service.rs       # Embedding service
│   │   ├── local.rs         # Local embeddings
│   │   └── openai.rs        # OpenAI embeddings
│   │
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── backend.rs       # StorageBackend trait
│   │   ├── sqlite.rs        # SQLite implementation
│   │   └── postgres.rs      # PostgreSQL implementation
│   │
│   ├── background/
│   │   ├── mod.rs
│   │   ├── tasks.rs         # Background task manager
│   │   ├── consolidation.rs
│   │   ├── archival.rs
│   │   └── evolution.rs
│   │
│   └── security/
│       ├── mod.rs
│       ├── secrets.rs       # Secret detection
│       └── pii.rs           # PII filtering
│
├── migrations/
│   ├── sqlite/
│   │   ├── 001_initial_schema.sql
│   │   ├── 002_add_indexes.sql
│   │   └── 003_add_fts.sql
│   └── postgres/
│       ├── 001_initial_schema.sql
│       └── 002_add_indexes.sql
│
├── tests/
│   ├── integration_tests.rs
│   ├── property_tests.rs
│   └── fixtures/
│
├── benches/
│   └── benchmarks.rs
│
└── docs/
    ├── architecture.md
    ├── api.md
    └── development.md
```

---

## Appendix C: Development Commands

```bash
# Build
cargo build
cargo build --release

# Test
cargo test
cargo test -- --nocapture  # Show output
cargo test test_name       # Run specific test

# Benchmark
cargo bench

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt

# Documentation
cargo doc --open

# Watch mode (requires cargo-watch)
cargo watch -x test
cargo watch -x run

# Run
cargo run -- serve
cargo run -- init
cargo run -- --help

# Install locally
cargo install --path .

# Clean
cargo clean
```

---

## Conclusion

This specification provides a complete blueprint for implementing mnemosyne in Rust. The design leverages Rust's strengths (zero-cost abstractions, memory safety, performance) while providing a clean, maintainable architecture suitable for incremental development using Claude Code.

**Key advantages of this Rust implementation**:
1. **Performance**: 5-10x faster than TypeScript for vector operations
2. **Memory Safety**: Compile-time guarantees prevent common bugs
3. **Type Safety**: Rich type system catches errors early
4. **Reliability**: No GC pauses, deterministic performance
5. **Developer Experience**: Excellent tooling (cargo, clippy, rustfmt)

**Next Steps**:
1. Review this specification
2. Set up Rust development environment
3. Start with Phase 0 (Foundation)
4. Use Claude Code to implement incrementally
5. Test continuously as you build

The specification includes typed holes marked with `todo!()` macros that make it easy to implement features incrementally while maintaining type safety throughout. Each `todo!()` has a clear description of what needs to be implemented, making it ideal for Claude Code-assisted development.
