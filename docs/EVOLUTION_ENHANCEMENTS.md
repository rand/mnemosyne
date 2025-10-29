# Evolution System Enhancements

This document details planned enhancements to the memory evolution system for intelligent, LLM-guided optimization.

## Status: PARTIAL IMPLEMENTATION

The evolution system has a solid foundation with 4 job types implemented:
- ✅ **Consolidation**: Duplicate detection (keyword-based, heuristic decisions)
- ✅ **Importance Recalibration**: Usage-based importance decay
- ✅ **Link Decay**: Untraversed link strength reduction
- ✅ **Archival**: Automatic archival of unused memories

**Missing**: LLM-guided intelligent decision making for consolidation

**When to implement**: After LLM service integration is more robust and cost-optimized

**Estimated effort**: 3-4 hours

---

## Enhancement 1: LLM-Guided Consolidation (3-4 hours)

### Current State

The consolidation job uses **heuristic-based decisions**:

```rust
// src/evolution/consolidation.rs:181-239

fn make_consolidation_decision(&self, cluster: &MemoryCluster) -> ConsolidationDecision {
    // Very high similarity (>0.95) → Supersede (keep newer)
    if cluster.avg_similarity > 0.95 {
        return supersede_older_with_newer();
    }

    // High similarity (0.85-0.95) → Keep (recommend manual merge)
    if cluster.avg_similarity > 0.85 {
        return keep_and_suggest_merge();
    }

    // Moderate similarity → Keep separate
    return keep_separate();
}
```

**Limitations:**
- Simple threshold-based logic
- Cannot understand semantic nuance
- Misses subtle differences between similar memories
- Cannot make informed merge decisions
- Manual intervention required for 0.85-0.95 similarity range

### Enhancement Needed

Replace heuristic logic with **LLM-guided decision making**:

```rust
// src/evolution/consolidation.rs

use crate::services::LlmService;

pub struct ConsolidationJob {
    storage: Arc<LibsqlStorage>,
    llm: Arc<LlmService>,  // ADD THIS
}

impl ConsolidationJob {
    pub fn new(storage: Arc<LibsqlStorage>, llm: Arc<LlmService>) -> Self {
        Self { storage, llm }
    }

    /// Make consolidation decision using LLM
    async fn make_llm_consolidation_decision(
        &self,
        cluster: &MemoryCluster,
    ) -> Result<ConsolidationDecision, JobError> {
        // Build prompt for LLM
        let prompt = self.build_consolidation_prompt(cluster);

        // Query LLM for decision
        let response = self.llm
            .query_with_structured_output(&prompt)
            .await
            .map_err(|e| JobError::ExecutionError(e.to_string()))?;

        // Parse LLM response
        self.parse_llm_decision(response, cluster)
    }

    /// Build prompt for LLM consolidation decision
    fn build_consolidation_prompt(&self, cluster: &MemoryCluster) -> String {
        format!(
            r#"You are analyzing similar memories for potential consolidation.

Cluster contains {} memories with average similarity of {:.2}:

{}

Analyze these memories and decide:
1. **MERGE**: Combine into single memory (truly duplicates)
2. **SUPERSEDE**: One memory obsoletes another (newer/better version)
3. **KEEP**: Keep separate (meaningful differences despite similarity)

For MERGE or SUPERSEDE, explain:
- Which memories to consolidate
- Key information to preserve
- Rationale for decision

For KEEP, explain:
- What meaningful differences exist
- Why both should be retained

Respond in JSON format:
{{
    "action": "MERGE" | "SUPERSEDE" | "KEEP",
    "primary_memory_id": "mem_xxx",
    "secondary_memory_ids": ["mem_yyy", "mem_zzz"],
    "rationale": "explanation",
    "preserved_content": "key facts to keep from secondary memories"
}}
"#,
            cluster.memories.len(),
            cluster.avg_similarity,
            self.format_cluster_for_llm(cluster)
        )
    }

    /// Format cluster memories for LLM consumption
    fn format_cluster_for_llm(&self, cluster: &MemoryCluster) -> String {
        cluster.memories.iter().enumerate()
            .map(|(i, mem)| {
                format!(
                    "Memory {}: [{}]\n  ID: {}\n  Created: {}\n  Summary: {}\n  Content: {}\n  Keywords: {}\n",
                    i + 1,
                    mem.memory_type,
                    mem.id,
                    mem.created_at.format("%Y-%m-%d"),
                    mem.summary,
                    &mem.content[..mem.content.len().min(200)],  // First 200 chars
                    mem.keywords.join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Parse LLM response into consolidation decision
    fn parse_llm_decision(
        &self,
        response: String,
        cluster: &MemoryCluster,
    ) -> Result<ConsolidationDecision, JobError> {
        // Parse JSON response
        let decision: LlmConsolidationResponse = serde_json::from_str(&response)
            .map_err(|e| JobError::ExecutionError(format!("Failed to parse LLM response: {}", e)))?;

        // Convert to ConsolidationDecision
        match decision.action.to_uppercase().as_str() {
            "MERGE" => {
                Ok(ConsolidationDecision {
                    action: ConsolidationAction::Merge,
                    memory_ids: cluster.memories.iter().map(|m| m.id.clone()).collect(),
                    superseded_id: None,
                    superseding_id: Some(decision.primary_memory_id),
                    reason: decision.rationale,
                })
            }
            "SUPERSEDE" => {
                Ok(ConsolidationDecision {
                    action: ConsolidationAction::Supersede,
                    memory_ids: cluster.memories.iter().map(|m| m.id.clone()).collect(),
                    superseded_id: decision.secondary_memory_ids.first().cloned(),
                    superseding_id: Some(decision.primary_memory_id),
                    reason: decision.rationale,
                })
            }
            "KEEP" => {
                Ok(ConsolidationDecision {
                    action: ConsolidationAction::Keep,
                    memory_ids: cluster.memories.iter().map(|m| m.id.clone()).collect(),
                    superseded_id: None,
                    superseding_id: None,
                    reason: decision.rationale,
                })
            }
            _ => Err(JobError::ExecutionError(format!(
                "Unknown action: {}",
                decision.action
            ))),
        }
    }
}

#[derive(Debug, Deserialize)]
struct LlmConsolidationResponse {
    action: String,
    primary_memory_id: MemoryId,
    secondary_memory_ids: Vec<MemoryId>,
    rationale: String,
    preserved_content: String,
}
```

### Benefits

**Intelligent Decisions:**
- Understands semantic nuance beyond similarity scores
- Recognizes when memories complement vs. duplicate each other
- Makes informed merge decisions with content preservation
- Explains reasoning for auditability

**Better Outcomes:**
- Fewer false positives (keeping actually-different memories)
- Fewer false negatives (merging subtle duplicates)
- Automatic handling of 0.85-0.95 similarity range
- Quality consolidation without manual intervention

**Example Scenarios LLM Handles Well:**

1. **Temporal Evolution**: Recognizes newer memory supersedes older outdated version
2. **Complementary Details**: Keeps memories with different but related information
3. **Format Variations**: Merges same information in different formats (code vs. prose)
4. **Abstraction Levels**: Distinguishes high-level concepts from specific implementations

---

## Enhancement 2: Vector Similarity Integration (1-2 hours)

### Current State

Consolidation uses **keyword overlap as similarity proxy**:

```rust
// src/evolution/consolidation.rs:61-77

fn keyword_overlap(&self, m1: &MemoryNote, m2: &MemoryNote) -> f32 {
    let keywords1: HashSet<_> = m1.keywords.iter().map(|k| k.to_lowercase()).collect();
    let keywords2: HashSet<_> = m2.keywords.iter().map(|k| k.to_lowercase()).collect();

    // Jaccard similarity
    let intersection = keywords1.intersection(&keywords2).count() as f32;
    let union = keywords1.union(&keywords2).count() as f32;

    intersection / union
}
```

**Limitations:**
- Keyword-based similarity is crude
- Misses semantic relationships
- Requires exact keyword matches
- No understanding of synonyms or related concepts

### Enhancement Needed

Use **actual vector similarity** (already available from Phase 11):

```rust
// src/evolution/consolidation.rs

async fn find_duplicate_candidates(&self, batch_size: usize) -> Result<Vec<(MemoryNote, MemoryNote, f32)>, JobError> {
    // Get active memories WITH embeddings
    let memories = self
        .storage
        .list_all_active_with_embeddings(Some(batch_size))
        .await
        .map_err(|e| JobError::ExecutionError(e.to_string()))?;

    let mut candidates = Vec::new();

    // Use vector similarity instead of keyword overlap
    for i in 0..memories.len() {
        for j in (i + 1)..memories.len() {
            let mem1 = &memories[i];
            let mem2 = &memories[j];

            // Calculate cosine similarity using embeddings
            if let (Some(emb1), Some(emb2)) = (&mem1.embedding, &mem2.embedding) {
                let similarity = cosine_similarity(emb1, emb2);

                // High vector similarity indicates potential duplicate
                if similarity > 0.90 {
                    candidates.push((mem1.clone(), mem2.clone(), similarity));
                }
            }
        }
    }

    Ok(candidates)
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}
```

### Benefits

**Semantic Understanding:**
- Detects similarity even with different wording
- Recognizes synonyms and related concepts
- More accurate duplicate detection
- Reduces false positives and false negatives

**Optimized Performance:**
- Leverage existing embedding infrastructure
- No additional embedding generation needed (already done on storage)
- Efficient cosine similarity computation

---

## Enhancement 3: Hybrid Decision Mode (Optional, 1 hour)

### Goal

Provide **fallback to heuristics** when LLM is unavailable or too expensive.

```rust
// src/evolution/config.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationConfig {
    /// Decision mode
    pub decision_mode: DecisionMode,

    /// LLM service configuration
    pub llm_config: Option<LlmConfig>,

    /// Cost limit per consolidation run
    pub max_cost_per_run_usd: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecisionMode {
    /// Use heuristics only (fast, free, less accurate)
    Heuristic,

    /// Use LLM for all decisions (slow, costs money, most accurate)
    LlmAlways,

    /// Use LLM selectively (hybrid approach)
    LlmSelective {
        /// Only use LLM for similarity in this range
        llm_range: (f32, f32),  // e.g., (0.80, 0.95)

        /// Use heuristics outside range
        heuristic_fallback: bool,
    },

    /// Try LLM, fall back to heuristics on error
    LlmWithFallback,
}

impl ConsolidationJob {
    async fn make_consolidation_decision(
        &self,
        cluster: &MemoryCluster,
        config: &ConsolidationConfig,
    ) -> Result<ConsolidationDecision, JobError> {
        match config.decision_mode {
            DecisionMode::Heuristic => {
                self.make_heuristic_decision(cluster)
            }
            DecisionMode::LlmAlways => {
                self.make_llm_consolidation_decision(cluster).await
            }
            DecisionMode::LlmSelective { llm_range, heuristic_fallback } => {
                let similarity = cluster.avg_similarity;

                if similarity >= llm_range.0 && similarity <= llm_range.1 {
                    // Use LLM for ambiguous cases
                    self.make_llm_consolidation_decision(cluster).await
                } else if heuristic_fallback {
                    // Use heuristics for clear cases
                    Ok(self.make_heuristic_decision(cluster))
                } else {
                    // Skip if outside range and no fallback
                    Ok(ConsolidationDecision::keep_all(cluster, "Outside LLM range"))
                }
            }
            DecisionMode::LlmWithFallback => {
                match self.make_llm_consolidation_decision(cluster).await {
                    Ok(decision) => Ok(decision),
                    Err(e) => {
                        tracing::warn!("LLM decision failed, falling back to heuristics: {}", e);
                        Ok(self.make_heuristic_decision(cluster))
                    }
                }
            }
        }
    }
}
```

### Configuration Example

```toml
[evolution.consolidation]
enabled = true
interval = 86400  # 24 hours
batch_size = 100
max_duration = 300  # 5 minutes

# Decision mode
decision_mode = "llm_selective"
llm_range = [0.80, 0.95]  # Use LLM only for ambiguous cases
heuristic_fallback = true  # Use heuristics for obvious cases
max_cost_per_run_usd = 0.50  # Budget limit

[evolution.consolidation.llm]
model = "claude-3-haiku-20240307"
max_tokens = 1000
temperature = 0.1
```

---

## Testing Requirements

### LLM Integration Tests

```rust
#[tokio::test]
async fn test_llm_consolidation_merge() {
    // Given: Two nearly identical memories
    let mem1 = create_memory("Rust uses ownership for memory safety");
    let mem2 = create_memory("Rust's ownership model ensures memory safety");

    // When: LLM analyzes cluster
    let decision = job.make_llm_consolidation_decision(&cluster).await.unwrap();

    // Then: Should recommend merge
    assert_eq!(decision.action, ConsolidationAction::Merge);
    assert!(decision.reason.contains("duplicate"));
}

#[tokio::test]
async fn test_llm_consolidation_supersede() {
    // Given: Old and new version of same information
    let old = create_memory_at("Rust 1.0 released in 2015", date("2015-05-15"));
    let new = create_memory_at("Rust 1.75 released in 2023", date("2023-12-28"));

    // When: LLM analyzes cluster
    let decision = job.make_llm_consolidation_decision(&cluster).await.unwrap();

    // Then: Should supersede old with new
    assert_eq!(decision.action, ConsolidationAction::Supersede);
    assert_eq!(decision.superseding_id, Some(new.id));
}

#[tokio::test]
async fn test_llm_consolidation_keep_separate() {
    // Given: Similar but distinct memories
    let mem1 = create_memory("Rust has ownership, borrowing, and lifetimes");
    let mem2 = create_memory("Rust has traits for polymorphism");

    // When: LLM analyzes cluster
    let decision = job.make_llm_consolidation_decision(&cluster).await.unwrap();

    // Then: Should keep separate
    assert_eq!(decision.action, ConsolidationAction::Keep);
    assert!(decision.reason.contains("meaningful differences"));
}
```

### Vector Similarity Tests

```rust
#[tokio::test]
async fn test_vector_similarity_detection() {
    // Given: Semantically similar memories with different wording
    let mem1 = create_memory("Python is great for data science");
    let mem2 = create_memory("Python excels at machine learning tasks");

    // When: Finding candidates using vector similarity
    let candidates = job.find_duplicate_candidates(100).await.unwrap();

    // Then: Should detect as similar despite different keywords
    let pair = candidates.iter().find(|(m1, m2, _)| {
        (m1.id == mem1.id && m2.id == mem2.id) ||
        (m1.id == mem2.id && m2.id == mem1.id)
    });
    assert!(pair.is_some());
    assert!(pair.unwrap().2 > 0.85);  // High similarity
}
```

---

## Cost & Performance Considerations

### LLM API Costs

**Claude Haiku pricing** (as of 2024):
- Input: $0.25 / million tokens
- Output: $1.25 / million tokens

**Consolidation job estimate:**
- Average cluster: 3 memories
- Average prompt: ~500 tokens
- Average response: ~150 tokens
- Cost per decision: ~$0.0003 (0.03 cents)

**For 100 memory batch:**
- ~30-40 clusters typical
- Total cost: ~$0.01-0.012 per run
- Daily cost (24hr interval): ~$0.36/month

**Optimization strategies:**
1. Use `LlmSelective` mode (only ambiguous cases)
2. Set `max_cost_per_run_usd` limit
3. Batch decisions in single LLM call
4. Cache LLM decisions for similar clusters

### Performance Impact

**Current (heuristic-only):**
- 100 memories processed: ~50-100ms
- No external API calls

**With LLM integration:**
- Vector similarity: +20-50ms (local computation)
- LLM decisions: +2-5s per cluster (API latency)
- Total: 30-40 clusters = 60-200s per run

**Mitigation:**
- Run as background job (already async)
- Use aggressive similarity thresholds (>0.90) to reduce clusters
- Parallel LLM requests for multiple clusters
- Fallback to heuristics under time pressure

---

## Implementation Checklist

- [ ] Add `llm: Arc<LlmService>` to `ConsolidationJob`
- [ ] Implement `make_llm_consolidation_decision()`
- [ ] Implement `build_consolidation_prompt()`
- [ ] Implement `parse_llm_decision()`
- [ ] Replace keyword overlap with vector similarity
- [ ] Add `DecisionMode` configuration
- [ ] Implement hybrid decision logic
- [ ] Add cost tracking and limits
- [ ] Write comprehensive tests (merge, supersede, keep)
- [ ] Add performance profiling
- [ ] Update documentation

---

## References

- LLM Service: `src/services/llm.rs`
- Vector Storage: `src/storage/vectors.rs`
- Consolidation Job: `src/evolution/consolidation.rs`
- Evolution Config: `src/evolution/config.rs`
