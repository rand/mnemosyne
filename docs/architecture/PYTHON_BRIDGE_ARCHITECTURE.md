# Python Bridge Architecture

**Status**: Phases 1-4 Complete (Python Agents Integrated)
**Last Updated**: 2025-11-07
**Commits**: 05b0098, e4bbbff, a354fe7, 14d38f4, 8ea8da1, 43b9783, 5a728f4, 83b62c3

---

## Overview

The mnemosyne multi-agent orchestration system uses a **Rust↔Python bridge architecture** to combine:
- **Rust Ractor supervision tree**: Fault tolerance, lifecycle management, heartbeats
- **Python Claude SDK agents**: LLM-powered intelligence and agent collaboration
- **PyO3 bridge**: Async-safe connection between Rust and Python

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                     Supervision Tree (Rust)                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│  │ Orchestrator │  │  Optimizer   │  │   Reviewer   │  ...    │
│  │   (Ractor)   │  │   (Ractor)   │  │   (Ractor)   │         │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘         │
│         │                  │                  │                  │
│         │ RegisterPythonBridge               │                  │
│         ▼                  ▼                  ▼                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │           ClaudeAgentBridge (PyO3)                       │  │
│  │  - spawn(): Creates Python agent                        │  │
│  │  - send_work(): Delegates to Python                     │  │
│  │  - record_error(): Tracks failures                      │  │
│  │  - restart(): Automatic recovery                        │  │
│  └──────────────┬───────────────────────────────────────────┘  │
└─────────────────┼───────────────────────────────────────────────┘
                  │ PyO3 FFI
                  ▼
┌─────────────────────────────────────────────────────────────────┐
│                  Python Claude SDK Agents                        │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  agent_factory.py: create_agent(role) → Agent instance  │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ✅ COMPLETE: All agent implementations integrated (Phase 4)   │
│  - orchestrator.py: OrchestratorAgent (AgentExecutionMixin)    │
│  - optimizer.py: OptimizerAgent (AgentExecutionMixin)          │
│  - reviewer.py: ReviewerAgent (AgentExecutionMixin)            │
│  - executor.py: ExecutorAgent (AgentExecutionMixin)            │
│                                                                  │
│  All implement: _execute_work_item(WorkItem) → WorkResult     │
└─────────────────────────────────────────────────────────────────┘
                  │
                  ▼
            Dashboard (SSE)
     (AgentInfo with AgentHealth)
```

## Phases Complete

### ✅ Phase 1: PyO3 Bridge Foundation
**Files Created**:
- `src/orchestration/claude_agent_bridge.rs`: Bridge implementation
- `src/orchestration/agents/base_agent.py`: WorkItem/WorkResult protocol
- `src/orchestration/agents/agent_factory.py`: Agent spawning

**Key Features**:
- Async-safe GIL management with `tokio::spawn_blocking`
- Type-safe Rust↔Python data conversion
- WorkItem → Python dict → Python agent → WorkResult
- Event broadcasting for dashboard visibility

---

### ✅ Phase 2: Actor Integration
**Files Modified**:
- `src/orchestration/actors/orchestrator.rs`
- `src/orchestration/actors/optimizer.rs`
- `src/orchestration/actors/reviewer.rs`
- `src/orchestration/actors/executor.rs`
- `src/orchestration/messages.rs`
- `src/orchestration/supervision.rs`

**Pattern Established**:
```rust
pub struct ActorState {
    #[cfg(feature = "python")]
    python_bridge: Option<ClaudeAgentBridge>,
}

#[cfg(feature = "python")]
pub fn register_python_bridge(&mut self, bridge: ClaudeAgentBridge) {
    self.python_bridge = Some(bridge);
}

// In Actor::handle()
#[cfg(feature = "python")]
ActorMessage::RegisterPythonBridge(bridge) => {
    state.register_python_bridge(bridge);
}
```

**Supervision Tree Integration**:
- Automatically spawns Python bridges on `start()`
- Graceful degradation if Python unavailable
- Error logged, Rust actors continue

**Error Handling**:
- `record_error()`: Increments counter, updates timestamp
- `should_restart()`: 5 errors/60s threshold
- `restart()`: Respawns Python agent, resets counters
- Automatic error tracking in `send_work()`

---

### ✅ Phase 3: Dashboard Health Tracking
**Files Modified**:
- `src/api/state.rs`: Added `AgentHealth` struct
- `src/api/events.rs`: Added health event types
- `src/orchestration/claude_agent_bridge.rs`: Health event broadcasting
- `src/orchestration/supervision.rs`: Health initialization

**New Data Structures**:
```rust
pub struct AgentHealth {
    pub error_count: usize,
    pub last_error: Option<DateTime<Utc>>,
    pub is_healthy: bool,
    pub last_restart: Option<DateTime<Utc>>,
}

pub struct AgentInfo {
    pub id: String,
    pub state: AgentState,
    pub updated_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
    pub health: Option<AgentHealth>,  // ← NEW
}
```

**New Event Types**:
- `AgentErrorRecorded`: Tracks individual errors
- `AgentHealthDegraded`: Warns at 3+ errors
- `AgentRestarted`: Records automatic restarts

**State Manager Updates**:
- Processes health events via `apply_event_static()`
- Updates agent health from error/degraded/restart events
- Initializes agents with healthy state

---

## Integration Tests

**File**: `tests/orchestration_bridge_integration.rs`

**Test Coverage**:
- `test_python_bridge_spawn_and_registration`: Validates bridge creation
- `test_work_delegation_to_python_agent`: Tests work submission
- `test_bridge_error_handling`: Verifies error messages
- `test_graceful_degradation_without_python_bridges`: Validates fallback
- `test_concurrent_work_processing`: Tests parallel execution

---

## Current State

### ✅ **Complete (Phases 1-4)**:
1. **Supervision tree** spawns Rust actors
2. **Python bridges** auto-spawn on startup (if Python available)
3. **Bridges register** with actors via `RegisterPythonBridge` messages
4. **Error tracking** records failures and triggers restart at threshold
5. **Health events** broadcast to dashboard via SSE
6. **StateManager** updates agent health from events
7. **Graceful degradation** when Python unavailable
8. **Python agent implementations** integrated with PyO3 bridge:
   - `src/orchestration/agents/orchestrator.py` ✅ **COMPLETE**
   - `src/orchestration/agents/optimizer.py` ✅ **COMPLETE**
   - `src/orchestration/agents/reviewer.py` ✅ **COMPLETE**
   - `src/orchestration/agents/executor.py` ✅ **COMPLETE**

### ✅ **Agent Integration Pattern**:
All agents implement the standard PyO3 bridge interface:
```python
from .base_agent import AgentExecutionMixin, WorkItem, WorkResult

class ExecutorAgent(AgentExecutionMixin):
    async def _execute_work_item(self, work_item: WorkItem) -> WorkResult:
        # Convert WorkItem to agent's internal format
        work_plan = {...}

        # Execute using existing agent methods
        result = await self.execute_work_plan(work_plan)

        # Convert result to WorkResult for Rust bridge
        return WorkResult(success=True, data=json.dumps(result), ...)
```

### ⚠️ **Remaining (Phase 5 - Testing & Hardening)**:
1. **Integration testing** with Python feature enabled
2. **API key configuration** (already exists in secrets system)
3. **End-to-end validation** with actual Claude SDK calls
4. **Production hardening** (error messages, logging, monitoring)

---

## Next Steps (Phase 5 - Testing & Validation)

### 1. Run Integration Tests
Test the complete Rust↔Python bridge with all agents:
```bash
# Run all integration tests with Python feature
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo test --test orchestration_bridge_integration --features python

# Run specific work delegation test (requires API key)
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo test --test orchestration_bridge_integration --features python test_work_delegation -- --ignored
```

### 2. Verify API Key Configuration
Ensure Anthropic API key is accessible:
```bash
# Check configuration status
mnemosyne config show-key

# Set API key if needed
export ANTHROPIC_API_KEY="sk-ant-..."
# OR
mnemosyne secrets init
```

### 3. Test Individual Agents
Validate each agent's integration:
```bash
# Test Executor agent
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo test --features python executor

# Test Reviewer agent
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo test --features python reviewer

# Test Optimizer agent
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo test --features python optimizer

# Test Orchestrator agent
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo test --features python orchestrator
```

### 4. Production Hardening
- Improve error messages and logging
- Add health check endpoints
- Implement graceful shutdown
- Add metrics and monitoring
- Document deployment procedures

---

## Configuration

### Python Feature Flag
```toml
# Cargo.toml
[features]
python = ["pyo3", "pyo3-asyncio"]
```

### Build Commands
```bash
# With Python support
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo build --features python

# Without Python (Rust-only supervision)
cargo build
```

### API Key Setup
```bash
# Method 1: Environment variable
export ANTHROPIC_API_KEY="sk-ant-..."

# Method 2: Encrypted config
mnemosyne secrets init

# Method 3: OS Keychain
mnemosyne config set-key "sk-ant-..."
```

---

## Troubleshooting

### Python Bridge Won't Spawn
**Symptom**: Supervision tree starts, but Python agents don't spawn.

**Check**:
1. Python feature enabled: `cargo build --features python`
2. Python initialized: `pyo3::prepare_freethreaded_python()`
3. Modules exist: `src/orchestration/agents/{orchestrator,optimizer,reviewer,executor}.py`
4. API key set: `echo $ANTHROPIC_API_KEY`

**Logs**:
```
WARN Failed to initialize Python bridge for Executor: Agent factory import failed
```

### Graceful Degradation
**Symptom**: System works but agents show "idle" in dashboard.

**Cause**: Python bridges failed to spawn (expected without Phase 4).

**Result**: Rust actors send heartbeats, dashboard shows agents, but no LLM intelligence.

---

## References

- **Mnemosyne memories**: `mnemosyne recall "phase2 OR phase3" --min-importance 8`
- **Git commits**: `git log --oneline --grep="feat(orchestration)"`
- **Integration tests**: `tests/orchestration_bridge_integration.rs`
- **PyO3 docs**: https://pyo3.rs/
- **Claude SDK**: https://docs.anthropic.com/claude/reference/client-sdks

---

## Contact & Continuation

For questions or to continue development after context compaction:

1. **Read this file** for architecture overview
2. **Check mnemosyne memories**: `mnemosyne recall "orchestration architecture"`
3. **Review recent commits**: `git log --oneline -20`
4. **Run tests**: `cargo test --features python`
5. **Start with Phase 4**: Implement Python agent modules
