# Resource Limits and Management

## Overview

Mnemosyne implements memory tracking and resource limits to prevent OOM crashes (exit code 143) and ensure stable operation under load.

## Memory Thresholds

### System Memory Usage

| Status | Threshold | Action |
|--------|-----------|--------|
| **Normal** | < 40% | Continue normal operation |
| **Moderate** | 40-60% | Log warning, continue operation |
| **High** | 60-80% | Log warning, trigger monitoring alerts |
| **Critical** | > 80% | Log error, recommend graceful shutdown |

### Component-Specific Limits

#### Event Broadcaster
- **Current Capacity**: 1,000 events
- **Recommended Limit**: 1,000 events with LRU eviction
- **TTL**: 60 seconds (to be implemented)
- **Memory Impact**: ~10 KB per event = ~10 MB max

#### Work Queue
- **Current Limit**: Unbounded (⚠️ Issue)
- **Recommended Limit**: 10,000 items with backpressure
- **Backpressure Strategy**: Block submission when limit reached
- **Memory Impact**: ~1 KB per item = ~10 MB max

#### Database Connections
- **Current**: No pooling (⚠️ Issue)
- **Recommended**: Pool of 10-50 connections
- **Per-Connection Memory**: ~1-5 MB
- **Total Limit**: 50-250 MB

#### Embeddings Cache
- **Local (fastembed)**: ~1 GB ONNX model
- **Cache Size**: Unbounded (⚠️ Issue)
- **Recommended Limit**: 10,000 embeddings = ~30 MB (768-dim)
- **Eviction**: LRU when limit reached

#### CRDT History (ICS)
- **Current**: Full history retained (⚠️ Issue)
- **Recommended**: Rolling window (last 1,000 operations)
- **Compaction**: Every 10 minutes or 1,000 ops
- **Memory Impact**: ~100 bytes per op = ~100 KB per session

## Resource Monitoring

### Instrumentation Points

```rust
use mnemosyne_core::diagnostics::{global_memory_tracker, start_memory_monitoring};

// Start monitoring (30s intervals)
let _monitor_task = start_memory_monitoring();

// Track allocations
global_memory_tracker().record_allocation(bytes);
global_memory_tracker().record_deallocation(bytes);

// Track component sizes
global_memory_tracker().set_work_queue_size(count);
global_memory_tracker().set_event_queue_size(count);
global_memory_tracker().increment_db_connections();
global_memory_tracker().increment_spawned_tasks();

// Check status
let snapshot = global_memory_tracker().snapshot();
let status = global_memory_tracker().check_thresholds();
```

### Diagnostic Commands

```bash
# Real-time monitoring
RUST_LOG=debug mnemosyne orchestrate

# Collect diagnostics snapshot
./scripts/diagnostics/collect-memory-diagnostics.sh

# Monitor specific process
watch -n 1 'ps aux | grep mnemosyne'

# macOS: Memory pressure
vm_stat 1

# Linux: Memory usage
free -h && watch -n 1 free -h
```

## System Limits

### File Descriptors

```bash
# Check current limit
ulimit -n

# Recommended minimum: 4,096
# Increase if needed:
ulimit -n 4096  # Session-only

# Permanent (macOS):
sudo launchctl limit maxfiles 65536 200000

# Permanent (Linux):
echo "* soft nofile 65536" | sudo tee -a /etc/security/limits.conf
echo "* hard nofile 200000" | sudo tee -a /etc/security/limits.conf
```

### Memory Limits

```bash
# Check current memory limit
ulimit -v  # Linux
ulimit -m  # macOS

# Docker: Set memory limit
docker run --memory="4g" --memory-swap="6g" mnemosyne

# Kubernetes: Resource limits
resources:
  limits:
    memory: "4Gi"
    cpu: "2"
  requests:
    memory: "2Gi"
    cpu: "1"
```

### Process Limits

```bash
# Maximum processes/threads
ulimit -u

# Recommended: 4,096
```

## Configuration

### Environment Variables

```bash
# Memory monitoring interval (seconds)
MNEMOSYNE_MEMORY_LOG_INTERVAL=30

# Critical threshold (percentage)
MNEMOSYNE_MEMORY_CRITICAL_PCT=80

# Event broadcaster capacity
MNEMOSYNE_EVENT_CAPACITY=1000

# Work queue max size
MNEMOSYNE_WORK_QUEUE_MAX=10000

# Database connection pool size
MNEMOSYNE_DB_POOL_SIZE=20

# Embeddings cache size
MNEMOSYNE_EMBEDDINGS_CACHE_SIZE=10000
```

### Configuration File

```toml
# config.toml

[resources]
# Memory monitoring
memory_log_interval_secs = 30
memory_critical_pct = 80

# Component limits
event_capacity = 1000
event_ttl_secs = 60
work_queue_max = 10000
db_pool_size = 20
embeddings_cache_size = 10000

# CRDT history
crdt_history_max_ops = 1000
crdt_compaction_interval_secs = 600
```

## Graceful Degradation

### High Memory (60-80%)

1. **Log Warning**: Alert operators
2. **Reduce Cache Sizes**: Evict LRU entries
3. **Throttle Requests**: Implement backpressure
4. **Defer Non-Critical Work**: Pause background jobs

### Critical Memory (> 80%)

1. **Log Error**: Immediate operator notification
2. **Stop Accepting New Work**: Reject new requests
3. **Complete In-Flight Work**: Allow current operations to finish
4. **Prepare for Shutdown**: Save state, close connections
5. **Recommend Restart**: Exit gracefully with status code

### OOM Killer Prevention

```bash
# Linux: Set OOM score adjustment (0 = neutral, -1000 = never kill)
echo -500 > /proc/$(pgrep mnemosyne)/oom_score_adj

# Docker: Disable OOM killer for container
docker run --oom-kill-disable mnemosyne

# Kubernetes: Set OOM score adjustment
apiVersion: v1
kind: Pod
metadata:
  name: mnemosyne
spec:
  containers:
  - name: mnemosyne
    securityContext:
      capabilities:
        add: ["SYS_RESOURCE"]
```

## Monitoring & Alerts

### Metrics to Track

- **Memory Usage**: current, peak, % of system total
- **Allocation Rate**: MB/sec
- **Component Sizes**: work queue, event queue, cache sizes
- **Connection Count**: database, network
- **Task Count**: spawned, active, completed

### Alert Thresholds

```yaml
alerts:
  - name: MemoryHigh
    condition: memory_usage_pct > 60
    severity: warning
    action: log

  - name: MemoryCritical
    condition: memory_usage_pct > 80
    severity: error
    action: page_operator

  - name: WorkQueueFull
    condition: work_queue_size > 9000
    severity: warning
    action: throttle_requests

  - name: ConnectionLeaks
    condition: db_connections > db_pool_size * 2
    severity: error
    action: restart_recommended
```

### Prometheus Integration (Planned)

```rust
// Export metrics for Prometheus
use prometheus::{Counter, Gauge, Registry};

let memory_gauge = Gauge::new("mnemosyne_memory_bytes", "Memory usage")?;
let work_queue_gauge = Gauge::new("mnemosyne_work_queue_size", "Work queue size")?;
let db_connections_gauge = Gauge::new("mnemosyne_db_connections", "DB connections")?;

// Update from memory tracker
let snapshot = global_memory_tracker().snapshot();
memory_gauge.set(snapshot.current_usage as f64);
work_queue_gauge.set(snapshot.work_queue_size as f64);
db_connections_gauge.set(snapshot.db_connections as f64);
```

## Troubleshooting

### Symptoms: Exit Code 143

**Cause**: SIGTERM from OOM killer or container orchestrator

**Diagnosis**:
```bash
# Check crash logs
./scripts/diagnostics/collect-memory-diagnostics.sh

# Check for OOM in system logs
dmesg | grep -i oom  # Linux
log show --predicate 'eventMessage contains "mnemosyne"' --last 1h  # macOS

# Review memory statistics before crash
tail -100 ~/.local/share/mnemosyne/logs/memory.log
```

**Resolution**:
1. Increase system memory or container limits
2. Enable memory profiling: `--features profiling`
3. Review AUDIT_2025_PHASE1.md for known issues
4. Apply resource limits (event capacity, work queue max)
5. Implement connection pooling
6. Add CRDT history compaction

### Symptoms: Slow Performance

**Cause**: Memory pressure, excessive cloning, unbounded growth

**Diagnosis**:
```bash
# Profile with jemalloc
cargo build --release --features profiling
MALLOC_CONF=prof:true ./target/release/mnemosyne orchestrate

# Check for leaks
valgrind --leak-check=full ./target/release/mnemosyne tui

# Monitor in real-time
/usr/bin/time -l ./target/release/mnemosyne orchestrate
```

**Resolution**:
1. Reduce clone frequency in hot paths
2. Pre-allocate Vec capacity
3. Implement caching with LRU eviction
4. Profile and optimize allocation-heavy code

## References

- [AUDIT_2025_PHASE1.md](../security/AUDIT_2025_PHASE1.md) - Stability audit findings
- [SECURITY_POLICY.md](../security/SECURITY_POLICY.md) - Security guidelines
- Exit code 143: SIGTERM (typically from OOM killer)
- [jemalloc profiling](https://github.com/tikv/jemallocator)

---
**Last Updated**: 2025-11-07
**Reviewed By**: Security Audit Team
