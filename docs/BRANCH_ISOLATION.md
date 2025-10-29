# Branch Isolation System

## Overview

The branch isolation system ensures that multiple AI agents (whether managed by Mnemosyne or launched directly via Claude Code) can safely work in parallel on different branches of the same project without accidentally interfering with each other.

## Core Principles

1. **Default Isolation**: Agents are isolated by default to prevent accidental conflicts
2. **Explicit Coordination**: Multiple agents can work together on the same branch when explicitly coordinated
3. **Auto-Approve Read-Only**: Read-only access is automatically approved without user prompts
4. **Orchestrator Bypass**: The orchestrator agent has special permissions to bypass isolation rules

## Architecture

### Components

```
┌─────────────────────────────────────────────────────────────┐
│                   Branch Coordinator                         │
│  (Orchestrates all components, handles join requests)       │
└───────────────┬─────────────────────────────────────────────┘
                │
    ┌───────────┴───────────┬──────────────┬──────────────────┐
    │                       │              │                  │
┌───▼────────┐   ┌─────────▼──────┐  ┌───▼──────────┐  ┌────▼────────┐
│   Branch   │   │  Conflict      │  │  Cross-      │  │  Notification│
│   Registry │   │  Notifier      │  │  Process     │  │  Task        │
│            │   │                │  │  Coordinator │  │              │
└────────────┘   └────────────────┘  └──────────────┘  └──────────────┘
```

### Branch Registry

Maintains assignments of agents to branches:
- Agent ID → Branch mapping
- Work intent (read-only, write specific files, full branch)
- Coordination mode (isolated or coordinated)
- Dynamic timeouts based on work phase

### Branch Guard

Validates branch access before allowing operations:
- Checks for conflicts with existing assignments
- Enforces isolation or coordination rules
- Applies orchestrator bypass when appropriate

### Conflict Notifier

Three-tier notification system:
- **On-save**: Notifies of new conflicts immediately when files are saved
- **Periodic**: Sends summaries of all conflicts every 20 minutes
- **Session-end**: Provides final conflict report before session ends

### Cross-Process Coordinator

Enables coordination between Mnemosyne-managed agents and directly-launched Claude Code agents:
- File-based state sharing (`.mnemosyne/branch_registry.json`)
- Message queue for coordination requests
- Process liveness detection via PID tracking and heartbeats

## Work Intent

Agents specify their work intent when joining a branch:

### ReadOnly
- View code and documentation
- No write operations
- Auto-approved (no user interaction required)
- Can coexist with any other agents

### Write(paths)
- Modify specific files
- Explicit list of file paths
- Conflict detection on overlapping files

### FullBranch
- Full access to entire branch
- Most restrictive intent
- Isolated mode blocks other agents by default

## Coordination Modes

### Isolated (Default)
- Single agent per branch
- Blocks other agents from joining
- Strongest isolation guarantee
- Use for: Independent features, critical refactoring

### Coordinated
- Multiple agents on same branch
- Conflict detection active
- Agents receive notifications
- Use for: Collaborative work, paired programming, testing

## Dynamic Timeouts

Assignment timeouts are calculated based on work phase complexity:

```rust
Phase::PromptToSpec      → 0.5x multiplier
Phase::SpecToFullSpec    → 1.0x multiplier
Phase::FullSpecToPlan    → 0.5x multiplier
Phase::PlanToArtifacts   → 2.0x multiplier
```

Base timeout: 1 hour
Example: If working on artifacts phase (2.0x), timeout = 2 hours

## Configuration

Configuration file: `.mnemosyne/config.toml`

```toml
[branch_isolation]
enabled = true
default_mode = "isolated"
auto_approve_readonly = true
orchestrator_bypass = true

[conflict_detection]
enabled = true
critical_paths = ["migrations/**", "schema/**", "**/.env"]
test_isolation = true

[notifications]
enabled = true
on_save = true
periodic_interval_minutes = 20
session_end_summary = true

[cross_process]
enabled = true
mnemosyne_dir = ".mnemosyne"
poll_interval_seconds = 2
heartbeat_timeout_seconds = 30
```

## CLI Commands

### Status
```bash
mnemosyne branch status              # Show current branch status
mnemosyne branch status --all        # Show all branches
```

### Join
```bash
mnemosyne branch join main read                              # Read-only
mnemosyne branch join feature/test write --files src/lib.rs  # Write specific files
mnemosyne branch join feature/test full --mode coordinated   # Full branch, coordinated
```

### Conflicts
```bash
mnemosyne branch conflicts           # Show conflicts for current agent
mnemosyne branch conflicts --all     # Show all conflicts
```

### Switch
```bash
mnemosyne branch switch feature/new full   # Switch to new branch
```

### Release
```bash
mnemosyne branch release             # Release current assignment
```

## Status Line Integration

Display branch status in your terminal prompt:

### Bash
```bash
# Add to ~/.bashrc
mnemosyne_prompt() {
    mnemosyne-status --format ansi 2>/dev/null || echo ""
}
PS1="$(mnemosyne_prompt) $PS1"
```

### Zsh
```zsh
# Add to ~/.zshrc
mnemosyne_prompt() {
    mnemosyne-status --format ansi 2>/dev/null || echo ""
}
PROMPT="$(mnemosyne_prompt) $PROMPT"
```

### Tmux
```
# Add to ~/.tmux.conf
set -g status-right '#(mnemosyne-status --format compact 2>/dev/null) | %H:%M'
```

## Best Practices

### For Single-Agent Work
1. Use isolated mode (default)
2. Request appropriate intent (read-only when possible)
3. Release assignment when done

### For Multi-Agent Collaboration
1. Use coordinated mode explicitly
2. Scope work intent to specific files when possible
3. Monitor conflict notifications
4. Communicate with other agents through commit messages

### For Testing
1. Test isolation is enabled by default
2. Multiple agents can work on different test files safely
3. Use coordinated mode for integration tests

## Troubleshooting

See [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for common issues and solutions.
