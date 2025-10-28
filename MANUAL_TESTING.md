# Manual Testing Guide - Orchestrated Launcher

This guide outlines manual testing steps for the new orchestrated launcher functionality.

## Prerequisites

1. Claude Code CLI installed and accessible in PATH
2. Mnemosyne built and installed: `cargo build --release`
3. API key configured: `mnemosyne secrets init`
4. Test in a git repository for namespace detection

## Test Cases

### 1. Default Behavior - Orchestrated Session Launch

**Test**: Running `mnemosyne` without arguments should launch Claude Code with full orchestration.

```bash
cd /path/to/test/project
mnemosyne
```

**Expected**:
- Claude Code window opens
- Multi-agent system is active (check status bar or agent list)
- Mnemosyne MCP server is connected
- Project namespace detected from git repository
- No errors in logs

**Verify**:
- Try `/memory-store` command in Claude Code
- Check that memories are stored with correct namespace
- Verify agents can access Mnemosyne tools

### 2. MCP Server Only Mode

**Test**: Running with `--serve` flag should start MCP server without launching Claude Code.

```bash
mnemosyne --serve
```

**Expected**:
- MCP server starts in stdio mode
- No Claude Code window opens
- Server responds to MCP protocol messages
- Can be used by external MCP clients

**Verify**:
- Send MCP initialize request via stdio
- Verify server responds with capabilities
- Test basic tool invocation (e.g., `mnemosyne.list`)

### 3. Legacy Serve Command

**Test**: `mnemosyne serve` subcommand should still work for backward compatibility.

```bash
mnemosyne serve
```

**Expected**:
- Same behavior as `--serve` flag
- MCP server starts without Claude Code

### 4. Namespace Detection

**Test**: Project namespace should be auto-detected from git.

```bash
cd /path/to/git/repo
mnemosyne
# In Claude Code session, store a memory and check namespace
```

**Expected**:
- Namespace is `project:<repo-name>`
- Falls back to "global" if not in git repo

### 5. Database Path Configuration

**Test**: Custom database path should be respected.

```bash
mnemosyne --db-path /tmp/test.db
```

**Expected**:
- Uses specified database path
- Creates parent directories if needed
- MCP config includes correct path

### 6. Claude Binary Detection

**Test**: Launcher should find Claude Code binary in common locations.

```bash
# Temporarily rename claude binary to test fallback behavior
which claude  # Note the location
# Test detection logic
```

**Expected**:
- Tries: `claude`, `/usr/local/bin/claude`, Homebrew paths
- Falls back to `which claude`
- Clear error message if not found

### 7. Agent Configuration

**Test**: All 4 agents should be properly configured with correct permissions.

```bash
mnemosyne
# In Claude Code, check agent list
```

**Expected**:
- Orchestrator: Read, Glob, Task tools only
- Optimizer: Read, Glob, SlashCommand tools
- Reviewer: Read, Grep, Bash (test) tools
- Executor: All tools (*)
- Each agent has detailed system prompt

### 8. Error Handling

**Test**: Graceful error handling for common issues.

```bash
# Remove Claude Code binary temporarily
mv /usr/local/bin/claude /usr/local/bin/claude.bak
mnemosyne
# Restore: mv /usr/local/bin/claude.bak /usr/local/bin/claude
```

**Expected**:
- Clear error message about missing Claude binary
- Instructions on how to install/configure
- No panic or crash

### 9. Log Output

**Test**: Appropriate logging at different levels.

```bash
mnemosyne --log-level debug
```

**Expected**:
- Informative logs about startup process
- Database path logged
- Agent configuration logged
- No sensitive information (e.g., API keys) in logs

### 10. Context Loading (Future)

**Test**: High-importance memories should be loaded at session start (when implemented).

```bash
# Store some high-importance memories first
mnemosyne remember -c "Important decision" -i 9
# Launch session
mnemosyne
```

**Expected** (when implemented):
- High-importance memories (≥7) loaded in startup prompt
- Context visible to all agents
- No performance impact on startup

## Integration Testing

### Test with Real Claude Code Session

1. Launch orchestrated session: `mnemosyne`
2. Create a memory: `/memory-store "Test architecture decision: use PostgreSQL for user data" importance:9`
3. Search memories: `/memory-search "database decisions"`
4. Verify namespace: Check that memory has `project:<repo-name>` namespace
5. Test agent coordination: Ask Orchestrator to coordinate a multi-step task
6. Verify sub-agent spawning: Check that Executor can spawn sub-agents

### Test Backward Compatibility

1. Existing MCP clients should still work with `mnemosyne serve`
2. All existing subcommands should function: `init`, `remember`, `recall`, `config`, `secrets`
3. Environment variables should still be respected: `MNEMOSYNE_DB_PATH`, `ANTHROPIC_API_KEY`

## Performance Testing

1. **Startup Time**: Orchestrated session should launch in <5 seconds
2. **Memory Operations**: Tool calls should complete in <1 second
3. **Context Loading**: No noticeable delay even with 100+ memories

## Cleanup

After testing, clean up test artifacts:

```bash
rm -f /tmp/test.db*
```

## Known Limitations

1. Context loading is currently a stub (returns empty string)
2. Daemon mode is implemented but not integrated with launcher yet
3. Windows support is limited (daemon functionality Unix-only)

## Reporting Issues

When reporting issues, include:
- OS and version
- Rust version (`rustc --version`)
- Claude Code version
- Full error logs with `--log-level debug`
- Steps to reproduce
