# Quick Start Guide

Get Mnemosyne running and store your first memory in **under 5 minutes**.

## What You'll Do

1. ✅ Install Mnemosyne (2 minutes)
2. ✅ Configure your API key (1 minute)
3. ✅ Store and retrieve your first memory (2 minutes)

**Time to Complete**: ~5 minutes

---

## Prerequisites Check

Before starting, verify you have:

```bash
# Rust 1.75+ installed
rustc --version
# Should show: rustc 1.75.0 or higher

# If not installed:
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

You'll also need an **Anthropic API key** from [console.anthropic.com](https://console.anthropic.com/).

---

## Step 1: Install Mnemosyne (2 minutes)

### One-Command Installation

```bash
# Clone the repository
git clone https://github.com/rand/mnemosyne.git
cd mnemosyne

# Run automated install script
./scripts/install/install.sh
```

The installer will:
- ✅ Build the Rust binary
- ✅ Install to `~/.local/bin/mnemosyne`
- ✅ Create database at `~/.local/share/mnemosyne/`
- ✅ Set up MCP integration with Claude Code
- ✅ Prompt for API key (you can skip and do it in Step 2)

**Output you should see:**
```
🎉 Mnemosyne installation complete!

✓ Binary installed: /Users/you/.local/bin/mnemosyne
✓ Database initialized: /Users/you/.local/share/mnemosyne/mnemosyne.db
✓ MCP server configured: /Users/you/.claude/mcp_config.json
```

### Verify Installation

```bash
# Check mnemosyne is in your PATH
which mnemosyne
# Should show: /Users/you/.local/bin/mnemosyne

# Test the command
mnemosyne --help
# Should show usage information
```

---

## Step 2: Configure API Key (1 minute)

Mnemosyne uses Claude Haiku to automatically enrich memories with summaries, tags, and semantic links.

### Option A: Interactive Setup (Recommended)

```bash
mnemosyne secrets init
```

This will prompt you to enter your Anthropic API key and encrypt it securely using [age](https://age-encryption.org/).

### Option B: Environment Variable

```bash
# Add to ~/.bashrc, ~/.zshrc, or ~/.profile
export ANTHROPIC_API_KEY=sk-ant-api03-YOUR_KEY_HERE

# Reload shell config
source ~/.zshrc  # or source ~/.bashrc
```

### Verify Configuration

```bash
mnemosyne config show-key
```

**Expected output:**
```
✓ API key is accessible via secure system
```

---

## Step 3: Your First Memory (2 minutes)

### Store a Memory

```bash
mnemosyne remember \
  --content "Decided to use LibSQL for storage because it supports native vector search and FTS5 full-text search" \
  --namespace "global" \
  --importance 8 \
  --format json
```

**What happens:**
1. Content is stored in the database
2. Claude Haiku generates a summary and keywords
3. Memory is classified (decision, pattern, bug, or context)
4. Semantic links to related memories are created

**Expected output:**
```json
{
  "id": "mem_abc123...",
  "content": "Decided to use LibSQL for storage...",
  "summary": "Storage decision: LibSQL for vector search and FTS5",
  "importance": 8,
  "tags": ["storage", "libsql", "architecture"],
  "memory_type": "decision",
  "created_at": "2025-10-27T12:34:56Z"
}
```

### Retrieve the Memory

```bash
mnemosyne recall \
  --query "storage decision" \
  --limit 5 \
  --format json
```

**Expected output:**
```json
{
  "results": [
    {
      "id": "mem_abc123...",
      "summary": "Storage decision: LibSQL for vector search and FTS5",
      "content": "Decided to use LibSQL for storage...",
      "score": 0.95,
      "importance": 8,
      "tags": ["storage", "libsql", "architecture"]
    }
  ]
}
```

### List All Memories

```bash
mnemosyne list \
  --namespace "global" \
  --limit 10 \
  --format json
```

---

## Step 4: Use in Claude Code (Bonus!)

Mnemosyne integrates seamlessly with Claude Code through the MCP protocol.

### Verify Integration

Open Claude Code and check that Mnemosyne tools are available:
- Look for "mnemosyne" in the MCP servers list
- You should see 8 tools: remember, recall, list, graph, context, consolidate, update, delete

### Use Slash Commands

In Claude Code, try these commands:

```
/memory-store Remember: Always run tests before committing code

/memory-search testing best practices

/memory-context

/memory-list
```

### Let Agents Use Memory Automatically

Claude Code's multi-agent system will automatically:
- 📥 Load project context at session start (via hooks)
- 🔍 Search memories when you ask questions
- 💾 Store important decisions during conversations
- 🔗 Link new memories to existing knowledge

---

## Next Steps

### Learn Common Workflows

See [Common Workflows](docs/guides/workflows.md) for practical patterns:
- Daily development session
- Debugging recurring issues
- Team knowledge sharing
- CI/CD integration

### Explore the MCP API

See [MCP Server Documentation](MCP_SERVER.md) for details on all 8 OODA-aligned tools:

**OBSERVE:**
- `mnemosyne.recall` - Search memories
- `mnemosyne.list` - List recent memories

**ORIENT:**
- `mnemosyne.graph` - Get memory graph
- `mnemosyne.context` - Get full context

**DECIDE:**
- `mnemosyne.remember` - Store new memory
- `mnemosyne.consolidate` - Merge similar memories

**ACT:**
- `mnemosyne.update` - Update existing memory
- `mnemosyne.delete` - Archive memory

### Configure Hooks

Automatic memory capture happens via hooks:
- **session-start**: Load project context automatically
- **pre-compact**: Preserve decisions before conversation compaction
- **post-commit**: Link git commits to architectural decisions

See [Hooks Testing Guide](HOOKS_TESTING.md) for details.

### Advanced Features

- **PyO3 Orchestration**: 10-20x faster multi-agent coordination
- **Project Namespaces**: Automatic memory isolation per project
- **Importance Decay**: Memories age naturally over time
- **Memory Consolidation**: Automatic deduplication

See [Architecture Documentation](ARCHITECTURE.md) for deep dive.

---

## Troubleshooting

If something didn't work:

### "mnemosyne: command not found"

```bash
# Add to PATH
export PATH="$HOME/.local/bin:$PATH"

# Or reinstall with custom location
./scripts/install/install.sh --bin-dir /usr/local/bin
```

### "No API key found"

```bash
# Set via environment variable
export ANTHROPIC_API_KEY=sk-ant-api03-YOUR_KEY

# Or use secrets management
mnemosyne secrets set ANTHROPIC_API_KEY
```

### "Database initialization failed"

```bash
# Create parent directory manually
mkdir -p ~/.local/share/mnemosyne

# Initialize with explicit path
mnemosyne --db-path ~/.local/share/mnemosyne/mnemosyne.db init
```

### More Help

See the comprehensive [Troubleshooting Guide](TROUBLESHOOTING.md) for solutions to common issues.

---

## Summary: What You Accomplished

✅ Installed Mnemosyne and configured Claude Code integration
✅ Set up secure API key management
✅ Stored your first memory with automatic LLM enrichment
✅ Retrieved memory using hybrid search
✅ Ready to use Mnemosyne in your daily development workflow

**Total time**: ~5 minutes ⚡️

---

## Quick Reference Card

```bash
# Store memory
mnemosyne remember --content "Your decision" --importance 8

# Search memories
mnemosyne recall --query "search terms"

# List recent memories
mnemosyne list --limit 10

# Get project context
mnemosyne recall --query "project context" --namespace "project:myproject"

# Export to markdown
mnemosyne export --output memories.md

# Check status
mnemosyne config show-key
```

### In Claude Code

```
/memory-store <content>        Store a new memory
/memory-search <query>         Search memories
/memory-context                Load full project context
/memory-list                   Browse all memories
/memory-export                 Export to markdown
/memory-consolidate            Review duplicates
```

---

## Get Help

- 📖 **[Full Documentation](DOCUMENTATION.md)** - Complete guide index
- 🔧 **[Troubleshooting](TROUBLESHOOTING.md)** - Common issues and solutions
- 💬 **[GitHub Discussions](https://github.com/rand/mnemosyne/discussions)** - Ask questions
- 🐛 **[Issue Tracker](https://github.com/rand/mnemosyne/issues)** - Report bugs

---

**Welcome to Mnemosyne!** 🧠✨

You now have a persistent memory system that makes your AI assistant smarter over time.

**Last Updated**: 2025-10-27
**Version**: 1.0.0
