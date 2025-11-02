# Mnemosyne Types Reference

**Last Updated**: 2025-11-02
**Source**: `src/types.rs`

Complete reference for core data types in the Mnemosyne memory system.

---

## Overview

Mnemosyne's type system provides:
- **Type Safety**: Newtype wrappers prevent ID confusion
- **Serialization**: All types support JSON via serde
- **Namespace Isolation**: Hierarchical scoping with priorities
- **Knowledge Graphs**: Typed relationships between memories
- **Lifecycle Management**: Timestamps and decay tracking

---

## Core Types

### `MemoryId`

Unique identifier for memories, wrapping UUID for type safety.

```rust
pub struct MemoryId(pub Uuid);
```

**Methods**:
- `new()` - Generate random ID
- `from_string(s: &str)` - Parse from string
- `to_string()` - Convert to string (via Display)

**Example**:
```rust
let id = MemoryId::new();
let parsed = MemoryId::from_string("550e8400-e29b-41d4-a716-446655440000")?;
println!("{}", id); // "550e8400-..."
```

---

### `Namespace`

Hierarchical isolation with priority-based retrieval.

```rust
pub enum Namespace {
    Global,
    Project { name: String },
    Session { project: String, session_id: String },
}
```

**Priority Levels**:
- `Session`: Priority 3 (searched first)
- `Project`: Priority 2
- `Global`: Priority 1 (searched last)

**String Representation**:
- Global: `"global"`
- Project: `"project:mnemosyne"`
- Session: `"session:mnemosyne:20251102-143022"`

**JSON Serialization**:
```json
// Global
{"type": "global"}

// Project
{"type": "project", "name": "mnemosyne"}

// Session
{"type": "session", "project": "mnemosyne", "session_id": "20251102-143022"}
```

**Methods**:
- `priority()` - Get priority level (1-3)
- `is_session()` - Check if session namespace
- `is_project()` - Check if project namespace
- `is_global()` - Check if global namespace

---

### `MemoryType`

Classification for organizational and filtering purposes.

```rust
pub enum MemoryType {
    ArchitectureDecision,  // System design choices
    CodePattern,           // Implementation approaches
    BugFix,                // Fixes and solutions
    Configuration,         // Settings and preferences
    Constraint,            // Requirements that must be satisfied
    Entity,                // Domain entities and concepts
    Insight,               // Learnings and observations
    Reference,             // External resources
    Preference,            // User preferences
    Task,                  // Action items
    AgentEvent,            // Orchestration events
}
```

**Type Factors** (importance multipliers):
- `ArchitectureDecision`: 1.2×
- `Constraint`: 1.1×
- `AgentEvent`: 1.0×
- `CodePattern`: 1.0×
- `BugFix`: 0.9×
- `Insight`: 0.9×
- `Task`: 0.9×
- Others: 0.8×

**Methods**:
- `type_factor()` - Get importance multiplier

---

### `LinkType`

Typed relationships for knowledge graph construction.

```rust
pub enum LinkType {
    Extends,       // B builds upon or extends A
    Contradicts,   // B contradicts or invalidates A
    Implements,    // B implements the concept in A
    References,    // B references or cites A
    Supersedes,    // B replaces or supersedes A
}
```

**Usage**:
- `Extends`: "New feature extends existing architecture"
- `Contradicts`: "Benchmark disproves earlier assumption"
- `Implements`: "Code implements RFC specification"
- `References`: "Decision references external documentation"
- `Supersedes`: "New approach replaces old pattern"

---

### `MemoryLink`

Link between memories with metadata.

```rust
pub struct MemoryLink {
    pub target_id: MemoryId,
    pub link_type: LinkType,
    pub strength: f32,                      // 0.0-1.0
    pub reason: String,                     // Human-readable explanation
    pub created_at: DateTime<Utc>,
    pub last_traversed_at: Option<DateTime<Utc>>,
    pub user_created: bool,                 // Manual links don't decay
}
```

**Link Strength**:
- Initial: 1.0 for manual links, 0.5-0.8 for LLM-generated
- Decays over time if not traversed
- Increases with co-access patterns
- User-created links are protected from decay

**Fields**:
- `target_id`: Target memory being linked to
- `link_type`: Relationship type
- `strength`: Link strength (0.0-1.0)
- `reason`: Explanation of why memories are related
- `created_at`: Link creation timestamp
- `last_traversed_at`: Last time link was followed during search
- `user_created`: If true, link won't decay automatically

---

### `MemoryNote`

Complete memory structure with all metadata.

```rust
pub struct MemoryNote {
    // === Identity ===
    pub id: MemoryId,
    pub namespace: Namespace,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // === Content ===
    pub content: String,              // Full content
    pub summary: String,               // LLM 1-2 sentence summary
    pub keywords: Vec<String>,         // LLM-extracted keywords
    pub tags: Vec<String>,             // Categorization tags
    pub context: String,               // When/why relevant

    // === Classification ===
    pub memory_type: MemoryType,
    pub importance: u8,                // 1-10 scale
    pub confidence: f32,               // 0.0-1.0 confidence

    // === Relationships ===
    pub links: Vec<MemoryLink>,
    pub related_files: Vec<String>,    // File paths
    pub related_entities: Vec<String>, // Components, services

    // === Lifecycle ===
    pub access_count: u32,
    pub last_accessed_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_archived: bool,
    pub superseded_by: Option<MemoryId>,

    // === Computational ===
    #[serde(skip)]
    pub embedding: Option<Vec<f32>>,   // Vector embedding
    pub embedding_model: String,       // Model name
}
```

**Methods**:

#### `decayed_importance() -> f32`
Calculate importance adjusted for age, access patterns, and type.

**Formula**:
```
decayed_importance = base_importance
                   × recency_factor
                   × type_factor
                   × (1 + access_bonus)

where:
  recency_factor = exp(-age_days / 180)  // 6-month half-life
  type_factor = memory_type.type_factor()
  access_bonus = ln(access_count) × 0.1
```

#### `recency_factor() -> f32`
Exponential decay with 6-month half-life.

#### `should_archive(threshold_days, min_importance) -> bool`
Check if memory should be archived based on age and importance.

---

### `SearchQuery`

Query specification for memory retrieval.

```rust
pub struct SearchQuery {
    pub query: String,                      // Search string
    pub namespace: Option<Namespace>,       // Filter by namespace
    pub memory_types: Vec<MemoryType>,      // Filter by types
    pub tags: Vec<String>,                  // Filter by tags
    pub min_importance: Option<u8>,         // Minimum importance
    pub max_age_days: Option<u32>,          // Maximum age
    pub limit: usize,                       // Result limit
    pub include_archived: bool,             // Include archived memories
}
```

**Default Values**:
- `limit`: 10
- `include_archived`: false
- Other filters: None (no filtering)

**Example**:
```rust
let query = SearchQuery {
    query: "authentication decisions".to_string(),
    namespace: Some(Namespace::Project { name: "myapp".to_string() }),
    memory_types: vec![MemoryType::ArchitectureDecision],
    min_importance: Some(7),
    limit: 5,
    ..Default::default()
};
```

---

### `SearchResult`

Search result with relevance scoring.

```rust
pub struct SearchResult {
    pub memory: MemoryNote,
    pub relevance_score: f32,              // Combined score
    pub keyword_score: f32,                // FTS5 score
    pub semantic_score: f32,               // Vector similarity
    pub graph_score: f32,                  // Graph connectivity
    pub explanation: String,               // Why this was returned
}
```

**Scoring**:
- `relevance_score`: Weighted combination of all scores
- `keyword_score`: Full-text search match (0.0-1.0)
- `semantic_score`: Vector cosine similarity (0.0-1.0)
- `graph_score`: Graph connectivity bonus (0.0-1.0)

**Score Weights** (configurable):
- Keyword: 0.4
- Semantic: 0.4
- Graph: 0.2

---

### `MemoryUpdates`

Partial update specification.

```rust
pub struct MemoryUpdates {
    pub content: Option<String>,
    pub importance: Option<u8>,
    pub tags: Option<Vec<String>>,        // Replaces all tags
    pub add_tags: Option<Vec<String>>,    // Appends tags
    pub memory_type: Option<MemoryType>,
    pub context: Option<String>,
}
```

**Usage**:
```rust
let updates = MemoryUpdates {
    importance: Some(9),
    add_tags: Some(vec!["critical".to_string()]),
    ..Default::default()
};

storage.update_memory(memory_id, updates).await?;
```

---

### `ConsolidationDecision`

Decision from memory consolidation analysis.

```rust
pub enum ConsolidationDecision {
    Merge {
        target_id: MemoryId,
        merged_content: String,
        reason: String,
    },
    Supersede {
        superseded_id: MemoryId,
        reason: String,
    },
    Keep {
        reason: String,
    },
}
```

**Decisions**:
- `Merge`: Combine two similar memories into one
- `Supersede`: Mark one memory as replaced by another
- `Keep`: Memories are distinct, keep both

---

## Orchestration Types

### `WorkItemId`

Unique identifier for work items in orchestration.

```rust
pub struct WorkItemId(pub Uuid);
```

Similar to `MemoryId` but for orchestration work queue.

---

### `Phase`

Work Plan Protocol phases.

```rust
pub enum Phase {
    Prompt,        // User request → specification
    Spec,          // Specification → full decomposition
    FullSpec,      // Decomposition → execution plan
    Plan,          // Execution plan → implementation
    Artifacts,     // Implementation complete
}
```

**Phase Transitions**:
```
Prompt → Spec → FullSpec → Plan → Artifacts
```

Exit criteria must be met before advancing.

---

### `WorkItemState`

Work item execution state.

```rust
pub enum WorkItemState {
    Pending,       // Not started
    InProgress,    // Currently executing
    Blocked,       // Waiting on dependencies
    Complete,      // Successfully finished
    Failed,        // Error occurred
}
```

---

### `AgentState`

Agent status in orchestration.

```rust
pub struct AgentState {
    pub id: AgentId,
    pub role: AgentRole,
    pub status: AgentStatus,              // Active, Idle, Busy, Failed
    pub current_work: Option<WorkItemId>,
    pub completed_work: Vec<WorkItemId>,
    pub last_heartbeat: DateTime<Utc>,
}
```

---

### `AgentRole`

Four primary agent roles.

```rust
pub enum AgentRole {
    Orchestrator,  // Central coordinator
    Optimizer,     // Context optimization
    Reviewer,      // Quality gates
    Executor,      // Primary work agent
}
```

---

## JSON Examples

### MemoryNote (Full Example)
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "namespace": {"type": "project", "name": "mnemosyne"},
  "created_at": "2025-11-02T12:00:00Z",
  "updated_at": "2025-11-02T14:30:00Z",
  "content": "Decided to use LibSQL for vector storage",
  "summary": "LibSQL chosen for native vector support and compatibility",
  "keywords": ["database", "vector", "storage", "libsql"],
  "tags": ["architecture", "storage", "decision"],
  "context": "Required for semantic search with low latency",
  "memory_type": "architecture_decision",
  "importance": 9,
  "confidence": 0.95,
  "links": [
    {
      "target_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
      "link_type": "extends",
      "strength": 0.85,
      "reason": "Builds on earlier vector search research",
      "created_at": "2025-11-02T12:00:00Z",
      "last_traversed_at": "2025-11-02T14:00:00Z",
      "user_created": false
    }
  ],
  "related_files": ["src/storage/libsql.rs"],
  "related_entities": ["StorageBackend", "VectorSearch"],
  "access_count": 12,
  "last_accessed_at": "2025-11-02T14:30:00Z",
  "expires_at": null,
  "is_archived": false,
  "superseded_by": null,
  "embedding_model": "text-embedding-3-small"
}
```

### SearchQuery Example
```json
{
  "query": "authentication decisions",
  "namespace": {"type": "project", "name": "api-backend"},
  "memory_types": ["architecture_decision", "constraint"],
  "tags": ["security"],
  "min_importance": 7,
  "max_age_days": 180,
  "limit": 5,
  "include_archived": false
}
```

### SearchResult Example
```json
{
  "memory": { /* MemoryNote structure */ },
  "relevance_score": 0.87,
  "keyword_score": 0.82,
  "semantic_score": 0.91,
  "graph_score": 0.45,
  "explanation": "Strong semantic match (0.91) and keyword match (0.82) for 'authentication'. Connected to 3 related memories."
}
```

---

## Type Conversions

### String Parsing

**Namespace from String**:
```rust
// Format: "global", "project:name", "session:project:id"
let ns = Namespace::from_str("project:mnemosyne")?;
```

**MemoryType from String** (with aliases):
```rust
// Canonical names
"architecture_decision" → ArchitectureDecision
"code_pattern" → CodePattern

// Aliases (case-insensitive)
"architecture" → ArchitectureDecision
"decision" → ArchitectureDecision
"pattern" → CodePattern
"bug" → BugFix
```

**LinkType from String**:
```rust
"extends" → Extends
"contradicts" → Contradicts
"implements" → Implements
"references" → References
"supersedes" → Supersedes
```

### JSON Serialization

All types use `serde` with snake_case field names:
- `MemoryType::ArchitectureDecision` → `"architecture_decision"`
- `LinkType::Extends` → `"extends"`
- `Namespace::Project` → `{"type": "project", "name": "..."}`

---

## See Also

- [AGENT_GUIDE.md](../AGENT_GUIDE.md) - Agent development guide
- [STORAGE_SCHEMA.md](STORAGE_SCHEMA.md) - Database schema
- [ARCHITECTURE.md](../ARCHITECTURE.md) - System architecture
- `src/types.rs` - Source code
