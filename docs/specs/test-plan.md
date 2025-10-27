# Test Plan - Mnemosyne Phase 9

**Phase**: Full Spec (Design)
**Date**: 2025-10-26

## Test Types

### Unit Tests

- [x] Storage CRUD operations (existing)
- [x] Graph traversal (existing)
- [x] Config management (existing)
- [x] Namespace detection (existing)
- [ ] Hybrid search algorithm
- [ ] FTS5 search edge cases
- [ ] LLM enrichment parsing
- [ ] Link generation logic
- [ ] Consolidation decisions
- [ ] MCP protocol handling
- [ ] Tool parameter validation

### Integration Tests

- [ ] Storage + LLM: Store with enrichment
- [ ] Storage + LLM: Generate and persist links
- [ ] Namespace + Storage: Isolation verification
- [ ] Hybrid search: Keyword + graph ranking
- [ ] Consolidation: Full workflow
- [ ] MCP server + tools: Request handling
- [ ] Access tracking: Increment on recall
- [ ] Graph expansion: Multi-hop traversal

### E2E Tests

- [ ] Store and recall workflow
- [ ] Context loading workflow
- [ ] Memory update workflow
- [ ] Consolidation workflow
- [ ] Export to markdown
- [ ] Export to JSON
- [ ] List with sorting
- [ ] Graph traversal from seeds

### Performance Benchmarks

- [ ] Retrieval latency (target: <200ms p95)
- [ ] Storage latency (target: <500ms p95)
- [ ] Memory usage (target: <100MB idle)
- [ ] Throughput (target: 100+ req/s)
- [ ] Database size (target: ~1MB/1000 memories)

## Coverage Targets

- **Critical path**: 90%+ (storage, search, MCP core)
- **Business logic**: 80%+ (LLM, consolidation)
- **UI layer**: 60%+ (formatting, commands)
- **Overall**: 70%+

## Test Dependencies

### Required

- SQLite 3.43+
- Anthropic API key (for LLM tests)
- Git (for namespace tests)
- Tokio runtime
- Temp file support

### Test Libraries to Add

```toml
[dev-dependencies]
tokio-test = "0.4"
tempfile = "3.8"
criterion = "0.5"
```

## Detailed Test Cases

### Integration Test 1: Hybrid Search End-to-End

**File**: `tests/integration/hybrid_search_test.rs`

**Setup**:
1. Create in-memory SQLite database
2. Initialize storage backend
3. Create 20 test memories with varying:
   - Content similarity (5 about databases, 5 about APIs, 5 about testing, 5 misc)
   - Importance (2-10 range)
   - Creation dates (spread over 60 days)
4. Create links between related memories

**Test Cases**:

1. **test_keyword_search_only**
   - Query: "database"
   - Expand graph: false
   - Assert: Only keyword matches returned (5 memories)
   - Assert: Sorted by relevance

2. **test_hybrid_search_with_graph_expansion**
   - Query: "database decisions"
   - Expand graph: true
   - Assert: Keyword matches + graph-expanded (8-12 memories)
   - Assert: Keyword matches scored higher than graph-expanded
   - Assert: Related memories within 2 hops included

3. **test_importance_weighting**
   - Query: generic term matching multiple memories
   - Assert: Higher importance memories ranked first
   - Assert: Importance weight ~20% of total score

4. **test_recency_decay**
   - Store 2 identical memories (same content, different dates)
   - Query for the content
   - Assert: Recent memory scored higher
   - Assert: 30-day old memory has ~0.5x recency score of today's

5. **test_empty_results**
   - Query: "nonexistent term"
   - Assert: Empty results
   - Assert: No errors

6. **test_large_result_set**
   - Create 100+ memories
   - Query: broad term
   - Limit: 10
   - Assert: Exactly 10 results
   - Assert: Top 10 by score

**Teardown**:
- Close database connections
- Clean up temp files

---

### Integration Test 2: LLM Enrichment Workflow

**File**: `tests/integration/llm_enrichment_test.rs`

**Setup**:
1. Create LLM service (real or mocked)
2. Create storage backend

**Test Cases**:

1. **test_enrich_architecture_decision** (requires API key)
   - Content: "Decided to use PostgreSQL for better ACID guarantees"
   - Assert: Summary generated
   - Assert: Keywords include "postgresql", "database", "acid"
   - Assert: Tags include relevant categorization
   - Assert: Memory type = ArchitectureDecision
   - Assert: Importance 7-9

2. **test_enrich_bug_fix** (requires API key)
   - Content: "Fixed race condition in order processing by adding mutex"
   - Assert: Memory type = BugFix
   - Assert: Keywords include "race condition", "mutex"
   - Assert: Importance 5-7

3. **test_link_generation** (requires API key)
   - Store memory A: "Use PostgreSQL"
   - Store memory B: "Database connection pooling"
   - Assert: Link created between A and B
   - Assert: Link type reasonable (References or Implements)
   - Assert: Link strength 0.3-1.0

4. **test_consolidation_merge_recommendation** (requires API key)
   - Store 2 very similar memories
   - Call should_consolidate
   - Assert: Recommendation = Merge
   - Assert: Reason provided

5. **test_consolidation_keep_both** (requires API key)
   - Store 2 distinct memories
   - Call should_consolidate
   - Assert: Recommendation = KeepBoth

---

### Integration Test 3: Namespace Isolation

**File**: `tests/integration/namespace_isolation_test.rs`

**Test Cases**:

1. **test_project_namespace_isolation**
   - Store memory in "project:app1"
   - Store memory in "project:app2"
   - Search in "project:app1"
   - Assert: Only app1 memory returned

2. **test_global_search**
   - Store in "global"
   - Store in "project:app1"
   - Search with namespace=null
   - Assert: Both returned

3. **test_session_namespace_hierarchy**
   - Store in "project:app1"
   - Store in session under app1
   - Search in session namespace
   - Assert: Correct priority/visibility

---

### E2E Test 1: MCP Store and Recall

**File**: `tests/e2e/mcp_workflows_test.rs`

**Test Cases**:

1. **test_mcp_remember_and_recall**
   - Setup: Start MCP server (in-process)
   - Send JSON-RPC remember request
   - Assert: Success response with memory_id
   - Send JSON-RPC recall request
   - Assert: Memory returned in results
   - Assert: Score > 0.5

2. **test_mcp_list_with_sorting**
   - Store 5 memories
   - Call list with sort_by: "recent"
   - Assert: Sorted by created_at DESC
   - Call list with sort_by: "importance"
   - Assert: Sorted by importance DESC

3. **test_mcp_consolidate_pairwise**
   - Store 2 similar memories
   - Call consolidate with both IDs
   - Assert: Recommendation provided
   - Call consolidate with auto_apply: true
   - Assert: Action taken

---

### Performance Benchmark 1: Retrieval Latency

**File**: `benches/retrieval_bench.rs`

**Setup**:
- Database with 1000 memories
- Pre-warmed caches

**Benchmarks**:

```rust
fn bench_keyword_search(c: &mut Criterion) {
    c.bench_function("keyword_search_1000", |b| {
        b.iter(|| {
            // Search for common term
            // Measure time to first result
        });
    });
}

fn bench_hybrid_search(c: &mut Criterion) {
    c.bench_function("hybrid_search_with_graph", |b| {
        b.iter(|| {
            // Hybrid search with expand_graph=true
            // Measure total latency
        });
    });
}

fn bench_graph_traversal(c: &mut Criterion) {
    c.bench_function("graph_traverse_2hops", |b| {
        b.iter(|| {
            // Traverse from 1 seed, 2 hops
            // Measure traversal time
        });
    });
}
```

**Assertions**:
- p95 < 200ms for keyword search
- p95 < 300ms for hybrid search
- p95 < 100ms for graph traversal alone

---

### Performance Benchmark 2: Storage Latency

**File**: `benches/storage_bench.rs`

**Benchmarks**:

```rust
fn bench_store_memory_no_llm(c: &mut Criterion) {
    c.bench_function("store_memory_no_enrichment", |b| {
        b.iter(|| {
            // Store pre-enriched memory
            // Measure database insert time
        });
    });
}

fn bench_store_memory_with_llm(c: &mut Criterion) {
    // Requires API key or mocked LLM
    c.bench_function("store_memory_with_enrichment", |b| {
        b.iter(|| {
            // Store raw content
            // Measure total time including LLM calls
        });
    });
}
```

**Assertions**:
- p95 < 50ms for database insert
- p95 < 500ms for full enrichment (including LLM)

---

## Test Execution Plan

### Phase 1: Setup
1. Add test dependencies to Cargo.toml
2. Create test directory structure
3. Create test fixtures and utilities

### Phase 2: Unit Tests
1. Add hybrid search unit tests
2. Add LLM parsing unit tests
3. Add MCP protocol unit tests
4. Run: `cargo test --lib`

### Phase 3: Integration Tests
1. Implement hybrid search integration test
2. Implement LLM enrichment tests (conditional on API key)
3. Implement namespace isolation tests
4. Run: `cargo test --test integration`

### Phase 4: E2E Tests
1. Implement MCP workflow tests
2. Implement export tests
3. Run: `cargo test --test e2e`

### Phase 5: Benchmarks
1. Implement retrieval benchmarks
2. Implement storage benchmarks
3. Run: `cargo bench`
4. Record baseline metrics

### Phase 6: Coverage
1. Run: `cargo tarpaulin --out Html`
2. Review coverage report
3. Add tests for uncovered paths
4. Re-run until targets met

## Success Criteria Checklist

- [ ] All existing tests still passing (27 tests)
- [ ] Integration tests implemented (8+ tests)
- [ ] E2E tests implemented (6+ tests)
- [ ] Benchmarks implemented and run
- [ ] Coverage >= 70% overall
- [ ] Coverage >= 90% for critical path
- [ ] Performance targets met (retrieval <200ms, storage <500ms)
- [ ] Baseline metrics documented
- [ ] CI configuration updated

## Risk Mitigation

### Risk: LLM tests require API key
**Mitigation**:
- Make LLM tests conditional with `#[ignore]` attribute
- Provide mocked LLM service for CI
- Document how to run with real API key

### Risk: Tests are slow
**Mitigation**:
- Use in-memory SQLite (`:memory:`)
- Parallelize test execution
- Cache test fixtures

### Risk: Flaky tests
**Mitigation**:
- Avoid timing-dependent assertions
- Use deterministic test data
- Properly clean up resources

### Risk: Coverage tools inaccurate
**Mitigation**:
- Use multiple coverage tools (tarpaulin + llvm-cov)
- Manually review critical paths
- Add integration tests for complex scenarios
