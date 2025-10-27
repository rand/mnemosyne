# Multi-Agent Orchestration Guide

**PyO3-powered low-latency coordination for Mnemosyne's 4-agent architecture**

---

## Overview

Mnemosyne's multi-agent orchestration system provides **10-20x faster** storage operations compared to subprocess calls through direct Rust↔Python integration via PyO3 bindings.

**Architecture**:
```
Claude Agent SDK (Python)
    ↓
mnemosyne_core (PyO3 bindings)
    ↓
Mnemosyne Storage (Rust)
```

**Key Performance Metrics**:
- Storage operations: **2.25ms** average (vs 20-50ms subprocess)
- List operations: **0.88ms** average (<1ms target achieved!)
- Search operations: **1.61ms** average
- **Result: 10-20x performance improvement**

---

## Quick Start

### Prerequisites

- Rust 1.75+ (for core library)
- Python 3.10-3.14 (for orchestration)
- [uv](https://github.com/astral-sh/uv) package manager

### Build PyO3 Bindings

```bash
# 1. Create Python virtual environment
uv venv .venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate

# 2. Install dependencies
uv pip install maturin pytest pytest-asyncio claude-agent-sdk rich

# 3. Build PyO3 bindings
export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1
maturin develop

# 4. Verify installation
python -c "import mnemosyne_core; print('✓ Bindings ready')"
```

### Test Integration

```bash
# Run non-API tests
pytest tests/orchestration/test_integration.py -v -m "not integration"

# Expected output: 9 passed
```

---

## PyO3 Bindings API

### PyStorage

Direct Rust storage access with thread-safe operations.

```python
from mnemosyne_core import PyStorage

# Create storage instance
storage = PyStorage("/path/to/database.db")

# Store a memory
memory = {
    "content": "Architecture decision: Use PostgreSQL",
    "namespace": "project:myapp",
    "importance": 8,
    "summary": "Database selection",
    "keywords": ["database", "architecture"],
    "tags": ["decision"],
    "context": "Sprint planning",
    "confidence": 0.9
}
mem_id = storage.store(memory)

# Retrieve memory
retrieved = storage.get(mem_id)
print(retrieved['content'])

# Search memories
results = storage.search("database", namespace="project:myapp", limit=10)

# List recent memories
recent = storage.list_recent(namespace="project:myapp", limit=20)

# Get statistics
stats = storage.get_stats(namespace="project:myapp")
print(f"Total memories: {stats['total_memories']}")
```

### PyMemory Types

```python
from mnemosyne_core import PyMemoryId, PyNamespace, PyMemory

# Memory ID handling
mem_id = PyMemoryId.new()  # Generate UUID
mem_id_str = str(mem_id)   # Convert to string

# Namespace handling
global_ns = PyNamespace.global_()
project_ns = PyNamespace.project("myapp")
session_ns = PyNamespace.session("myapp", "session-123")
```

### PyCoordinator

Cross-agent coordination primitives for shared state management.

```python
from mnemosyne_core import PyCoordinator

# Create coordinator
coordinator = PyCoordinator()

# Coordinate agent state
# (Implementation details depend on specific coordination needs)
```

---

## 4-Agent Architecture

### Agent 1: Orchestrator
**Role**: Central coordinator and state manager

**Responsibilities**:
- Coordinate handoffs between agents
- Monitor execution state across parallel workstreams
- Prevent race conditions and deadlocks
- Preserve context before compaction
- Maintain global work graph

**Location**: `src/orchestration/agents/orchestrator.py`

### Agent 2: Optimizer
**Role**: Context and resource optimization specialist

**Responsibilities**:
- Construct optimal context payloads for each agent
- Apply ACE principles (incremental updates, structured accumulation)
- Monitor context sources (agents, files, commits, plans, skills)
- Prevent brevity bias and context collapse
- Dynamically discover and load relevant skills

**Location**: `src/orchestration/agents/optimizer.py`

### Agent 3: Reviewer
**Role**: Quality assurance and validation specialist

**Responsibilities**:
- Validate intent satisfaction, documentation, test coverage
- Fact-check claims and references
- Check for anti-patterns and technical debt
- Block work until quality standards met
- Mark "COMPLETE" only when all gates pass

**Location**: `src/orchestration/agents/reviewer.py`

### Agent 4: Executor
**Role**: Primary work agent and sub-agent manager

**Responsibilities**:
- Follow Work Plan Protocol (Phases 1-4)
- Execute atomic tasks from plans
- Spawn sub-agents for safe parallel work
- Apply loaded skills
- Challenge vague requirements
- Implement code, tests, documentation

**Location**: `src/orchestration/agents/executor.py`

---

## Python Orchestration Layer

### Engine

Main orchestration engine that coordinates all agents.

**Location**: `src/orchestration/engine.py`

**Key Methods**:
```python
from src.orchestration.engine import OrchestrationEngine

# Initialize engine
engine = OrchestrationEngine(storage)

# Start orchestration
await engine.start()

# Execute work plan
result = await engine.execute_plan(plan)

# Stop engine
await engine.stop()
```

### Parallel Executor

Manages concurrent sub-agent execution for parallel tasks.

**Location**: `src/orchestration/parallel_executor.py`

**Features**:
- Task dependency resolution
- Concurrent execution with proper coordination
- Rollback on failure
- Progress tracking

### Context Monitor

Low-latency monitoring of agent state and context usage.

**Location**: `src/orchestration/context_monitor.py`

**Features**:
- 10ms polling intervals (vs 100ms minimum for subprocess)
- Real-time context budget tracking
- Alert when context > 75% threshold
- Compression recommendations

### Dashboard

Progress visualization and agent coordination display.

**Location**: `src/orchestration/dashboard.py`

**Features**:
- Real-time agent status
- Task progress tracking
- Performance metrics
- Context usage visualization

---

## Performance Tuning

### Storage Operation Optimization

**Use batch operations when possible**:
```python
# Bad: Multiple individual stores
for memory in memories:
    storage.store(memory)

# Good: Consider batching in application logic
# (PyO3 operations are already optimized at the Rust level)
```

**Reuse storage instance**:
```python
# Bad: Create new instance for each operation
storage1 = PyStorage("/path/db")
storage1.store(memory1)
storage2 = PyStorage("/path/db")
storage2.store(memory2)

# Good: Reuse instance
storage = PyStorage("/path/db")
storage.store(memory1)
storage.store(memory2)
```

### Context Budget Management

**Monitor context usage**:
```python
from src.orchestration.context_monitor import ContextMonitor

monitor = ContextMonitor()
await monitor.start()

# Check context usage
usage = monitor.get_usage()
if usage > 0.75:
    # Trigger compression
    await optimizer.compress_context()
```

**Allocate context budget**:
- Critical: 40%
- Skills: 30%
- Project: 20%
- General: 10%

### Parallel Execution

**Identify parallelizable tasks**:
```python
from src.orchestration.parallel_executor import ParallelExecutor

executor = ParallelExecutor()

# Execute independent tasks in parallel
tasks = [
    ("task1", task1_fn, args1),
    ("task2", task2_fn, args2),
    ("task3", task3_fn, args3),
]

results = await executor.execute_parallel(tasks)
```

**Avoid parallelizing**:
- Tasks with dependencies
- Tasks that modify shared state
- Tasks that require sequential ordering

---

## Troubleshooting

### PyO3 Build Issues

**Error: "Python interpreter version too new"**
```bash
# Solution: Use forward compatibility flag
export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1
maturin develop
```

**Error: "Undefined symbols for architecture arm64"**
```bash
# Solution: Use maturin, not cargo build
# PyO3 extensions must be built with maturin
maturin develop  # Not: cargo build --features python
```

**Error: "Module not found: mnemosyne_core"**
```bash
# Solution: Ensure virtual environment is activated
source .venv/bin/activate
maturin develop
python -c "import mnemosyne_core"
```

### Performance Issues

**Storage operations slower than expected**
```python
# Check if using debug build
# Solution: Build release version
maturin develop --release
```

**High memory usage**
```python
# Check for storage instance leaks
# Solution: Reuse storage instances, don't create many
storage = PyStorage(db_path)  # Create once
# Use storage for all operations
```

### Test Failures

**Import errors in tests**
```bash
# Solution: Rebuild bindings before testing
maturin develop
pytest tests/orchestration/
```

**Agent initialization failures**
```bash
# Check API key availability
python -c "import os; print('API key:', 'SET' if os.getenv('ANTHROPIC_API_KEY') else 'NOT SET')"

# Set API key if needed
export ANTHROPIC_API_KEY=sk-ant-...
```

---

## Integration with Claude Agent SDK

### Basic Setup

```python
from claude_agent_sdk import Agent
from mnemosyne_core import PyStorage

# Initialize Mnemosyne storage
storage = PyStorage("~/.local/share/mnemosyne/mnemosyne.db")

# Create agent with memory access
class MemoryAwareAgent(Agent):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.storage = storage

    async def remember(self, content, importance=7):
        """Store a memory during execution"""
        memory = {
            "content": content,
            "namespace": f"project:{self.project_name}",
            "importance": importance,
            "context": f"Agent execution: {self.current_task}",
            "summary": content[:100],
            "keywords": [],
            "tags": ["agent-generated"],
            "confidence": 0.8
        }
        return self.storage.store(memory)

    async def recall(self, query, limit=5):
        """Retrieve relevant memories"""
        return self.storage.search(
            query,
            namespace=f"project:{self.project_name}",
            limit=limit
        )

# Use agent with memory
agent = MemoryAwareAgent(name="executor")
await agent.remember("Implemented user authentication")
memories = await agent.recall("authentication")
```

### Memory-Augmented Decision Making

```python
async def make_decision_with_context(agent, decision_context):
    """Make decisions informed by past memories"""

    # Recall relevant past decisions
    memories = await agent.recall(decision_context, limit=10)

    # Build context from memories
    context = "\\n\\n".join([
        f"Past decision: {m['summary']} (importance: {m['importance']})"
        for m in memories
    ])

    # Make decision with historical context
    decision = await agent.decide(
        question=decision_context,
        context=context
    )

    # Store new decision as memory
    await agent.remember(
        content=f"Decision: {decision}",
        importance=8
    )

    return decision
```

---

## Performance Benchmarks

### Storage Operations

| Operation | Time (ms) | Comparison |
|-----------|-----------|------------|
| Store | 2.25 | 10-20x faster than subprocess (20-50ms) |
| Get | 0.5 | Direct memory access |
| Search | 1.61 | FTS5 optimized |
| List Recent | 0.88 | **<1ms target achieved!** |

### Python Test Results

```
tests/orchestration/test_integration.py::TestAgentInitialization
  test_executor_initialization ✓ PASSED
  test_orchestrator_initialization ✓ PASSED
  test_optimizer_initialization ✓ PASSED
  test_reviewer_initialization ✓ PASSED

tests/orchestration/test_integration.py::TestEngineConfiguration
  test_engine_initialization ✓ PASSED
  test_engine_start_stop ✓ PASSED

tests/orchestration/test_integration.py
  test_bindings_available ✓ PASSED
  test_claude_sdk_importable ✓ PASSED
  test_api_key_info ✓ PASSED

9 passed, 6 deselected (integration tests require API key)
```

---

## Development Workflow

### 1. Make Rust Changes

```bash
# Edit Rust code
vim src/storage/libsql.rs

# Rebuild bindings
maturin develop

# Test in Python
python -c "from mnemosyne_core import PyStorage; ..."
```

### 2. Make Python Changes

```bash
# Edit Python code
vim src/orchestration/agents/executor.py

# Run tests
pytest tests/orchestration/test_integration.py -v
```

### 3. Full Validation

```bash
# Rust tests
cargo test --lib

# Python tests
pytest tests/orchestration/ -v

# Performance benchmark
python tests/benchmarks/storage_performance.py
```

---

## Related Documentation

- [ARCHITECTURE.md](ARCHITECTURE.md) - Overall system design
- [CONTRIBUTING.md](CONTRIBUTING.md) - Development guidelines
- [MCP_SERVER.md](MCP_SERVER.md) - MCP protocol and tools
- [HOOKS_TESTING.md](HOOKS_TESTING.md) - Automatic memory capture
- [README.md](README.md) - Project overview

---

## Support

For issues or questions:
- Check troubleshooting section above
- Review test files in `tests/orchestration/`
- Open an issue: https://github.com/rand/mnemosyne/issues

---

**Last Updated**: 2025-10-27
**Status**: Production Ready (Phase 6 Complete)
