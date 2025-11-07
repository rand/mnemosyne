# Phase 5: Production Hardening Plan

**Status**: In Progress
**Created**: 2025-11-07

---

## Overview

Phase 5 focuses on production readiness for the Python bridge architecture.

## Current State Analysis

### ✅ Already Implemented (Phases 1-4)

**Rust Side:**
- ✅ Error tracking (`error_count`, `record_error()`)
- ✅ Health monitoring (`AgentHealth`, health events)
- ✅ Automatic restart logic (`should_restart()`, `restart()`)
- ✅ Tracing/logging with `tracing` crate
- ✅ Event broadcasting to dashboard (SSE)
- ✅ Graceful degradation when Python unavailable

**Python Side:**
- ✅ PyO3 bridge interface (`AgentExecutionMixin`)
- ✅ All 4 agents integrated (Orchestrator, Optimizer, Reviewer, Executor)
- ✅ WorkItem/WorkResult protocol
- ✅ Basic error handling (try/except in `_execute_work_item()`)

**Configuration:**
- ✅ API key management (secrets system, OS keychain)
- ✅ PyO3 feature flag (`--features python`)

### ⚠️ Production Gaps

**Python Side Issues:**
1. **Logging**: Using `print()` instead of proper logging
2. **Error Context**: Generic error messages lack context
3. **Metrics**: No performance tracking
4. **Validation**: No input validation in agents
5. **Documentation**: Missing troubleshooting guides

**Testing Gaps:**
1. **Integration tests**: 3 tests ignored (require Python environment)
2. **Claude SDK**: No tests with actual API calls
3. **Error scenarios**: Limited error case coverage
4. **Performance**: No benchmarks or profiling

**Deployment Gaps:**
1. **Dependencies**: Python dependencies not documented
2. **Environment**: No environment validation
3. **Monitoring**: No production metrics
4. **Troubleshooting**: Limited diagnostic tools

---

## Phase 5 Tasks

### 5.1 ✅ Analyze Requirements (COMPLETE)
- Analyzed current state
- Identified production gaps
- Created this plan

### 5.2 ✅ Add Python Logging Infrastructure (COMPLETE)

**Goal**: Replace `print()` with proper structured logging

**Implementation**:
```python
# Add to each Python agent file
import logging
import sys

# Configure logging
logger = logging.getLogger(__name__)

# In agent methods
logger.info(f"Agent {self.config.agent_id} starting session")
logger.warning(f"Evaluation system not available: {e}")
logger.error(f"Failed to execute work item: {e}", exc_info=True)
```

**Files to Update**:
- `src/orchestration/agents/executor.py`
- `src/orchestration/agents/reviewer.py`
- `src/orchestration/agents/optimizer.py`
- `src/orchestration/agents/orchestrator.py`
- `src/orchestration/agents/base_agent.py`

**Completion Summary** (Commits: d787950, 2efb2ed, 6dc45f0):
- ✅ Created `logging_config.py` with structured logging
- ✅ Executor: Comprehensive logging (session, execution, errors)
- ✅ Reviewer: Review lifecycle and quality gate logging
- ✅ Optimizer: Replaced all print() with logger calls
- ✅ Orchestrator: Logger configured and imported

**Benefits**:
- Structured logging with levels (DEBUG, INFO, WARNING, ERROR)
- Integration with Rust logging via PyO3
- Log rotation and filtering
- Production debugging capability

### 5.3 Improve Error Messages and Context (HIGH PRIORITY)

**Goal**: Provide actionable error information

**Current Issues**:
```python
# Generic
return WorkResult(success=False, error=f"Executor error: {type(e).__name__}: {str(e)}")
```

**Improved**:
```python
# Contextual
return WorkResult(
    success=False,
    error=f"Executor failed during {phase}: {type(e).__name__}: {str(e)}\n"
          f"Work item ID: {work_item.id}\n"
          f"Phase: {work_item.phase}\n"
          f"Troubleshooting: Check API key and Claude SDK installation"
)
```

**Add Context**:
- Work item details (ID, phase, description excerpt)
- Agent state (session active, loaded skills, etc.)
- Environment info (Python version, SDK availability)
- Troubleshooting hints

### 5.4 Add Input Validation (MEDIUM PRIORITY)

**Goal**: Validate WorkItems before processing

**Implementation**:
```python
async def _execute_work_item(self, work_item: WorkItem) -> WorkResult:
    # Validate input
    if not work_item.id:
        return WorkResult(success=False, error="WorkItem missing ID")
    if not work_item.description:
        return WorkResult(success=False, error="WorkItem missing description")

    # Validate state
    if not self._session_active:
        logger.warning("Session not active, starting...")
        await self.start_session()

    # Execute...
```

**Validation Checks**:
- Required fields (id, description)
- Field constraints (phase valid, priority in range)
- Agent state (session active, dependencies loaded)
- Resource availability (API key, skills directory)

### 5.5 Add Performance Metrics (MEDIUM PRIORITY)

**Goal**: Track execution performance

**Implementation**:
```python
import time

async def _execute_work_item(self, work_item: WorkItem) -> WorkResult:
    start_time = time.time()

    try:
        result = await self.execute_work_plan(work_plan)

        # Record metrics
        duration = time.time() - start_time
        logger.info(f"Work item {work_item.id} completed in {duration:.2f}s")

        # Store in coordinator metrics
        self.coordinator.set_metric(f"{self.config.agent_id}_duration", duration)

        return WorkResult(...)
    except Exception as e:
        duration = time.time() - start_time
        logger.error(f"Work item {work_item.id} failed after {duration:.2f}s: {e}")
        raise
```

**Metrics to Track**:
- Execution duration (per agent, per work item)
- Success/failure rates
- Context utilization
- Skill loading time
- API call latency

### 5.6 Integration Testing with Python Environment (HIGH PRIORITY)

**Goal**: Enable the 3 ignored tests

**Requirements**:
1. Build PyO3 bindings: `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo build --features python`
2. Install Python dependencies: `uv pip install anthropic claude-agent-sdk`
3. Configure API key: `export ANTHROPIC_API_KEY=...`
4. Run tests: `cargo test --features python`

**Tests to Enable**:
- `test_python_bridge_spawn_and_registration` - Validates bridge creation
- `test_work_delegation_to_python_agent` - Tests work submission
- `test_concurrent_work_processing` - Tests parallel execution

**Success Criteria**:
- All 5 tests passing
- No panics or crashes
- Proper error handling
- Clean shutdown

### 5.7 End-to-End Validation with Claude SDK (MEDIUM PRIORITY)

**Goal**: Test with actual Claude API calls

**Test Scenarios**:
1. **Simple Work**: Execute basic task, verify Claude response
2. **Complex Workflow**: Multi-step plan with agent coordination
3. **Error Recovery**: API failures, retries, fallback
4. **Context Limits**: Large inputs, context preservation
5. **Concurrent Work**: Multiple agents processing in parallel

**Implementation**:
```rust
#[tokio::test]
#[ignore] // Requires API key and network
async fn test_executor_with_claude_sdk() {
    // ... setup ...

    let work_item = WorkItem {
        id: "test-1".to_string(),
        description: "Write a simple hello world function in Python".to_string(),
        phase: "implementation".to_string(),
        priority: 1,
        // ...
    };

    let result = bridge.send_work(work_item).await.unwrap();
    assert!(result.success);
    assert!(result.data.is_some());

    // Verify Claude generated code
    let data = result.data.unwrap();
    assert!(data.contains("def hello_world"));
}
```

### 5.8 Python Dependency Management (LOW PRIORITY)

**Goal**: Document and validate Python dependencies

**Create**: `src/orchestration/agents/requirements.txt`
```txt
anthropic>=0.40.0
claude-agent-sdk>=0.1.0  # If available
pydantic>=2.0.0
```

**Create**: `src/orchestration/agents/pyproject.toml` (for uv)
```toml
[project]
name = "mnemosyne-agents"
version = "0.1.0"
requires-python = ">=3.10"
dependencies = [
    "anthropic>=0.40.0",
    "pydantic>=2.0.0",
]
```

**Add Validation**:
```python
# In base_agent.py
import sys

def validate_environment():
    """Validate Python environment and dependencies."""
    if sys.version_info < (3, 10):
        raise RuntimeError(f"Python 3.10+ required, got {sys.version}")

    try:
        import anthropic
    except ImportError:
        raise RuntimeError("anthropic package not installed. Run: uv pip install anthropic")
```

### 5.9 Troubleshooting Documentation (LOW PRIORITY)

**Create**: `docs/TROUBLESHOOTING.md`

**Sections**:
1. **Common Issues**:
   - ModuleNotFoundError: mnemosyne
   - API key not configured
   - Python bridge won't spawn
   - Agent health degraded

2. **Diagnostic Commands**:
   ```bash
   # Check Python environment
   python --version

   # Check API key
   mnemosyne config show-key

   # Check agent health
   curl http://localhost:3000/agents

   # Check logs
   RUST_LOG=debug cargo run
   ```

3. **Error Reference**:
   - Error codes and meanings
   - Recovery procedures
   - When to restart vs. rebuild

---

## Implementation Order

**Priority 1 (Critical)**:
1. ✅ 5.1: Analyze requirements
2. ⏳ 5.2: Add Python logging
3. ⏳ 5.3: Improve error messages
4. ⏳ 5.6: Enable integration tests

**Priority 2 (Important)**:
5. 5.4: Add input validation
6. 5.5: Add performance metrics
7. 5.7: E2E validation with Claude SDK

**Priority 3 (Nice to Have)**:
8. 5.8: Python dependency management
9. 5.9: Troubleshooting documentation

---

## Success Criteria

Phase 5 is complete when:
- [ ] All Python agents use structured logging
- [ ] Error messages include actionable context
- [ ] All integration tests passing (5/5)
- [ ] Input validation prevents invalid states
- [ ] Performance metrics tracked
- [ ] E2E test with Claude SDK succeeds
- [ ] Python dependencies documented
- [ ] Troubleshooting guide created

---

## Timeline Estimate

- **Priority 1**: 2-3 hours (logging, errors, tests)
- **Priority 2**: 2-3 hours (validation, metrics, E2E)
- **Priority 3**: 1-2 hours (docs, dependencies)
- **Total**: 5-8 hours

---

## Next Steps

1. Start with 5.2: Add Python logging infrastructure
2. Immediately follow with 5.3: Improve error messages
3. Run 5.6: Enable and validate integration tests
4. If tests pass, proceed with Priority 2 items
5. Complete with documentation (Priority 3)
