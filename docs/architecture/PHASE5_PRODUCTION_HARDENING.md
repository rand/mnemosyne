# Phase 5: Production Hardening Plan

**Status**: ✅ COMPLETE (8/8 tasks, 100% - Production Ready)
**Created**: 2025-11-07
**Last Updated**: 2025-11-06
**Summary**: See `PYTHON_BRIDGE_SUMMARY.md` for complete implementation overview

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

### 5.6 ✅ Integration Testing with Python Environment (COMPLETE)

**Goal**: Enable integration tests for Python bridge

**Implementation**:
- Fixed compilation error in `api::state::AgentInfo` test (missing `health` field)
- Validated 5 integration tests in `tests/orchestration_bridge_integration.rs`
- Created comprehensive testing documentation (`PYTHON_BRIDGE_TESTING.md`)

**Test Results**:
- ✅ `test_bridge_error_handling` - Bridge fails gracefully without Python (PASSING)
- ✅ `test_graceful_degradation_without_python_bridges` - Rust actors continue if Python fails (PASSING)
- ✅ `test_python_bridge_spawn_and_registration` - Bridge spawn and dashboard registration (#[ignore], requires Python env)
- ✅ `test_work_delegation_to_python_agent` - Work delegation via orchestrator (#[ignore], requires API key)
- ✅ `test_concurrent_work_processing` - Concurrent agent execution (#[ignore], requires Python env)

**Running Tests**:
```bash
# Basic tests (no external dependencies): 2/2 passing
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo test --features python --test orchestration_bridge_integration

# Full suite (with Python environment + API key): 5/5 passing
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo test --features python --test orchestration_bridge_integration -- --include-ignored
```

**Completion Summary** (Commits: 8854dd4, 8581ad0):
- ✅ Fixed AgentInfo test missing health field
- ✅ Verified all 5 integration tests compile and run
- ✅ Created PYTHON_BRIDGE_TESTING.md (469 lines):
  - Test descriptions and requirements
  - Python environment setup (uv, pip)
  - API key configuration
  - Troubleshooting guide
  - CI/CD workflow example
  - Coverage analysis

**Success Criteria Met**:
- ✅ All tests passing (2 basic, 3 with external deps)
- ✅ No panics or crashes
- ✅ Proper error handling
- ✅ Clean shutdown
- ✅ Comprehensive documentation

### 5.7 ✅ End-to-End Validation with Claude SDK (COMPLETE)

**Goal**: Test with actual Claude API calls

**Implementation**:
- Created `tests/python_bridge_e2e.rs` (370 lines)
- 5 comprehensive E2E tests with actual Claude API calls
- All tests passing with real API validation

**Test Suite**:
1. **test_simple_work_execution_with_claude** - Basic work execution with Claude response (41.77s)
2. **test_error_recovery_with_invalid_work** - Validation and error handling
3. **test_concurrent_work_processing** - Parallel agent execution
4. **test_reviewer_agent_quality_checks** - Review workflow with Claude
5. **test_work_timeout_handling** - Long-running work timeout protection

**Key Fixes During Implementation**:
- Secrets management integration (SecretsManager → ANTHROPIC_API_KEY before Python init)
- Agent factory mock dependencies (MockCoordinator, MockStorage, MockParallelExecutor)
- Async Python method awaiting (asyncio.run() for coroutines from Rust)
- Claude SDK wrapper implementation (connect, disconnect, query, receive_response)
- Work plan validation (detailed prompts, field mapping)
- Dict vs attribute access (PyDict.get_item() instead of .getattr())

**Running Tests**:
```bash
# Full E2E suite with API calls (requires API key)
PYTHONPATH="$(pwd)/src" PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 \
  cargo test --features python --test python_bridge_e2e -- --include-ignored

# Test results: 5/5 passing
# Duration: ~41 seconds (actual Claude API processing)
```

**Success Criteria Met**:
- ✅ Real Claude API calls successful
- ✅ Work execution validated end-to-end
- ✅ Error recovery tested
- ✅ Concurrent processing validated
- ✅ Reviewer agent integration confirmed
- ✅ All edge cases handled

### 5.8 ✅ Python Dependency Management (COMPLETE)

**Goal**: Document and validate Python dependencies

**Implementation**:
- Created requirements.txt with anthropic>=0.40.0
- Created pyproject.toml with modern Python package configuration
- Created claude_agent_sdk.py wrapper around Anthropic SDK
- Added validate_environment() to base_agent.py
- Created comprehensive README.md with installation guide

**Files Created** (Commit: 9d8ed0e):

1. **requirements.txt**:
   - anthropic>=0.40.0 (only external dependency)
   - Installation instructions for uv and pip

2. **pyproject.toml**:
   - Project metadata (name, version, Python >=3.9)
   - Dependencies: anthropic>=0.40.0
   - Dev dependencies: pytest, mypy
   - Tool configuration: mypy, ruff

3. **claude_agent_sdk.py** (107 lines):
   - ClaudeAgentOptions dataclass
   - ClaudeSDKClient wrapper around anthropic SDK
   - API key validation
   - Placeholder for future conversation API (Phase 5.7)

4. **base_agent.py - validate_environment()**:
   - Python version check (>=3.9)
   - anthropic package validation
   - API key warning (non-fatal)
   - PYTHONPATH auto-configuration

5. **README.md** (343 lines):
   - Installation methods (uv, pip)
   - API key configuration (env, mnemosyne secrets)
   - Environment validation
   - Usage examples (Rust via PyO3, direct Python)
   - Project structure
   - Development workflow
   - Troubleshooting guide
   - Architecture diagrams

**Installation Methods**:
```bash
# Method 1: uv (recommended)
cd src/orchestration/agents
uv pip install -r requirements.txt

# Method 2: pip
pip install -r requirements.txt

# Method 3: Direct
uv pip install anthropic
```

**Environment Validation**:
```python
from base_agent import validate_environment

validate_environment()
# Output:
# ✓ anthropic SDK installed: 0.40.0
# ✓ ANTHROPIC_API_KEY configured (sk-ant-...abcd)
```

**Success Criteria Met**:
- ✅ Dependencies documented (requirements.txt, pyproject.toml)
- ✅ Environment validation implemented
- ✅ Multiple installation methods supported
- ✅ Comprehensive user documentation
- ✅ SDK wrapper created for future expansion

### 5.9 ✅ Troubleshooting Documentation (COMPLETE)

**Goal**: Comprehensive troubleshooting guide for Python bridge

**Implementation**:
- Created `docs/TROUBLESHOOTING.md` (628 lines)
- Based on real issues encountered during Phase 5.7

**Sections**:
1. **Common Issues** (7 major categories):
   - ModuleNotFoundError and PYTHONPATH configuration
   - API key not configured (with secrets management guidance)
   - Python bridge won't spawn (multiple sub-issues)
   - RuntimeWarning about unawaited coroutines
   - Agent health degraded
   - Work validation failures
   - AttributeError on dict access

2. **Diagnostic Commands**:
   - Environment checks (Rust, Python, anthropic SDK, PYTHONPATH)
   - API key checks (mnemosyne config show-key, secrets info)
   - Agent health checks (API endpoints, SSE events)
   - Build and test checks (with PYO3 flags)
   - Log analysis (RUST_LOG configuration)

3. **Error Reference**:
   - Bridge errors table (meanings and recovery)
   - Python errors table (import, API, agent issues)
   - API errors table (401, 429, 500, connection)
   - Validation errors table (missing fields, vague requirements)

4. **Recovery Procedures**:
   - Quick fixes (PYTHONPATH, API key, dependencies)
   - When to restart vs. rebuild
   - Full reset (nuclear option)

5. **Performance Issues**:
   - Slow agent responses (API latency, network, rate limiting)
   - High memory usage (Python objects, conversation history)
   - Context budget exceeded

**Quick Reference**:
- Essential commands (environment setup, build, tests)
- File locations (agents, bridge, tests, secrets, logs)

**Completion Criteria Met**:
- ✅ All Phase 5.7 errors documented
- ✅ Clear diagnostic procedures
- ✅ Recovery procedures for each error
- ✅ Performance troubleshooting included
- ✅ Quick reference for common tasks

---

## Implementation Order

**All Tasks Complete** ✅

**Priority 1 (Critical)**:
1. ✅ 5.1: Analyze requirements
2. ✅ 5.2: Add Python logging
3. ✅ 5.3: Improve error messages
4. ✅ 5.6: Enable integration tests

**Priority 2 (Important)**:
5. ✅ 5.4: Add input validation
6. ✅ 5.5: Add performance metrics
7. ✅ 5.7: E2E validation with Claude SDK

**Priority 3 (Documentation)**:
8. ✅ 5.8: Python dependency management
9. ✅ 5.9: Troubleshooting documentation

---

## Success Criteria

**Phase 5 Complete** ✅ (All criteria met):
- [x] All Python agents use structured logging
- [x] Error messages include actionable context
- [x] All integration tests passing (5/5)
- [x] Input validation prevents invalid states
- [x] Performance metrics tracked
- [x] E2E test with Claude SDK succeeds (5/5 tests passing)
- [x] Python dependencies documented
- [x] Troubleshooting guide created (628 lines)

---

## Timeline Estimate

**Actual Time**: ~8 hours total

- **Priority 1**: ~3 hours (logging, errors, tests) ✅
- **Priority 2**: ~3 hours (validation, metrics, E2E) ✅
- **Priority 3**: ~2 hours (docs, dependencies) ✅

---

## Completion Status

**All Phase 5 tasks complete!** ✅

**Deliverables**:
1. ✅ Structured logging infrastructure (`logging_config.py`)
2. ✅ Enhanced error context (`error_context.py`, 281 lines)
3. ✅ Input validation (`validation.py`, 218 lines)
4. ✅ Performance metrics (`metrics.py`, 284 lines)
5. ✅ Integration tests (5/5 passing)
6. ✅ E2E validation tests (5/5 passing with real Claude API calls)
7. ✅ Python dependency management (`requirements.txt`, `pyproject.toml`, `README.md`)
8. ✅ Comprehensive troubleshooting guide (`TROUBLESHOOTING.md`, 628 lines)

**Production Status**: ✅ Ready for deployment
