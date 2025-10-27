# Advanced Agent Features - v2.0 Implementation Plan

**Status**: Planned
**Priority**: P2 (Enhancement)
**Dependencies**: Multi-Agent Orchestration (Phase 6, complete)

---

## Overview

Enhance the 4-agent orchestration system with advanced memory capabilities that enable smarter, more autonomous behavior:

1. **Agent-specific memory views** - Each agent sees memories relevant to its role
2. **Role-based access control** - Agents can only modify their own memories
3. **Custom importance scoring** - Agents weight importance differently
4. **Memory prefetching** - Predict and load memories before needed

**Goal**: Agents become more specialized, efficient, and autonomous in their use of memory.

---

## Problem Analysis

### Current Limitations

**Shared Memory View**:
- All agents see same memories
- No role-specific filtering
- Optimizer sees Executor's implementation details
- Reviewer sees Orchestrator's coordination notes
- Information overload

**Uniform Importance**:
- Same importance score for all agents
- Reviewer cares about test coverage
- Executor cares about implementation patterns
- Orchestrator cares about coordination history
- One-size-fits-all approach fails

**Reactive Memory Access**:
- Agents query memory when needed
- Latency on every query (50-200ms)
- No anticipation of future needs
- Repeated queries for same information
- Context switching overhead

**No Access Control**:
- Any agent can modify any memory
- No audit trail of who changed what
- Risk of cross-agent conflicts
- Unclear ownership

### Example Scenario

**Current**: Executor searches "authentication patterns"
- Returns: Architecture decisions, test plans, code snippets, coordination notes
- Result: Information overload, slow to find relevant pattern
- Latency: 150ms query time

**Improved**: Executor with smart prefetching
- Agent role: Executor → filter by `memory_type=implementation`
- Prefetched: Common patterns loaded at session start
- Cached: Recently used patterns in memory
- Result: Instant access to relevant code patterns
- Latency: <5ms (cache hit)

---

## Technical Design

### Architecture

```
┌──────────────────────────────────────────────────────┐
│              Agent Memory Manager                    │
├──────────────────────────────────────────────────────┤
│                                                      │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────┐│
│  │ Orchestrator│  │  Optimizer   │  │  Reviewer  ││
│  │   View      │  │    View      │  │    View    ││
│  │             │  │              │  │            ││
│  │ - Coord.    │  │ - Skills     │  │ - Tests    ││
│  │ - Handoffs  │  │ - Context    │  │ - Quality  ││
│  └─────────────┘  └──────────────┘  └────────────┘│
│                                                      │
│  ┌─────────────┐                                    │
│  │  Executor   │                                    │
│  │    View     │                                    │
│  │             │                                    │
│  │ - Impl.     │                                    │
│  │ - Patterns  │                                    │
│  └─────────────┘                                    │
│                                                      │
│  ┌────────────────────────────────────────────┐    │
│  │         Prefetch Engine                    │    │
│  │  - Pattern detection                       │    │
│  │  - Predictive loading                      │    │
│  │  - Cache management                        │    │
│  └────────────────────────────────────────────┘    │
│                                                      │
└──────────────────────────────────────────────────────┘
           ↓
    ┌──────────────┐
    │   Storage    │
    └──────────────┘
```

---

## Feature 1: Agent-Specific Memory Views

**Goal**: Each agent sees only memories relevant to its role

**Design**:
```rust
pub struct AgentMemoryView {
    agent: AgentRole,
    storage: Arc<LibSqlStorage>,
    filters: MemoryFilters,
}

pub enum AgentRole {
    Orchestrator,
    Optimizer,
    Reviewer,
    Executor,
}

impl AgentMemoryView {
    pub async fn search(&self, query: &str) -> Result<Vec<MemoryNote>> {
        let mut filters = self.filters.clone();

        // Add role-specific filters
        match self.agent {
            AgentRole::Orchestrator => {
                filters.memory_types = vec![
                    MemoryType::Decision,
                    MemoryType::Architecture,
                    MemoryType::Coordination,
                ];
            }
            AgentRole::Optimizer => {
                filters.memory_types = vec![
                    MemoryType::Decision,
                    MemoryType::Pattern,
                    MemoryType::Skill,
                ];
            }
            AgentRole::Reviewer => {
                filters.memory_types = vec![
                    MemoryType::Bug,
                    MemoryType::Test,
                    MemoryType::Decision,
                ];
            }
            AgentRole::Executor => {
                filters.memory_types = vec![
                    MemoryType::Implementation,
                    MemoryType::Pattern,
                    MemoryType::Bug,
                ];
            }
        }

        self.storage.search_with_filters(query, &filters).await
    }
}
```

**Memory Type Mapping**:
```
Orchestrator:
  - Decision: Architectural choices
  - Architecture: System design
  - Coordination: Agent handoffs
  - Learning: Process improvements

Optimizer:
  - Decision: Context allocation strategies
  - Pattern: Common optimization patterns
  - Skill: Loaded skills and relevance
  - Learning: Context budget tuning

Reviewer:
  - Bug: Known issues and fixes
  - Test: Test coverage and strategies
  - Decision: Quality standards
  - Learning: Review patterns

Executor:
  - Implementation: Code patterns
  - Pattern: Design patterns
  - Bug: Bug fixes and workarounds
  - Learning: Successful implementations
```

**SQL Implementation**:
```sql
-- Add agent_role column
ALTER TABLE memories ADD COLUMN visible_to TEXT; -- JSON array of agent roles

-- Filter by agent role
SELECT * FROM memories
WHERE visible_to LIKE '%"executor"%'
  AND memory_type IN ('implementation', 'pattern', 'bug')
  AND importance >= ?
ORDER BY importance DESC, accessed_at DESC
LIMIT ?;
```

---

## Feature 2: Role-Based Access Control

**Goal**: Agents can only create/modify memories they own

**Design**:
```rust
pub struct MemoryAccessControl {
    agent: AgentRole,
    storage: Arc<LibSqlStorage>,
}

impl MemoryAccessControl {
    pub async fn create_memory(
        &self,
        content: &str,
        metadata: MemoryMetadata
    ) -> Result<MemoryId> {
        // Add agent ownership
        let mut metadata = metadata;
        metadata.created_by = self.agent.to_string();
        metadata.visible_to = self.default_visibility();

        self.storage.store_memory(content, metadata).await
    }

    pub async fn update_memory(
        &self,
        memory_id: &MemoryId,
        updates: MemoryUpdates
    ) -> Result<()> {
        // Check ownership
        let memory = self.storage.get(memory_id).await?;

        if memory.created_by != self.agent.to_string() {
            return Err(MemoryError::PermissionDenied {
                agent: self.agent.to_string(),
                memory_id: memory_id.clone(),
            });
        }

        self.storage.update_memory(memory_id, updates).await
    }

    fn default_visibility(&self) -> Vec<AgentRole> {
        match self.agent {
            AgentRole::Orchestrator => vec![
                AgentRole::Orchestrator,
                AgentRole::Optimizer,  // Needs coordination info
            ],
            AgentRole::Optimizer => vec![
                AgentRole::Optimizer,
                AgentRole::Executor,  // Needs skill info
            ],
            AgentRole::Reviewer => vec![
                AgentRole::Reviewer,
                AgentRole::Executor,  // Needs quality feedback
            ],
            AgentRole::Executor => vec![
                AgentRole::Executor,
                AgentRole::Reviewer,  // Reviews need impl details
            ],
        }
    }
}
```

**Schema Updates**:
```sql
-- Add ownership tracking
ALTER TABLE memories ADD COLUMN created_by TEXT;  -- Agent role
ALTER TABLE memories ADD COLUMN modified_by TEXT; -- Agent role
ALTER TABLE memories ADD COLUMN visible_to TEXT;  -- JSON array

CREATE INDEX idx_memories_created_by ON memories(created_by);
CREATE INDEX idx_memories_visible_to ON memories(visible_to);

-- Audit trail for modifications
CREATE TABLE memory_modifications (
    id TEXT PRIMARY KEY,
    memory_id TEXT NOT NULL,
    agent_role TEXT NOT NULL,
    modification_type TEXT NOT NULL,  -- create, update, delete, archive
    timestamp INTEGER NOT NULL,
    changes TEXT,  -- JSON of what changed
    FOREIGN KEY (memory_id) REFERENCES memories(id)
);
```

**Visibility Rules**:
- **Create**: Agent owns memory, sets default visibility
- **Read**: Agent can read if in `visible_to` list
- **Update**: Only owner can update
- **Delete/Archive**: Only owner can archive
- **Override**: Human user can see/modify all

---

## Feature 3: Custom Importance Scoring

**Goal**: Each agent weights memory importance differently

**Design**:
```rust
pub struct CustomImportanceScorer {
    agent: AgentRole,
    weights: ImportanceWeights,
}

pub struct ImportanceWeights {
    pub base_importance: f32,
    pub access_frequency: f32,
    pub recency: f32,
    pub relevance_to_role: f32,
}

impl CustomImportanceScorer {
    pub fn score(&self, memory: &MemoryNote, context: &SearchContext) -> f32 {
        let base = memory.importance;
        let access = self.access_score(memory);
        let recency = self.recency_score(memory);
        let relevance = self.relevance_score(memory, context);

        // Role-specific weighting
        let weights = self.role_weights();
        weights.base_importance * base +
        weights.access_frequency * access +
        weights.recency * recency +
        weights.relevance_to_role * relevance
    }

    fn role_weights(&self) -> &ImportanceWeights {
        match self.agent {
            AgentRole::Orchestrator => &ImportanceWeights {
                base_importance: 0.3,
                access_frequency: 0.2,
                recency: 0.4,  // Coordination info becomes stale quickly
                relevance_to_role: 0.1,
            },
            AgentRole::Optimizer => &ImportanceWeights {
                base_importance: 0.4,
                access_frequency: 0.3,  // Frequently used patterns matter
                recency: 0.1,
                relevance_to_role: 0.2,
            },
            AgentRole::Reviewer => &ImportanceWeights {
                base_importance: 0.5,  // Quality standards are stable
                access_frequency: 0.1,
                recency: 0.2,
                relevance_to_role: 0.2,
            },
            AgentRole::Executor => &ImportanceWeights {
                base_importance: 0.2,
                access_frequency: 0.4,  // Recent successful patterns
                recency: 0.3,
                relevance_to_role: 0.1,
            },
        }
    }

    fn relevance_score(&self, memory: &MemoryNote, context: &SearchContext) -> f32 {
        // Match memory type to agent role
        let type_match = match (self.agent, &memory.memory_type) {
            (AgentRole::Orchestrator, MemoryType::Decision) => 1.0,
            (AgentRole::Optimizer, MemoryType::Skill) => 1.0,
            (AgentRole::Reviewer, MemoryType::Test) => 1.0,
            (AgentRole::Executor, MemoryType::Implementation) => 1.0,
            _ => 0.5,
        };

        // Match context (current task, phase, etc.)
        let context_match = self.match_context(memory, context);

        (type_match + context_match) / 2.0
    }
}
```

**Agent-Specific Preferences**:

**Orchestrator**:
- Values: Recent coordination decisions, handoff protocols
- Weights: Recency > Base > Access > Relevance

**Optimizer**:
- Values: Frequently used skills, context strategies
- Weights: Base > Access > Relevance > Recency

**Reviewer**:
- Values: Quality standards, test patterns, known bugs
- Weights: Base > Relevance > Recency > Access

**Executor**:
- Values: Recent successful implementations, code patterns
- Weights: Access > Recency > Base > Relevance

---

## Feature 4: Memory Prefetching

**Goal**: Predict and preload memories before agents need them

**Design**:
```rust
pub struct MemoryPrefetcher {
    agent: AgentRole,
    storage: Arc<LibSqlStorage>,
    cache: Arc<RwLock<LruCache<MemoryId, MemoryNote>>>,
    predictor: PrefetchPredictor,
}

pub struct PrefetchPredictor {
    patterns: Vec<PrefetchPattern>,
}

pub struct PrefetchPattern {
    pub trigger: PrefetchTrigger,
    pub memories_to_load: Vec<MemoryQuery>,
}

pub enum PrefetchTrigger {
    SessionStart,
    PhaseTransition(WorkPhase),
    TaskStart(TaskType),
    MemoryAccess(MemoryId),  // Co-access patterns
}

impl MemoryPrefetcher {
    pub async fn on_session_start(&self) -> Result<()> {
        // Preload common memories for this agent
        let queries = self.session_start_queries();

        for query in queries {
            let memories = self.storage.search(&query.text, query.limit).await?;
            self.populate_cache(memories).await;
        }

        Ok(())
    }

    pub async fn on_phase_transition(&self, phase: WorkPhase) -> Result<()> {
        match (self.agent, phase) {
            (AgentRole::Executor, WorkPhase::Implementation) => {
                // Prefetch implementation patterns
                self.prefetch("implementation patterns", 20).await?;
                self.prefetch("code snippets", 10).await?;
            }
            (AgentRole::Reviewer, WorkPhase::Review) => {
                // Prefetch test strategies and quality standards
                self.prefetch("test coverage", 10).await?;
                self.prefetch("quality standards", 5).await?;
            }
            (AgentRole::Orchestrator, WorkPhase::Planning) => {
                // Prefetch past plans and coordination decisions
                self.prefetch("planning decisions", 15).await?;
                self.prefetch("task dependencies", 10).await?;
            }
            _ => {}
        }

        Ok(())
    }

    pub async fn on_memory_access(&self, memory_id: &MemoryId) -> Result<()> {
        // Load linked memories (co-access pattern)
        let memory = self.storage.get(memory_id).await?;

        for link in &memory.links {
            if !self.cache.read().await.contains(&link.target_id) {
                let linked = self.storage.get(&link.target_id).await?;
                self.cache.write().await.put(link.target_id.clone(), linked);
            }
        }

        Ok(())
    }

    async fn prefetch(&self, query: &str, limit: usize) -> Result<()> {
        let memories = self.storage.search(query, limit).await?;
        self.populate_cache(memories).await;
        Ok(())
    }

    async fn populate_cache(&self, memories: Vec<MemoryNote>) {
        let mut cache = self.cache.write().await;
        for memory in memories {
            cache.put(memory.id.clone(), memory);
        }
    }
}
```

**Prefetch Patterns**:

**Session Start** (all agents):
- Top 20 most accessed memories
- Memories accessed in last session
- High importance (>8) memories

**Executor - Task Start**:
- Similar past implementations
- Related code patterns
- Linked bug fixes

**Reviewer - Review Start**:
- Test strategies for component
- Quality standards for language
- Known issues and workarounds

**Optimizer - Context Budget Low**:
- Context compression strategies
- Skill unloading priorities
- Past successful optimizations

**Orchestrator - Parallel Work**:
- Agent handoff protocols
- Dependency resolution patterns
- Deadlock prevention strategies

**Cache Management**:
```rust
pub struct PrefetchCache {
    capacity: usize,  // 1000 memories
    cache: LruCache<MemoryId, MemoryNote>,
    hits: AtomicU64,
    misses: AtomicU64,
}

impl PrefetchCache {
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed) as f64;
        let misses = self.misses.load(Ordering::Relaxed) as f64;
        hits / (hits + misses)
    }

    pub async fn get(&self, id: &MemoryId) -> Option<MemoryNote> {
        if let Some(memory) = self.cache.get(id) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            Some(memory.clone())
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }
}
```

---

## Implementation Plan

### Phase 1: Agent Memory Views (2 weeks)

**Tasks**:
1. Add `visible_to` column to memories table
2. Implement `AgentMemoryView` with role-based filtering
3. Add role-specific search filters
4. Update MCP tools to accept agent role parameter
5. Test with all 4 agents

**Deliverables**:
- `src/agents/memory_view.rs` - Role-based views
- Migration for visibility tracking
- Updated MCP server with role parameter

**Success Criteria**:
- Each agent sees only relevant memories
- Search results filtered by role
- No cross-contamination of information

---

### Phase 2: Access Control (1 week)

**Tasks**:
1. Add ownership tracking (`created_by`, `modified_by`)
2. Implement permission checks
3. Add audit trail for modifications
4. Create admin override for humans
5. Test permission enforcement

**Deliverables**:
- `src/agents/access_control.rs` - Permission system
- `memory_modifications` audit table
- CLI command to view audit log

**Success Criteria**:
- Agents can only modify own memories
- All modifications tracked
- Humans can override restrictions

---

### Phase 3: Custom Importance Scoring (1 week)

**Tasks**:
1. Implement role-specific importance weights
2. Add custom scoring to search
3. Create configurable weight profiles
4. Test scoring with each agent role
5. Benchmark impact on search relevance

**Deliverables**:
- `src/agents/importance_scorer.rs` - Custom scoring
- Configuration for weight profiles
- Benchmark results

**Success Criteria**:
- Each agent gets different search rankings
- Relevance improves for role-specific queries
- Configurable per-agent weights

---

### Phase 4: Memory Prefetching (2 weeks)

**Tasks**:
1. Implement LRU cache for memories
2. Add prefetch triggers (session, phase, task)
3. Create co-access pattern detection
4. Implement cache warming strategies
5. Monitor cache hit rates

**Deliverables**:
- `src/agents/prefetcher.rs` - Prefetch engine
- `src/agents/cache.rs` - LRU cache implementation
- Cache metrics and monitoring

**Success Criteria**:
- Cache hit rate > 70%
- Latency reduced from 50-200ms to <5ms for cached
- Memory usage < 100MB for cache

---

### Phase 5: Integration & Optimization (1 week)

**Tasks**:
1. Integrate all features with orchestration engine
2. Add agent-specific dashboards
3. Create performance benchmarks
4. Tune cache sizes and weights
5. Documentation and examples

**Deliverables**:
- Complete integration with `src/orchestration/engine.py`
- Agent memory usage documentation
- Performance comparison report

---

## Performance Targets

| Metric | Current | Target | Improvement |
|--------|---------|--------|-------------|
| Search latency (cache hit) | 50-200ms | <5ms | 10-40x |
| Search relevance | 70% | 85% | +15% |
| Memory usage per agent | Shared | 50MB | Isolated |
| Cache hit rate | N/A | 70%+ | New |
| Prefetch accuracy | N/A | 60%+ | New |

---

## Testing Strategy

**Unit Tests**:
- Role-based filtering
- Permission checks
- Importance scoring formulas
- Cache eviction logic

**Integration Tests**:
- End-to-end agent memory access
- Multi-agent coordination with private memories
- Prefetching accuracy
- Cache coherence

**Performance Tests**:
- Cache hit rate measurement
- Latency reduction validation
- Memory usage under load
- Concurrent agent access

**User Acceptance**:
- Agents find relevant memories faster
- Less information overload
- Improved task completion time
- Reduced context switching

---

## Success Metrics

**Quantitative**:
- Search latency: <5ms for cache hits (90%+ hit rate)
- Search relevance: +15% improvement per agent
- Task completion time: -20% reduction
- Context switching: -30% reduction

**Qualitative**:
- Agents more autonomous
- Less manual memory management
- Clearer agent specialization
- Better audit trail

---

## Risks & Mitigations

**Risk**: Cache thrashing (low hit rate)
**Mitigation**: Tune cache size, prefetch more aggressively, monitor patterns

**Risk**: Permission system too restrictive
**Mitigation**: Default to permissive, admin override, configurable policies

**Risk**: Custom scoring reduces accuracy
**Mitigation**: A/B testing, fallback to default scoring, tunable weights

**Risk**: Prefetching wastes resources
**Mitigation**: Lazy prefetch, monitor hit rates, adaptive patterns

---

## Future Enhancements

**Collaborative Filtering**:
- "Agents who accessed this memory also accessed..."
- Cross-agent memory recommendations

**Memory Sharing Protocols**:
- Explicit memory sharing between agents
- Temporary permissions for handoffs
- Expiring access grants

**Adaptive Prefetching**:
- Learn prefetch patterns from usage
- Personalized prefetch per agent instance
- Context-aware predictions

**Memory Compression**:
- Summarize old memories
- Archive rarely accessed content
- Progressive detail loading

---

## References

- [LRU cache implementation in Rust](https://docs.rs/lru/latest/lru/)
- [Prefetching in distributed systems](https://dl.acm.org/doi/10.1145/3373376.3378477)
- [Role-based access control](https://csrc.nist.gov/projects/role-based-access-control)

---

**Last Updated**: 2025-10-27
**Author**: Mnemosyne Development Team
**Status**: Ready for implementation (after v2.0 Phase 1-3 complete)
