# Background Memory Evolution - v2.0 Implementation Plan

**Status**: Planned
**Priority**: P1 (Quality of Life)
**Dependencies**: None (can start immediately)

---

## Overview

Implement autonomous background processes that continuously evolve and optimize the memory system without user intervention. Memories should become more organized, relevant, and accurate over time through:

1. **Periodic consolidation** - Merge duplicates, supersede outdated information
2. **Link strength decay** - Weaken irrelevant connections over time
3. **Importance recalibration** - Adjust importance based on actual usage patterns
4. **Automatic archival** - Move unused memories to archive (non-destructive)

**Goal**: Self-organizing knowledge base that improves with age, reducing manual maintenance burden.

---

## Problem Analysis

### Current Limitations

**Manual Consolidation**:
- Users must explicitly run `/memory-consolidate`
- Duplicates accumulate unnoticed
- Similar memories remain unlinked
- Outdated information persists

**Static Importance**:
- Importance set at creation never changes
- Frequently accessed memories don't gain priority
- Rarely accessed memories don't decay
- No signal of actual utility

**Inactive Memories**:
- Old, never-accessed memories clutter search results
- No automatic cleanup mechanism
- Database bloat over time
- Reduced search relevance

**Link Drift**:
- Initial links may become irrelevant
- No decay for weak connections
- Strong links never weaken
- Graph becomes dense and noisy

### Example Evolution Scenario

**Day 0**: Store "Use Redis for caching"
- Importance: 7
- Links: 0
- Access count: 0

**Week 1**: Accessed 5 times, linked to 3 related memories
- Importance: 7 → **8** (high usage)
- Links: 0 → **3** (strong: 0.9)
- Access count: 5

**Month 3**: Not accessed in 60 days, links still strong
- Importance: 8 → **6** (decay)
- Links: 3 (strong: 0.9 → **0.6** - moderate)
- Access count: 5 (stale)

**Month 6**: Still inactive, superseded by "Migrated to Valkey"
- Status: active → **archived**
- Importance: 6 (frozen)
- Links preserved but weakened
- Searchable but deprioritized

---

## Technical Design

### Architecture

```
┌─────────────────────────────────────────────────┐
│         Background Evolution Service            │
├─────────────────────────────────────────────────┤
│                                                 │
│  ┌─────────────┐  ┌──────────────┐            │
│  │ Consolidation│  │ Importance   │            │
│  │ Job          │  │ Recalibration│            │
│  │ (daily)      │  │ (weekly)     │            │
│  └─────────────┘  └──────────────┘            │
│                                                 │
│  ┌─────────────┐  ┌──────────────┐            │
│  │ Link Decay  │  │ Archival      │            │
│  │ Job          │  │ Job           │            │
│  │ (weekly)     │  │ (monthly)     │            │
│  └─────────────┘  └──────────────┘            │
│                                                 │
└─────────────────────────────────────────────────┘
          ↓                    ↓
    ┌──────────┐         ┌──────────┐
    │ SQLite   │         │ LLM      │
    │ Storage  │         │ Service  │
    └──────────┘         └──────────┘
```

### Job Scheduler

**Design**: Simple cron-like scheduler in Rust

```rust
pub struct BackgroundScheduler {
    storage: Arc<LibSqlStorage>,
    llm: Arc<LlmService>,
    config: EvolutionConfig,
}

pub struct EvolutionConfig {
    pub enabled: bool,
    pub consolidation_interval: Duration,   // 24 hours
    pub importance_interval: Duration,      // 7 days
    pub link_decay_interval: Duration,      // 7 days
    pub archival_interval: Duration,        // 30 days
}

impl BackgroundScheduler {
    pub async fn start(&self) {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(self.config.consolidation_interval) => {
                        self.run_consolidation().await;
                    }
                    _ = tokio::time::sleep(self.config.importance_interval) => {
                        self.run_importance_recalibration().await;
                    }
                    _ = tokio::time::sleep(self.config.link_decay_interval) => {
                        self.run_link_decay().await;
                    }
                    _ = tokio::time::sleep(self.config.archival_interval) => {
                        self.run_archival().await;
                    }
                }
            }
        });
    }
}
```

**Execution Strategy**:
- Runs in background thread (non-blocking)
- Only when system idle (no active queries)
- Batch processing to limit resource usage
- Incremental (process subset each run)
- Idempotent (safe to re-run)

---

## Feature 1: Periodic Consolidation

**Goal**: Automatically detect and merge duplicate or superseded memories

**Algorithm**:
```rust
pub async fn run_consolidation(&self) -> Result<ConsolidationReport> {
    // 1. Find consolidation candidates
    let candidates = self.find_duplicate_candidates().await?;

    // 2. Cluster by similarity
    let clusters = self.cluster_similar_memories(&candidates).await?;

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
                self.storage.supersede_memory(
                    decision.old_id,
                    decision.new_id
                ).await?;
                superseded += 1;
            }
            ConsolidationAction::Keep => {
                // Do nothing
            }
        }
    }

    Ok(ConsolidationReport { merged, superseded })
}
```

**Duplicate Detection**:
- Vector similarity > 0.95 (nearly identical)
- Keyword overlap > 80%
- Same namespace
- Created within 7 days

**Consolidation Rules**:
- **Merge**: Identical content, different phrasing
- **Supersede**: Newer memory replaces older
- **Keep**: Distinct information

**Safety**:
- Never delete, only mark as superseded
- Preserve audit trail
- Reversible (can un-merge)
- User notification of changes

---

## Feature 2: Importance Recalibration

**Goal**: Adjust importance based on actual usage patterns

**Algorithm**:
```rust
pub async fn run_importance_recalibration(&self) -> Result<RecalibrationReport> {
    let memories = self.storage.list_all_active().await?;
    let mut updated = 0;

    for memory in memories {
        let new_importance = self.calculate_importance(&memory).await?;

        if (new_importance - memory.importance).abs() > 1.0 {
            self.storage.update_importance(
                &memory.id,
                new_importance
            ).await?;
            updated += 1;
        }
    }

    Ok(RecalibrationReport { updated })
}
```

**Importance Formula**:
```rust
fn calculate_importance(memory: &MemoryNote) -> f32 {
    let base_importance = memory.importance;
    let access_factor = calculate_access_factor(memory);
    let recency_factor = calculate_recency_factor(memory);
    let link_factor = calculate_link_factor(memory);

    // Weighted combination
    base_importance * 0.3 +
    access_factor * 0.4 +
    recency_factor * 0.2 +
    link_factor * 0.1
}

fn calculate_access_factor(memory: &MemoryNote) -> f32 {
    let total_accesses = memory.access_count as f32;
    let days_since_creation = memory.days_since_creation();
    let accesses_per_day = total_accesses / days_since_creation.max(1.0);

    // More than 1 access/day = high importance (10)
    // Less than 1 access/week = low importance (3)
    (accesses_per_day * 10.0).clamp(3.0, 10.0)
}

fn calculate_recency_factor(memory: &MemoryNote) -> f32 {
    let days_since_access = memory.days_since_last_access();

    // Exponential decay (30-day half-life)
    10.0 * 0.5_f32.powf(days_since_access / 30.0)
}

fn calculate_link_factor(memory: &MemoryNote) -> f32 {
    let inbound_links = memory.incoming_links.len() as f32;
    let outbound_links = memory.outgoing_links.len() as f32;

    // Well-connected = more important
    ((inbound_links * 2.0 + outbound_links) / 3.0).min(10.0)
}
```

**Decay Curves**:
- **Heavy use**: Importance ↑ (max 10)
- **Moderate use**: Importance stable
- **Rare use**: Importance ↓ (min 1)
- **Never accessed**: Decay to 2, then archive

**Thresholds**:
- High importance: 8-10 (frequently accessed)
- Medium importance: 5-7 (occasionally accessed)
- Low importance: 3-4 (rarely accessed)
- Archive candidate: 1-2 (never accessed)

---

## Feature 3: Link Strength Decay

**Goal**: Weaken irrelevant connections over time

**Algorithm**:
```rust
pub async fn run_link_decay(&self) -> Result<DecayReport> {
    let links = self.storage.list_all_links().await?;
    let mut weakened = 0;
    let mut removed = 0;

    for link in links {
        let decay_factor = calculate_decay_factor(&link);
        let new_strength = link.strength * decay_factor;

        if new_strength < 0.1 {
            // Remove weak links
            self.storage.remove_link(&link.id).await?;
            removed += 1;
        } else if new_strength != link.strength {
            // Weaken moderate links
            self.storage.update_link_strength(
                &link.id,
                new_strength
            ).await?;
            weakened += 1;
        }
    }

    Ok(DecayReport { weakened, removed })
}

fn calculate_decay_factor(link: &MemoryLink) -> f32 {
    let days_since_traversal = link.days_since_last_traversal();
    let days_since_creation = link.days_since_creation();

    // Links decay if not traversed
    if days_since_traversal > 90 {
        0.5  // Halve strength after 3 months of non-use
    } else if days_since_traversal > 180 {
        0.25 // Quarter strength after 6 months
    } else if days_since_creation > 365 && days_since_traversal > 30 {
        0.8  // Slight decay for old, unused links
    } else {
        1.0  // No decay
    }
}
```

**Decay Rules**:
- Traversed links strengthen (up to 1.0)
- Untraversed links weaken (down to 0.0)
- Remove links below 0.1 strength
- Never decay manually created links

**Traversal Tracking**:
```sql
-- Track link usage
CREATE TABLE link_traversals (
    link_id TEXT NOT NULL,
    traversed_at INTEGER NOT NULL,
    search_context TEXT
);

-- Update link strength on traversal
UPDATE memory_links
SET strength = MIN(strength + 0.05, 1.0),
    last_traversed_at = ?
WHERE id = ?;
```

---

## Feature 4: Automatic Archival

**Goal**: Move unused memories to archive without deleting

**Algorithm**:
```rust
pub async fn run_archival(&self) -> Result<ArchivalReport> {
    let candidates = self.storage.find_archival_candidates().await?;
    let mut archived = 0;

    for memory in candidates {
        if self.should_archive(&memory) {
            self.storage.archive_memory(&memory.id).await?;
            archived += 1;
        }
    }

    Ok(ArchivalReport { archived })
}

fn should_archive(memory: &MemoryNote) -> bool {
    let days_since_access = memory.days_since_last_access();
    let importance = memory.importance;
    let access_count = memory.access_count;

    // Archive if:
    // - Never accessed AND >180 days old
    // - Low importance (<3) AND >90 days since last access
    // - Very low importance (<2) AND >30 days since last access

    (access_count == 0 && days_since_access > 180) ||
    (importance < 3.0 && days_since_access > 90) ||
    (importance < 2.0 && days_since_access > 30)
}
```

**Archive Behavior**:
- Memories stay in database (no deletion)
- Excluded from default search
- Searchable with `include_archived: true` flag
- Can be unarchived manually
- Preserves all metadata and links

**Schema**:
```sql
ALTER TABLE memories ADD COLUMN archived_at INTEGER;
CREATE INDEX idx_memories_archived ON memories(archived_at) WHERE archived_at IS NOT NULL;
```

---

## Configuration

**User-Configurable Settings**:
```toml
[evolution]
enabled = true

[evolution.consolidation]
enabled = true
interval_hours = 24
batch_size = 50
similarity_threshold = 0.95

[evolution.importance]
enabled = true
interval_days = 7
decay_half_life_days = 30
min_importance = 1.0
max_importance = 10.0

[evolution.link_decay]
enabled = true
interval_days = 7
decay_threshold_days = 90
removal_strength = 0.1

[evolution.archival]
enabled = true
interval_days = 30
never_accessed_days = 180
low_importance_days = 90
```

**CLI Commands**:
```bash
# Run jobs manually
mnemosyne evolve consolidate
mnemosyne evolve importance
mnemosyne evolve links
mnemosyne evolve archive

# View evolution history
mnemosyne evolve history

# Configure settings
mnemosyne config evolution.enabled true
mnemosyne config evolution.consolidation.interval_hours 48

# Dry-run (show what would change)
mnemosyne evolve consolidate --dry-run
```

---

## Implementation Plan

### Phase 1: Infrastructure (1 week)

**Tasks**:
1. Add evolution config schema
2. Implement job scheduler
3. Add evolution history tracking
4. Create CLI commands
5. Add dry-run mode

**Deliverables**:
- `src/evolution/scheduler.rs` - Background job scheduler
- `src/evolution/config.rs` - Configuration management
- `migrations/007_evolution.sql` - Schema for evolution history

---

### Phase 2: Importance Recalibration (1 week)

**Tasks**:
1. Implement importance calculation formulas
2. Add access tracking (last_accessed_at, access_count)
3. Create recalibration job
4. Add importance history tracking
5. Test decay curves

**Deliverables**:
- `src/evolution/importance.rs` - Importance recalibration logic
- Unit tests for decay formulas
- Benchmarks for recalibration performance

---

### Phase 3: Link Strength Decay (1 week)

**Tasks**:
1. Add link traversal tracking
2. Implement decay formulas
3. Create decay job
4. Add link strengthening on use
5. Test decay behavior

**Deliverables**:
- `src/evolution/links.rs` - Link decay logic
- Schema for link traversals
- Integration tests

---

### Phase 4: Automatic Archival (1 week)

**Tasks**:
1. Add archived_at column
2. Implement archival criteria
3. Create archival job
4. Add unarchive command
5. Update search to exclude archived by default

**Deliverables**:
- `src/evolution/archival.rs` - Archival logic
- Migration for archive support
- Updated search filters

---

### Phase 5: Periodic Consolidation (2 weeks)

**Tasks**:
1. Implement duplicate detection
2. Add clustering algorithm
3. Create consolidation job
4. Add LLM-guided decisions
5. Test consolidation safety

**Deliverables**:
- `src/evolution/consolidation.rs` - Consolidation logic
- Consolidation decision prompts
- Safety tests (no data loss)

---

### Phase 6: Monitoring & Observability (1 week)

**Tasks**:
1. Add evolution metrics
2. Create evolution dashboard
3. Add notifications for major changes
4. Add rollback capability
5. Create evolution report

**Deliverables**:
- Evolution metrics endpoint
- CLI command: `mnemosyne evolve report`
- Notification system

---

## Performance Considerations

**Resource Limits**:
- Max 1000 memories per batch
- Max 100ms per memory (importance calc)
- Max 5 minutes per job run
- Run only when system idle

**Optimization**:
- Incremental processing (resume on crash)
- Parallel batch processing
- Cache frequently accessed data
- Skip recently processed memories

**Monitoring**:
```rust
pub struct EvolutionMetrics {
    pub last_run: DateTime<Utc>,
    pub duration_ms: u64,
    pub memories_processed: usize,
    pub changes_made: usize,
    pub errors: usize,
}
```

---

## Testing Strategy

**Unit Tests**:
- Importance calculation formulas
- Link decay logic
- Archival criteria
- Consolidation clustering

**Integration Tests**:
- End-to-end job execution
- Multi-job coordination
- Error recovery
- Rollback behavior

**Property Tests**:
- Importance never exceeds [1, 10]
- Link strength never exceeds [0, 1]
- Archival is reversible
- No data loss on consolidation

**Manual Testing**:
- Create test database with 1000 memories
- Run evolution for 30 simulated days
- Verify expected changes
- Check search relevance improvement

---

## Success Metrics

**Quantitative**:
- Duplicate reduction: 30%+ fewer duplicates
- Search relevance: +10% improvement
- Database size: Stable or shrinking
- Active memories: 80% have importance > 5

**Qualitative**:
- Users report better search results
- Less manual consolidation needed
- Faster search (fewer irrelevant results)
- "Set it and forget it" experience

---

## Risks & Mitigations

**Risk**: Background jobs consume too many resources
**Mitigation**: Rate limiting, idle detection, user-configurable intervals

**Risk**: Incorrect importance adjustments
**Mitigation**: Conservative formulas, manual override, rollback capability

**Risk**: Accidental data loss
**Mitigation**: No deletion (only archival), audit trail, dry-run mode

**Risk**: Job failures accumulate
**Mitigation**: Error recovery, resume from checkpoint, monitoring

---

## References

- [Importance decay in memory systems](https://en.wikipedia.org/wiki/Forgetting_curve)
- [Link strength in knowledge graphs](https://arxiv.org/abs/2011.12731)
- [Automatic knowledge base consolidation](https://arxiv.org/abs/1906.08347)

---

**Last Updated**: 2025-10-27
**Author**: Mnemosyne Development Team
**Status**: Ready for implementation
