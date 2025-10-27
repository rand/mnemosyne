# Vector Similarity Search - v2.0 Implementation Plan

**Status**: Planned
**Priority**: P0 (High Impact)
**Blocked By**: fastembed compilation issues (v1.0)
**Alternative Approach**: Remote embedding services

---

## Overview

Add semantic vector search to complement the existing FTS5 keyword search and graph traversal, creating a truly hybrid retrieval system with three complementary methods:
- **Keyword search** (FTS5): Exact term matching, fast, handles technical terms
- **Vector search** (embeddings): Semantic similarity, handles paraphrasing
- **Graph traversal**: Relationship discovery, context expansion

**Goal**: Improve search accuracy from current 70-80% baseline to 85-95% by capturing semantic meaning beyond keywords.

---

## Problem Analysis

### Current Limitations

**FTS5 Keyword Search**:
- Misses paraphrased queries ("authenticate user" vs "verify login")
- Poor with synonyms ("bug" vs "defect" vs "issue")
- No semantic understanding ("fast" vs "performance")
- Requires exact term overlap

**Example Failure Case**:
```
Memory: "Implemented rate limiting to prevent API abuse"
Query:  "How do we handle too many requests?"
Result: ❌ No match (no overlapping keywords)

With vectors: ✅ High similarity (semantic equivalence)
```

### Why It Matters

1. **Natural language queries**: Users don't always use exact technical terms
2. **Cross-language concepts**: "rate limit" = "throttle" = "slow down"
3. **Architectural patterns**: Can find similar decisions by concept, not just keywords
4. **Memory consolidation**: Detect semantic duplicates even with different wording

---

## Technical Design

### Architecture

```
Query
  ↓
[1] Generate embedding (1536 dims)
  ↓
[2] Vector search in SQLite (K nearest neighbors)
  ↓
[3] Combine with FTS5 + graph results
  ↓
[4] Weighted ranking (vector=40%, keyword=30%, graph=20%, importance=10%)
  ↓
Results
```

### Embedding Service Options

#### Option A: Remote Service (Recommended for v2.0)
**Provider**: Anthropic Embeddings API (when available) or Voyage AI

**Pros**:
- No local dependencies
- Always up-to-date models
- No compilation issues
- Works on all platforms

**Cons**:
- Network latency (~50-100ms per request)
- Requires API key
- Cost per embedding (~$0.0001 per 1K tokens)

**Implementation**:
```rust
pub struct RemoteEmbeddingService {
    client: reqwest::Client,
    api_key: String,
    model: String, // "voyage-large-2-instruct" or anthropic when available
}

impl RemoteEmbeddingService {
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let response = self.client
            .post("https://api.voyageai.com/v1/embeddings")
            .json(&json!({
                "model": self.model,
                "input": text,
            }))
            .send()
            .await?;

        let data = response.json::<EmbeddingResponse>().await?;
        Ok(data.embeddings[0].clone())
    }
}
```

#### Option B: Local Embeddings (Future)
**Library**: fastembed-rs (when stable)

**Pros**:
- No network latency (<5ms)
- No API costs
- Works offline
- Privacy (no data sent externally)

**Cons**:
- Compilation issues (blocked v1.0)
- Larger binary size (+50MB)
- GPU acceleration complex
- Model updates manual

**Deferred Until**: fastembed compilation issues resolved, ONNX runtime stable

---

### Vector Storage

**Option 1: sqlite-vec Extension (Recommended)**
```sql
-- Create vector table
CREATE VIRTUAL TABLE memory_vectors USING vec0(
    memory_id TEXT PRIMARY KEY,
    embedding FLOAT[1536]
);

-- Insert vector
INSERT INTO memory_vectors(memory_id, embedding)
VALUES (?, vec_f32(?));

-- K-nearest neighbors search
SELECT memory_id, distance
FROM memory_vectors
WHERE embedding MATCH ?
ORDER BY distance
LIMIT 10;
```

**Why sqlite-vec**:
- Native SQLite extension (no external DB)
- Fast KNN search (<10ms for 10K vectors)
- SIMD-optimized distance calculations
- Supports both cosine and L2 distance
- Maintained by Alex Garcia (sqlite ecosystem contributor)

**Installation**:
```bash
# macOS ARM64
curl -L -O https://github.com/asg017/sqlite-vec/releases/download/v0.1.7/sqlite-vec-0.1.7-loadable-macos-aarch64.tar.gz
tar -xzf sqlite-vec-0.1.7-loadable-macos-aarch64.tar.gz
# Load extension in SQLite
```

**Option 2: Turso Native Vectors**
- Built-in to Turso (no extension needed)
- Same API as sqlite-vec
- Available if using Turso as backend

---

### Hybrid Search Algorithm

**Weighted Ranking**:
```rust
pub struct HybridSearchScorer {
    weights: SearchWeights,
}

pub struct SearchWeights {
    vector: f32,     // 0.40 - Semantic similarity
    keyword: f32,    // 0.30 - Exact term matching
    graph: f32,      // 0.20 - Relationship relevance
    importance: f32, // 0.10 - Memory importance
}

impl HybridSearchScorer {
    pub fn score(&self, result: &SearchCandidate) -> f32 {
        let vector_score = 1.0 - result.vector_distance; // Convert distance to similarity
        let keyword_score = result.fts5_rank;
        let graph_score = result.graph_depth_penalty;
        let importance_score = result.importance / 10.0;

        self.weights.vector * vector_score +
        self.weights.keyword * keyword_score +
        self.weights.graph * graph_score +
        self.weights.importance * importance_score
    }
}
```

**Search Flow**:
1. **Parallel retrieval**:
   - Vector search: Top 20 by cosine similarity
   - FTS5 search: Top 20 by keyword match
   - Graph traversal: 2-hop expansion from top 5 keywords
2. **Merge & deduplicate**: Combine all candidates
3. **Rerank**: Apply weighted scoring
4. **Return**: Top N results

---

## Implementation Plan

### Phase 1: Remote Embeddings (2-3 weeks)

**Tasks**:
1. ✅ Research embedding providers (Voyage AI, Cohere, OpenAI)
2. ✅ Design embedding service interface
3. Add remote embedding client:
   - HTTP client with retries
   - Rate limiting
   - Batch embedding support
   - API key management
4. Add embedding storage:
   - Install sqlite-vec extension
   - Create schema migration
   - Add CRUD operations for vectors
5. Implement vector search:
   - KNN query builder
   - Distance calculation
   - Result mapping
6. Integrate with hybrid search:
   - Parallel execution
   - Score merging
   - Weight tuning
7. Testing:
   - Unit tests for embedding client
   - Integration tests with sqlite-vec
   - Search accuracy benchmarks
   - Performance profiling

**Deliverables**:
- `src/embeddings/remote.rs` - Remote embedding service
- `src/storage/vectors.rs` - Vector CRUD operations
- `migrations/006_vector_search.sql` - Schema for vectors
- Benchmarks comparing FTS5 vs hybrid search

**Success Criteria**:
- Search accuracy: 85%+ (up from 70-80%)
- Latency: <200ms p95 (including embedding generation)
- Cost: <$0.01 per 100 searches

---

### Phase 2: Local Embeddings (Future)

**Blocked By**: fastembed compilation issues

**Tasks** (when unblocked):
1. Add fastembed-rs dependency
2. Download and bundle model weights
3. Add local embedding service:
   - Model loading
   - Inference pipeline
   - GPU acceleration (optional)
4. Add fallback logic:
   - Try local first
   - Fall back to remote on error
   - Config to prefer local/remote
5. Performance optimization:
   - Batch inference
   - Model caching
   - SIMD optimization
6. Testing:
   - Accuracy comparison (local vs remote)
   - Latency benchmarks
   - Memory usage profiling

**Deliverables**:
- `src/embeddings/local.rs` - Local embedding service
- `src/embeddings/hybrid.rs` - Local + remote fallback
- Performance comparison report

**Success Criteria**:
- Latency: <5ms for local embeddings
- Accuracy: Within 2% of remote embeddings
- Binary size increase: <100MB

---

### Phase 3: Advanced Features (Future)

**Semantic Deduplication**:
- Detect duplicate memories by vector similarity
- Suggest consolidation when distance < 0.1
- Auto-merge with LLM confirmation

**Contextual Embeddings**:
- Embed memory + context together
- Namespace-aware embeddings
- Project-specific fine-tuning

**Multi-vector Search**:
- Separate vectors for summary, content, keywords
- Weighted combination of multiple embeddings
- Better handling of long memories (chunking)

**Query Expansion**:
- Generate semantic variations of query
- Search with multiple embeddings
- Rerank combined results

---

## Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Search accuracy | 85-95% | Up from 70-80% baseline |
| Latency (remote) | <200ms p95 | Includes embedding + search |
| Latency (local) | <50ms p95 | When fastembed available |
| Memory overhead | +50MB | For sqlite-vec + model cache |
| Cost per search | <$0.0001 | Remote embedding cost |

---

## Migration Strategy

**Backwards Compatibility**:
- Vector search is additive (doesn't break existing FTS5)
- Old memories work without vectors
- Lazy embedding generation on search
- Config flag to disable vector search

**Rollout Plan**:
1. Add vector storage schema (empty)
2. Enable embedding service (opt-in)
3. Background job to embed existing memories
4. Enable hybrid search (default to keyword if no vectors)
5. Monitor accuracy and latency
6. Tune weights based on real usage

**Database Migration**:
```sql
-- Add vector table
CREATE VIRTUAL TABLE IF NOT EXISTS memory_vectors USING vec0(
    memory_id TEXT PRIMARY KEY,
    embedding FLOAT[1536],
    generated_at INTEGER NOT NULL
);

-- Add index for efficient joins
CREATE INDEX IF NOT EXISTS idx_memory_vectors_generated
ON memory_vectors(generated_at);
```

---

## Testing Strategy

**Unit Tests**:
- Embedding service (mock API responses)
- Vector storage CRUD
- Distance calculations
- Score merging

**Integration Tests**:
- End-to-end embedding → storage → search
- Hybrid search with real data
- Fallback behavior (remote → local)

**Accuracy Evaluation**:
- Curated test set of 100 query-memory pairs
- Human-labeled relevance scores
- Compare FTS5 vs hybrid vs vector-only
- Measure precision@K, recall@K, NDCG

**Performance Benchmarks**:
- Latency at 1K, 10K, 100K memories
- Throughput (queries per second)
- Memory usage scaling
- API cost estimation

---

## Risks & Mitigations

**Risk**: Remote API downtime or rate limits
**Mitigation**: Graceful fallback to FTS5-only search, caching, retry logic

**Risk**: Embedding API cost too high
**Mitigation**: Cache embeddings aggressively, batch requests, monitor budget

**Risk**: Search latency increases
**Mitigation**: Parallel search execution, aggressive result limits, caching

**Risk**: Vector search less accurate than expected
**Mitigation**: Tunable weights, A/B testing, fallback to keyword

**Risk**: SQLite vector extension installation issues
**Mitigation**: Bundle extension, detect at runtime, fallback to remote vector DB

---

## Dependencies

**Rust Crates**:
- `reqwest` - HTTP client for remote embeddings
- `serde_json` - JSON serialization
- `libsql` or `rusqlite` - SQLite with extension loading
- `fastembed` - Local embeddings (future)

**External Services**:
- Voyage AI API (or alternative) - Remote embeddings
- sqlite-vec extension - Vector search in SQLite

**System Requirements**:
- SQLite 3.41+ (for `vec0` virtual table)
- 2GB RAM minimum (for vector index)
- Network access (for remote embeddings)

---

## Success Metrics

**Quantitative**:
- Search accuracy: 85%+ (15% improvement)
- Latency p95: <200ms (remote), <50ms (local)
- User satisfaction: 4.5/5 stars
- Query diversity: 30% more unique queries

**Qualitative**:
- Natural language queries work
- Users find related memories without exact keywords
- Reduced "no results" searches
- Better consolidation suggestions

---

## References

- [sqlite-vec documentation](https://github.com/asg017/sqlite-vec)
- [Voyage AI embeddings API](https://docs.voyageai.com/)
- [Hybrid search best practices](https://www.pinecone.io/learn/hybrid-search/)
- [Embedding evaluation methods](https://arxiv.org/abs/2401.00368)

---

**Last Updated**: 2025-10-27
**Author**: Mnemosyne Development Team
**Status**: Ready for implementation (blocked on embedding service selection)
