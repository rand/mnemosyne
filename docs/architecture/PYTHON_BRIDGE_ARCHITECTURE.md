# Python Bridge Architecture

**Status**: Phases 1-3 Complete (Infrastructure Ready)
**Last Updated**: 2025-11-07
**Commits**: 05b0098, e4bbbff, a354fe7, 14d38f4, 8ea8da1, 43b9783

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
│  ⚠️  MISSING: Agent implementations (Phase 4)                   │
│  Need to create:                                                │
│  - orchestrator.py: OrchestratorAgent                           │
│  - optimizer.py: OptimizerAgent                                 │
│  - reviewer.py: ReviewerAgent                                   │
│  - executor.py: ExecutorAgent                                   │
│                                                                  │
│  Each implements: AgentExecutionMixin._execute_work_item()     │
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

### ✅ **Working**:
1. **Supervision tree** spawns Rust actors
2. **Python bridges** auto-spawn on startup (if Python available)
3. **Bridges register** with actors via `RegisterPythonBridge` messages
4. **Error tracking** records failures and triggers restart at threshold
5. **Health events** broadcast to dashboard via SSE
6. **StateManager** updates agent health from events
7. **Graceful degradation** when Python unavailable

### ⚠️ **Missing (Phase 4 - CRITICAL)**:
1. **Python agent implementations** don't exist yet:
   - `src/orchestration/agents/orchestrator.py` ← **NEEDED**
   - `src/orchestration/agents/optimizer.py` ← **NEEDED**
   - `src/orchestration/agents/reviewer.py` ← **NEEDED**
   - `src/orchestration/agents/executor.py` ← **NEEDED**

2. **Each must implement**:
   ```python
   from .base_agent import AgentExecutionMixin, WorkItem, WorkResult

   class ExecutorAgent(AgentExecutionMixin):
       async def _execute_work_item(self, work_item: WorkItem) -> WorkResult:
           # TODO: Integrate with Claude SDK
           # TODO: Execute work using LLM
           # TODO: Return result with memory_ids
           pass
   ```

3. **Claude SDK Integration**:
   - API key management (use existing secrets system)
   - Session management (`start_session()`, `stop_session()`)
   - Prompt engineering for each role
   - Memory integration (store execution artifacts)

---

## Next Steps (Phase 4)

### 1. Create Executor Agent (Simplest)
```bash
# Create file: src/orchestration/agents/executor.py

from .base_agent import AgentExecutionMixin, WorkItem, WorkResult
import anthropic
import os

class ExecutorConfig:
    agent_id: str = "executor"
    model: str = "claude-3-7-sonnet-20250219"

class ExecutorAgent(AgentExecutionMixin):
    def __init__(self, config, storage=None, skills_cache=None):
        self.config = config
        self.storage = storage
        self.client = None

    async def start_session(self):
        api_key = os.getenv("ANTHROPIC_API_KEY")
        if not api_key:
            raise RuntimeError("ANTHROPIC_API_KEY not set")
        self.client = anthropic.AsyncAnthropic(api_key=api_key)

    async def stop_session(self):
        self.client = None

    async def _execute_work_item(self, work_item: WorkItem) -> WorkResult:
        try:
            # Call Claude SDK
            response = await self.client.messages.create(
                model=self.config.model,
                max_tokens=4096,
                messages=[{
                    "role": "user",
                    "content": work_item.description
                }]
            )

            return WorkResult(
                success=True,
                data=response.content[0].text,
                memory_ids=[]
            )
        except Exception as e:
            return WorkResult(
                success=False,
                error=str(e)
            )
```

### 2. Create Other Agents
Follow same pattern for Orchestrator, Optimizer, Reviewer.

### 3. Test End-to-End
```bash
# With Python agents implemented:
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo test --test orchestration_bridge_integration --features python test_work_delegation -- --ignored
```

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
