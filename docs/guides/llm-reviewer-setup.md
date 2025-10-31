# LLM Reviewer Setup Guide

This guide covers setting up the LLM-enhanced Reviewer agent for semantic validation in Mnemosyne's multi-agent orchestration system.

## Overview

The LLM-enhanced Reviewer uses Claude API (via Python bindings) to provide deep semantic validation beyond pattern matching:

- **Requirement Extraction**: Automatically extract testable requirements from user intent
- **Intent Validation**: Verify implementation satisfies original intent
- **Completeness Checking**: Ensure all requirements are fully implemented
- **Correctness Analysis**: Validate logic soundness and error handling
- **Improvement Guidance**: Generate actionable feedback for failed reviews

## Prerequisites

- **Rust**: 1.70+ with `cargo`
- **Python**: 3.9-3.13 (PyO3 limitation - not 3.14+)
- **uv**: Python package manager (`curl -LsSf https://astral.sh/uv/install.sh | sh`)
- **Claude API Key**: Anthropic API key with Claude access

## Installation Steps

### 1. Install Python Dependencies

The project uses `uv` for Python package management:

```bash
# Install uv if not already installed
curl -LsSf https://astral.sh/uv/install.sh | sh

# Install Python dependencies (from project root)
cd /Users/rand/src/mnemosyne
uv sync
```

**Required Python packages** (defined in `pyproject.toml`):
- `anthropic` - Claude API client
- `pyo3` - Rust-Python interop
- `pytest` - Testing framework
- `pytest-asyncio` - Async test support

### 2. Set Environment Variables

The LLM reviewer requires a Claude API key:

```bash
# Add to ~/.zshrc or ~/.bashrc
export ANTHROPIC_API_KEY="sk-ant-api03-..."

# Reload shell configuration
source ~/.zshrc  # or source ~/.bashrc
```

**Verify API key is set**:
```bash
echo $ANTHROPIC_API_KEY
# Should output: sk-ant-api03-...
```

### 3. Enable Python Feature Flag

The LLM functionality is behind a feature flag for conditional compilation.

**For development/testing**:
```bash
# Build with python feature
cargo build --features python

# Run tests with python feature
cargo test --features python --lib orchestration::actors::reviewer

# Run application with python feature
cargo run --features python
```

**For release builds**:
```bash
cargo build --release --features python
```

**In Cargo.toml** (already configured):
```toml
[features]
default = []
python = ["pyo3"]

[dependencies]
pyo3 = { version = "0.22", features = ["extension-module"], optional = true }
```

### 4. Verify Installation

**Step 1: Check Rust compilation**
```bash
cargo check --features python
# Should complete without errors
```

**Step 2: Run Rust tests**
```bash
cargo test --features python --lib orchestration::actors::reviewer::tests
# All tests should pass
```

**Step 3: Run Python tests**
```bash
uv run pytest tests/orchestration/test_reviewer_agent.py -v
# All tests should pass (requires compatible Python version)
```

**Step 4: Verify Python-Rust integration**
```bash
cargo test --features python --lib orchestration::actors::reviewer::tests::test_python_memory_format_conversion
# Should pass, validating memory format conversion
```

## Configuration

### ReviewerConfig Options

Configure LLM behavior in your application:

```rust
use mnemosyne::orchestration::actors::reviewer::ReviewerConfig;

let config = ReviewerConfig {
    max_llm_retries: 3,              // Retry attempts (default: 3)
    llm_timeout_secs: 60,            // Timeout per call (default: 60s)
    enable_llm_validation: true,     // Enable LLM (default: true)
    llm_model: "claude-3-5-sonnet-20241022".to_string(), // Model name
    max_context_tokens: 4096,        // Context limit (default: 4096)
    llm_temperature: 0.0,            // Temperature (default: 0.0)
};

state.update_config(config);
```

**Recommended settings by use case**:

| Use Case | Model | Timeout | Retries | Temperature |
|----------|-------|---------|---------|-------------|
| **Development** | claude-3-haiku | 30s | 2 | 0.0 |
| **Production** | claude-3-5-sonnet | 60s | 3 | 0.0 |
| **Complex validation** | claude-3-opus | 120s | 5 | 0.1 |

### Python ReviewerAgent Configuration

The Python ReviewerAgent (in `src/orchestration/agents/reviewer.py`) can be customized:

```python
from src.orchestration.agents.reviewer import ReviewerAgent, ReviewerConfig

config = ReviewerConfig(
    agent_id="reviewer",
    strict_mode=True,              # Fail on any gate failure
    min_test_coverage=0.70,        # 70% minimum coverage
    allowed_tools=["Read", "Glob", "Grep"],  # Claude tools
    permission_mode="default"
)

reviewer = ReviewerAgent(config, coordinator, storage)
```

## Usage

### Basic Usage

```rust
use mnemosyne::orchestration::actors::reviewer::{ReviewerActor, ReviewerState};

// Create reviewer
let mut state = ReviewerState::new(storage, namespace);

// Register Python reviewer (enables LLM)
#[cfg(feature = "python")]
state.register_py_reviewer(py_reviewer);

// Submit work for review
// Requirements automatically extracted from intent
// Review validates against requirements
// Failed reviews include improvement guidance
```

### Registering Python Reviewer

From Rust side:

```rust
#[cfg(feature = "python")]
{
    use pyo3::prelude::*;
    use std::sync::Arc;

    // Create Python ReviewerAgent instance (from Python code)
    let py_reviewer: Arc<PyObject> = /* ... */;

    // Register with Rust reviewer
    state.register_py_reviewer(py_reviewer);
    // LLM validation now enabled
}
```

### Disabling LLM Validation

To temporarily disable (falls back to pattern matching):

```rust
state.disable_llm_validation();

// Re-enable
let mut config = state.config.clone();
config.enable_llm_validation = true;
state.update_config(config);
```

## Troubleshooting

### Issue: PyO3 Version Incompatibility

**Symptom**: Build error: "configured Python interpreter version (3.14) is newer than PyO3's maximum supported version (3.13)"

**Solution**: Use Python 3.9-3.13
```bash
# Check Python version
uv run python --version

# If 3.14+, install compatible Python version
# Option 1: Use pyenv
pyenv install 3.13
pyenv local 3.13

# Option 2: Set PYO3_USE_ABI3_FORWARD_COMPATIBILITY (not recommended)
export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1
cargo build --features python
```

### Issue: Missing ANTHROPIC_API_KEY

**Symptom**: Runtime error: "API key not found"

**Solution**: Set environment variable
```bash
export ANTHROPIC_API_KEY="sk-ant-api03-..."
# Verify
echo $ANTHROPIC_API_KEY
```

### Issue: LLM Calls Timing Out

**Symptom**: `LlmTimeout` error after 60s

**Solution**: Increase timeout
```rust
config.llm_timeout_secs = 120;  // 2 minutes
config.max_context_tokens = 2048;  // Reduce context if slow
```

### Issue: Retry Exhaustion

**Symptom**: `LlmRetryExhausted` error after max attempts

**Solutions**:
1. **Check API key validity**
   ```bash
   curl https://api.anthropic.com/v1/messages \
     -H "anthropic-version: 2023-06-01" \
     -H "x-api-key: $ANTHROPIC_API_KEY" \
     -H "content-type: application/json" \
     -d '{"model":"claude-3-5-sonnet-20241022","max_tokens":10,"messages":[{"role":"user","content":"test"}]}'
   ```

2. **Increase retry limit**
   ```rust
   config.max_llm_retries = 5;
   ```

3. **Check network connectivity**
   ```bash
   ping api.anthropic.com
   ```

### Issue: Python Import Errors

**Symptom**: "ModuleNotFoundError: No module named 'anthropic'"

**Solution**: Install dependencies with uv
```bash
uv sync
# Or manually
uv add anthropic pyo3 pytest pytest-asyncio
```

### Issue: Tests Failing

**Symptom**: Test failures with `--features python`

**Debug steps**:
```bash
# 1. Check compilation
cargo check --features python

# 2. Run specific test with logs
RUST_LOG=mnemosyne_core::orchestration::actors::reviewer=debug \
  cargo test --features python --lib \
  orchestration::actors::reviewer::tests::test_pattern_matching_fallback

# 3. Check Python side
uv run pytest tests/orchestration/test_reviewer_agent.py -v -s
```

### Issue: Build Performance

**Symptom**: Build with `--features python` is slow

**Solution**: Use sccache for caching
```bash
# Install sccache
cargo install sccache

# Configure Cargo
export RUSTC_WRAPPER=sccache

# Build
cargo build --features python
```

## Performance Tuning

### Context Budget Optimization

**Default limits**:
- **max_context_tokens**: 4096
- **memories_per_call**: 20 (intent), 10 (completeness/correctness)
- **content_chars_per_memory**: 500 (collect), 200 (format)

**Optimization strategies**:

1. **Reduce context for faster calls**:
   ```rust
   config.max_context_tokens = 2048;  // Halves context window
   ```

2. **Increase for complex validation**:
   ```rust
   config.max_context_tokens = 8192;  // Doubles context window
   ```

3. **Adjust memory limits** (in `python_bindings/reviewer.rs`):
   - Edit `collect_implementation_from_memories()` memory limit (line 276)
   - Edit `execution_memories_to_python_format()` memory limit (line 308)

### Retry Strategy Tuning

**Default**: 3 retries with exponential backoff (1s → 2s → 4s)

**For high-reliability tasks**:
```rust
config.max_llm_retries = 5;  // Total ~67s max
```

**For fast iteration**:
```rust
config.max_llm_retries = 1;  // No retries
config.llm_timeout_secs = 30;  // Quick timeout
```

## Monitoring

### Enable Detailed Logging

```bash
# Debug level for reviewer
RUST_LOG=mnemosyne_core::orchestration::actors::reviewer=debug cargo run --features python

# Trace level for all LLM calls
RUST_LOG=mnemosyne_core::orchestration::actors::reviewer=trace cargo run --features python

# Info level for production
RUST_LOG=mnemosyne_core::orchestration::actors::reviewer=info cargo run --features python
```

### Track Metrics

Monitor in production:
- **LLM call success rate**: Track `Ok()` vs `Err()` from retry_llm_operation
- **Average retry count**: Log `attempt` number on success
- **Requirement satisfaction rate**: Track satisfied vs unsatisfied requirements
- **Review pass/fail ratio**: Track `gates_passed` results

## Best Practices

### 1. Model Selection

| Model | Use Case | Cost | Speed |
|-------|----------|------|-------|
| **claude-3-haiku** | Simple tasks, dev/test | Low | Fast |
| **claude-3-5-sonnet** | Standard validation | Medium | Balanced |
| **claude-3-opus** | Complex analysis | High | Slow |

### 2. Temperature Settings

- **Validation** (0.0): Deterministic, consistent
- **Generation** (0.3-0.7): Creative guidance
- **Extraction** (0.0-0.1): Structured output

### 3. Graceful Degradation

Always provide fallback for LLM failures:
```rust
// LLM validation with automatic fallback
#[cfg(feature = "python")]
if state.config.enable_llm_validation && state.py_reviewer.is_some() {
    match retry_llm_operation!(...) {
        Ok(result) => return Ok(result),
        Err(e) => {
            tracing::warn!("LLM failed, falling back: {}", e);
            // Continue to pattern matching
        }
    }
}

// Pattern matching fallback
pattern_based_validation()
```

### 4. Explicit Requirements

Provide explicit requirements when possible to avoid extraction overhead:
```rust
let mut work_item = WorkItem::new(...);
work_item.requirements = vec![
    "Implement authentication middleware".to_string(),
    "Add session validation".to_string(),
];
```

## Next Steps

- Read [LLM Reviewer Guide](./llm-reviewer.md) for detailed feature documentation
- Review [Multi-Agent Architecture](../specs/multi-agent-architecture.md)
- Check [Python Bindings](../../src/python_bindings/reviewer.rs) source code

## Support

For issues or questions:
- **GitHub Issues**: https://github.com/yourusername/mnemosyne/issues
- **Discussions**: https://github.com/yourusername/mnemosyne/discussions
