# Mnemosyne Python Orchestration Agents

Python agents for mnemosyne multi-agent orchestration system, integrated with Rust via PyO3.

## Overview

This package provides four specialized agents that integrate with the Rust supervision tree via the Claude SDK:

- **Executor** - Primary work agent for implementation tasks
- **Reviewer** - Quality assurance and validation
- **Optimizer** - Context and resource optimization
- **Orchestrator** - Central coordinator and state manager

## Requirements

**Python Version**: 3.9 or higher

**Dependencies**:
- `anthropic>=0.40.0` - Official Anthropic Python SDK

All other dependencies are Python standard library.

## Installation

### Method 1: Using uv (Recommended)

```bash
# Install uv if not already installed
curl -LsSf https://astral.sh/uv/install.sh | sh

# Install dependencies
cd src/orchestration/agents
uv pip install -r requirements.txt

# OR: Install from pyproject.toml
uv pip install -e .
```

### Method 2: Using pip

```bash
cd src/orchestration/agents
pip install -r requirements.txt

# OR: Install from pyproject.toml
pip install -e .
```

### Method 3: Install anthropic directly

```bash
pip install anthropic
# OR
uv pip install anthropic
```

## API Key Configuration

The agents require an Anthropic API key to communicate with Claude.

### Option 1: Environment Variable (Session-only)

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

### Option 2: mnemosyne Secrets (Persistent)

```bash
# Interactive setup
mnemosyne secrets init

# OR: Direct setup
mnemosyne config set-key "sk-ant-..."

# Verify configuration
mnemosyne config show-key
```

**Get API Key**: https://console.anthropic.com/settings/keys

## Environment Validation

Validate your environment before running agents:

```python
from base_agent import validate_environment

# Validates:
# - Python version >= 3.9
# - anthropic package installed
# - ANTHROPIC_API_KEY set (warns if missing)
# - PYTHONPATH includes agents directory
validate_environment()
```

**Output**:
```
✓ anthropic SDK installed: 0.40.0
✓ ANTHROPIC_API_KEY configured (sk-ant-...abcd)
```

## Usage

### From Rust (via PyO3 Bridge)

Agents are automatically spawned by the Rust supervision tree:

```rust
use mnemosyne_core::orchestration::ClaudeAgentBridge;
use mnemosyne_core::launcher::agents::AgentRole;

// Spawn Python agent
let bridge = ClaudeAgentBridge::spawn(
    AgentRole::Executor,
    event_broadcaster.sender()
).await?;

// Send work to agent
let work_item = WorkItem::new("Implement feature", ...);
let result = bridge.send_work(work_item).await?;
```

See `tests/orchestration_bridge_integration.rs` for full examples.

### Direct Python Usage (Testing)

```python
from agent_factory import create_agent

# Create agent
agent = create_agent("executor")

# Start Claude SDK session
agent.start_session()

# Execute work
work_dict = {
    "id": "work-1",
    "description": "Implement hello world",
    "phase": "plan_to_artifacts",
    "priority": 5
}
result = await agent.execute_work(work_dict)

# Stop session
agent.stop_session()
```

## Project Structure

```
src/orchestration/agents/
├── README.md                  # This file
├── requirements.txt           # Pip dependencies
├── pyproject.toml            # Modern Python package config
│
├── __init__.py               # Package exports
├── agent_factory.py          # Agent creation factory
├── base_agent.py             # Base classes and validation
├── claude_agent_sdk.py       # Claude SDK wrapper
│
├── executor.py               # Executor agent implementation
├── reviewer.py               # Reviewer agent implementation
├── optimizer.py              # Optimizer agent implementation
├── orchestrator.py           # Orchestrator agent implementation
│
├── logging_config.py         # Structured logging setup
├── error_context.py          # Enhanced error handling
├── validation.py             # Input validation
└── metrics.py                # Performance metrics tracking
```

## Development

### Running Tests

```bash
# From repository root
cd /Users/rand/src/mnemosyne

# Run Python agent tests (no API key required)
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo test --features python --test orchestration_bridge_integration

# Run with API key for full integration tests
export ANTHROPIC_API_KEY="sk-ant-..."
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo test --features python --test orchestration_bridge_integration -- --include-ignored
```

### Type Checking

```bash
# Install dev dependencies
uv pip install -e ".[dev]"

# Run mypy
mypy src/orchestration/agents/*.py
```

### Code Quality

```bash
# Install ruff
uv pip install ruff

# Lint
ruff check .

# Format
ruff format .
```

## Troubleshooting

### Import Error: No module named 'anthropic'

**Solution**:
```bash
uv pip install anthropic
# OR
pip install anthropic
```

### Import Error: No module named 'mnemosyne'

**Solution**: Add agents directory to PYTHONPATH:
```bash
export PYTHONPATH="${PWD}/src/orchestration/agents:${PYTHONPATH}"
```

Or call `validate_environment()` which adds it automatically.

### RuntimeError: ANTHROPIC_API_KEY not set

**Solution**: Configure API key (see "API Key Configuration" above)

### Python version too old

**Solution**: Install Python 3.9+:
```bash
# macOS
brew install python@3.11

# Or use pyenv
pyenv install 3.11.0
pyenv local 3.11.0
```

## Architecture

### PyO3 Bridge Integration

```
┌─────────────────┐
│ Rust Actors     │
│ (Supervision)   │
└────────┬────────┘
         │
         ↓
┌─────────────────────────┐
│ ClaudeAgentBridge       │
│ (PyO3 Wrapper)          │
└────────┬────────────────┘
         │
         ↓
┌─────────────────────────┐
│ Python Agents           │
│ - execute_work()        │
│ - start_session()       │
│ - stop_session()        │
└────────┬────────────────┘
         │
         ↓
┌─────────────────────────┐
│ Anthropic SDK           │
│ (Claude API)            │
└─────────────────────────┘
```

### Agent Lifecycle

1. **Spawn**: Rust creates ClaudeAgentBridge → Python agent instance
2. **Start**: `start_session()` initializes Claude SDK client
3. **Work**: `execute_work(WorkItem)` processes tasks via Claude
4. **Stop**: `stop_session()` cleans up Claude SDK connection

### Work Item Protocol

**Rust → Python** (WorkItem):
```python
{
    "id": "work-1",
    "description": "Task description",
    "phase": "plan_to_artifacts",
    "priority": 5,
    "consolidated_context_id": "ctx-1",  # optional
    "review_feedback": ["Fix X", "Add Y"],  # optional
    "review_attempt": 1  # optional
}
```

**Python → Rust** (WorkResult):
```python
{
    "success": True,
    "data": "Serialized result",  # optional
    "memory_ids": ["mem-1", "mem-2"],  # optional
    "error": None  # or error message if failed
}
```

## Production Hardening

Phase 5 production hardening features:

- ✅ **Structured Logging** - Multi-level logging with environment config
- ✅ **Error Context** - Enhanced errors with troubleshooting hints
- ✅ **Input Validation** - Early validation before expensive operations
- ✅ **Performance Metrics** - Duration, success rate, quality gate tracking
- ✅ **Environment Validation** - Python version, dependencies, API key checks

## Related Documentation

- **Testing Guide**: `docs/architecture/PYTHON_BRIDGE_TESTING.md`
- **Phase 5 Plan**: `docs/architecture/PHASE5_PRODUCTION_HARDENING.md`
- **Bridge Implementation**: `src/orchestration/claude_agent_bridge.rs`
- **API Key Setup**: `SECRETS_MANAGEMENT.md`

## License

MIT

---

**Last Updated**: 2025-11-06
**Maintainer**: Mnemosyne Python Bridge Team
