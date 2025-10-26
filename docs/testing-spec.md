# Comprehensive Testing Specification - Mnemosyne

**Phase**: 9 - Testing & Validation
**Status**: Spec Phase
**Date**: 2025-10-26

## Overview

This document defines the comprehensive testing strategy for Mnemosyne to ensure reliability, performance, and correctness across all components.

## Current Test Status

**Existing Tests**: 27 unit tests passing
- Config management (4 tests)
- Storage backend (8 tests)
- Type system (3 tests)
- LLM service (1 test, ignored - requires API key)
- Namespace detection (12 tests)
- Error handling (varies)

**Coverage Gaps**:
- No integration tests between components
- No E2E tests for MCP workflows
- No performance benchmarks
- No slash command testing
- Limited error scenario coverage

## Testing Requirements

### 1. Unit Tests (Target: 90% coverage for critical paths)

**Components to Test**:

#### Storage Layer (`src/storage/sqlite.rs`)
- [x] Basic CRUD operations (existing)
- [x] Graph traversal (existing)
- [ ] Hybrid search algorithm
- [ ] FTS5 keyword search edge cases
- [ ] Transaction rollback scenarios
- [ ] Concurrent access patterns
- [ ] Database migration failures

#### LLM Service (`src/services/llm.rs`)
- [ ] Memory enrichment with various content types
- [ ] Link generation logic
- [ ] Consolidation decision accuracy
- [ ] API error handling (rate limits, network failures)
- [ ] Response parsing edge cases
- [ ] Empty/malformed content handling

#### Namespace Detection (`src/services/namespace.rs`)
- [x] Git root detection (existing)
- [x] CLAUDE.md parsing (existing)
- [ ] Invalid git repositories
- [ ] Malformed CLAUDE.md files
- [ ] Nested git repositories
- [ ] Permission errors

#### MCP Server (`src/mcp/`)
- [ ] JSON-RPC protocol compliance
- [ ] Request validation
- [ ] Error response formatting
- [ ] Concurrent request handling
- [ ] Tool parameter validation

#### MCP Tools (`src/mcp/tools.rs`)
- [ ] All 8 tools with valid inputs
- [ ] Invalid parameter handling
- [ ] Namespace parsing edge cases
- [ ] Empty result sets
- [ ] Large result sets (>1000 memories)

### 2. Integration Tests (Target: 80% coverage)

**Test Scenarios**:

#### Storage + LLM Integration
```rust
#[tokio::test]
async fn test_store_memory_with_enrichment() {
    // 1. Create storage and LLM service
    // 2. Store raw content
    // 3. Verify LLM enrichment occurred
    // 4. Verify keywords/tags extracted
    // 5. Verify links generated
    // 6. Verify searchable via FTS5
}
```

#### Namespace + Storage Integration
```rust
#[tokio::test]
async fn test_namespace_isolation() {
    // 1. Create memories in different namespaces
    // 2. Search within namespace
    // 3. Verify isolation
    // 4. Search globally
    // 5. Verify all returned
}
```

#### Hybrid Search Integration
```rust
#[tokio::test]
async fn test_hybrid_search_end_to_end() {
    // 1. Store 10+ memories with links
    // 2. Perform keyword search
    // 3. Verify graph expansion
    // 4. Verify ranking correctness
    // 5. Verify recency decay
    // 6. Verify importance weighting
}
```

#### Consolidation Workflow
```rust
#[tokio::test]
async fn test_consolidation_workflow() {
    // 1. Create similar memories
    // 2. Call consolidate
    // 3. Verify LLM recommendation
    // 4. Apply consolidation
    // 5. Verify merge/supersede occurred
    // 6. Verify links preserved
}
```

### 3. End-to-End Tests (Target: 100% of user workflows)

**MCP Tool Workflows**:

#### Workflow 1: Store and Recall
```rust
#[tokio::test]
async fn test_e2e_store_and_recall() {
    // 1. Start MCP server
    // 2. Send remember request via JSON-RPC
    // 3. Verify response with memory_id
    // 4. Send recall request with query
    // 5. Verify memory returned
    // 6. Verify score and ranking
}
```

#### Workflow 2: Context Loading
```rust
#[tokio::test]
async fn test_e2e_context_loading() {
    // 1. Store multiple memories in project
    // 2. Call context tool
    // 3. Verify recent memories included
    // 4. Verify important memories included
    // 5. Verify graph built correctly
}
```

#### Workflow 3: Memory Evolution
```rust
#[tokio::test]
async fn test_e2e_memory_evolution() {
    // 1. Store initial memory
    // 2. Update content
    // 3. Verify version tracking
    // 4. Store superseding memory
    // 5. Verify supersession marked
    // 6. Search only returns active
}
```

#### Workflow 4: Export and Import
```rust
#[tokio::test]
async fn test_e2e_export() {
    // 1. Store diverse memories
    // 2. Export to markdown
    // 3. Verify format
    // 4. Export to JSON
    // 5. Verify structure
    // (Import test deferred to v2.0)
}
```

### 4. Performance Benchmarks (Target: Meet specified thresholds)

**Benchmarks to Implement**:

#### Retrieval Latency
- **Target**: <200ms p95
- **Test**: Search query with 1000 memories in DB
- **Measure**:
  - Keyword search latency
  - Hybrid search latency
  - Graph traversal latency
  - Total end-to-end latency

#### Storage Latency
- **Target**: <500ms p95 (including LLM)
- **Test**: Store memory with enrichment
- **Measure**:
  - LLM enrichment time
  - Link generation time
  - Database insert time
  - Total end-to-end time

#### Memory Usage
- **Target**: <100MB idle, <500MB under load
- **Test**: Server with 10,000 memories
- **Measure**:
  - Idle RSS
  - Peak RSS during search
  - Memory growth over time

#### Throughput
- **Target**: 100+ concurrent requests
- **Test**: Concurrent recall requests
- **Measure**:
  - Requests per second
  - Error rate
  - Response time distribution

#### Database Size
- **Target**: ~1MB per 1000 memories
- **Test**: Store 10,000 memories
- **Measure**:
  - Database file size
  - FTS5 index size
  - Growth rate

### 5. Error Scenario Tests

**Scenarios to Cover**:

#### Network Failures
- [ ] LLM API timeout
- [ ] LLM API rate limiting
- [ ] LLM API authentication failure
- [ ] Malformed API responses

#### Database Failures
- [ ] SQLite file locked
- [ ] Disk full during write
- [ ] Corrupted database file
- [ ] Migration failure

#### Input Validation
- [ ] Empty content
- [ ] Extremely long content (>1MB)
- [ ] Invalid namespace format
- [ ] Invalid memory IDs
- [ ] Malformed JSON-RPC requests

#### Resource Exhaustion
- [ ] Too many concurrent connections
- [ ] Memory limit exceeded
- [ ] Query timeout

### 6. Slash Command Tests

**Test Approach**: Integration tests that verify slash command expansion

```rust
#[test]
fn test_memory_store_command_parsing() {
    // 1. Parse command arguments
    // 2. Verify namespace detection
    // 3. Verify importance extraction
    // 4. Verify content extraction
}

#[tokio::test]
async fn test_memory_search_command_output() {
    // 1. Store test memories
    // 2. Execute search command
    // 3. Verify output formatting
    // 4. Verify star ratings
    // 5. Verify table layout
}
```

## Test Organization

### File Structure
```
tests/
├── integration/
│   ├── hybrid_search_test.rs
│   ├── namespace_isolation_test.rs
│   ├── consolidation_workflow_test.rs
│   ├── mcp_protocol_test.rs
│   └── fixtures/
│       ├── sample_memories.json
│       └── test_project/
│           └── CLAUDE.md
├── e2e/
│   ├── mcp_workflows_test.rs
│   ├── slash_commands_test.rs
│   └── export_test.rs
└── benches/
    ├── retrieval_bench.rs
    ├── storage_bench.rs
    ├── throughput_bench.rs
    └── memory_usage_bench.rs
```

### Test Data Fixtures

Create reusable test data:

```rust
// tests/fixtures/mod.rs
pub fn sample_memories() -> Vec<MemoryNote> {
    vec![
        MemoryNote {
            content: "Decided to use PostgreSQL...".to_string(),
            memory_type: MemoryType::ArchitectureDecision,
            importance: 9,
            tags: vec!["database".into(), "postgresql".into()],
            // ... rest of fields
        },
        // ... more samples
    ]
}

pub fn sample_links() -> Vec<(MemoryId, MemoryId, LinkType)> {
    // Predefined link structure for graph tests
}

pub fn create_test_project() -> TempDir {
    // Creates temp directory with git repo and CLAUDE.md
}
```

## Coverage Targets

### Overall Coverage
- **Critical Path**: 90%+ (storage, search, MCP tools)
- **Business Logic**: 80%+ (LLM integration, consolidation)
- **UI Layer**: 60%+ (slash commands, formatting)
- **Overall**: 70%+

### Component-Specific Targets
- Storage backend: 95%
- MCP tools: 90%
- LLM service: 85%
- Namespace detection: 90%
- Error handling: 80%

## Test Execution

### Local Development
```bash
# Run all tests
cargo test

# Run specific test suite
cargo test --test integration

# Run with output
cargo test -- --nocapture

# Run benchmarks
cargo bench

# Check coverage
cargo tarpaulin --out Html
```

### CI/CD Pipeline
```yaml
test:
  - cargo test --all-features
  - cargo test --test integration
  - cargo test --test e2e
  - cargo bench --no-run  # Verify benchmarks compile
  - cargo tarpaulin --out Xml
```

## Performance Baseline

**Establish baseline metrics** on reference hardware:
- MacBook Pro M1, 16GB RAM
- SQLite database with 10,000 memories
- Concurrent load: 10 requests

Record results in `docs/performance-baseline.md` for regression detection.

## Test Quality Standards

### All Tests Must:
- [ ] Have descriptive names (what they test)
- [ ] Include setup and teardown
- [ ] Use fixtures for test data
- [ ] Not depend on execution order
- [ ] Clean up resources (temp files, DB connections)
- [ ] Use assertions with clear failure messages
- [ ] Document complex test logic

### Integration Tests Must:
- [ ] Test realistic scenarios
- [ ] Use production-like configurations
- [ ] Verify all side effects
- [ ] Test error paths
- [ ] Measure and assert timing

### Benchmarks Must:
- [ ] Run for sufficient iterations (>100)
- [ ] Use representative data
- [ ] Report statistical measures (mean, p50, p95, p99)
- [ ] Include warmup period
- [ ] Control for external factors

## Success Criteria

Phase 9 is complete when:
- [ ] All unit tests passing (existing 27 + new)
- [ ] Integration test suite implemented (15+ tests)
- [ ] E2E test suite implemented (8+ workflows)
- [ ] Performance benchmarks established
- [ ] Coverage targets met (70%+ overall)
- [ ] Performance targets met (retrieval <200ms, storage <500ms)
- [ ] CI/CD pipeline configured
- [ ] Baseline metrics documented

## Deferred to v2.0

- Load testing with >100k memories
- Chaos engineering tests
- Fuzz testing for input validation
- Cross-platform compatibility testing (Windows, Linux)
- Memory leak detection (long-running tests)
- Slash command interactive tests (require user input)

## Dependencies

**Test Environment Requirements**:
- SQLite 3.43+
- Anthropic API key (for LLM tests)
- Git (for namespace detection tests)
- Sufficient disk space (~1GB for test databases)
- Network access (for LLM API tests)

**Test Libraries**:
- `tokio-test` - Async test support
- `tempfile` - Temporary directories
- `mockito` or `wiremock` - HTTP mocking
- `criterion` - Benchmarking framework
- `tarpaulin` - Coverage reporting

## Next Steps

1. **Full Spec Phase**: Design detailed test cases
2. **Plan Phase**: Create execution order and dependencies
3. **Implementation Phase**: Write tests following the plan
4. **Validation Phase**: Run tests and verify coverage
5. **Documentation Phase**: Record baseline metrics
