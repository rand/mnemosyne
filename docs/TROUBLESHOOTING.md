# Troubleshooting Guide

**Comprehensive troubleshooting for Mnemosyne Python bridge and orchestration system**

This guide covers common issues encountered when running mnemosyne with Python agents, Claude SDK integration, and multi-agent orchestration.

---

## Table of Contents

1. [Common Issues](#common-issues)
2. [Diagnostic Commands](#diagnostic-commands)
3. [Error Reference](#error-reference)
4. [Recovery Procedures](#recovery-procedures)
5. [Performance Issues](#performance-issues)

---

## Common Issues

### 1. ModuleNotFoundError: No module named 'mnemosyne'

**Symptom**:
```
Agent factory import failed: ModuleNotFoundError: No module named 'mnemosyne'
```

**Root Cause**: Python can't find the mnemosyne orchestration modules because PYTHONPATH isn't set correctly.

**Solution**:
```bash
# Set PYTHONPATH to include the src directory
export PYTHONPATH="/path/to/mnemosyne/src:${PYTHONPATH}"

# For project root:
export PYTHONPATH="$(pwd)/src:${PYTHONPATH}"

# Verify it's set correctly:
echo $PYTHONPATH
```

**For Tests**:
```bash
PYTHONPATH="/path/to/mnemosyne/src" cargo test --features python
```

---

### 2. API Key Not Configured

**Symptom**:
```
RuntimeError: ANTHROPIC_API_KEY environment variable not set.
Get your API key from: https://console.anthropic.com/settings/keys
```

**Root Cause**: Python agents need access to the Anthropic API key, which must be loaded before Python initialization.

**Solution (Priority Order)**:

1. **Environment Variable** (highest priority):
   ```bash
   export ANTHROPIC_API_KEY="sk-ant-api03-..."
   ```

2. **Age-Encrypted Config** (recommended for local development):
   ```bash
   # Initialize secrets system
   mnemosyne secrets init

   # Enter API key when prompted
   # Key stored in: ~/.config/mnemosyne/secrets.age
   ```

3. **Verify Configuration**:
   ```bash
   # Check if API key is configured
   mnemosyne config show-key

   # Should show: API key configured: sk-ant-a...ygAA
   ```

**For E2E Tests**: Ensure API key is loaded BEFORE `pyo3::prepare_freethreaded_python()`:
```rust
// In Rust test:
let secrets = SecretsManager::new()?;
let api_key = secrets.get_secret("ANTHROPIC_API_KEY")?;
std::env::set_var("ANTHROPIC_API_KEY", api_key.expose_secret());

pyo3::prepare_freethreaded_python();  // AFTER setting env var
```

**Reference**: See `SECRETS_MANAGEMENT.md` for detailed documentation on secrets system.

---

### 3. Python Bridge Won't Spawn

**Symptom**:
```
Failed to spawn executor bridge: Other("Agent creation failed: ...")
```

**Common Causes**:

#### 3a. Missing Python Dependencies
```
ImportError: anthropic package not installed
```

**Solution**:
```bash
# Install anthropic SDK
pip install --user anthropic

# Or using uv (recommended):
cd src/orchestration/agents
uv pip install anthropic

# Verify installation:
python -c "import anthropic; print(anthropic.__version__)"
```

#### 3b. Import Path Issues
```
Agent factory import failed: ModuleNotFoundError: No module named 'mnemosyne.orchestration.agents.agent_factory'
```

**Solution**:
```bash
# Check package structure exists:
ls -la src/mnemosyne/__init__.py
ls -la src/mnemosyne/orchestration  # Should be symlink to ../orchestration

# If missing, create:
mkdir -p src/mnemosyne
echo '__version__ = "2.1.1"' > src/mnemosyne/__init__.py
cd src/mnemosyne && ln -s ../orchestration orchestration
```

#### 3c. Agent Initialization Failures
```
Agent creation failed: TypeError: ExecutorAgent.__init__() got an unexpected keyword argument
```

**Solution**: Check that `agent_factory.py` passes correct parameters to agent constructors:
```python
# ExecutorAgent requires: config, coordinator, storage, parallel_executor
# OrchestratorAgent requires: config, coordinator, storage, context_monitor
# OptimizerAgent requires: config, skills_directory, storage
# ReviewerAgent requires: config, storage
```

For standalone testing, `agent_factory.py` provides mock dependencies (MockCoordinator, MockStorage, etc.).

---

### 4. RuntimeWarning: coroutine was never awaited

**Symptom**:
```
RuntimeWarning: coroutine 'ExecutorAgent.start_session' was never awaited
RuntimeWarning: coroutine 'AgentExecutionMixin.execute_work' was never awaited
```

**Root Cause**: Rust bridge called async Python methods without awaiting the coroutines.

**Solution**: Use `asyncio.run()` to execute Python async methods from Rust:

```rust
// Import asyncio
let asyncio = py.import_bound("asyncio")?;

// Call async method to get coroutine
let coro = agent_ref.call_method0("start_session")?;

// Run coroutine
asyncio.call_method1("run", (coro,))?;
```

**Fixed in**: commit `43876e3` - "Properly await Python async methods in bridge"

---

### 5. Agent Health Degraded

**Symptom**:
```
Agent state: degraded
Health check failing
```

**Diagnostic Steps**:

1. **Check Agent Status**:
   ```bash
   # Via API (if dashboard running):
   curl http://localhost:3000/agents | jq

   # Expected output:
   # {
   #   "agents": [
   #     {"id": "executor-agent", "state": "active", "health": "healthy"}
   #   ]
   # }
   ```

2. **Check Logs**:
   ```bash
   # Enable debug logging:
   RUST_LOG=debug cargo run

   # Python agent logs:
   # Look for lines with [mnemosyne.orchestration.executor]
   ```

3. **Common Causes**:
   - API key expired or invalid
   - Network connectivity issues to api.anthropic.com
   - Python interpreter crashed
   - Out of memory

---

### 6. Work Validation Failures

**Symptom**:
```
Work plan validation failed: ['Vague requirement: 'simple'', 'Prompt lacks detail']
```

**Root Cause**: ExecutorAgent validates work plans before execution to ensure clear requirements.

**Validation Requirements**:
- ✅ Prompt specified (not empty)
- ✅ Tech stack defined
- ✅ Success criteria provided
- ✅ Adequate detail (>10 words)
- ✅ No vague terms ("simple", "quick", "just", "easy")
- ✅ Covers: what, why, how, constraints, scope

**Solution**: Provide detailed work plans:

**❌ Bad (vague)**:
```python
"Write a simple hello world function"
```

**✅ Good (detailed)**:
```python
"Create a hello_world() function in Python. The function should print
'Hello, World!' to stdout using the print() function. This is needed
for testing the Python bridge with actual Claude API calls. The function
must execute without errors and return None."
```

**To Bypass Validation** (for testing only):
Set `challenge_vague_requirements: false` in `ExecutorConfig`.

---

### 7. AttributeError: 'dict' object has no attribute 'success'

**Symptom**:
```
Failed to get success: AttributeError: 'dict' object has no attribute 'success'
```

**Root Cause**: Python `execute_work()` returns a dict, but Rust code used `.getattr()` (attribute access) instead of dict item access.

**Solution**: Use `PyDict` and `.get_item()`:

```rust
// ❌ Wrong (attribute access):
let success = result.getattr("success")?.extract::<bool>()?;

// ✅ Correct (dict item access):
let result_dict = result.downcast::<PyDict>()?;
let success = result_dict
    .get_item("success")?
    .ok_or_else(|| MnemosyneError::Other("Missing 'success' key".to_string()))?
    .extract::<bool>()?;
```

**Fixed in**: commit `e743341` - "Use dict item access instead of attribute access"

---

## Diagnostic Commands

### Environment Checks

```bash
# Check Rust version (1.70+ required):
rustc --version

# Check Python version (3.9+ required):
python --version

# Check anthropic SDK:
python -c "import anthropic; print(f'anthropic {anthropic.__version__}')"

# Check PYTHONPATH:
echo $PYTHONPATH

# Verify mnemosyne can be imported:
python -c "from mnemosyne.orchestration.agents import agent_factory; print('✓ Import successful')"
```

### API Key Checks

```bash
# Check if API key is configured:
mnemosyne config show-key

# Check secrets system status:
mnemosyne secrets info

# List configured secrets:
mnemosyne secrets list

# Test API key validity:
python -c "import anthropic; client = anthropic.Anthropic(); print('✓ API key valid')"
```

### Agent Health Checks

```bash
# Check agent status via API:
curl http://localhost:3000/agents | jq

# Check dashboard SSE events:
curl -N http://localhost:3000/events

# Check if Python agents can be created:
python -c "
from mnemosyne.orchestration.agents.agent_factory import create_agent
agent = create_agent('executor')
print(f'✓ Agent created: {agent}')
"
```

### Build and Test Checks

```bash
# Build with Python feature:
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo build --features python

# Run integration tests (no external deps):
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo test --features python --test orchestration_bridge_integration

# Run E2E tests (requires API key):
PYTHONPATH="$(pwd)/src" PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 \
  cargo test --features python --test python_bridge_e2e -- --include-ignored

# Check for Python import errors:
PYTHONPATH="$(pwd)/src" python -c "
from mnemosyne.orchestration.agents import executor, orchestrator, optimizer, reviewer
print('✓ All agents import successfully')
"
```

### Log Analysis

```bash
# Enable debug logging:
export RUST_LOG=debug

# Enable Python agent logging:
export RUST_LOG=mnemosyne::orchestration=debug

# Run with verbose output:
RUST_LOG=debug cargo run -- <command>

# Filter logs by component:
cargo run 2>&1 | grep "mnemosyne.orchestration.executor"

# Check for errors:
cargo run 2>&1 | grep -i "error\|failed\|panic"
```

---

## Error Reference

### Bridge Errors

| Error | Meaning | Recovery |
|-------|---------|----------|
| `Agent factory import failed` | Python module import error | Check PYTHONPATH, verify package structure |
| `Failed to spawn executor bridge` | Agent creation failed | Check API key, Python dependencies |
| `Session start failed` | Claude SDK connection error | Verify API key, network connectivity |
| `Agent execution failed` | Work processing error | Check logs for Python traceback |
| `Failed to extract success` | Result conversion error | Verify execute_work returns proper dict |

### Python Errors

| Error | Meaning | Recovery |
|-------|---------|----------|
| `ModuleNotFoundError: mnemosyne` | Import path issue | Set PYTHONPATH to src directory |
| `ImportError: anthropic package not installed` | Missing dependency | Install with `pip install anthropic` |
| `RuntimeError: ANTHROPIC_API_KEY not set` | API key not configured | Run `mnemosyne secrets init` |
| `TypeError: __init__() got unexpected keyword` | Wrong agent parameters | Check agent_factory.py |
| `AttributeError: 'NoneType' object has no attribute` | Missing mock dependency | Verify MockCoordinator, MockStorage instantiated |

### API Errors

| Error | Meaning | Recovery |
|-------|---------|----------|
| `401 Unauthorized` | Invalid API key | Regenerate key at console.anthropic.com |
| `429 Too Many Requests` | Rate limit exceeded | Wait and retry with backoff |
| `500 Internal Server Error` | Claude API issue | Wait and retry |
| `Connection timeout` | Network issue | Check firewall, proxy settings |

### Validation Errors

| Error | Meaning | Recovery |
|-------|---------|----------|
| `Missing prompt/requirements` | WorkItem.description empty | Provide non-empty description |
| `Tech stack not specified` | No tech_stack in work plan | Add tech_stack field |
| `Vague requirement: 'simple'` | Unclear prompt | Provide specific, detailed requirements |
| `Prompt lacks detail` | Insufficient information | Add what, why, how, constraints, scope |

---

## Recovery Procedures

### Quick Fixes

**Issue**: Tests failing with import errors
```bash
# Fix PYTHONPATH and retry:
export PYTHONPATH="$(pwd)/src"
cargo test --features python
```

**Issue**: API key not found
```bash
# Re-initialize secrets:
mnemosyne secrets init

# Or set environment variable:
export ANTHROPIC_API_KEY="sk-ant-..."
```

**Issue**: Python dependencies missing
```bash
# Install required packages:
pip install --user anthropic

# Or create virtual environment:
python -m venv venv
source venv/bin/activate
pip install anthropic
```

### When to Restart vs. Rebuild

**Restart** (agent process):
- API key changed
- Configuration updated
- Agent state corrupted
- Memory leak suspected

```bash
# Restart via signal:
pkill -f mnemosyne
cargo run
```

**Rebuild** (recompile):
- Code changes to Rust
- PyO3 binding changes
- Feature flags changed
- Dependency updates

```bash
# Clean rebuild:
cargo clean
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo build --features python
```

**Full Reset** (nuclear option):
- Persistent issues
- Corrupted build artifacts
- Mystery errors

```bash
# Clean everything:
cargo clean
rm -rf target/
rm -rf src/mnemosyne/orchestration  # Symlink
rm -f Cargo.lock

# Rebuild from scratch:
mkdir -p src/mnemosyne
echo '__version__ = "2.1.1"' > src/mnemosyne/__init__.py
cd src/mnemosyne && ln -s ../orchestration orchestration
cd ../..
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo build --features python
```

---

## Performance Issues

### Slow Agent Responses

**Symptom**: Agent takes >60 seconds to respond

**Diagnostic**:
```bash
# Check if it's API call latency:
time python -c "
import anthropic
client = anthropic.Anthropic()
response = client.messages.create(
    model='claude-sonnet-4-20250514',
    max_tokens=100,
    messages=[{'role': 'user', 'content': 'Hello'}]
)
print(f'Response: {response.content[0].text}')
"
```

**Common Causes**:
- Network latency to api.anthropic.com (check with `ping api.anthropic.com`)
- Large context (check message history length)
- Rate limiting (429 errors in logs)

**Solutions**:
- Use faster model (claude-haiku-4-20250514)
- Reduce max_tokens
- Clear conversation history periodically

### High Memory Usage

**Symptom**: Memory usage grows over time

**Diagnostic**:
```bash
# Monitor memory:
ps aux | grep mnemosyne

# Check for memory leaks:
valgrind --leak-check=full target/debug/mnemosyne
```

**Common Causes**:
- Python objects not released
- Conversation history accumulation
- Mock storage not clearing

**Solutions**:
- Restart agents periodically
- Limit conversation history size
- Clear mock storage buffers

### Context Budget Exceeded

**Symptom**:
```
Context utilization at 95%
```

**Solution**:
- Trigger context preservation (automatic at 75%)
- Compress old messages
- Archive non-critical data

---

## Getting Help

If this guide doesn't resolve your issue:

1. **Check Logs**: Run with `RUST_LOG=debug` and examine full output
2. **Search Issues**: https://github.com/rand/mnemosyne/issues
3. **Open Issue**: Include:
   - Full error message and backtrace
   - Environment details (OS, Rust version, Python version)
   - Relevant logs
   - Steps to reproduce
4. **Reference Docs**:
   - `PYTHON_BRIDGE_ARCHITECTURE.md` - Architecture overview
   - `PYTHON_BRIDGE_TESTING.md` - Testing guide
   - `SECRETS_MANAGEMENT.md` - API key configuration
   - `PHASE5_PRODUCTION_HARDENING.md` - Production hardening details

---

## Quick Reference

### Essential Commands

```bash
# Set up environment:
export PYTHONPATH="$(pwd)/src"
export ANTHROPIC_API_KEY="sk-ant-..."  # Or use mnemosyne secrets init

# Build with Python support:
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo build --features python

# Run tests:
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo test --features python

# Run E2E tests:
PYTHONPATH="$(pwd)/src" PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 \
  cargo test --features python --test python_bridge_e2e -- --include-ignored

# Check agent health:
curl http://localhost:3000/agents | jq

# View logs:
RUST_LOG=debug cargo run
```

### File Locations

- **Python agents**: `src/orchestration/agents/*.py`
- **Rust bridge**: `src/orchestration/claude_agent_bridge.rs`
- **Tests**: `tests/python_bridge_e2e.rs`, `tests/orchestration_bridge_integration.rs`
- **Secrets**: `~/.config/mnemosyne/secrets.age`, `~/.config/mnemosyne/identity.key`
- **Logs**: stdout/stderr (configure with `RUST_LOG`)

---

**Last Updated**: Phase 5.7 E2E validation completion (2025-11-06)
