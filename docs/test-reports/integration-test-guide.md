# Integration Test Execution Guide

**Date**: October 26, 2025
**Status**: Ready for Execution
**Prerequisites**: API key configured, Python 3.11 environment

---

## Overview

This guide provides instructions for running the multi-agent orchestration integration tests that validate the Claude Agent SDK implementation.

## Test Categories

### 1. Unit Tests (No API Key Required) ✅

These tests validate agent initialization and configuration without making API calls.

```bash
# Activate Python environment
source .venv/bin/activate

# Run all unit tests
pytest tests/orchestration/test_integration.py -v -m "not integration"

# Or run specific test classes
pytest tests/orchestration/test_integration.py::TestAgentInitialization -v
pytest tests/orchestration/test_integration.py::TestEngineConfiguration -v
```

**Expected Results**:
- ✅ 9/9 tests passing
- Tests complete in <1 second
- No API calls made

**Current Status**: All passing as of October 26, 2025

---

### 2. Integration Tests (Requires API Key) ⏸️

These tests validate actual Claude Agent SDK integration with real API calls.

#### Prerequisites

1. **Verify API key is accessible**:
```bash
./target/debug/mnemosyne config show-key
# Should show: ✓ API key configured: sk-ant-a...
# This checks the secure system (age encrypted file → OS keychain → env var)
```

2. **Run tests with secure key access** (recommended):
```bash
# The mnemosyne binary will automatically access the key securely
# No need to export to environment
pytest tests/orchestration/ -v
```

3. **Alternative: Export to environment** (for CI/CD or manual testing):
```bash
# Option A: Export directly (most secure for CI/CD)
export ANTHROPIC_API_KEY="sk-ant-your-key-here"

# Option B: Get from mnemosyne secure system (requires jq for parsing)
export ANTHROPIC_API_KEY=$(./target/debug/mnemosyne config show-key --format json 2>/dev/null | jq -r '.key')

# Verify export
if [ -n "$ANTHROPIC_API_KEY" ]; then
    echo "✓ API key exported (${#ANTHROPIC_API_KEY} characters)"
else
    echo "✗ API key not available"
fi
```

**Note**: The secure system automatically checks (in priority order):
1. `ANTHROPIC_API_KEY` environment variable
2. Age-encrypted file: `~/.config/mnemosyne/secrets.age`
3. OS keychain (macOS/Windows/Linux)

For most use cases, just ensure `mnemosyne config show-key` works and the tests will access the key securely.

#### Running Integration Tests

```bash
# Activate environment
source .venv/bin/activate

# Run all integration tests
pytest tests/orchestration/test_integration.py -v -m integration

# Or run specific test suites
pytest tests/orchestration/test_integration.py::TestAgentSDKIntegration -v
pytest tests/orchestration/test_integration.py::TestEndToEndWorkflow -v
```

**Test Suite: TestAgentSDKIntegration** (4 tests)
- `test_executor_session_lifecycle` - Validates session start/stop
- `test_executor_context_manager` - Tests async context manager
- `test_optimizer_skill_discovery` - Tests skill discovery with Claude
- `test_reviewer_quality_gates` - Tests quality gate evaluation

**Test Suite: TestEndToEndWorkflow** (2 tests)
- `test_simple_work_plan_execution` - Validates basic workflow
- `test_work_plan_with_validation` - Tests reviewer validation

**Expected Duration**: ~2-5 minutes (includes real Claude API calls)

**Expected Cost**: ~$0.05-0.10 per test run (using Claude 3.5 Haiku)

#### Interpreting Results

**Success Indicators**:
- All tests pass
- Agent sessions start and stop cleanly
- Messages stored in PyStorage
- Quality gates evaluated correctly

**Common Failures**:
1. **Connection errors**: Check network and API key validity
2. **Timeout errors**: Increase timeout in test configuration
3. **Tool access errors**: Verify permission modes configured correctly

---

## Integration Test Details

### Test: executor_session_lifecycle

**Purpose**: Validate ExecutorAgent can start and stop Claude sessions

**What it tests**:
- ClaudeSDKClient initialization
- Session activation
- Status reporting
- Clean shutdown

**API calls**: 1-2 (session management only, no work execution)

---

### Test: executor_context_manager

**Purpose**: Validate async context manager pattern

**What it tests**:
- `async with executor:` pattern
- Automatic session cleanup
- Exception handling

**API calls**: 1-2 (session management)

---

### Test: optimizer_skill_discovery

**Purpose**: Validate Optimizer can discover relevant skills using Claude

**What it tests**:
- Task analysis by Claude
- Skill directory scanning
- Relevance scoring
- Context budget allocation

**API calls**: 2-3 (skill analysis and context optimization)

**Test scenario**: "Write a Rust function to parse JSON"

**Expected behavior**:
- Claude identifies Rust-related skills
- Allocates context budget appropriately
- Returns structured allocation result

---

### Test: reviewer_quality_gates

**Purpose**: Validate Reviewer can evaluate all 7 quality gates

**What it tests**:
- Gate evaluation logic
- Feedback generation
- Pass/fail determination
- Confidence scoring

**API calls**: 2-4 (quality analysis)

**Test artifact**: Simple documented function with tests

**Expected behavior**:
- All gates evaluated
- Specific feedback provided
- Overall pass/fail determined

---

### Test: simple_work_plan_execution

**Purpose**: Validate complete workflow through engine

**What it tests**:
- Engine initialization
- Work plan execution
- Orchestrator coordination
- Status tracking

**API calls**: 5-10 (full workflow)

**Test scenario**: "Analyze this simple task: count to 5"

**Expected behavior**:
- Work plan accepted
- Execution completes
- Result structure valid

---

### Test: work_plan_with_validation

**Purpose**: Validate work with reviewer validation

**What it tests**:
- Execution + review workflow
- Quality gate enforcement
- Feedback loop

**API calls**: 8-15 (execution + review)

**Test scenario**: "Create a simple Python function" with success criteria

**Expected behavior**:
- Work executed
- Review performed
- Statistics recorded

---

## Troubleshooting

### Issue: API key not accessible

**Symptom**: `mnemosyne config show-key` returns "No API key found"

**Solution**:
```bash
# Set the key using the secure system
./target/debug/mnemosyne config set-key
# This will prompt for your key and encrypt it with age

# Or export to environment (for CI/CD)
export ANTHROPIC_API_KEY="sk-ant-your-key-here"

# Verify it's accessible
./target/debug/mnemosyne config show-key
```

**Priority order**: The system checks env var → age file → OS keychain. For most users, `mnemosyne config set-key` is the recommended approach.

---

### Issue: Tests timeout

**Symptom**: Tests fail with timeout errors

**Solutions**:
1. Increase timeout in test configuration
2. Check network connectivity
3. Verify API key is valid and has credits

---

### Issue: Import errors

**Symptom**: `ImportError: cannot import name 'OrchestrationEngine'`

**Solution**:
```bash
# Rebuild PyO3 bindings
maturin develop --features python

# Verify import
python -c "from orchestration import OrchestrationEngine; print('✓ Import successful')"
```

---

### Issue: API rate limiting

**Symptom**: 429 errors from Anthropic API

**Solution**:
1. Wait 1-2 minutes between test runs
2. Run tests individually with delays
3. Use smaller test scenarios

---

## Manual Test Execution

For manual validation without pytest:

```python
import asyncio
from orchestration import (
    ExecutorAgent, ExecutorConfig,
    mnemosyne_core
)

async def test_executor():
    coordinator = mnemosyne_core.PyCoordinator()
    storage = mnemosyne_core.PyStorage("test.db")

    config = ExecutorConfig(agent_id="test")
    executor = ExecutorAgent(config, coordinator, storage, None)

    async with executor:
        print("✓ Executor session started")
        status = executor.get_status()
        print(f"Session active: {status['session_active']}")

    print("✓ Executor session stopped")

if __name__ == "__main__":
    asyncio.run(test_executor())
```

---

## Next Steps After Integration Tests

1. **If all tests pass**:
   - Mark task as complete
   - Proceed to E2E human workflow tests
   - Update gap analysis with results

2. **If tests fail**:
   - Document failures in gap analysis
   - Create issues for bugs found
   - Prioritize fixes (P0/P1/P2)
   - Re-run after fixes

3. **Performance concerns**:
   - Record API call latency
   - Note any timeouts or slow responses
   - Consider caching strategies

---

## Test Execution Checklist

Before running integration tests:
- [ ] Build latest code: `cargo build && maturin develop --features python`
- [ ] Verify API key: `./target/debug/mnemosyne config show-key`
- [ ] Export API key: `export ANTHROPIC_API_KEY=...`
- [ ] Activate venv: `source .venv/bin/activate`
- [ ] Verify imports: `python -c "from orchestration import OrchestrationEngine"`

During test execution:
- [ ] Monitor API usage
- [ ] Watch for timeout errors
- [ ] Check network connectivity

After test execution:
- [ ] Document results in gap analysis
- [ ] Commit any bug fixes
- [ ] Update test status in todos
- [ ] Clean up test databases

---

## Security Considerations

✅ **Secure practices**:
- API key stored in OS keychain (not in code/env files)
- Environment variable only for runtime
- No key logging or display in test output
- Test databases use temporary files

❌ **Avoid**:
- Committing API keys to git
- Logging full API keys
- Storing keys in plain text files
- Sharing test output with keys

---

## References

- **Test Implementation**: `tests/orchestration/test_integration.py`
- **Agent Implementations**: `src/orchestration/agents/*.py`
- **Refactoring Summary**: `docs/design/multi-agent-refactoring-summary.md`
- **Gap Analysis**: `docs/gap-analysis.md`
- **Architecture**: `docs/specs/multi-agent-architecture.md`
