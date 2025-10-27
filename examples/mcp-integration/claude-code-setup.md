# Claude Code Setup Guide

Complete guide to integrating Mnemosyne with Claude Code via MCP (Model Context Protocol).

## Table of Contents

- [Prerequisites](#prerequisites)
- [Automatic Setup](#automatic-setup)
- [Manual Setup](#manual-setup)
- [Verification](#verification)
- [Configuration Options](#configuration-options)
- [Troubleshooting](#troubleshooting)
- [Usage Examples](#usage-examples)

---

## Prerequisites

Before setting up Mnemosyne with Claude Code:

1. **Claude Code installed** (macOS/Linux)
   ```bash
   # Check Claude Code is available
   which claude
   ```

2. **Mnemosyne installed**
   ```bash
   # Check Mnemosyne is available
   which mnemosyne
   mnemosyne --version
   ```

3. **API key configured**
   ```bash
   # Verify API key is set
   mnemosyne secrets get ANTHROPIC_API_KEY
   ```

---

## Automatic Setup

The installation script automatically configures MCP integration:

```bash
# Default: Adds MCP config to project's .claude/mcp_config.json
./scripts/install/install.sh

# Global: Adds MCP config to ~/.claude/mcp_config.json
./scripts/install/install.sh --global-mcp

# Skip MCP: Install without MCP configuration
./scripts/install/install.sh --no-mcp
```

**What the installer does:**
1. Creates `.claude/mcp_config.json` (or updates `~/.claude/mcp_config.json`)
2. Adds Mnemosyne MCP server configuration
3. Configures server with appropriate environment variables
4. Sets up slash command integration

---

## Manual Setup

If you need to manually configure MCP integration:

### Project-Specific Setup

Create `.claude/mcp_config.json` in your project root:

```json
{
  "mcpServers": {
    "mnemosyne": {
      "command": "mnemosyne",
      "args": ["serve"],
      "env": {
        "MNEMOSYNE_NAMESPACE": "project:your-project-name"
      },
      "description": "Project-aware memory system"
    }
  }
}
```

### Global Setup

Create or update `~/.claude/mcp_config.json`:

```json
{
  "mcpServers": {
    "mnemosyne": {
      "command": "mnemosyne",
      "args": ["serve"],
      "env": {
        "MNEMOSYNE_NAMESPACE": "global"
      },
      "description": "Global memory system for all projects"
    }
  }
}
```

### Restart Claude Code

After creating/updating MCP config:

```bash
# If Claude Code is running, restart it
# macOS: Cmd+Q to quit, then reopen
# Linux: pkill claude && claude
```

---

## Verification

### 1. Check MCP Server Status

In Claude Code, the Mnemosyne MCP server should appear in the available tools:

```
Tools:
  - mnemosyne_store_memory
  - mnemosyne_recall_memory
  - mnemosyne_update_memory
  - mnemosyne_delete_memory
  - mnemosyne_graph_traverse
  - mnemosyne_export_memories
```

### 2. Test Memory Storage

Ask Claude Code to store a test memory:

```
Store a memory: "Testing Mnemosyne MCP integration"
```

Claude Code should use the `mnemosyne_store_memory` tool.

### 3. Test Memory Recall

Ask Claude Code to recall the memory:

```
What do you remember about Mnemosyne integration?
```

Claude Code should use the `mnemosyne_recall_memory` tool.

### 4. Verify Database

Check that the memory was stored:

```bash
mnemosyne recall --query "MCP integration" --format json
```

You should see the test memory in the results.

---

## Configuration Options

### Environment Variables

Configure the MCP server behavior via environment variables:

```json
{
  "mcpServers": {
    "mnemosyne": {
      "command": "mnemosyne",
      "args": ["serve"],
      "env": {
        "MNEMOSYNE_NAMESPACE": "project:myapp",
        "MNEMOSYNE_DB_PATH": "/custom/path/mnemosyne.db",
        "MNEMOSYNE_LOG_LEVEL": "info",
        "ANTHROPIC_API_KEY": "${ANTHROPIC_API_KEY}"
      }
    }
  }
}
```

**Available environment variables:**

| Variable | Description | Default |
|----------|-------------|---------|
| `MNEMOSYNE_NAMESPACE` | Default namespace for memories | `"global"` |
| `MNEMOSYNE_DB_PATH` | Database path | `~/.local/share/mnemosyne/mnemosyne.db` |
| `MNEMOSYNE_LOG_LEVEL` | Logging level (debug/info/warn/error) | `"info"` |
| `ANTHROPIC_API_KEY` | API key for LLM enrichment | (required) |

### Multiple Namespaces

You can configure multiple Mnemosyne servers for different projects:

```json
{
  "mcpServers": {
    "mnemosyne-global": {
      "command": "mnemosyne",
      "args": ["serve"],
      "env": {
        "MNEMOSYNE_NAMESPACE": "global"
      }
    },
    "mnemosyne-project": {
      "command": "mnemosyne",
      "args": ["serve"],
      "env": {
        "MNEMOSYNE_NAMESPACE": "project:myapp"
      }
    }
  }
}
```

---

## Troubleshooting

### MCP Server Not Appearing

**Symptom**: Mnemosyne tools don't appear in Claude Code.

**Solutions**:
1. Check MCP config file exists:
   ```bash
   cat .claude/mcp_config.json
   # or
   cat ~/.claude/mcp_config.json
   ```

2. Verify JSON syntax:
   ```bash
   jq . .claude/mcp_config.json
   ```

3. Restart Claude Code completely (quit and reopen)

4. Check Claude Code logs for MCP errors:
   ```bash
   tail -f ~/.claude/logs/mcp.log
   ```

### Memory Storage Fails

**Symptom**: `mnemosyne_store_memory` tool returns an error.

**Solutions**:
1. Check API key is configured:
   ```bash
   mnemosyne secrets get ANTHROPIC_API_KEY
   ```

2. Test CLI directly:
   ```bash
   mnemosyne remember --content "test" --importance 5
   ```

3. Check database is writable:
   ```bash
   ls -la ~/.local/share/mnemosyne/
   ```

### Memory Recall Empty

**Symptom**: `mnemosyne_recall_memory` returns no results.

**Solutions**:
1. Verify memories exist:
   ```bash
   mnemosyne recall --query "test" --format json
   ```

2. Check namespace configuration:
   ```bash
   # View MCP config namespace
   jq '.mcpServers.mnemosyne.env.MNEMOSYNE_NAMESPACE' .claude/mcp_config.json
   ```

3. Try recalling from all namespaces:
   ```bash
   mnemosyne recall --query "test" --all-namespaces
   ```

### Server Crashes on Startup

**Symptom**: MCP server starts then immediately exits.

**Solutions**:
1. Check Mnemosyne binary is accessible:
   ```bash
   which mnemosyne
   ```

2. Test server manually:
   ```bash
   mnemosyne serve
   # Should print: "Mnemosyne MCP server running on stdio"
   ```

3. Check for port conflicts (if using TCP):
   ```bash
   lsof -i :5173  # or whatever port you configured
   ```

4. Review logs:
   ```bash
   RUST_LOG=debug mnemosyne serve
   ```

---

## Usage Examples

### Example 1: Store a Decision

In Claude Code:

```
Store this decision: We decided to use LibSQL for storage because it has
native vector search support and is SQLite-compatible.
```

Claude Code will use `mnemosyne_store_memory` to store the decision.

### Example 2: Recall Context

In Claude Code:

```
What do you remember about our storage decisions?
```

Claude Code will use `mnemosyne_recall_memory` to find relevant memories.

### Example 3: Update a Memory

In Claude Code:

```
Update the storage decision memory to note that we're using connection pooling.
```

Claude Code will use `mnemosyne_update_memory` to modify the memory.

### Example 4: Graph Traversal

In Claude Code:

```
Show me all memories related to the authentication system.
```

Claude Code will use `mnemosyne_graph_traverse` to find connected memories.

### Example 5: Export Documentation

In Claude Code:

```
Export all architecture decisions to a markdown file.
```

Claude Code will use `mnemosyne_export_memories` to create a file.

---

## Integration Patterns

### Pattern 1: Session Start Hook

Automatically load context when starting a session:

Create `.claude/hooks/session-start.sh`:

```bash
#!/usr/bin/env bash
# Load project context at session start
mnemosyne recall \
  --query "architecture decision important" \
  --namespace "project:$(basename $PWD)" \
  --min-importance 7 \
  --limit 10
```

### Pattern 2: Post-Commit Hook

Automatically link commits to memories:

Create `.claude/hooks/post-commit.sh`:

```bash
#!/usr/bin/env bash
# Link latest commit to related memories
COMMIT_MSG=$(git log -1 --pretty=%B)
COMMIT_HASH=$(git log -1 --pretty=%h)

mnemosyne remember \
  --content "Commit $COMMIT_HASH: $COMMIT_MSG" \
  --importance 5 \
  --namespace "project:$(basename $PWD)" \
  --tags "commit,$(basename $PWD)"
```

### Pattern 3: Pre-Compact Hook

Save context before Claude Code compacts:

Create `.claude/hooks/pre-compact.sh`:

```bash
#!/usr/bin/env bash
# Export session context before compaction
mnemosyne export \
  --namespace "session:$(date +%Y%m%d)" \
  --output ".claude/context-backups/$(date +%Y%m%d_%H%M%S).md"
```

---

## Next Steps

- **[Slash Commands Reference](slash-commands.md)** - Learn custom slash commands
- **[MCP API Reference](../../MCP_SERVER.md)** - Detailed tool documentation
- **[Common Workflows](../workflows/README.md)** - Usage patterns
- **[Troubleshooting Guide](../../TROUBLESHOOTING.md)** - Fix common issues

---

**Version**: 1.0.0
**Last Updated**: 2025-10-27
