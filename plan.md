# Execution Plan - Phase 9: Comprehensive Testing

**Date**: 2025-10-26
**Phase**: Plan (Execution Order)

## Critical Path

1. Add test dependencies → Setup infrastructure → Write tests → Verify coverage
2. Integration tests first (validates components work together)
3. E2E tests second (validates user workflows)
4. Benchmarks third (establishes baseline)
5. Coverage verification fourth (ensures quality targets met)

## Parallel Streams

### Stream A: Test Infrastructure (Blocking)
**Must complete before any test implementation**

- [ ] Task A1: Add test dependencies to Cargo.toml
  - tokio-test = "0.4"
  - tempfile = "3.8"
  - criterion = "0.5"

- [ ] Task A2: Create test directory structure
  ```
  tests/
  ├── integration/
  │   └── mod.rs
  ├── e2e/
  │   └── mod.rs
  ├── fixtures/
  │   ├── mod.rs
  │   └── sample_data.rs
  └── common/
      └── mod.rs
  ```

- [ ] Task A3: Create test utilities
  - `create_test_storage()` - In-memory SQLite
  - `sample_memories()` - Test data fixtures
  - `create_test_project()` - Temp git repo with CLAUDE.md
  - `mock_llm_service()` - Optional mock for CI

**Dependencies**: None
**Estimated time**: 30 minutes

---

### Stream B: Integration Tests (After Stream A)
**Can be written in parallel after infrastructure ready**

#### Sub-stream B1: Hybrid Search Test
- [ ] Task B1.1: Write `tests/integration/hybrid_search_test.rs`
  - test_keyword_search_only
  - test_hybrid_search_with_graph_expansion
  - test_importance_weighting
  - test_recency_decay
  - test_empty_results
  - test_large_result_set

**Dependencies**: Task A1, A2, A3
**Estimated time**: 2 hours

#### Sub-stream B2: Namespace Isolation Test
- [ ] Task B2.1: Write `tests/integration/namespace_isolation_test.rs`
  - test_project_namespace_isolation
  - test_global_search
  - test_session_namespace_hierarchy

**Dependencies**: Task A1, A2, A3
**Estimated time**: 1 hour

#### Sub-stream B3: LLM Integration Test (Optional - requires API key)
- [ ] Task B3.1: Write `tests/integration/llm_enrichment_test.rs`
  - test_enrich_architecture_decision #[ignore]
  - test_enrich_bug_fix #[ignore]
  - test_link_generation #[ignore]
  - test_consolidation_merge #[ignore]
  - test_consolidation_keep_both #[ignore]

**Dependencies**: Task A1, A2, A3
**Estimated time**: 2 hours
**Note**: Mark tests with `#[ignore]` attribute, document how to run with API key

---

### Stream C: E2E Tests (After Stream A, can parallel with Stream B)

#### Sub-stream C1: MCP Workflow Test
- [ ] Task C1.1: Write `tests/e2e/mcp_workflows_test.rs`
  - test_mcp_remember_and_recall
  - test_mcp_list_with_sorting
  - test_mcp_update_memory
  - test_mcp_delete_memory
  - test_mcp_graph_traversal
  - test_mcp_consolidate_pairwise

**Dependencies**: Task A1, A2, A3
**Estimated time**: 3 hours

#### Sub-stream C2: Export Test
- [ ] Task C2.1: Write `tests/e2e/export_test.rs`
  - test_export_to_markdown
  - test_export_to_json
  - test_export_with_namespace_filter

**Dependencies**: Task A1, A2, A3
**Estimated time**: 1 hour

---

### Stream D: Benchmarks (After Stream A)
**Can parallel with B and C if desired**

- [ ] Task D1: Write `benches/retrieval_bench.rs`
  - bench_keyword_search
  - bench_hybrid_search
  - bench_graph_traversal

- [ ] Task D2: Write `benches/storage_bench.rs`
  - bench_store_memory_no_llm
  - bench_update_memory
  - bench_access_tracking

**Dependencies**: Task A1, A2, A3
**Estimated time**: 2 hours

---

### Stream E: Coverage & Documentation (After all tests written)

- [ ] Task E1: Run all tests
  ```bash
  cargo test
  cargo test --test integration
  cargo test --test e2e
  cargo test -- --ignored  # LLM tests with API key
  ```

- [ ] Task E2: Run benchmarks and record baseline
  ```bash
  cargo bench > docs/performance-baseline.md
  ```

- [ ] Task E3: Check coverage
  ```bash
  cargo tarpaulin --out Html
  open tarpaulin-report.html
  ```

- [ ] Task E4: Add tests for uncovered critical paths
  - Identify gaps from coverage report
  - Write targeted tests
  - Re-run coverage

- [ ] Task E5: Document baseline metrics
  - Create `docs/performance-baseline.md`
  - Record: retrieval latency, storage latency, memory usage
  - Include: system specs, test conditions, p50/p95/p99

**Dependencies**: All of B, C, D complete
**Estimated time**: 2 hours

---

## Dependencies Graph

```
A (Infrastructure)
├─> B1 (Hybrid Search Tests)
├─> B2 (Namespace Tests)
├─> B3 (LLM Tests)
├─> C1 (MCP E2E Tests)
├─> C2 (Export Tests)
└─> D (Benchmarks)
    └─> E (Coverage & Docs)
```

## Integration Points (Typed Holes)

### Interface 1: Test Utilities → Tests
```rust
// tests/fixtures/mod.rs
pub fn create_test_storage() -> SqliteStorage;
pub fn sample_memories(count: usize) -> Vec<MemoryNote>;
pub fn create_test_project(name: &str) -> TempDir;
```

Used by: All integration and E2E tests

### Interface 2: Test Data → Integration Tests
```rust
// tests/fixtures/sample_data.rs
pub struct TestData {
    pub database_memories: Vec<MemoryNote>,
    pub api_memories: Vec<MemoryNote>,
    pub bug_memories: Vec<MemoryNote>,
    pub links: Vec<(MemoryId, MemoryId, LinkType)>,
}

pub fn load_test_data() -> TestData;
```

Used by: Hybrid search tests, consolidation tests

### Interface 3: Mock LLM → LLM Tests
```rust
// tests/common/mock_llm.rs
pub struct MockLlmService {
    responses: HashMap<String, MockResponse>,
}

impl MockLlmService {
    pub fn with_responses(responses: HashMap<String, MockResponse>) -> Self;
}
```

Used by: CI tests when API key not available

---

## Execution Timeline

### Session 1 (Now): Infrastructure + First Tests
- [ ] Complete Stream A (30 min)
- [ ] Start B1: Hybrid search test (2 hours)
- **Checkpoint**: Commit infrastructure + first integration test

### Session 2: Core Integration Tests
- [ ] Complete B2: Namespace tests (1 hour)
- [ ] Start B3: LLM tests (2 hours, mark ignored)
- **Checkpoint**: Commit integration test suite

### Session 3: E2E Tests
- [ ] Complete C1: MCP workflow tests (3 hours)
- [ ] Complete C2: Export tests (1 hour)
- **Checkpoint**: Commit E2E test suite

### Session 4: Benchmarks & Coverage
- [ ] Complete D: Benchmarks (2 hours)
- [ ] Complete E: Coverage verification (2 hours)
- **Checkpoint**: Commit benchmarks + coverage docs

**Total Estimated Time**: 12-15 hours across 4 sessions

---

## Rollback Strategy

If tests reveal fundamental issues:

1. **Integration test failures** → Fix component bugs, update tests
2. **E2E test failures** → Review MCP protocol implementation
3. **Performance failures** → Profile and optimize before proceeding
4. **Coverage gaps** → Add targeted unit tests

**Critical**: Do not proceed to next stream until previous stream passing

---

## Quality Gates

### Before committing infrastructure:
- [ ] Directory structure correct
- [ ] Dependencies compile
- [ ] Helper functions have doc comments

### Before committing integration tests:
- [ ] All tests passing or properly ignored
- [ ] Test data realistic
- [ ] Teardown proper (no resource leaks)

### Before committing E2E tests:
- [ ] Full workflows tested
- [ ] Error paths covered
- [ ] Timing assertions reasonable

### Before committing benchmarks:
- [ ] Benchmarks run successfully
- [ ] Baseline metrics documented
- [ ] System specs recorded

### Before declaring Phase 9 complete:
- [ ] All tests passing (except ignored)
- [ ] Coverage >= 70%
- [ ] Critical path coverage >= 90%
- [ ] Performance targets met
- [ ] Documentation updated

---

## Notes

- LLM tests are optional but valuable - run with `cargo test -- --ignored` when API key available
- Use `cargo test -- --nocapture` for debugging
- Benchmarks should run on consistent hardware for baseline
- Coverage tools may miss async code - manually verify critical async paths
