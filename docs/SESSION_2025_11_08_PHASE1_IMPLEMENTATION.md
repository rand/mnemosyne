# Phase 1 Implementation Notes - Session 2025-11-08

## Current State Analysis

### Default `mnemosyne` Command Flow (main.rs:416-544)
1. Creates API server on port 3000
2. Uses `AgentSpawner` to spawn 4 separate Python processes
3. Launches Claude Code via `launcher::launch_orchestrated_session()`
4. Waits for Claude Code to exit
5. Shuts down agents and API server

### Problem
- Uses AgentSpawner (separate processes) instead of PyO3 bridge (in-process)
- Launches Claude Code instead of providing standalone orchestration
- No way to submit work directly to agents

## Phase 1.1: Restructure Default Command

### Files to Modify

#### 1. `src/main.rs` (None case, lines 416-544)

**Current**:
```rust
None => {
    // Spawn AgentSpawner processes
    // Launch Claude Code
}
```

**New**:
```rust
None => {
    // Start API server
    // Start OrchestrationEngine with PyO3
    // Launch interactive mode
}
```

### Implementation Steps

1. **Create `src/cli/interactive.rs`** - New file for interactive mode
   - Read user input from stdin
   - Parse commands (help, quit, status, work:, recall:)
   - Submit work to OrchestrationEngine
   - Display results

2. **Modify `src/main.rs` None case** (lines 416-544)
   - Keep API server creation (lines 431-448)
   - REMOVE AgentSpawner creation (lines 469-512)
   - REMOVE launch_orchestrated_session call (lines 516-523)
   - ADD OrchestrationEngine creation with PyO3
   - ADD interactive mode launch
   - ADD proper cleanup

3. **Update `src/cli/mod.rs`** - Add interactive module
   ```rust
   pub mod interactive;
   ```

### Key Design Decisions

1. **OrchestrationEngine Config**:
   - `use_python_bridge: true` - Force PyO3 instead of subprocess
   - `max_concurrent_executors: 4` - Default parallelism

2. **Interactive Mode UX**:
   - Prompt: `mnemosyne> `
   - Commands:
     - `help` - Show commands
     - `quit`/`exit` - Shutdown
     - `status` - Show agent status
     - `recall <query>` - Search memories
     - `work: <description>` - Submit work
     - Any other text - Treated as work description

3. **Work Submission Flow**:
   ```
   User types: "Create hello.txt with 'Hello World'"
   ↓
   Create WorkItem { description, phase: Spec, ... }
   ↓
   engine.submit_work(item)
   ↓
   Orchestrator analyzes → assigns to Executor
   ↓
   Executor calls Python via PyO3
   ↓
   Results displayed
   ```

## Phase 1.2: Create Interactive Mode

### Module Structure: `src/cli/interactive.rs`

```rust
use mnemosyne_core::{
    orchestration::OrchestrationEngine,
    storage::MemoryStore,
    error::Result,
};
use std::io::{self, Write};

pub async fn run(
    engine: OrchestrationEngine,
    memory: MemoryStore
) -> Result<()> {
    println!("\n📋 Mnemosyne Multi-Agent System");
    println!("Dashboard: http://127.0.0.1:3000");
    println!("Type 'help' for commands, 'quit' to exit\n");

    loop {
        print!("mnemosyne> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim() {
            "help" => show_help(),
            "quit" | "exit" => break,
            "status" => show_status(&engine).await?,
            cmd if cmd.starts_with("work:") => {
                let desc = cmd.strip_prefix("work:").unwrap().trim();
                submit_work(&engine, desc).await?;
            },
            cmd if cmd.starts_with("recall:") => {
                let query = cmd.strip_prefix("recall:").unwrap().trim();
                recall_memories(&memory, query).await?;
            },
            "" => continue,
            desc => submit_work(&engine, desc).await?,
        }
    }

    Ok(())
}

async fn submit_work(engine: &OrchestrationEngine, description: &str) -> Result<()> {
    println!("Submitting work: {}", description);

    let item = WorkItem {
        id: WorkItemId::new(),
        description: description.to_string(),
        phase: Phase::Spec,
        dependencies: vec![],
        status: WorkItemStatus::Pending,
    };

    let result = engine.submit_work(item).await?;
    println!("✓ Work submitted: {}", result.id);

    Ok(())
}

// ... other helper functions
```

## Testing Plan

### Manual Test

```bash
# 1. Run mnemosyne
mnemosyne

# Expected output:
# 📋 Mnemosyne Multi-Agent System
# Dashboard: http://127.0.0.1:3000
# Type 'help' for commands, 'quit' to exit
#
# mnemosyne> _

# 2. Check dashboard in browser
open http://127.0.0.1:3000

# Expected: 4 agents shown with PyO3 bridge status

# 3. Submit work
mnemosyne> work: Create test.txt with content "Hello"

# Expected: Work submitted, visible in dashboard

# 4. Exit
mnemosyne> quit

# Expected: Clean shutdown, no warnings
```

### Integration Test

Create `tests/cli/interactive_test.rs`:
```rust
#[tokio::test]
async fn test_default_command_launches_interactive() {
    // Spawn mnemosyne in background
    let child = Command::new("mnemosyne")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    // Wait for startup
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Check dashboard API
    let resp = reqwest::get("http://127.0.0.1:3000/api/agents").await.unwrap();
    assert_eq!(resp.status(), 200);

    let agents: Vec<AgentInfo> = resp.json().await.unwrap();
    assert_eq!(agents.len(), 4);

    // All should have PyO3 bridge
    for agent in agents {
        assert_eq!(agent.agent_type, AgentType::PythonBridge);
    }

    // Cleanup
    child.kill().unwrap();
}
```

## Risks & Mitigations

### Risk: Breaking existing Claude Code integration
**Mitigation**: Keep `orchestrate` subcommand unchanged. Only modify default (None) case.

### Risk: PyO3 bridge not initializing properly
**Mitigation**: Add detailed logging, graceful degradation to show error and exit cleanly.

### Risk: Users expect Claude Code launch
**Mitigation**: Update README to explain new default behavior. Provide `--claude` flag to launch old behavior if needed.

## Next Steps (For Next Session)

1. Implement `src/cli/interactive.rs`
2. Modify `src/main.rs` None case
3. Test manually
4. Commit Phase 1.1 and 1.2
5. Move to Phase 2

## Context Budget

This session: 124k/200k tokens (62% used)
Remaining: 76k tokens

**Recommendation**: Commit this design document, start fresh session for implementation to avoid context issues.
