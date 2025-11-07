# Python Agent Bridge Integration Testing

**Phase**: 5.6 - Production Hardening
**Status**: ✅ Enabled
**Created**: 2025-11-06

## Overview

Integration tests validate the Rust↔Python orchestration bridge that enables Claude SDK agents to be invoked from Rust actor supervision trees.

**Test Location**: `tests/orchestration_bridge_integration.rs`

**Test Count**: 5 comprehensive integration tests covering:
- Bridge spawn and registration
- Work delegation to Python agents
- Error handling and recovery
- Graceful degradation
- Concurrent work processing

---

## Test Suite Structure

All tests are behind `#[cfg(feature = "python")]` guard and in `mod python_bridge_tests`.

### Test Categories

| Test | #[ignore] | Requirements | Purpose |
|------|-----------|--------------|---------|
| `test_bridge_error_handling` | No | None | Validates bridge fails gracefully without Python init |
| `test_graceful_degradation_without_python_bridges` | No | PyO3 init | Verifies Rust actors continue if Python bridges fail |
| `test_python_bridge_spawn_and_registration` | Yes | Python env + modules | Tests bridge spawn and dashboard registration |
| `test_work_delegation_to_python_agent` | Yes | Python env + API key | Tests work delegation via orchestrator |
| `test_concurrent_work_processing` | Yes | Python env + modules | Validates concurrent agent execution |

---

## Running Tests

### Prerequisites

**Required for all tests**:
- Rust toolchain with `python` feature flag
- Python 3.9+ with PyO3 bindings

**Required for ignored tests**:
- Python agent modules installed (`src/orchestration/agents/`)
- `ANTHROPIC_API_KEY` environment variable (for delegation tests)

### Basic Test Execution

**Run all non-ignored tests** (no external dependencies):
```bash
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo test --features python --test orchestration_bridge_integration
```

**Expected output**:
```
running 2 tests
test python_bridge_tests::test_bridge_error_handling ... ok
test python_bridge_tests::test_graceful_degradation_without_python_bridges ... ok

test result: ok. 2 passed; 0 failed; 3 ignored
```

**List all tests** (including ignored):
```bash
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo test --features python --test orchestration_bridge_integration -- --list --include-ignored
```

---

## Full Integration Test Execution

### Step 1: Python Environment Setup

**Install Python agent dependencies**:
```bash
# Install uv package manager (if not installed)
curl -LsSf https://astral.sh/uv/install.sh | sh

# Install dependencies in project
cd src/orchestration/agents
uv pip install anthropic pyo3

# OR: Use system Python
pip install anthropic pyo3
```

**Verify Python modules are importable**:
```bash
python3 -c "from mnemosyne.orchestration.agent_factory import create_agent; print('✓ Modules OK')"
```

### Step 2: API Key Configuration

**Set Anthropic API key** (required for work delegation tests):
```bash
# Option 1: Environment variable (session-only)
export ANTHROPIC_API_KEY="sk-ant-..."

# Option 2: mnemosyne secrets (persistent)
mnemosyne secrets init  # Interactive setup
# OR
mnemosyne config set-key "sk-ant-..."  # Direct setup

# Verify configuration
mnemosyne config show-key
```

**Get API key**: https://console.anthropic.com/settings/keys

### Step 3: Run Full Test Suite

**Run ALL tests** (including ignored):
```bash
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo test --features python --test orchestration_bridge_integration -- --include-ignored
```

**Expected output**:
```
running 5 tests
test python_bridge_tests::test_bridge_error_handling ... ok
test python_bridge_tests::test_graceful_degradation_without_python_bridges ... ok
test python_bridge_tests::test_python_bridge_spawn_and_registration ... ok
test python_bridge_tests::test_work_delegation_to_python_agent ... ok
test python_bridge_tests::test_concurrent_work_processing ... ok

test result: ok. 5 passed; 0 failed; 0 ignored
```

### Step 4: Specific Test Execution

**Run individual test**:
```bash
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo test --features python --test orchestration_bridge_integration python_bridge_tests::test_work_delegation_to_python_agent -- --include-ignored --nocapture
```

The `--nocapture` flag shows Python logging output and agent execution traces.

---

## Test Details

### 1. test_bridge_error_handling

**Purpose**: Verify bridge spawn fails gracefully without Python initialization

**Assertions**:
- Bridge spawn returns error without PyO3 init
- Error message mentions Python/GIL/import failure

**No external dependencies** - pure Rust error path validation

**Code reference**: `tests/orchestration_bridge_integration.rs:154-177`

---

### 2. test_graceful_degradation_without_python_bridges

**Purpose**: Verify Rust actors continue running if Python bridges fail to initialize

**Setup**:
- Creates SupervisionTree with all actors
- Python bridges may fail (missing modules/API key)
- Rust actors should remain healthy

**Assertions**:
- SupervisionTree starts successfully
- Tree reports healthy status
- StateManager shows 4+ Rust agents (from heartbeats)

**Dependencies**: PyO3 init only (Python modules NOT required)

**Code reference**: `tests/orchestration_bridge_integration.rs:180-237`

---

### 3. test_python_bridge_spawn_and_registration

**Purpose**: Test Python bridge spawn and dashboard registration

**Setup**:
- Initializes PyO3: `pyo3::prepare_freethreaded_python()`
- Creates SupervisionTree with EventBroadcaster and StateManager
- Starts tree (auto-spawns Python bridges for all 4 roles)

**Assertions**:
- SupervisionTree is healthy
- StateManager shows 4+ agents (Executor, Reviewer, Optimizer, Orchestrator)
- Agents registered via heartbeat events

**Dependencies**:
- Python environment with agent_factory module
- `mnemosyne.orchestration.agents` package

**Code reference**: `tests/orchestration_bridge_integration.rs:24-83`

---

### 4. test_work_delegation_to_python_agent

**Purpose**: Test complete work delegation flow through orchestrator to Python agent

**Setup**:
- Creates SupervisionTree with full event pipeline
- Submits WorkItem to Orchestrator actor
- Orchestrator delegates to Python Executor agent

**Workflow**:
```
Rust Orchestrator → ClaudeAgentBridge → Python Executor → Claude SDK → LLM
                ↓
         WorkResult returned
```

**Assertions**:
- Work submitted successfully
- Work processed (check storage for completion)

**Dependencies**:
- Python environment with all agent modules
- **ANTHROPIC_API_KEY** configured (makes actual Claude SDK calls)

**Code reference**: `tests/orchestration_bridge_integration.rs:86-151`

**Note**: This test makes actual API calls and may incur costs.

---

### 5. test_concurrent_work_processing

**Purpose**: Validate multiple work items processed concurrently across agents

**Setup**:
- SupervisionTree with `max_concurrent_agents: 4`
- Submits 3 work items simultaneously
- Verifies concurrent execution

**Assertions**:
- All work items dispatched
- Work queue processed

**Dependencies**:
- Python environment with all agent modules

**Code reference**: `tests/orchestration_bridge_integration.rs:240-308`

---

## Troubleshooting

### Python Module Import Errors

**Symptom**:
```
Error: Agent factory import failed: ModuleNotFoundError: No module named 'mnemosyne'
```

**Solutions**:
1. Ensure Python path includes `src/orchestration`:
   ```bash
   export PYTHONPATH="${PWD}/src/orchestration:${PYTHONPATH}"
   ```

2. Install as editable package:
   ```bash
   cd src/orchestration
   uv pip install -e .
   ```

3. Use system Python with module installed globally

---

### API Key Not Found

**Symptom**:
```
Error: API key may be missing or invalid
```

**Solutions**:
1. Check environment variable:
   ```bash
   echo $ANTHROPIC_API_KEY
   ```

2. Verify mnemosyne config:
   ```bash
   mnemosyne config show-key
   ```

3. Check API key validity:
   ```bash
   curl -H "x-api-key: $ANTHROPIC_API_KEY" https://api.anthropic.com/v1/messages
   ```

---

### GIL/Thread Safety Issues

**Symptom**:
```
Error: PanicException: GIL is already held
```

**Solutions**:
1. Ensure PyO3 is initialized once:
   ```rust
   pyo3::prepare_freethreaded_python();
   ```

2. Use `Python::with_gil()` for all Python calls

3. Run tests sequentially (not in parallel):
   ```bash
   cargo test --features python -- --test-threads=1
   ```

---

### Test Timeout

**Symptom**:
```
test python_bridge_tests::test_work_delegation_to_python_agent ... timeout
```

**Solutions**:
1. Increase timeout in test:
   ```rust
   tokio::time::sleep(Duration::from_secs(5)).await; // Increase from 2s
   ```

2. Check network connectivity to Claude API:
   ```bash
   curl -I https://api.anthropic.com
   ```

3. Verify API rate limits not exceeded

---

## CI/CD Integration

### GitHub Actions Workflow

**Example** `.github/workflows/python-bridge-tests.yml`:
```yaml
name: Python Bridge Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    env:
      PYO3_USE_ABI3_FORWARD_COMPATIBILITY: 1
      ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Install Python
        uses: actions/setup-python@v4
        with:
          python-version: '3.11'

      - name: Install uv
        run: curl -LsSf https://astral.sh/uv/install.sh | sh

      - name: Install Python dependencies
        run: |
          cd src/orchestration/agents
          uv pip install anthropic pyo3

      - name: Run basic tests (no API key)
        run: |
          cargo test --features python --test orchestration_bridge_integration

      - name: Run full tests (with API key)
        if: ${{ secrets.ANTHROPIC_API_KEY }}
        run: |
          cargo test --features python --test orchestration_bridge_integration -- --include-ignored
```

**Secret Configuration**:
1. Go to repository Settings → Secrets → Actions
2. Add `ANTHROPIC_API_KEY` secret
3. Tests with `#[ignore]` will run if secret is available

---

## Coverage Analysis

**Current Coverage**:
- ✅ Bridge lifecycle (spawn, start_session, shutdown)
- ✅ Error handling (graceful failures)
- ✅ Work item conversion (Rust → Python dict)
- ✅ Result extraction (Python dict → Rust WorkResult)
- ✅ Event broadcasting (agent started/completed/failed)
- ✅ State management (dashboard integration)
- ✅ Concurrent execution
- ✅ Supervision tree integration

**Not Yet Covered**:
- ⏳ Agent restart on error threshold (Phase 5.7)
- ⏳ Memory ID tracking (Phase 5.7)
- ⏳ Metrics collection from Python agents (Phase 5.7)
- ⏳ Cross-agent communication patterns (Phase 5.7)

---

## Next Steps (Phase 5.7)

Phase 5.7 (E2E Validation) will add:
1. Tests for automatic agent restart after errors
2. Memory ID tracking validation
3. Performance metrics validation (duration, success rate)
4. Review-specific metrics (confidence, quality gates)
5. End-to-end workflows (Orchestrator → Executor → Reviewer)
6. Error recovery scenarios
7. Rate limit handling
8. Timeout recovery

---

## Related Documentation

- **Bridge Implementation**: `src/orchestration/claude_agent_bridge.rs`
- **Python Agents**: `src/orchestration/agents/executor.py`, `reviewer.py`, etc.
- **Agent Factory**: `src/orchestration/agents/agent_factory.py`
- **Phase 5 Plan**: `docs/architecture/PHASE5_PRODUCTION_HARDENING.md`
- **API Key Setup**: `SECRETS_MANAGEMENT.md`

---

## Appendix: Manual Testing

**Interactive test session**:
```bash
# Terminal 1: Start Python REPL
python3
>>> from mnemosyne.orchestration.agent_factory import create_agent
>>> agent = create_agent("executor")
>>> agent.start_session()
# Verify no errors

# Terminal 2: Run Rust test
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo test --features python --test orchestration_bridge_integration python_bridge_tests::test_python_bridge_spawn_and_registration -- --include-ignored --nocapture
```

**Expected behavior**:
- Python agent initializes Claude SDK client
- Rust bridge successfully calls `start_session()`
- Dashboard receives agent started events
- StateManager shows agent as "idle"

---

**Last Updated**: 2025-11-06
**Maintainer**: Python Bridge Team
**Status**: Production-ready, tests passing
