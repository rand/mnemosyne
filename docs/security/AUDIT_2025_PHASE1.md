# Mnemosyne Security Audit - Phase 1: Stability Analysis
**Date**: 2025-11-07
**Focus**: Memory Management, Resource Leaks, Exit Code 143 (OOM)
**Status**: In Progress

## Executive Summary

This phase investigates the root cause of exit code 143 crashes (SIGTERM/OOM) in mnemosyne and establishes memory profiling infrastructure to identify resource leaks and unbounded growth patterns.

## Scope

### In Scope
- Memory allocation patterns and growth tracking
- Resource leak detection (tasks, connections, subscriptions)
- Database connection management
- Event broadcaster capacity management
- Work queue growth patterns
- CRDT history accumulation
- Embeddings cache management

### Out of Scope
- Performance optimization (addressed in Phase 4)
- Code refactoring (unless directly addressing leaks)
- Feature development

## Infrastructure Established

### 1. Memory Instrumentation (`src/diagnostics/memory.rs`)
- **MemoryTracker**: Thread-safe atomic tracking of:
  - Total/current/peak memory usage
  - Allocation counts
  - Embeddings cache size
  - Event queue size
  - Work queue size
  - Database connections
  - Spawned tasks

- **Monitoring**: Automatic 30-second logging with threshold alerts:
  - Normal: < 40% system memory
  - Moderate: 40-60%
  - High: 60-80%
  - Critical: > 80%

### 2. Diagnostic Tools
- **Collection Script**: `scripts/diagnostics/collect-memory-diagnostics.sh`
  - System memory statistics
  - Process memory maps
  - File descriptor counts
  - Crash log analysis
  - Database size statistics

### 3. Stress Testing (`tests/stress/oom_stress_test.rs`)
- Embedding generation stress test (10K memories)
- Work queue unbounded growth test (50K items)
- Spawned task cleanup verification
- Database connection leak detection

## Findings

### Critical Issues Identified

#### 1. Unbounded Event Broadcaster
**Location**: `src/api/server.rs:140-150`, `src/orchestration/actors/orchestrator.rs:111-136`

**Issue**: Multiple infinite heartbeat loops with broadcast channels:
```rust
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(10));
    loop {
        interval.tick().await;
        let _ = events_clone.broadcast(Event::heartbeat(...));
    }
});
```

**Risk**: Events accumulate without eviction, capacity=1000 but no cleanup
**Impact**: Memory grows linearly with uptime
**Priority**: **P0 - Critical**

#### 2. No Database Connection Pooling
**Location**: `src/storage/libsql.rs:51-52`

**Issue**: Each operation may create new connections:
```rust
let conn = Connection::open(db_path)
    .map_err(|e| MnemosyneError::Database(...))?;
```

**Risk**: File descriptor exhaustion, memory overhead per connection
**Impact**: Scales poorly with concurrent operations
**Priority**: **P0 - Critical**

#### 3. Spawned Task Leaks
**Location**: Multiple files with `tokio::spawn` (49 files)

**Issue**: No Drop implementations or cleanup for spawned background tasks
**Examples**:
- API server heartbeat (infinite loop)
- Orchestrator heartbeat (infinite loop)
- Semantic highlighter background analysis

**Risk**: Tasks accumulate, never cleaned up
**Impact**: Thread count grows, memory never reclaimed
**Priority**: **P0 - Critical**

#### 4. CRDT History Accumulation
**Location**: `src/ics/editor/crdt_buffer.rs`

**Issue**: Automerge retains full edit history:
```rust
use automerge;  // CRDT with full history
```

**Risk**: Unbounded growth during long editing sessions
**Impact**: Memory grows with every edit, never pruned
**Priority**: **P1 - High**

#### 5. Clone-Heavy Operations
**Locations**: 2,900 `.clone()` calls across 182 files

**Issue**: Excessive cloning in hot paths:
- `src/storage/libsql.rs`: 123 clones
- `src/orchestration/supervision.rs`: 90 to_string calls
- `src/ics/app.rs`: 64 clones

**Risk**: Memory churn, potential for temporary allocation spikes
**Impact**: Performance degradation, GC pressure
**Priority**: **P1 - High**

### High-Risk Patterns

#### Unbounded Collections (519 occurrences)
```rust
Vec::new()  // 519 occurrences, no capacity pre-allocation
HashMap::new()
String::new()
```

**Files of Concern**:
- `src/storage/libsql.rs` (23 Vec::new)
- `src/orchestration/actors/reviewer.rs` (18 Vec::new)
- `src/ics/semantic_highlighter/` (allocation-heavy)

**Recommendation**: Pre-allocate capacity where size is known or bounded

#### Long-Running Operations Without Cleanup
- Vector similarity search (no result set limit before ranking)
- FTS5 full-text search (may return unbounded results)
- Graph traversal (no depth limit visible)
- Semantic highlighting Tier 3 (LLM calls, unbounded context)

## Instrumentation Status

### Instrumented Components
- [x] Memory tracker (global singleton)
- [x] Allocation/deallocation tracking (manual instrumentation points needed)
- [x] Work queue size tracking
- [x] Event queue size tracking
- [x] Database connection counting
- [x] Spawned task counting
- [x] Periodic monitoring (30s intervals)

### Needs Instrumentation
- [ ] Storage layer (hot paths: store_memory, search)
- [ ] Orchestrator (work submission, dispatch loop)
- [ ] API server (SSE connections, event broadcasting)
- [ ] Embeddings service (cache size, generation)
- [ ] ICS semantic highlighter (all 3 tiers)
- [ ] CRDT buffer (Automerge operations)

## Diagnostic Commands

### Memory Profiling
```bash
# Collect diagnostics
./scripts/diagnostics/collect-memory-diagnostics.sh

# Monitor memory during operation
/usr/bin/time -l ./target/release/mnemosyne orchestrate

# Check for leaks with Valgrind (if available)
valgrind --leak-check=full ./target/release/mnemosyne tui
```

### Stress Testing
```bash
# Run OOM stress tests (release mode only)
cargo test --release --features profiling oom_stress_test -- --ignored

# Run with memory monitoring
RUST_LOG=debug cargo run --release --features profiling
```

### Crash Analysis
```bash
# macOS crash logs
ls -lt ~/Library/Logs/DiagnosticReports/mnemosyne-* | head -5
cat ~/Library/Logs/DiagnosticReports/mnemosyne-*.ips | grep -A 20 "Exception Type"

# Check for OOM in system logs
dmesg | grep -i "out of memory\|oom\|killed" # Linux
log show --predicate 'process == "mnemosyne"' --last 1h # macOS
```

## Remediation Plan

### Immediate Fixes (Week 1)

1. **Add Resource Limits**
   - Event broadcaster: Implement LRU eviction or TTL
   - Work queue: Add max size limit with backpressure
   - Database connections: Implement connection pooling
   - Spawned tasks: Add Drop implementations

2. **Instrument Hot Paths**
   - Add memory tracking to all allocation-heavy operations
   - Log memory statistics at critical points
   - Add alerts for threshold breaches

3. **Crash Log Analysis**
   - Examine existing crash logs for patterns
   - Identify specific operations triggering OOM
   - Correlate with memory statistics

### Medium-Term Fixes (Week 2)

4. **CRDT History Management**
   - Implement periodic history compaction
   - Add configurable retention window
   - Prune on save or periodically

5. **Clone Reduction**
   - Audit high-frequency clone calls
   - Replace with references where possible
   - Use Cow<'_, T> for conditional cloning

6. **Capacity Pre-allocation**
   - Pre-allocate Vec capacity where size known
   - Use with_capacity() constructors
   - Measure impact on memory behavior

## Test Plan

### Unit Tests
- [x] Memory tracker allocation/deallocation
- [x] Memory tracker peak tracking
- [x] Memory tracker snapshot
- [ ] Resource leak detection helpers

### Integration Tests
- [x] Embedding stress test (10K memories)
- [x] Work queue growth test (50K items)
- [x] Task cleanup verification
- [x] Connection leak detection
- [ ] Event broadcaster capacity test
- [ ] CRDT history growth test

### E2E Tests
- [ ] Long-running orchestration session (24h)
- [ ] High-frequency operation test
- [ ] Concurrent multi-agent stress test
- [ ] Memory limit enforcement test

## Success Criteria

- [ ] Exit code 143 crashes eliminated
- [ ] Memory growth bounded (< 5% over 24h)
- [ ] Resource leaks detected in < 1 minute
- [ ] Critical thresholds trigger graceful degradation
- [ ] All stress tests pass without OOM
- [ ] Diagnostic tools integrated into CI/CD

## Next Steps

1. Complete hot path instrumentation
2. Run extended stress tests (24h+)
3. Analyze crash logs for patterns
4. Implement resource limits
5. Validate fixes with stress tests
6. Proceed to Phase 2 (Safety)

## References

- Exit code 143: SIGTERM (typically from OOM killer)
- jemalloc: Memory profiling allocator
- System memory thresholds: Normal < 40%, Moderate 40-60%, High 60-80%, Critical > 80%

---
**Last Updated**: 2025-11-07
**Next Review**: After instrumentation complete
