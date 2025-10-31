# Migration Guide: TUI Wrapper → Composable Tools

## Overview

Mnemosyne v2.1+ replaces the TUI wrapper mode (`mnemosyne tui`) with a composable architecture of standalone tools that work alongside Claude Code instead of wrapping it.

**Why the change?**
- The TUI wrapper created "TUI-in-TUI" conflicts (broken rendering, unresponsive input)
- Wrapping Claude Code's terminal interface is fundamentally incompatible
- Standalone tools provide better separation of concerns and reliability

---

## Quick Migration

### Old Workflow (Deprecated)

```bash
# ❌ No longer supported
mnemosyne tui
```

**Problems:**
- Terminal rendering conflicts
- Input forwarding broken
- Unstable and unreliable
- Single tool doing too much

### New Workflow (Recommended)

```bash
# ✅ Edit context (Terminal 1)
mnemosyne-ics context.md

# ✅ Chat with Claude (Terminal 2)
claude

# ✅ Monitor activity (Terminal 3 - coming soon)
mnemosyne dash
```

**Benefits:**
- Each tool owns its terminal (no conflicts)
- Composable via tmux/screen
- Stable and reliable
- Unix philosophy: do one thing well

---

## Tool Comparison

| Feature | Old (`mnemosyne tui`) | New (Composable Tools) |
|---------|----------------------|------------------------|
| Context Editing | Embedded panel (Ctrl+E) | `mnemosyne-ics` standalone (manual) |
| Claude Chat | PTY wrapper | Direct `claude` command |
| Memory Server | Manual wrapper | **Auto-launched by Claude** |
| Dashboard | Embedded (Ctrl+D) | `mnemosyne dash` (planned) |
| Memory Access | Via MCP | Via MCP (unchanged) |
| Terminal Control | Conflicts | Clean separation |
| Stability | ❌ Broken | ✅ Reliable |
| Composability | ❌ Monolithic | ✅ Unix pipes/tmux |

**Important:** The memory server (`mnemosyne serve`) is **never launched manually** - Claude Code handles it automatically via MCP configuration!

---

## Detailed Workflows

### Basic Workflow: Memory-Enhanced Chat

**What you need:** Just Claude Code (memory is 100% automatic!)

```bash
claude
```

**How it works (behind the scenes):**
1. Claude Code reads `.claude/mcp_config.json`
2. Auto-launches: `mnemosyne serve` (background process)
3. Connects via stdio (JSON-RPC protocol)
4. Memory integration "just works" - transparent to you
5. Auto-shuts down when `claude` exits

**You never run:** `mnemosyne serve` manually - Claude Code manages it!

**When to use:** Quick questions, simple tasks, most daily work

**Key point:** Memory is **completely automatic**. You don't need to do anything!

---

### Intermediate Workflow: Context-Heavy Work

**What you need:** ICS (manual) + Claude Code (auto-memory)

```bash
# Terminal 1: Edit context (YOU launch this manually)
mnemosyne-ics architecture.md

# Edit structured context:
## Authentication System
#auth/jwt.rs:TokenValidator
@validate_token
?AuthMiddleware - needs implementation

# Save: Ctrl+S

# Terminal 2: Work with Claude (memory auto-launches)
claude --context architecture.md
```

**How it works:**
- **Manual:** You launch `mnemosyne-ics` for rich context editing
- **Automatic:** Claude Code auto-launches `mnemosyne serve` for memory
- **File-based:** Context passed via file (familiar workflow)
- **Seamless:** Both memory and context available to Claude

**When to use:** Feature development, refactoring, architecture work

**Key distinction:**
- `mnemosyne-ics` = **Manual** context editor (you control)
- `mnemosyne serve` = **Automatic** memory server (Claude manages)

---

### Advanced Workflow: Multi-Tool Layout

**What you need:** ICS + Claude + Dashboard (tmux/screen)

```bash
# Setup tmux layout
tmux split-window -h -p 30 'mnemosyne dash'
tmux split-window -v -p 50 'mnemosyne-ics context.md'
tmux select-pane -t 0
claude
```

**Layout:**
```
┌──────────────────────┬──────────────┐
│                      │  Dashboard   │
│   Claude Code        │  (Monitor)   │
│   (Main Chat)        │              │
│                      │              │
├──────────────────────┤              │
│   ICS Editor         │              │
│   (Context)          │              │
└──────────────────────┴──────────────┘
```

**When to use:** Complex projects, orchestration, observability needed

---

## ICS (Integrated Context Studio)

### New Standalone Binary

```bash
mnemosyne-ics [OPTIONS] [FILE]
```

### Features

**Full Terminal Control:**
- No conflicts with other TUIs
- Professional text editing
- Syntax highlighting
- Semantic validation

**Templates:**
```bash
mnemosyne-ics --template api context.md      # API design
mnemosyne-ics --template architecture doc.md # Architecture decisions
mnemosyne-ics --template bugfix fix.md       # Bug fix context
mnemosyne-ics --template feature feat.md     # Feature implementation
mnemosyne-ics --template refactor ref.md     # Refactoring plans
```

**Keyboard Shortcuts:**
- `Ctrl+Q`: Quit
- `Ctrl+S`: Save
- `Ctrl+M`: Memory panel (search/recall)
- `Ctrl+N`: Next typed hole
- `Ctrl+H`: Holes list
- `Ctrl+D`: Diagnostics
- `?`: Help

### Semantic Features

**File References:** `#src/main.rs` → Validated, clickable
**Symbol References:** `@function_name` → Auto-completion
**Typed Holes:** `?ComponentName` → Track TODOs
**Memory Integration:** Search memories inline

---

## Dashboard (Coming Soon)

### Real-Time Monitoring

```bash
mnemosyne dash
```

**Features:**
- Agent activity tracking
- Memory access patterns
- Performance metrics
- Event stream
- System health

**TUI Mode:**
```
┌─ Active Agents ──────────────┐
│ ● Orchestrator (idle)        │
│ ● Optimizer (active)         │
│ ● Reviewer (waiting)         │
└──────────────────────────────┘

┌─ Event Stream ───────────────┐
│ 10:23:45 [Executor] Started  │
│ 10:23:47 [Memory] Recalled   │
└──────────────────────────────┘
```

**Web Mode (Optional):**
```bash
mnemosyne dash --web
# Opens http://localhost:3000
```

---

## Migration Checklist

### For Individual Users

- [ ] Update to Mnemosyne v2.1+
- [ ] Stop using `mnemosyne tui`
- [ ] Install new binary: `mnemosyne-ics`
- [ ] Try basic workflow (just `claude`)
- [ ] Experiment with ICS for context editing
- [ ] Set up tmux layout (optional)

### For Teams

- [ ] Update team documentation
- [ ] Share new workflows
- [ ] Update CI/CD if using TUI mode
- [ ] Train on ICS features
- [ ] Set up shared tmux scripts

### For Scripts/Automation

```bash
# Old
mnemosyne tui  # ❌ Remove

# New
mnemosyne-ics context.md &  # Edit in background
claude --context context.md  # Use file-based context
```

---

## Troubleshooting

### ICS won't start

**Error:** `ICS requires a terminal (TTY)`

**Solution:**
```bash
# If stdin/stdout redirected:
mnemosyne-ics context.md < /dev/tty

# Or run in proper terminal
```

### Can't find `mnemosyne-ics`

**Solution:**
```bash
# Reinstall to get new binary
curl -fsSL https://mnemosyne.sh/install.sh | sh

# Or build from source
cd mnemosyne
cargo build --release --bin mnemosyne-ics
```

### Missing context in Claude

**Cause:** File not passed correctly

**Solution:**
```bash
# Use absolute path
claude --context $(pwd)/context.md

# Or create .claude/context.md (auto-loaded)
```

### Terminal size issues

**Error:** `Cannot determine terminal size`

**Solution:**
```bash
# Set TERM variable
export TERM=xterm-256color

# Or check tmux/screen config
```

---

## Understanding Commands

### What commands exist and when to use them?

| Command | Launch Mode | Purpose | Who Runs It |
|---------|-------------|---------|-------------|
| `mnemosyne serve` | **Automatic** | MCP server for memory | Claude Code (not you!) |
| `mnemosyne-ics [file]` | **Manual** | Context editor (TUI) | You |
| `mnemosyne dash` | **Manual** | Monitoring dashboard | You (coming soon) |
| `mnemosyne remember` | **Manual** | CLI memory operations | You (optional) |
| `mnemosyne recall` | **Manual** | CLI memory search | You (optional) |

**99% of users only need:**
```bash
claude  # Everything else is automatic!
```

**For advanced context editing, add:**
```bash
mnemosyne-ics context.md  # Manual launch
```

### How does MCP auto-launch work?

When you run `claude`, it:
1. Looks for `.claude/mcp_config.json` in your project
2. Sees the mnemosyne server configuration:
   ```json
   {
     "mcpServers": {
       "mnemosyne": {
         "command": "mnemosyne",
         "args": ["serve"]  // ← This command
       }
     }
   }
   ```
3. Launches `mnemosyne serve` as background process
4. Connects via stdio (JSON-RPC)
5. Shuts it down when you exit `claude`

**You never type `mnemosyne serve` yourself!**

---

## FAQ

### Why can't you fix the TUI wrapper?

The fundamental issue is that both Mnemosyne's TUI and Claude Code's TUI try to control the same terminal. This creates unavoidable conflicts:
- Both enter raw mode
- Both handle input events
- Both manage alternate screen
- Input forwarding is complex and fragile

Separate tools avoid this entirely.

### Will the TUI wrapper be removed completely?

The code remains for now (deprecated), but may be removed in v3.0. The new architecture is superior in every way.

### Do I lose any features?

No! Features are preserved and improved:
- ✅ ICS: Standalone (even better!)
- ✅ Memory: Via MCP (**automatic**, no manual launch!)
- ✅ Dashboard: Coming soon (standalone)
- ✅ Context editing: File-based (more flexible)

**Actually better:** Memory is now completely automatic via MCP!

### Can I still use one terminal?

Yes, but less practical:
```bash
# Edit context, then close ICS
mnemosyne-ics context.md
# Now use claude
claude
```

Recommended: Use tmux/screen for better UX.

### What about Windows?

All tools work on Windows:
- ✅ `mnemosyne-ics` - Full support
- ✅ `claude` - Full support
- ⚠️ tmux alternative: Windows Terminal tabs

---

## Getting Help

**Documentation:**
- [ICS Guide](./ICS_README.md)
- [Architecture](../ARCHITECTURE.md)
- [MCP Integration](../MCP_SERVER.md)

**Community:**
- GitHub Issues: https://github.com/rand/mnemosyne/issues
- Discussions: https://github.com/rand/mnemosyne/discussions

**Quick Start:**
```bash
# 1. Install/update
curl -fsSL https://mnemosyne.sh/install.sh | sh

# 2. Configure MCP (one-time setup)
# Add to your project's .claude/mcp_config.json:
{
  "mcpServers": {
    "mnemosyne": {
      "command": "mnemosyne",
      "args": ["serve"]
    }
  }
}

# 3. Use Claude (memory automatic!)
claude

# 4. Optional: Try ICS for context editing
mnemosyne-ics test.md
```

**That's it!** Memory is automatic from step 3 onward.

---

## What's Next?

### Phase 2: Dashboard (Q1 2025)

Real-time monitoring dashboard with:
- TUI and Web modes
- Agent activity tracking
- Performance metrics
- Event streaming API

### Phase 3: Enhanced Integration (Q2 2025)

- IDE plugins (VS Code, IntelliJ)
- Web-based context editor
- Collaborative editing (CRDT)
- Advanced visualizations

### Phase 4: Platform Expansion (Q3 2025)

- Mobile apps (iOS, Android)
- Browser extensions
- Desktop apps (Electron)
- Cloud sync options

---

**Questions?** Open an issue or discussion on GitHub!
