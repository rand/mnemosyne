# v2.0 Component Decomposition & Typed Holes

**Purpose**: Define all components, their interfaces, dependencies, and integration points

**Status**: Phase 2 - Full Specification

**Date**: 2025-10-27

---

## Table of Contents

1. [Stream 1: Vector Similarity Search](#stream-1-vector-similarity-search)
2. [Stream 2: Background Memory Evolution](#stream-2-background-memory-evolution)
3. [Stream 3: Advanced Agent Features](#stream-3-advanced-agent-features)
4. [Cross-Stream Integration Points](#cross-stream-integration-points)
5. [Typed Holes (Interfaces)](#typed-holes-interfaces)

---

## Stream 1: Vector Similarity Search

### Component 1.1: Remote Embedding Service

**File**: `src/embeddings/remote.rs`

**Responsibilities**:
- Generate embeddings from text via Voyage AI API
- Handle API authentication and errors
- Batch embedding requests for efficiency
- Rate limiting and retry logic

**Dependencies**:
- `reqwest` - HTTP client
- `serde_json` - JSON serialization
- `tokio` - Async runtime
- Secrets management (existing v1.0)

**Typed Hole #1: EmbeddingService Trait**
```rust
#[async_trait]
pub trait EmbeddingService: Send + Sync {
    /// Generate embedding for a single text
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;

    /// Generate embeddings for multiple texts (batched)
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError>;

    /// Get embedding dimensionality (e.g., 1536 for Voyage AI)
    fn dimensions(&self) -> usize;

    /// Get model name
    fn model_name(&self) -> &str;
}

// Implementation
pub struct RemoteEmbeddingService {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl EmbeddingService for RemoteEmbeddingService { ... }
```

**Constraints**:
- Must handle rate limits (429 status)
- Must timeout after 30 seconds
- Must validate embedding dimensions match expected
- Must not log API keys

**Edge Cases**:
- Empty text input → Return zero vector or error?
- Very long text (>8K tokens) → Truncate or chunk?
- Network failure → Retry 3x with exponential backoff
- Invalid API key → Clear error message
- API returns wrong dimensions → Error

---

### Component 1.2: Vector Storage

**File**: `src/storage/vectors.rs`

**Responsibilities**:
- Store and retrieve vectors from sqlite-vec
- Perform K-nearest neighbors (KNN) search
- Manage vector lifecycle (create, delete, update)

**Dependencies**:
- `libsql` - Database access
- sqlite-vec extension (loaded at runtime)
- Existing `MemoryId` type

**Typed Hole #2: VectorStorage Trait**
```rust
#[async_trait]
pub trait VectorStorage: Send + Sync {
    /// Store vector for a memory
    async fn store_vector(
        &self,
        memory_id: &MemoryId,
        embedding: &[f32],
    ) -> Result<(), StorageError>;

    /// Get vector for a memory
    async fn get_vector(
        &self,
        memory_id: &MemoryId,
    ) -> Result<Option<Vec<f32>>, StorageError>;

    /// K-nearest neighbors search
    async fn search_similar(
        &self,
        query_embedding: &[f32],
        limit: usize,
        min_similarity: f32,
    ) -> Result<Vec<(MemoryId, f32)>, StorageError>;

    /// Delete vector for a memory
    async fn delete_vector(
        &self,
        memory_id: &MemoryId,
    ) -> Result<(), StorageError>;

    /// Count total vectors stored
    async fn count_vectors(&self) -> Result<usize, StorageError>;
}

pub struct SqliteVectorStorage {
    db: Arc<Database>,
    dimensions: usize,
}

impl VectorStorage for SqliteVectorStorage { ... }
```

**Constraints**:
- Vectors must be 1536 dimensions (matching Voyage AI)
- Memory ID must exist in memories table (foreign key)
- Search limit must be >0 and <1000
- Similarity threshold must be 0.0-1.0

**Edge Cases**:
- sqlite-vec extension not loaded → Clear error message
- Vector dimensions mismatch → Error
- Memory ID doesn't exist → Should we auto-delete orphaned vectors?
- Database full → Storage error with disk space info

---

### Component 1.3: Hybrid Search Orchestrator

**File**: `src/search/hybrid.rs`

**Responsibilities**:
- Orchestrate parallel searches (vector + keyword + graph)
- Merge and deduplicate results
- Apply weighted ranking
- Return top N results

**Dependencies**:
- `EmbeddingService` (typed hole #1)
- `VectorStorage` (typed hole #2)
- Existing `LibSqlStorage` (keyword/graph search)

**Typed Hole #3: HybridSearcher Trait**
```rust
#[async_trait]
pub trait HybridSearcher: Send + Sync {
    /// Perform hybrid search
    async fn search(
        &self,
        query: &str,
        options: SearchOptions,
    ) -> Result<Vec<ScoredMemory>, SearchError>;
}

pub struct DefaultHybridSearcher {
    embeddings: Arc<dyn EmbeddingService>,
    vectors: Arc<dyn VectorStorage>,
    storage: Arc<LibSqlStorage>,
    weights: SearchWeights,
}

pub struct SearchOptions {
    pub limit: usize,
    pub min_importance: Option<f32>,
    pub namespace: Option<Namespace>,
    pub include_archived: bool,
}

pub struct SearchWeights {
    pub vector: f32,      // 0.40
    pub keyword: f32,     // 0.30
    pub graph: f32,       // 0.20
    pub importance: f32,  // 0.10
}

pub struct ScoredMemory {
    pub memory: MemoryNote,
    pub total_score: f32,
    pub scores: ScoreBreakdown,
}

pub struct ScoreBreakdown {
    pub vector: f32,
    pub keyword: f32,
    pub graph: f32,
    pub importance: f32,
}

impl HybridSearcher for DefaultHybridSearcher {
    async fn search(
        &self,
        query: &str,
        options: SearchOptions,
    ) -> Result<Vec<ScoredMemory>, SearchError> {
        // 1. Generate query embedding
        let query_embedding = self.embeddings.embed(query).await?;

        // 2. Parallel search
        let (vector_results, keyword_results, graph_results) = tokio::join!(
            self.vectors.search_similar(&query_embedding, 20, 0.7),
            self.storage.search_fts5(query, 20),
            self.storage.search_graph(query, 2, 5)
        );

        // 3. Merge and deduplicate
        let mut candidates = HashMap::new();
        for (id, score) in vector_results? { ... }
        for (id, score) in keyword_results? { ... }
        for (id, score) in graph_results? { ... }

        // 4. Weighted ranking
        let mut scored: Vec<ScoredMemory> = candidates
            .into_iter()
            .map(|(id, candidate)| self.score_candidate(candidate))
            .collect();

        scored.sort_by(|a, b| b.total_score.partial_cmp(&a.total_score).unwrap());
        scored.truncate(options.limit);

        Ok(scored)
    }
}
```

**Constraints**:
- All weights must sum to 1.0
- Scores must be normalized to 0.0-1.0 before weighting
- If any search method fails, continue with remaining methods
- If all methods fail, return error

**Edge Cases**:
- Vector search disabled (no API key) → Use keyword + graph only
- Empty query → Error or return recent memories?
- All searches return 0 results → Return empty vec
- Duplicate results from multiple methods → Keep highest individual score

---

### Component 1.4: Migration & Schema

**File**: `migrations/006_vector_search.sql`

**Responsibilities**:
- Create vec0 virtual table for vectors
- Add indexes for performance
- Ensure sqlite-vec extension is loaded

**Schema**:
```sql
-- Load sqlite-vec extension
-- Note: Must be loaded before creating virtual table
-- Path is platform-specific

-- Vector storage
CREATE VIRTUAL TABLE IF NOT EXISTS memory_vectors USING vec0(
    memory_id TEXT PRIMARY KEY,
    embedding FLOAT[1536],
    generated_at INTEGER NOT NULL,
    model TEXT NOT NULL
);

-- Index for efficient lookup
CREATE INDEX IF NOT EXISTS idx_memory_vectors_generated
ON memory_vectors(generated_at);

-- Trigger to clean up orphaned vectors
CREATE TRIGGER IF NOT EXISTS cleanup_orphaned_vectors
AFTER DELETE ON memories
BEGIN
    DELETE FROM memory_vectors WHERE memory_id = OLD.id;
END;
```

**Constraints**:
- Extension must be loaded BEFORE table creation
- Embedding dimensions must match model (1536 for Voyage)
- generated_at must be Unix timestamp
- memory_id must reference valid memory

**Edge Cases**:
- Extension not found → Migration fails with clear error
- Extension wrong version → Version check
- Table already exists (rerunning migration) → IF NOT EXISTS handles

---

## Stream 2: Background Memory Evolution

### Component 2.1: Job Scheduler

**File**: `src/evolution/scheduler.rs`

**Responsibilities**:
- Schedule periodic background jobs
- Detect system idle state
- Manage job execution (no overlapping runs)
- Track execution history

**Dependencies**:
- `tokio` - Async runtime and timers
- Evolution config (typed hole #4)
- Job trait (typed hole #5)

**Typed Hole #4: EvolutionConfig**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionConfig {
    pub enabled: bool,
    pub consolidation: JobConfig,
    pub importance: JobConfig,
    pub link_decay: JobConfig,
    pub archival: JobConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobConfig {
    pub enabled: bool,
    pub interval: Duration,
    pub batch_size: usize,
    pub max_duration: Duration,
}

impl EvolutionConfig {
    pub fn from_file(path: &Path) -> Result<Self, ConfigError>;
    pub fn validate(&self) -> Result<(), ConfigError>;
}
```

**Typed Hole #5: EvolutionJob Trait**
```rust
#[async_trait]
pub trait EvolutionJob: Send + Sync {
    /// Job name (for logging)
    fn name(&self) -> &str;

    /// Run the job
    async fn run(&self, config: &JobConfig) -> Result<JobReport, JobError>;

    /// Check if job should run now (based on last run time)
    async fn should_run(&self) -> Result<bool, JobError>;
}

pub struct BackgroundScheduler {
    storage: Arc<LibSqlStorage>,
    llm: Arc<LlmService>,
    config: EvolutionConfig,
    jobs: Vec<Arc<dyn EvolutionJob>>,
    running: Arc<AtomicBool>,
}

impl BackgroundScheduler {
    pub async fn start(&self) -> Result<(), SchedulerError> {
        self.running.store(true, Ordering::SeqCst);

        loop {
            if !self.running.load(Ordering::SeqCst) {
                break;
            }

            // Check if system is idle
            if !self.is_idle().await? {
                tokio::time::sleep(Duration::from_secs(60)).await;
                continue;
            }

            // Run jobs that are due
            for job in &self.jobs {
                if job.should_run().await? {
                    let report = job.run(&self.config).await?;
                    self.record_job_run(job.name(), &report).await?;
                }
            }

            tokio::time::sleep(Duration::from_secs(300)).await;
        }

        Ok(())
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    async fn is_idle(&self) -> Result<bool, SchedulerError> {
        // Check if no active queries in last 5 minutes
        let last_query = self.storage.get_last_query_time().await?;
        Ok(last_query < Utc::now() - Duration::from_secs(300))
    }
}
```

**Constraints**:
- Only one instance of each job can run at a time
- Jobs must timeout after max_duration
- Must not run during active queries
- Must track last run time

**Edge Cases**:
- Job crashes → Log error, continue to next job
- Job takes too long → Timeout and log
- Multiple schedulers (different processes) → Lock file
- System never idle → Configurable "force run" time?

---

### Component 2.2: Importance Recalibration

**File**: `src/evolution/importance.rs`

**Responsibilities**:
- Calculate new importance based on usage
- Update memory importance scores
- Track importance history

**Dependencies**:
- `LibSqlStorage` (read memories, update importance)
- `EvolutionJob` trait (typed hole #5)

**Implementation**:
```rust
pub struct ImportanceRecalibrator {
    storage: Arc<LibSqlStorage>,
}

#[async_trait]
impl EvolutionJob for ImportanceRecalibrator {
    fn name(&self) -> &str {
        "importance_recalibration"
    }

    async fn run(&self, config: &JobConfig) -> Result<JobReport, JobError> {
        let mut updated = 0;
        let memories = self.storage.list_all_active().await?;

        for memory in memories.into_iter().take(config.batch_size) {
            let new_importance = self.calculate_importance(&memory)?;

            if (new_importance - memory.importance).abs() > 1.0 {
                self.storage.update_importance(&memory.id, new_importance).await?;
                updated += 1;
            }
        }

        Ok(JobReport {
            memories_processed: memories.len(),
            changes_made: updated,
            duration: start.elapsed(),
            errors: 0,
        })
    }

    async fn should_run(&self) -> Result<bool, JobError> {
        let last_run = self.storage.get_last_job_run("importance_recalibration").await?;
        Ok(last_run.map_or(true, |t| t < Utc::now() - Duration::from_days(7)))
    }
}

impl ImportanceRecalibrator {
    fn calculate_importance(&self, memory: &MemoryNote) -> Result<f32, JobError> {
        let base = memory.importance;
        let access = self.access_factor(memory);
        let recency = self.recency_factor(memory);
        let links = self.link_factor(memory);

        // Weighted combination
        let score = base * 0.3 + access * 0.4 + recency * 0.2 + links * 0.1;

        // Clamp to [1.0, 10.0]
        Ok(score.clamp(1.0, 10.0))
    }

    fn access_factor(&self, memory: &MemoryNote) -> f32 {
        let accesses_per_day = memory.access_count as f32 / memory.days_since_creation().max(1.0);
        (accesses_per_day * 10.0).clamp(3.0, 10.0)
    }

    fn recency_factor(&self, memory: &MemoryNote) -> f32 {
        let days_since_access = memory.days_since_last_access();
        10.0 * 0.5_f32.powf(days_since_access / 30.0)
    }

    fn link_factor(&self, memory: &MemoryNote) -> f32 {
        let inbound = memory.incoming_links.len() as f32;
        let outbound = memory.outgoing_links.len() as f32;
        ((inbound * 2.0 + outbound) / 3.0).min(10.0)
    }
}
```

**Constraints**:
- Importance must stay in [1.0, 10.0]
- Must not update if change < 1.0 (avoid thrashing)
- Must track access_count and last_accessed_at
- Formula weights must sum to 1.0

**Edge Cases**:
- Memory never accessed (access_count = 0) → Use only base + recency + links
- Memory created today → Avoid division by zero in days_since_creation
- Very old memory → Recency factor approaches 0
- No links → link_factor = 0

---

### Component 2.3: Link Strength Decay

**File**: `src/evolution/links.rs`

**Responsibilities**:
- Weaken untraversed links over time
- Remove very weak links (<0.1 strength)
- Strengthen traversed links

**Dependencies**:
- `LibSqlStorage` (link CRUD)
- `EvolutionJob` trait (typed hole #5)

**Implementation**:
```rust
pub struct LinkDecayJob {
    storage: Arc<LibSqlStorage>,
}

#[async_trait]
impl EvolutionJob for LinkDecayJob {
    fn name(&self) -> &str {
        "link_decay"
    }

    async fn run(&self, config: &JobConfig) -> Result<JobReport, JobError> {
        let links = self.storage.list_all_links().await?;
        let mut weakened = 0;
        let mut removed = 0;

        for link in links.into_iter().take(config.batch_size) {
            let decay_factor = self.calculate_decay(&link)?;
            let new_strength = link.strength * decay_factor;

            if new_strength < 0.1 {
                self.storage.remove_link(&link.id).await?;
                removed += 1;
            } else if new_strength != link.strength {
                self.storage.update_link_strength(&link.id, new_strength).await?;
                weakened += 1;
            }
        }

        Ok(JobReport {
            memories_processed: links.len(),
            changes_made: weakened + removed,
            duration: start.elapsed(),
            errors: 0,
        })
    }

    async fn should_run(&self) -> Result<bool, JobError> {
        let last_run = self.storage.get_last_job_run("link_decay").await?;
        Ok(last_run.map_or(true, |t| t < Utc::now() - Duration::from_days(7)))
    }
}

impl LinkDecayJob {
    fn calculate_decay(&self, link: &MemoryLink) -> Result<f32, JobError> {
        let days_since_traversal = link.days_since_last_traversal();
        let days_since_creation = link.days_since_creation();

        if days_since_traversal > 180 {
            Ok(0.25)  // Quarter strength after 6 months
        } else if days_since_traversal > 90 {
            Ok(0.5)   // Half strength after 3 months
        } else if days_since_creation > 365 && days_since_traversal > 30 {
            Ok(0.8)   // Slight decay for old, unused links
        } else {
            Ok(1.0)   // No decay
        }
    }
}
```

**Constraints**:
- Link strength must stay in [0.0, 1.0]
- Must not decay manually created links (user_created = true)
- Must track last_traversed_at
- Remove threshold is 0.1 (configurable?)

**Edge Cases**:
- Link never traversed (last_traversed_at = null) → Treat as days_since_creation
- Link created today → No decay
- Bidirectional links → Decay independently
- Orphaned link (source or target deleted) → Auto-remove

---

### Component 2.4: Automatic Archival

**File**: `src/evolution/archival.rs`

**Responsibilities**:
- Identify archival candidates
- Archive unused memories
- Preserve all data (non-destructive)

**Dependencies**:
- `LibSqlStorage` (mark as archived)
- `EvolutionJob` trait (typed hole #5)

**Implementation**:
```rust
pub struct ArchivalJob {
    storage: Arc<LibSqlStorage>,
}

#[async_trait]
impl EvolutionJob for ArchivalJob {
    fn name(&self) -> &str {
        "archival"
    }

    async fn run(&self, config: &JobConfig) -> Result<JobReport, JobError> {
        let candidates = self.storage.find_archival_candidates().await?;
        let mut archived = 0;

        for memory in candidates.into_iter().take(config.batch_size) {
            if self.should_archive(&memory)? {
                self.storage.archive_memory(&memory.id).await?;
                archived += 1;
            }
        }

        Ok(JobReport {
            memories_processed: candidates.len(),
            changes_made: archived,
            duration: start.elapsed(),
            errors: 0,
        })
    }

    async fn should_run(&self) -> Result<bool, JobError> {
        let last_run = self.storage.get_last_job_run("archival").await?;
        Ok(last_run.map_or(true, |t| t < Utc::now() - Duration::from_days(30)))
    }
}

impl ArchivalJob {
    fn should_archive(&self, memory: &MemoryNote) -> Result<bool, JobError> {
        let days_since_access = memory.days_since_last_access();
        let importance = memory.importance;
        let access_count = memory.access_count;

        Ok(
            (access_count == 0 && days_since_access > 180) ||
            (importance < 3.0 && days_since_access > 90) ||
            (importance < 2.0 && days_since_access > 30)
        )
    }
}
```

**Constraints**:
- Must never delete (only mark archived_at)
- Archived memories still searchable with flag
- Must preserve links
- Reversible via unarchive command

**Edge Cases**:
- Memory accessed yesterday but very low importance → Don't archive
- Memory never accessed but very high importance (9-10) → Don't archive
- Namespace-specific archival policies → Future enhancement

---

### Component 2.5: Periodic Consolidation

**File**: `src/evolution/consolidation.rs`

**Responsibilities**:
- Detect duplicate/similar memories
- Use LLM to decide merge vs supersede vs keep
- Execute consolidation safely

**Dependencies**:
- `VectorStorage` (typed hole #2) - Similarity search
- `LibSqlStorage` - Memory CRUD
- `LlmService` - Consolidation decisions
- `EvolutionJob` trait (typed hole #5)

**⚠️ BLOCKED**: Requires vector search (Component 1.2) complete

**Implementation**:
```rust
pub struct ConsolidationJob {
    storage: Arc<LibSqlStorage>,
    vectors: Arc<dyn VectorStorage>,
    llm: Arc<LlmService>,
}

#[async_trait]
impl EvolutionJob for ConsolidationJob {
    fn name(&self) -> &str {
        "consolidation"
    }

    async fn run(&self, config: &JobConfig) -> Result<JobReport, JobError> {
        // 1. Find candidates (vector similarity + keyword overlap)
        let candidates = self.find_duplicate_candidates(config).await?;

        // 2. Cluster similar memories
        let clusters = self.cluster_memories(&candidates)?;

        // 3. LLM decision for each cluster
        let decisions = self.llm.decide_consolidation(&clusters).await?;

        // 4. Execute consolidations
        let mut merged = 0;
        let mut superseded = 0;

        for decision in decisions {
            match decision.action {
                ConsolidationAction::Merge => {
                    self.storage.merge_memories(&decision.memory_ids).await?;
                    merged += 1;
                }
                ConsolidationAction::Supersede => {
                    self.storage.supersede_memory(decision.old_id, decision.new_id).await?;
                    superseded += 1;
                }
                ConsolidationAction::Keep => {}
            }
        }

        Ok(JobReport {
            memories_processed: candidates.len(),
            changes_made: merged + superseded,
            duration: start.elapsed(),
            errors: 0,
        })
    }

    async fn should_run(&self) -> Result<bool, JobError> {
        let last_run = self.storage.get_last_job_run("consolidation").await?;
        Ok(last_run.map_or(true, |t| t < Utc::now() - Duration::from_days(1)))
    }
}

impl ConsolidationJob {
    async fn find_duplicate_candidates(&self, config: &JobConfig) -> Result<Vec<MemoryNote>, JobError> {
        let recent = self.storage.list_recent(config.batch_size * 2).await?;
        let mut candidates = Vec::new();

        for memory in &recent {
            // Get similar memories via vector search
            let embedding = self.vectors.get_vector(&memory.id).await?
                .ok_or(JobError::MissingVector(memory.id.clone()))?;

            let similar = self.vectors.search_similar(&embedding, 10, 0.95).await?;

            for (sim_id, similarity) in similar {
                if sim_id != memory.id && similarity > 0.95 {
                    let sim_memory = self.storage.get(&sim_id).await?;
                    candidates.push(sim_memory);
                }
            }
        }

        Ok(candidates)
    }

    fn cluster_memories(&self, candidates: &[MemoryNote]) -> Result<Vec<MemoryCluster>, JobError> {
        // Simple clustering by similarity threshold
        // Future: Use proper clustering algorithm
        todo!("Implement clustering")
    }
}
```

**Constraints**:
- Never delete (only supersede/merge)
- Must preserve audit trail
- Must get LLM approval before consolidation
- Similarity threshold: 0.95 (very similar)

**Edge Cases**:
- Memories similar but different namespace → Keep separate
- Memories similar but different time period → May be evolution, don't merge
- LLM service down → Skip consolidation this run
- No duplicates found → OK, report 0 changes

---

### Component 2.6: Schema Updates

**File**: `migrations/007_evolution.sql`

**Schema**:
```sql
-- Access tracking for importance recalibration
ALTER TABLE memories ADD COLUMN access_count INTEGER DEFAULT 0;
ALTER TABLE memories ADD COLUMN last_accessed_at INTEGER;

-- Archival support
ALTER TABLE memories ADD COLUMN archived_at INTEGER;
CREATE INDEX IF NOT EXISTS idx_memories_archived ON memories(archived_at);

-- Link traversal tracking for decay
ALTER TABLE memory_links ADD COLUMN last_traversed_at INTEGER;
ALTER TABLE memory_links ADD COLUMN user_created BOOLEAN DEFAULT 0;

-- Job execution history
CREATE TABLE IF NOT EXISTS evolution_job_runs (
    id TEXT PRIMARY KEY,
    job_name TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    completed_at INTEGER,
    status TEXT NOT NULL,  -- success, error, timeout
    memories_processed INTEGER DEFAULT 0,
    changes_made INTEGER DEFAULT 0,
    error_message TEXT
);

CREATE INDEX IF NOT EXISTS idx_job_runs_name ON evolution_job_runs(job_name, completed_at);

-- Importance history (optional, for analysis)
CREATE TABLE IF NOT EXISTS importance_history (
    memory_id TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    old_importance REAL NOT NULL,
    new_importance REAL NOT NULL,
    reason TEXT,
    FOREIGN KEY (memory_id) REFERENCES memories(id)
);
```

**Constraints**:
- access_count >= 0
- last_accessed_at <= current time
- archived_at nullable (null = active)
- Job run IDs must be unique

---

## Stream 3: Advanced Agent Features

### Component 3.1: Agent Memory Views

**File**: `src/agents/memory_view.rs`

**Responsibilities**:
- Filter memories by agent role
- Map memory types to agent interests
- Inject role-specific search filters

**Dependencies**:
- `LibSqlStorage`
- Agent role enum (typed hole #6)

**Typed Hole #6: AgentRole**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentRole {
    Orchestrator,
    Optimizer,
    Reviewer,
    Executor,
}

impl AgentRole {
    pub fn memory_types(&self) -> Vec<MemoryType> {
        match self {
            AgentRole::Orchestrator => vec![
                MemoryType::Decision,
                MemoryType::Architecture,
                MemoryType::Coordination,
            ],
            AgentRole::Optimizer => vec![
                MemoryType::Decision,
                MemoryType::Pattern,
                MemoryType::Skill,
            ],
            AgentRole::Reviewer => vec![
                MemoryType::Bug,
                MemoryType::Test,
                MemoryType::Decision,
            ],
            AgentRole::Executor => vec![
                MemoryType::Implementation,
                MemoryType::Pattern,
                MemoryType::Bug,
            ],
        }
    }
}
```

**Typed Hole #7: AgentMemoryView**
```rust
pub struct AgentMemoryView {
    agent: AgentRole,
    storage: Arc<LibSqlStorage>,
}

impl AgentMemoryView {
    pub fn new(agent: AgentRole, storage: Arc<LibSqlStorage>) -> Self {
        Self { agent, storage }
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<MemoryNote>, StorageError> {
        let mut filters = SearchFilters::default();
        filters.memory_types = self.agent.memory_types();
        filters.limit = limit;

        self.storage.search_with_filters(query, &filters).await
    }

    pub async fn list_recent(&self, limit: usize) -> Result<Vec<MemoryNote>, StorageError> {
        let mut filters = SearchFilters::default();
        filters.memory_types = self.agent.memory_types();
        filters.limit = limit;
        filters.order_by = OrderBy::RecentlyAccessed;

        self.storage.list_with_filters(&filters).await
    }
}
```

**Constraints**:
- Agent role must be valid enum variant
- Memory type filters applied via SQL WHERE IN clause
- Must not bypass filters (security)

**Edge Cases**:
- Agent role not recognized → Default to no filtering?
- No memories match agent's types → Return empty vec
- Memory has no type set → Visible to all agents?

---

### Component 3.2: Role-Based Access Control

**File**: `src/agents/access_control.rs`

**Responsibilities**:
- Track memory ownership (created_by)
- Enforce update permissions (owner only)
- Log all modifications for audit trail
- Provide admin override

**Dependencies**:
- `LibSqlStorage`
- `AgentRole` (typed hole #6)

**Typed Hole #8: MemoryAccessControl**
```rust
pub struct MemoryAccessControl {
    agent: AgentRole,
    storage: Arc<LibSqlStorage>,
}

impl MemoryAccessControl {
    pub async fn create_memory(
        &self,
        content: &str,
        metadata: MemoryMetadata,
    ) -> Result<MemoryId, AccessError> {
        let mut metadata = metadata;
        metadata.created_by = Some(self.agent);
        metadata.visible_to = self.default_visibility();

        let id = self.storage.store_memory(content, metadata).await?;
        self.log_modification(&id, ModificationType::Create).await?;

        Ok(id)
    }

    pub async fn update_memory(
        &self,
        memory_id: &MemoryId,
        updates: MemoryUpdates,
    ) -> Result<(), AccessError> {
        let memory = self.storage.get(memory_id).await?;

        // Check ownership
        if memory.created_by != Some(self.agent) && !self.is_admin() {
            return Err(AccessError::PermissionDenied {
                agent: self.agent,
                memory_id: memory_id.clone(),
            });
        }

        self.storage.update_memory(memory_id, updates).await?;
        self.log_modification(memory_id, ModificationType::Update).await?;

        Ok(())
    }

    fn default_visibility(&self) -> Vec<AgentRole> {
        match self.agent {
            AgentRole::Orchestrator => vec![AgentRole::Orchestrator, AgentRole::Optimizer],
            AgentRole::Optimizer => vec![AgentRole::Optimizer, AgentRole::Executor],
            AgentRole::Reviewer => vec![AgentRole::Reviewer, AgentRole::Executor],
            AgentRole::Executor => vec![AgentRole::Executor, AgentRole::Reviewer],
        }
    }

    fn is_admin(&self) -> bool {
        // Human users have admin override
        std::env::var("MNEMOSYNE_ADMIN_MODE").is_ok()
    }

    async fn log_modification(
        &self,
        memory_id: &MemoryId,
        mod_type: ModificationType,
    ) -> Result<(), AccessError> {
        self.storage.insert_modification_log(ModificationLog {
            id: Uuid::new_v4().to_string(),
            memory_id: memory_id.clone(),
            agent_role: self.agent,
            modification_type: mod_type,
            timestamp: Utc::now(),
            changes: None,
        }).await?;

        Ok(())
    }
}
```

**Constraints**:
- Only owner can update (unless admin)
- All modifications logged
- visible_to must be non-empty vec
- Admin mode requires env var

**Edge Cases**:
- Memory has no owner (created_by = null) → Allow all updates?
- Agent tries to delete (not update) → Separate permission check
- Audit log table full → Rotate old entries?
- Human user without admin flag → How to distinguish from agents?

---

### Component 3.3: Custom Importance Scoring

**File**: `src/agents/importance_scorer.rs`

**Responsibilities**:
- Calculate role-specific importance scores
- Weight components differently per agent
- Apply during search ranking

**Dependencies**:
- `AgentRole` (typed hole #6)
- Search context

**Implementation**:
```rust
pub struct CustomImportanceScorer {
    agent: AgentRole,
}

impl CustomImportanceScorer {
    pub fn score(&self, memory: &MemoryNote, context: &SearchContext) -> f32 {
        let weights = self.get_weights();

        let base = memory.importance / 10.0;  // Normalize to 0-1
        let access = self.access_score(memory);
        let recency = self.recency_score(memory);
        let relevance = self.relevance_score(memory, context);

        weights.base * base +
        weights.access * access +
        weights.recency * recency +
        weights.relevance * relevance
    }

    fn get_weights(&self) -> ImportanceWeights {
        match self.agent {
            AgentRole::Orchestrator => ImportanceWeights {
                base: 0.3,
                access: 0.2,
                recency: 0.4,
                relevance: 0.1,
            },
            AgentRole::Optimizer => ImportanceWeights {
                base: 0.4,
                access: 0.3,
                recency: 0.1,
                relevance: 0.2,
            },
            AgentRole::Reviewer => ImportanceWeights {
                base: 0.5,
                access: 0.1,
                recency: 0.2,
                relevance: 0.2,
            },
            AgentRole::Executor => ImportanceWeights {
                base: 0.2,
                access: 0.4,
                recency: 0.3,
                relevance: 0.1,
            },
        }
    }

    fn access_score(&self, memory: &MemoryNote) -> f32 {
        let count = memory.access_count as f32;
        (count / 10.0).min(1.0)
    }

    fn recency_score(&self, memory: &MemoryNote) -> f32 {
        let days = memory.days_since_last_access();
        (1.0 / (1.0 + days / 30.0)).clamp(0.0, 1.0)
    }

    fn relevance_score(&self, memory: &MemoryNote, context: &SearchContext) -> f32 {
        // Match memory type to agent role
        if self.agent.memory_types().contains(&memory.memory_type) {
            0.8
        } else {
            0.3
        }
    }
}

pub struct ImportanceWeights {
    pub base: f32,
    pub access: f32,
    pub recency: f32,
    pub relevance: f32,
}
```

**Constraints**:
- Weights must sum to 1.0
- Scores must be normalized to [0, 1]
- Must be deterministic (same inputs → same output)

**Edge Cases**:
- access_count very high (>100) → Clamp at 1.0
- Memory accessed just now → recency = 1.0
- Memory type not in agent's list → Lower relevance, not 0

---

### Component 3.4: Memory Prefetching

**File**: `src/agents/prefetcher.rs`

**Responsibilities**:
- Predict which memories agents will need
- Preload to LRU cache
- Track cache hit rate
- Adaptive prefetching patterns

**Dependencies**:
- `AgentMemoryView` (typed hole #7)
- `LruCache` (lru crate)

**Typed Hole #9: MemoryPrefetcher**
```rust
pub struct MemoryPrefetcher {
    agent: AgentRole,
    view: Arc<AgentMemoryView>,
    cache: Arc<RwLock<LruCache<MemoryId, MemoryNote>>>,
    metrics: Arc<PrefetchMetrics>,
}

pub struct PrefetchMetrics {
    hits: AtomicU64,
    misses: AtomicU64,
    prefetch_count: AtomicU64,
}

impl PrefetchMetrics {
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed) as f64;
        let misses = self.misses.load(Ordering::Relaxed) as f64;
        if hits + misses == 0.0 {
            0.0
        } else {
            hits / (hits + misses)
        }
    }
}

impl MemoryPrefetcher {
    pub async fn on_session_start(&self) -> Result<(), PrefetchError> {
        // Prefetch top accessed memories
        let recent_accessed = self.view.list_recent(20).await?;
        self.populate_cache(recent_accessed).await;

        // Prefetch high importance for this role
        let important = self.view.search("", 10).await?
            .into_iter()
            .filter(|m| m.importance >= 8.0)
            .collect();
        self.populate_cache(important).await;

        Ok(())
    }

    pub async fn on_phase_transition(&self, phase: WorkPhase) -> Result<(), PrefetchError> {
        let query = match (self.agent, phase) {
            (AgentRole::Executor, WorkPhase::Implementation) => "implementation patterns",
            (AgentRole::Reviewer, WorkPhase::Review) => "test coverage quality",
            (AgentRole::Orchestrator, WorkPhase::Planning) => "planning coordination",
            _ => return Ok(()),
        };

        let memories = self.view.search(query, 15).await?;
        self.populate_cache(memories).await;

        Ok(())
    }

    pub async fn get(&self, id: &MemoryId) -> Option<MemoryNote> {
        let mut cache = self.cache.write().await;
        if let Some(memory) = cache.get(id) {
            self.metrics.hits.fetch_add(1, Ordering::Relaxed);
            Some(memory.clone())
        } else {
            self.metrics.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    async fn populate_cache(&self, memories: Vec<MemoryNote>) {
        let mut cache = self.cache.write().await;
        for memory in memories {
            cache.put(memory.id.clone(), memory);
            self.metrics.prefetch_count.fetch_add(1, Ordering::Relaxed);
        }
    }
}
```

**Constraints**:
- Cache size: 1000 memories (configurable)
- Memory overhead: ~50MB
- Hit rate target: 70%+
- Must not prefetch during low memory

**Edge Cases**:
- Cache full → LRU eviction
- Prefetch same memory multiple times → Idempotent (updates position in LRU)
- Session start with empty database → Prefetch returns 0, OK
- Phase transition not recognized → Skip prefetch

---

### Component 3.5: Schema Updates

**File**: `migrations/008_agent_features.sql`

**Schema**:
```sql
-- Agent ownership and visibility
ALTER TABLE memories ADD COLUMN created_by TEXT;  -- AgentRole as string
ALTER TABLE memories ADD COLUMN modified_by TEXT;
ALTER TABLE memories ADD COLUMN visible_to TEXT;  -- JSON array of AgentRole

CREATE INDEX IF NOT EXISTS idx_memories_created_by ON memories(created_by);

-- Audit trail
CREATE TABLE IF NOT EXISTS memory_modifications (
    id TEXT PRIMARY KEY,
    memory_id TEXT NOT NULL,
    agent_role TEXT NOT NULL,
    modification_type TEXT NOT NULL,  -- create, update, delete, archive
    timestamp INTEGER NOT NULL,
    changes TEXT,  -- JSON of what changed
    FOREIGN KEY (memory_id) REFERENCES memories(id)
);

CREATE INDEX IF NOT EXISTS idx_modifications_memory ON memory_modifications(memory_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_modifications_agent ON memory_modifications(agent_role, timestamp);
```

**Constraints**:
- created_by/modified_by must be valid AgentRole (or null for human)
- visible_to must be valid JSON array
- modification_type must be enum value
- timestamp must be Unix epoch

---

## Cross-Stream Integration Points

### Integration Point 1: Consolidation ↔ Vector Search

**Dependency**: Consolidation (Stream 2.5) requires Vector Storage (Stream 1.2)

**Interface**:
```rust
// Stream 2 depends on Stream 1
impl ConsolidationJob {
    async fn find_duplicate_candidates(&self) -> Result<Vec<MemoryNote>> {
        // Uses VectorStorage trait (typed hole #2) from Stream 1
        let embedding = self.vectors.get_vector(&memory.id).await?;
        let similar = self.vectors.search_similar(&embedding, 10, 0.95).await?;
        ...
    }
}
```

**Coordination Strategy**:
- Stream 1 must complete Component 1.2 (VectorStorage) before Stream 2 starts Component 2.5
- Stream 2 can work on Components 2.1-2.4 in parallel with Stream 1
- Integration test created after both complete

---

### Integration Point 2: Agent Views ↔ Hybrid Search

**Dependency**: Agent views (Stream 3.1) can optionally use Hybrid Search (Stream 1.3)

**Interface**:
```rust
// Stream 3 can optionally use Stream 1
impl AgentMemoryView {
    pub async fn search(&self, query: &str) -> Result<Vec<MemoryNote>> {
        // Check if hybrid search available
        if let Some(hybrid) = &self.hybrid_searcher {
            hybrid.search(query, options).await
        } else {
            // Fallback to FTS5
            self.storage.search_fts5(query, options.limit).await
        }
    }
}
```

**Coordination Strategy**:
- Stream 3 can start independently
- When Stream 1 completes, Stream 3 adds hybrid search integration
- Backward compatible (works with or without hybrid search)

---

### Integration Point 3: Schema Migrations

**Dependency**: All streams add schema migrations (006, 007, 008)

**Coordination**:
- Migration numbers assigned upfront (no conflicts)
- Each migration tested independently
- Integration test runs all migrations in sequence
- Coordinator (main agent) reviews all SQL before merging

**Migration Sequence**:
```
migrations/
├── 006_vector_search.sql      (Stream 1)
├── 007_evolution.sql           (Stream 2)
└── 008_agent_features.sql      (Stream 3)
```

---

## Typed Holes (Interfaces)

### Summary of Typed Holes

| Hole # | Name | Defined By | Used By | Status |
|--------|------|------------|---------|--------|
| #1 | `EmbeddingService` | Stream 1.1 | Stream 1.3 | Defined |
| #2 | `VectorStorage` | Stream 1.2 | Stream 1.3, 2.5 | Defined |
| #3 | `HybridSearcher` | Stream 1.3 | Stream 3.1 (optional) | Defined |
| #4 | `EvolutionConfig` | Stream 2.1 | All Stream 2 jobs | Defined |
| #5 | `EvolutionJob` | Stream 2.1 | Stream 2.2-2.5 | Defined |
| #6 | `AgentRole` | Stream 3.1 | All Stream 3 | Defined |
| #7 | `AgentMemoryView` | Stream 3.1 | Stream 3.4 | Defined |
| #8 | `MemoryAccessControl` | Stream 3.2 | External (MCP) | Defined |
| #9 | `MemoryPrefetcher` | Stream 3.4 | Orchestration engine | Defined |

### Verification Checklist

For each typed hole:
- [ ] Trait or struct fully defined
- [ ] All method signatures specified
- [ ] Error types defined
- [ ] Constraints documented
- [ ] Edge cases listed
- [ ] Dependencies identified
- [ ] Used by X components verified

**Status**: All 9 typed holes defined ✅

---

**Version**: 1.0
**Status**: Phase 2 Complete - Ready for Test Plan
**Last Updated**: 2025-10-27
