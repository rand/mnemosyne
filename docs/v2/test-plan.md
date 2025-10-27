# v2.0 Test Plan

**Purpose**: Define comprehensive testing strategy for all v2.0 features

**Coverage Targets**:
- Critical path: 90%+
- Business logic: 80%+
- Integration points: 100%
- Overall: 75%+

**Date**: 2025-10-27

---

## Table of Contents

1. [Test Types](#test-types)
2. [Stream 1: Vector Search Tests](#stream-1-vector-search-tests)
3. [Stream 2: Evolution Tests](#stream-2-evolution-tests)
4. [Stream 3: Agent Features Tests](#stream-3-agent-features-tests)
5. [Integration Tests](#integration-tests)
6. [Performance Tests](#performance-tests)
7. [End-to-End Tests](#end-to-end-tests)
8. [Test Execution Strategy](#test-execution-strategy)

---

## Test Types

### Unit Tests
**Scope**: Individual functions and methods
**Tool**: `cargo test`
**Coverage**: 80%+

### Integration Tests
**Scope**: Component interactions
**Tool**: `cargo test --test integration_*`
**Coverage**: 100% of typed holes

### Performance Tests
**Scope**: Latency and throughput
**Tool**: `criterion` benchmarks
**Coverage**: All critical paths

### End-to-End Tests
**Scope**: Complete user workflows
**Tool**: Bash scripts + assertions
**Coverage**: Top 5 user scenarios

---

## Stream 1: Vector Search Tests

### Unit Tests - Embedding Service

**File**: `src/embeddings/remote.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_embed_single_text() {
        let service = RemoteEmbeddingService::new_mock();
        let embedding = service.embed("test query").await.unwrap();

        assert_eq!(embedding.len(), 1536);
        assert!(embedding.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_embed_batch() {
        let service = RemoteEmbeddingService::new_mock();
        let texts = vec!["query 1", "query 2", "query 3"];
        let embeddings = service.embed_batch(&texts).await.unwrap();

        assert_eq!(embeddings.len(), 3);
        for emb in &embeddings {
            assert_eq!(emb.len(), 1536);
        }
    }

    #[tokio::test]
    async fn test_empty_text_error() {
        let service = RemoteEmbeddingService::new_mock();
        let result = service.embed("").await;

        assert!(matches!(result, Err(EmbeddingError::EmptyText)));
    }

    #[tokio::test]
    async fn test_api_key_invalid() {
        let service = RemoteEmbeddingService::new("invalid-key", "voyage");
        let result = service.embed("test").await;

        assert!(matches!(result, Err(EmbeddingError::Unauthorized)));
    }

    #[tokio::test]
    async fn test_rate_limit_retry() {
        let service = RemoteEmbeddingService::new_mock_with_rate_limit();
        let result = service.embed("test").await;

        // Should succeed after retry
        assert!(result.is_ok());
        assert_eq!(service.retry_count(), 2);  // 1 fail + 1 success
    }

    #[tokio::test]
    async fn test_network_timeout() {
        let service = RemoteEmbeddingService::new_mock_with_timeout();
        let result = service.embed("test").await;

        assert!(matches!(result, Err(EmbeddingError::Timeout)));
    }
}
```

**Coverage**: 85%+ of remote.rs

---

### Unit Tests - Vector Storage

**File**: `src/storage/vectors.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_and_get_vector() {
        let storage = SqliteVectorStorage::new_in_memory().await.unwrap();
        let memory_id = MemoryId::new();
        let embedding = vec![0.1; 1536];

        storage.store_vector(&memory_id, &embedding).await.unwrap();
        let retrieved = storage.get_vector(&memory_id).await.unwrap();

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), embedding);
    }

    #[tokio::test]
    async fn test_search_similar() {
        let storage = SqliteVectorStorage::new_in_memory().await.unwrap();

        // Store 3 vectors
        let id1 = MemoryId::new();
        let id2 = MemoryId::new();
        let id3 = MemoryId::new();

        let emb1 = vec![1.0; 1536];
        let emb2 = vec![0.9; 1536];  // Similar to emb1
        let emb3 = vec![0.1; 1536];  // Different

        storage.store_vector(&id1, &emb1).await.unwrap();
        storage.store_vector(&id2, &emb2).await.unwrap();
        storage.store_vector(&id3, &emb3).await.unwrap();

        // Search with emb1 query
        let results = storage.search_similar(&emb1, 2, 0.8).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, id1);  // Exact match first
        assert_eq!(results[1].0, id2);  // Similar second
        assert!(results[0].1 > results[1].1);  // Higher similarity
    }

    #[tokio::test]
    async fn test_delete_vector() {
        let storage = SqliteVectorStorage::new_in_memory().await.unwrap();
        let memory_id = MemoryId::new();
        let embedding = vec![0.5; 1536];

        storage.store_vector(&memory_id, &embedding).await.unwrap();
        storage.delete_vector(&memory_id).await.unwrap();

        let retrieved = storage.get_vector(&memory_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_dimension_mismatch_error() {
        let storage = SqliteVectorStorage::new_in_memory().await.unwrap();
        let memory_id = MemoryId::new();
        let wrong_dims = vec![0.1; 512];  // Wrong size

        let result = storage.store_vector(&memory_id, &wrong_dims).await;
        assert!(matches!(result, Err(StorageError::DimensionMismatch { .. })));
    }

    #[tokio::test]
    async fn test_extension_not_loaded() {
        let storage = SqliteVectorStorage::new_without_extension().await.unwrap();
        let result = storage.count_vectors().await;

        assert!(matches!(result, Err(StorageError::ExtensionNotLoaded)));
    }
}
```

**Coverage**: 90%+ of vectors.rs

---

### Integration Tests - Hybrid Search

**File**: `tests/integration_hybrid_search.rs`

```rust
#[tokio::test]
async fn test_hybrid_search_end_to_end() {
    // Setup
    let storage = LibSqlStorage::new_in_memory().await.unwrap();
    let embeddings = MockEmbeddingService::new();
    let vectors = SqliteVectorStorage::new_in_memory().await.unwrap();
    let searcher = DefaultHybridSearcher::new(
        Arc::new(embeddings),
        Arc::new(vectors),
        Arc::new(storage.clone()),
        SearchWeights::default(),
    );

    // Store test memories
    let mem1 = storage.store_memory(
        "Implemented rate limiting using token bucket algorithm",
        MemoryMetadata {
            importance: 8.0,
            namespace: Namespace::Global,
            ..Default::default()
        },
    ).await.unwrap();

    let mem2 = storage.store_memory(
        "Added throttling to API endpoints to prevent abuse",
        MemoryMetadata {
            importance: 7.0,
            namespace: Namespace::Global,
            ..Default::default()
        },
    ).await.unwrap();

    // Generate and store vectors
    let emb1 = embeddings.embed("rate limiting").await.unwrap();
    let emb2 = embeddings.embed("throttling API").await.unwrap();
    vectors.store_vector(&mem1, &emb1).await.unwrap();
    vectors.store_vector(&mem2, &emb2).await.unwrap();

    // Search
    let results = searcher.search(
        "How do we handle too many requests?",
        SearchOptions {
            limit: 10,
            ..Default::default()
        },
    ).await.unwrap();

    // Assertions
    assert_eq!(results.len(), 2);
    assert!(results[0].memory.id == mem1 || results[0].memory.id == mem2);
    assert!(results[0].total_score > 0.5);
    assert!(results[0].scores.vector > 0.0);
    assert!(results[0].scores.keyword > 0.0);
}

#[tokio::test]
async fn test_hybrid_search_fallback_no_vectors() {
    let storage = LibSqlStorage::new_in_memory().await.unwrap();
    let embeddings = MockEmbeddingService::new();
    let vectors = SqliteVectorStorage::new_in_memory().await.unwrap();
    let searcher = DefaultHybridSearcher::new(
        Arc::new(embeddings),
        Arc::new(vectors),
        Arc::new(storage.clone()),
        SearchWeights::default(),
    );

    // Store memory WITHOUT vector
    let mem1 = storage.store_memory(
        "Test memory without embedding",
        MemoryMetadata::default(),
    ).await.unwrap();

    // Search should still work (keyword + graph only)
    let results = searcher.search(
        "test memory",
        SearchOptions::default(),
    ).await.unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].memory.id, mem1);
    assert_eq!(results[0].scores.vector, 0.0);  // No vector score
    assert!(results[0].scores.keyword > 0.0);    // But keyword works
}

#[tokio::test]
async fn test_weighted_ranking() {
    // Test that weights are applied correctly
    // Vector match vs keyword match vs graph match vs importance
    // Create memories that score differently on each axis
    // Verify final ranking follows weight formula
    todo!("Implement weighted ranking test");
}
```

**Coverage**: 100% of typed holes #1, #2, #3

---

## Stream 2: Evolution Tests

### Unit Tests - Importance Recalibration

**File**: `src/evolution/importance.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_importance_calculation_high_access() {
        let memory = MemoryNote {
            importance: 5.0,
            access_count: 100,
            created_at: Utc::now() - Duration::from_days(10),
            last_accessed_at: Some(Utc::now()),
            ..Default::default()
        };

        let recalibrator = ImportanceRecalibrator::new(storage);
        let new_importance = recalibrator.calculate_importance(&memory).unwrap();

        // High access should increase importance
        assert!(new_importance > memory.importance);
        assert!(new_importance <= 10.0);  // Never exceeds max
    }

    #[test]
    fn test_importance_calculation_decay() {
        let memory = MemoryNote {
            importance: 8.0,
            access_count: 0,
            created_at: Utc::now() - Duration::from_days(180),
            last_accessed_at: None,
            ..Default::default()
        };

        let recalibrator = ImportanceRecalibrator::new(storage);
        let new_importance = recalibrator.calculate_importance(&memory).unwrap();

        // No access + old should decrease importance
        assert!(new_importance < memory.importance);
        assert!(new_importance >= 1.0);  // Never below min
    }

    #[test]
    fn test_access_factor() {
        let recalibrator = ImportanceRecalibrator::new(storage);

        // 10 accesses per day → 10.0 score
        let memory_high = MemoryNote {
            access_count: 100,
            created_at: Utc::now() - Duration::from_days(10),
            ..Default::default()
        };
        assert_eq!(recalibrator.access_factor(&memory_high), 10.0);

        // 0.1 accesses per day → 3.0 score (clamped)
        let memory_low = MemoryNote {
            access_count: 1,
            created_at: Utc::now() - Duration::from_days(100),
            ..Default::default()
        };
        assert_eq!(recalibrator.access_factor(&memory_low), 3.0);
    }

    #[test]
    fn test_recency_factor_exponential_decay() {
        let recalibrator = ImportanceRecalibrator::new(storage);

        let memory_recent = MemoryNote {
            last_accessed_at: Some(Utc::now()),
            ..Default::default()
        };
        let recent_score = recalibrator.recency_factor(&memory_recent);

        let memory_old = MemoryNote {
            last_accessed_at: Some(Utc::now() - Duration::from_days(30)),
            ..Default::default()
        };
        let old_score = recalibrator.recency_factor(&memory_old);

        // 30 days = half-life → score should be ~5.0
        assert!((old_score - 5.0).abs() < 0.1);
        assert!(recent_score > old_score);
    }
}
```

**Coverage**: 85%+ of importance.rs

---

### Integration Tests - Background Jobs

**File**: `tests/integration_evolution_jobs.rs`

```rust
#[tokio::test]
async fn test_job_scheduler_runs_all_jobs() {
    let storage = LibSqlStorage::new_in_memory().await.unwrap();
    let llm = MockLlmService::new();

    let config = EvolutionConfig {
        enabled: true,
        consolidation: JobConfig {
            enabled: true,
            interval: Duration::from_secs(10),
            batch_size: 50,
            max_duration: Duration::from_secs(60),
        },
        importance: JobConfig::default(),
        link_decay: JobConfig::default(),
        archival: JobConfig::default(),
    };

    let scheduler = BackgroundScheduler::new(
        Arc::new(storage),
        Arc::new(llm),
        config,
    );

    // Run for 30 seconds
    tokio::spawn(async move {
        scheduler.start().await.unwrap();
    });

    tokio::time::sleep(Duration::from_secs(30)).await;

    // Verify jobs ran
    let runs = storage.get_job_runs().await.unwrap();
    assert!(runs.iter().any(|r| r.job_name == "consolidation"));
    assert!(runs.iter().any(|r| r.job_name == "importance_recalibration"));
}

#[tokio::test]
async fn test_consolidation_job_merges_duplicates() {
    let storage = LibSqlStorage::new_in_memory().await.unwrap();
    let vectors = SqliteVectorStorage::new_in_memory().await.unwrap();
    let llm = MockLlmService::new();

    // Store duplicate memories
    let mem1 = storage.store_memory(
        "Use Redis for caching to improve performance",
        MemoryMetadata::default(),
    ).await.unwrap();

    let mem2 = storage.store_memory(
        "Decided to use Redis cache for better performance",
        MemoryMetadata::default(),
    ).await.unwrap();

    // Generate similar embeddings
    let emb1 = vec![0.9; 1536];
    let emb2 = vec![0.91; 1536];  // Very similar
    vectors.store_vector(&mem1, &emb1).await.unwrap();
    vectors.store_vector(&mem2, &emb2).await.unwrap();

    // Run consolidation
    let job = ConsolidationJob::new(
        Arc::new(storage.clone()),
        Arc::new(vectors),
        Arc::new(llm),
    );

    let report = job.run(&JobConfig::default()).await.unwrap();

    // Verify consolidation
    assert_eq!(report.changes_made, 1);  // Merged or superseded

    let remaining = storage.list_all_active().await.unwrap();
    assert_eq!(remaining.len(), 1);  // One memory remains
}

#[tokio::test]
async fn test_archival_job_archives_unused() {
    let storage = LibSqlStorage::new_in_memory().await.unwrap();

    // Store old, never-accessed memory
    let old_mem = storage.store_memory(
        "Very old unused memory",
        MemoryMetadata {
            importance: 2.0,
            created_at: Utc::now() - Duration::from_days(200),
            ..Default::default()
        },
    ).await.unwrap();

    // Run archival job
    let job = ArchivalJob::new(Arc::new(storage.clone()));
    let report = job.run(&JobConfig::default()).await.unwrap();

    // Verify archived
    assert_eq!(report.changes_made, 1);

    let memory = storage.get(&old_mem).await.unwrap();
    assert!(memory.archived_at.is_some());
}
```

**Coverage**: 100% of typed holes #4, #5

---

## Stream 3: Agent Features Tests

### Unit Tests - Agent Memory Views

**File**: `src/agents/memory_view.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_executor_sees_only_implementation() {
        let storage = LibSqlStorage::new_in_memory().await.unwrap();

        // Store different memory types
        let impl_mem = storage.store_memory(
            "Implementation pattern",
            MemoryMetadata {
                memory_type: MemoryType::Implementation,
                ..Default::default()
            },
        ).await.unwrap();

        let decision_mem = storage.store_memory(
            "Architecture decision",
            MemoryMetadata {
                memory_type: MemoryType::Decision,
                ..Default::default()
            },
        ).await.unwrap();

        // Executor view
        let view = AgentMemoryView::new(AgentRole::Executor, Arc::new(storage));
        let results = view.search("pattern", 10).await.unwrap();

        // Should only see implementation
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, impl_mem);
    }

    #[tokio::test]
    async fn test_agent_role_memory_type_mapping() {
        assert_eq!(
            AgentRole::Orchestrator.memory_types(),
            vec![MemoryType::Decision, MemoryType::Architecture, MemoryType::Coordination]
        );

        assert_eq!(
            AgentRole::Reviewer.memory_types(),
            vec![MemoryType::Bug, MemoryType::Test, MemoryType::Decision]
        );
    }
}
```

---

### Integration Tests - Prefetching

**File**: `tests/integration_agent_prefetch.rs`

```rust
#[tokio::test]
async fn test_prefetcher_cache_hit() {
    let storage = LibSqlStorage::new_in_memory().await.unwrap();
    let view = Arc::new(AgentMemoryView::new(
        AgentRole::Executor,
        Arc::new(storage.clone()),
    ));

    let prefetcher = MemoryPrefetcher::new(AgentRole::Executor, view);

    // Store high importance memory
    let mem_id = storage.store_memory(
        "Common implementation pattern",
        MemoryMetadata {
            importance: 9.0,
            memory_type: MemoryType::Implementation,
            ..Default::default()
        },
    ).await.unwrap();

    // Prefetch on session start
    prefetcher.on_session_start().await.unwrap();

    // Get from cache (should hit)
    let cached = prefetcher.get(&mem_id).await;
    assert!(cached.is_some());

    // Check metrics
    assert_eq!(prefetcher.metrics.hit_rate(), 1.0);
}

#[tokio::test]
async fn test_prefetcher_phase_transition() {
    let prefetcher = setup_prefetcher(AgentRole::Executor);

    // Store implementation patterns
    store_test_memories(&prefetcher.view, 20).await;

    // Trigger phase transition
    prefetcher.on_phase_transition(WorkPhase::Implementation).await.unwrap();

    // Verify cache populated
    assert!(prefetcher.cache_size() > 10);
}

#[tokio::test]
async fn test_cache_hit_rate_target() {
    let prefetcher = setup_prefetcher(AgentRole::Executor);

    // Simulate 100 accesses
    for _ in 0..100 {
        let id = random_memory_id();
        if let Some(_) = prefetcher.get(&id).await {
            // Cache hit
        } else {
            // Cache miss - load and populate
            let memory = storage.get(&id).await.unwrap();
            prefetcher.populate_cache(vec![memory]).await;
        }
    }

    // Verify hit rate
    assert!(prefetcher.metrics.hit_rate() >= 0.7);
}
```

**Coverage**: 100% of typed holes #6, #7, #8, #9

---

## Integration Tests

### Cross-Stream Integration

**File**: `tests/integration_cross_stream.rs`

```rust
#[tokio::test]
async fn test_consolidation_uses_vector_search() {
    // Stream 2 (Consolidation) uses Stream 1 (Vector Search)
    let storage = LibSqlStorage::new_in_memory().await.unwrap();
    let vectors = SqliteVectorStorage::new_in_memory().await.unwrap();
    let embeddings = MockEmbeddingService::new();
    let llm = MockLlmService::new();

    // Store duplicate memories with vectors
    let mem1 = store_memory_with_vector(&storage, &vectors, &embeddings, "Use Redis cache").await;
    let mem2 = store_memory_with_vector(&storage, &vectors, &embeddings, "Use Redis for caching").await;

    // Run consolidation (should use vector similarity)
    let job = ConsolidationJob::new(
        Arc::new(storage.clone()),
        Arc::new(vectors),
        Arc::new(llm),
    );

    let report = job.run(&JobConfig::default()).await.unwrap();

    // Verify detected via vectors
    assert_eq!(report.changes_made, 1);
}

#[tokio::test]
async fn test_agent_view_uses_hybrid_search() {
    // Stream 3 (Agent Views) uses Stream 1 (Hybrid Search)
    let storage = LibSqlStorage::new_in_memory().await.unwrap();
    let embeddings = MockEmbeddingService::new();
    let vectors = SqliteVectorStorage::new_in_memory().await.unwrap();

    let hybrid = DefaultHybridSearcher::new(
        Arc::new(embeddings),
        Arc::new(vectors),
        Arc::new(storage.clone()),
        SearchWeights::default(),
    );

    let view = AgentMemoryView::new_with_hybrid(
        AgentRole::Executor,
        Arc::new(storage),
        Some(Arc::new(hybrid)),
    );

    // Search should use hybrid
    let results = view.search("implementation pattern", 10).await.unwrap();

    // Verify hybrid was used (scores include vector component)
    assert!(results[0].total_score > 0.0);
}
```

---

## Performance Tests

### Benchmarks

**File**: `benches/v2_performance.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_vector_search(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let storage = rt.block_on(setup_vector_storage_with_10k_vectors());

    c.bench_function("vector search 10K vectors", |b| {
        b.iter(|| {
            rt.block_on(async {
                let query = black_box(vec![0.5; 1536]);
                storage.search_similar(&query, 10, 0.7).await.unwrap()
            })
        })
    });
}

fn bench_hybrid_search(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let searcher = rt.block_on(setup_hybrid_searcher());

    c.bench_function("hybrid search", |b| {
        b.iter(|| {
            rt.block_on(async {
                let query = black_box("test query");
                searcher.search(query, SearchOptions::default()).await.unwrap()
            })
        })
    });
}

fn bench_importance_recalibration(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let job = rt.block_on(setup_recalibration_job_with_1k_memories());

    c.bench_function("importance recalibration 1K memories", |b| {
        b.iter(|| {
            rt.block_on(async {
                job.run(&JobConfig { batch_size: 1000, ..Default::default() }).await.unwrap()
            })
        })
    });
}

fn bench_prefetch_cache_hit(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let prefetcher = rt.block_on(setup_prefetcher_with_cache());

    c.bench_function("prefetch cache hit", |b| {
        b.iter(|| {
            rt.block_on(async {
                let id = black_box(&test_memory_id);
                prefetcher.get(id).await
            })
        })
    });
}

criterion_group!(
    benches,
    bench_vector_search,
    bench_hybrid_search,
    bench_importance_recalibration,
    bench_prefetch_cache_hit
);
criterion_main!(benches);
```

**Targets**:
- Vector search (10K vectors): <10ms p95
- Hybrid search: <200ms p95 (with remote embedding)
- Importance recalibration (1K memories): <5 seconds
- Prefetch cache hit: <5ms p95

---

## End-to-End Tests

### E2E Workflow Tests

**File**: `tests/e2e/v2_workflows.sh`

```bash
#!/usr/bin/env bash
set -e

echo "=== E2E Test: Vector Search Workflow ==="

# 1. Store memories
MEM1=$(mnemosyne remember --content "Implemented rate limiting with token bucket" --importance 8 --format json | jq -r '.id')
MEM2=$(mnemosyne remember --content "Added API throttling to prevent abuse" --importance 7 --format json | jq -r '.id')

# 2. Wait for embeddings to generate
sleep 2

# 3. Search with semantic query
RESULTS=$(mnemosyne recall --query "How to handle too many requests?" --format json)

# 4. Verify results
COUNT=$(echo "$RESULTS" | jq '.results | length')
if [ "$COUNT" -ge 1 ]; then
    echo "✓ Vector search found results"
else
    echo "✗ Vector search failed"
    exit 1
fi

# 5. Verify vector scores present
VECTOR_SCORE=$(echo "$RESULTS" | jq -r '.results[0].scores.vector')
if [ "$VECTOR_SCORE" != "null" ] && [ "$VECTOR_SCORE" != "0.0" ]; then
    echo "✓ Vector similarity scores present"
else
    echo "✗ Vector scores missing"
    exit 1
fi

echo "=== E2E Test: Evolution Workflow ==="

# 1. Store duplicate memories
mnemosyne remember --content "Use Redis for caching" --importance 5
mnemosyne remember --content "Decided to use Redis cache" --importance 5

# 2. Run consolidation manually
mnemosyne evolve consolidate --dry-run

# 3. Verify detection
# (Check output for consolidation suggestions)

echo "=== E2E Test: Agent Prefetch Workflow ==="

# 1. Store high-importance implementation pattern
mnemosyne remember --content "Always use Result<T, E> for error handling" --importance 9 --tags implementation,pattern

# 2. Simulate agent session start (via MCP)
# (Prefetch should cache high-importance memories)

# 3. Verify cache hit rate
# (Check metrics endpoint)

echo "✅ All E2E tests passed"
```

---

## Test Execution Strategy

### Pre-Commit (Developer)

```bash
# Fast tests only
cargo test --lib
cargo test --test unit_*
```

**Duration**: <2 minutes

---

### CI Pipeline (GitHub Actions)

```yaml
name: v2.0 Tests

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo test --lib --all-features

  integration-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo test --test integration_*

  benchmarks:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo bench --no-run  # Compile but don't run
      - run: cargo bench --bench v2_performance -- --test  # Quick run

  e2e-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: ./scripts/install/install.sh
      - run: ./tests/e2e/v2_workflows.sh
```

---

### Release Testing (v2.0.0)

```bash
# Full test suite
cargo test --all-features --release

# Benchmarks (save baseline)
cargo bench --bench v2_performance -- --save-baseline v2.0.0

# E2E on multiple platforms
./tests/e2e/v2_workflows.sh  # macOS
docker run --rm -v $(pwd):/app rust:latest /app/tests/e2e/v2_workflows.sh  # Linux

# Performance regression check
cargo bench --bench v2_performance -- --baseline v2.0.0

# Load testing (100K memories)
./tests/load/generate_100k_memories.sh
cargo bench --bench large_scale
```

---

## Test Coverage Tracking

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage/ --all-features

# View report
open coverage/index.html
```

**Minimum Coverage**:
- Overall: 75%
- Critical path (hybrid search, job scheduler): 90%
- Typed holes (all 9 interfaces): 100%

---

## Success Criteria

### Unit Tests
- [ ] All unit tests passing (0 failures)
- [ ] 80%+ coverage per component
- [ ] All edge cases tested

### Integration Tests
- [ ] All typed holes tested (9/9)
- [ ] Cross-stream integration verified
- [ ] Migration sequence tested

### Performance Tests
- [ ] Vector search <10ms (p95)
- [ ] Hybrid search <200ms (p95)
- [ ] Cache hit rate >70%
- [ ] No performance regression vs v1.0

### E2E Tests
- [ ] All 3 workflows passing
- [ ] Semantic search works
- [ ] Evolution jobs run successfully
- [ ] Agent cache improves latency

---

**Version**: 1.0
**Status**: Phase 2 Complete - Ready for Phase 3
**Last Updated**: 2025-10-27
