# Phase 2 Analysis: Architecture Deep Dive - Session 2025-11-08

## Executive Summary

**CRITICAL FINDING**: The entire orchestration pipeline is ALREADY IMPLEMENTED and functional. The issue is NOT missing implementation - it's that the executor tries to use Claude Agent SDK in a context where it doesn't exist.

## Complete Message Flow (VERIFIED)

```
┌─────────────────────────────────────────────────────────────┐
│ 1. Interactive Mode (src/cli/interactive.rs)               │
│    User types: "Create hello.txt"                          │
│    → Creates WorkItem                                       │
│    → orchestrator.send_message(SubmitWork(item))           │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│ 2. Orchestrator Actor (actors/orchestrator.rs:200-245)     │
│    → handle_submit_work():                                  │
│      • Adds item to work queue                              │
│      • Persists AgentEvent::WorkItemAssigned                │
│      • Calls dispatch_work()                                │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│ 3. Dispatch Work (actors/orchestrator.rs:249-315)          │
│    → Gets ready items from queue                            │
│    → For each item:                                         │
│      • Calls optimizer.DiscoverSkills (unless for optimizer)│
│      • Calls optimizer.LoadContextMemories                  │
│      • Sends executor.ExecuteWork(item)                     │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│ 4. Executor Actor (actors/executor.rs:140-220)             │
│    → execute_work(state, item):                             │
│      • Marks item as active                                 │
│      • Persists AgentEvent::WorkItemStarted                 │
│      • IF python_bridge exists:                             │
│          bridge.send_work(item)                             │
│        ELSE:                                                 │
│          sleep(100ms) // Simulation                         │
│      • Sends orchestrator.WorkCompleted                     │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│ 5. PyO3 Bridge (claude_agent_bridge.rs:245-320)            │
│    → send_work(item):                                       │
│      • Sets agent state = Active                            │
│      • Broadcasts agent_started event                       │
│      • spawn_blocking(|| Python::with_gil(|py| {            │
│          agent.call_method1("execute_work", (py_work,))     │
│        }))                                                   │
│      • Extracts WorkResult from Python                      │
│      • Returns WorkResult to Rust                           │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│ 6. Python AgentExecutionMixin (base_agent.py:102-134)      │
│    → execute_work(work_dict):                               │
│      • Converts dict → WorkItem                             │
│      • Calls self._execute_work_item(work_item)             │
│      • Converts WorkResult → dict                           │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│ 7. Python Executor (executor.py:473-560)                   │
│    → _execute_work_item(work_item):                         │
│      • Validates work item                                  │
│      • Converts WorkItem → work_plan dict                   │
│      • Calls execute_work_plan(work_plan)                   │
│      • Returns WorkResult                                   │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│ 8. Execute Work Plan (executor.py:176-259)                 │
│    → execute_work_plan(work_plan):                          │
│      • Validates work plan                                  │
│      • ⚠️  PROBLEM: await self.claude_client.query(prompt)  │
│      • ⚠️  Tries to use Claude Agent SDK                    │
│      • ⚠️  But there's NO Claude Code session!              │
│      • Returns artifacts (empty)                            │
└─────────────────────────────────────────────────────────────┘
```

## The Problem

**Line 217 in executor.py:**
```python
await self.claude_client.query(execution_prompt)
```

This tries to send messages to a Claude Code session that **DOESN'T EXIST** in this context:
- The default `mnemosyne` command launches a standalone process
- It does NOT launch Claude Code
- `claude_client` is the Claude Agent SDK which requires an active Claude Code session
- Therefore, this call will fail or return nothing

## The Solution: Zero Framework Cognition

Following the user's directive about **Zero Framework Cognition** (https://steve-yegge.medium.com/zero-framework-cognition-a-way-to-build-resilient-ai-applications-56b090ed3e69):

> "agents should have deterministic logic, but CAN and SHOULD make LLM calls"

### Current (Broken)
```python
# Tries to delegate to Claude Code session (doesn't exist)
await self.claude_client.query(execution_prompt)
```

### ZFC Approach (Correct)
```python
# Direct LLM API calls for decision points
# Deterministic state machine for flow control

if work_item.phase == Phase.Spec:
    # Use LLM to analyze and create spec
    spec = await make_llm_call(
        prompt=f"Analyze this request: {work_item.description}",
        tools=["create_file", "read_file", "edit_file"]
    )
    # Deterministic: Always transition to next phase
    return WorkResult(success=True, phase=Phase.Planning)

elif work_item.phase == Phase.Planning:
    # Use LLM to create plan
    plan = await make_llm_call(
        prompt=f"Create plan for: {spec}",
        tools=["create_file"]
    )
    # Deterministic: Always transition to next phase
    return WorkResult(success=True, phase=Phase.Implementation)

# etc.
```

## What Needs to Change

### File: `src/orchestration/agents/executor.py`

**Function**: `execute_work_plan` (lines 176-259)

**Current Approach**: Tries to use Claude Agent SDK
**New Approach**: Direct Anthropic API calls + file operations

**Changes Needed**:
1. Remove dependency on `self.claude_client`
2. Add direct Anthropic API client (via environment variable `ANTHROPIC_API_KEY`)
3. Implement deterministic phase transitions:
   - Spec → Planning → Implementation → Validation
4. For each phase:
   - Make LLM call with appropriate prompt and tools
   - Execute tool calls (file operations)
   - Store results in memory
   - Return WorkResult with artifacts
5. Add circuit breaker for LLM failures:
   - Max retries: 3
   - Exponential backoff
   - Graceful degradation to simple execution

## Implementation Plan

### Phase 2.1: Add Direct Anthropic API Integration

**File**: `src/orchestration/agents/executor.py`

**New Dependencies**:
```python
from anthropic import Anthropic, AsyncAnthropic
import os
```

**New Method**:
```python
async def _make_llm_call(
    self,
    prompt: str,
    system: str,
    tools: List[Dict],
    max_tokens: int = 4096
) -> Dict[str, Any]:
    """
    Make direct LLM API call with tool use.

    ZFC principle: Deterministic state machine, LLM for decisions.
    """
    client = AsyncAnthropic(api_key=os.getenv("ANTHROPIC_API_KEY"))

    response = await client.messages.create(
        model="claude-sonnet-4-5-20250929",
        max_tokens=max_tokens,
        system=system,
        messages=[{"role": "user", "content": prompt}],
        tools=tools
    )

    return response
```

### Phase 2.2: Reimplement execute_work_plan

**Replace**: Lines 176-259 in executor.py
**New Implementation**:

```python
async def execute_work_plan(self, work_plan: Dict[str, Any]) -> Dict[str, Any]:
    """
    Execute work using direct LLM calls + file operations.

    ZFC: Deterministic state machine, LLM for content generation.
    """
    phase = work_plan.get("phase", "spec")
    description = work_plan.get("prompt", "")

    # Define tools for file operations
    tools = [
        {
            "name": "create_file",
            "description": "Create a new file with content",
            "input_schema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "content": {"type": "string"}
                },
                "required": ["path", "content"]
            }
        },
        # ... more tools
    ]

    # Make LLM call for this phase
    response = await self._make_llm_call(
        prompt=f"Execute {phase} phase: {description}",
        system="You are an executor agent. Use tools to complete work.",
        tools=tools
    )

    # Execute tool calls
    artifacts = []
    for content_block in response.content:
        if content_block.type == "tool_use":
            result = await self._execute_tool(content_block)
            artifacts.append(result)

    return {
        "status": "success",
        "artifacts": artifacts,
        "phase": phase
    }
```

### Phase 2.3: Add Tool Execution

**New Method**:
```python
async def _execute_tool(self, tool_use: ToolUse) -> Dict[str, Any]:
    """Execute a tool call from LLM."""
    if tool_use.name == "create_file":
        path = tool_use.input["path"]
        content = tool_use.input["content"]

        # Actually create the file
        with open(path, "w") as f:
            f.write(content)

        return {
            "tool": "create_file",
            "path": path,
            "success": True
        }

    # ... implement other tools
```

## Testing Plan

### Manual Test
```bash
# 1. Start mnemosyne
mnemosyne

# 2. Submit work
mnemosyne> Create a file hello.txt with content "Hello World"

# Expected:
# - Work submitted message
# - Executor receives work
# - LLM call made
# - File created
# - Work completed message
# - Check: ls hello.txt (should exist)
```

### Integration Test
```rust
#[tokio::test]
async fn test_work_execution_via_pyo3() {
    // Create engine
    let engine = create_test_engine().await;

    // Submit work
    let item = WorkItem {
        description: "Create test.txt".to_string(),
        // ...
    };
    engine.orchestrator().cast(SubmitWork(item));

    // Wait for completion
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Verify file exists
    assert!(Path::new("test.txt").exists());
}
```

## Benefits of This Approach

1. **Real work execution**: Actually creates files, runs code, etc.
2. **No Claude Code dependency**: Works standalone
3. **ZFC compliant**: Deterministic state machine + LLM calls
4. **Observable**: All events broadcast to dashboard
5. **Testable**: Can test without external dependencies
6. **Resilient**: Circuit breaker prevents cascading failures

## Summary

- ✅ **Orchestration pipeline**: Fully implemented and working
- ✅ **Message passing**: All actors communicate correctly
- ✅ **PyO3 bridge**: Rust ↔ Python integration works
- ⚠️  **Executor implementation**: Needs replacement
  - Current: Tries to use Claude Agent SDK (doesn't work)
  - Solution: Direct Anthropic API calls + file operations
  - Approach: Zero Framework Cognition principles

**Next Step**: Implement Phase 2.1-2.3 to replace executor's LLM integration.
