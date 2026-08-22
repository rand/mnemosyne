# Hermes Agent Integration Guide

Mnemosyne works with [Hermes](https://github.com/NousResearch)-powered personal
agents out of the box. The MCP server is fully agent-agnostic — it speaks
standard JSON-RPC 2.0 / Model Context Protocol over stdio, so any agent harness
that supports MCP (or raw JSON-RPC subprocess tools) can use it as persistent
semantic memory.

This guide covers the recommended setup for a personal agent running a local
Hermes model (Hermes 2/3/4 via Ollama, llama.cpp, vLLM, or any
OpenAI-compatible server).

---

## Why Mnemosyne for a Personal Agent

| Capability | Personal-agent benefit |
|---|---|
| Semantic memory (LibSQL vector + FTS5 + graph) | Recall past decisions, preferences, and facts across sessions |
| Project-aware namespaces | Separate memory scopes per project, or one `global` scope for the whole person |
| Local-only storage | Privacy-first: memories never leave your machine |
| OODA tool set (8 MCP tools) | Observe → Orient → Decide → Act loop maps naturally onto agent tool use |
| Evolution system | Automatic consolidation/importance scoring keeps memory lean over time |

## Prerequisites

```bash
# Build the binary (or use cargo install --path .)
cargo build --release
# Binary at target/release/mnemosyne
```

No `ANTHROPIC_API_KEY` is required for core memory operations (store, recall,
list, graph). LLM-enhanced features (Reviewer agent, consolidation analysis)
are optional and work with any OpenAI-compatible endpoint — see
[Optional: Hermes as the LLM backend](#optional-hermes-as-the-llm-backend).

## Step 1: Verify the MCP server

```bash
# Smoke test: initialize handshake
echo '{"jsonrpc":"2.0","method":"initialize","id":1}' | ./target/release/mnemosyne serve
```

Expected: a JSON-RPC response with `serverInfo.name = "mnemosyne"`.

## Step 2: Register with your Hermes agent

### If your harness supports MCP config files

Add mnemosyne as a stdio MCP server (equivalent of `claude_desktop_config.json` /
`mcp_config.json` for other harnesses):

```json
{
  "mcpServers": {
    "mnemosyne": {
      "command": "/absolute/path/to/mnemosyne/target/release/mnemosyne",
      "args": ["serve"],
      "env": {
        "MNEMOSYNE_DB_PATH": "~/.local/share/mnemosyne/memory.db"
      }
    }
  }
}
```

A ready-to-use copy is in [`examples/hermes/mcp-config.json`](../examples/hermes/mcp-config.json).

### If your harness spawns raw subprocess tools

Wrap the same command: spawn `mnemosyne serve`, write newline-delimited JSON-RPC
to stdin, read responses from stdout (logs go to stderr). See
[`MCP_SERVER.md`](../MCP_SERVER.md) for the full wire protocol.

## Step 3: Project metadata (optional)

Mnemosyne auto-detects the current project by walking up to the nearest `.git`
directory and reading the first metadata file it finds, in priority order:

1. `CLAUDE.md` (backward compatibility)
2. `AGENTS.md` (cross-agent standard)
3. `HERMES.md` (Hermes-specific)

For a personal-agent workspace, drop a `HERMES.md` (or `AGENTS.md`) at the
repo root:

```markdown
---
project: my-personal-workspace
description: "Tasks, preferences, and decisions for my Hermes agent"
---

# My Personal Workspace
```

Memories stored while inside that directory are automatically scoped to the
`project:my-personal-workspace` namespace.

## The 8 OODA Tools

| Phase | Tool | Use it for |
|---|---|---|
| Observe | `mnemosyne.recall` | Semantic search over memories |
| Observe | `mnemosyne.list` | Browse recent memories in a namespace |
| Orient | `mnemosyne.graph` | Explore memory relationships |
| Orient | `mnemosyne.context` | Load a context bundle for the current task |
| Decide | `mnemosyne.remember` | Store a new memory (insight/decision/task/reference) |
| Decide | `mnemosyne.consolidate` | Merge duplicates, recompute importance |
| Act | `mnemosyne.update` | Amend an existing memory |
| Act | `mnemosyne.delete` | Remove a memory |

Typical personal-agent system-prompt snippet:

```text
You have persistent memory via the mnemosyne tools.
- At the start of a session, call mnemosyne.context to load relevant memories.
- When the user states a durable preference or decision, call mnemosyne.remember.
- Prefer project namespaces for work topics; use the global namespace for
  personal facts.
```

## Optional: Hermes as the LLM backend

Core memory works without any LLM API. If you want LLM-enhanced review and
consolidation, point Mnemosyne at your local Hermes endpoint using the
OpenAI-compatible API exposed by Ollama / vLLM / llama.cpp:

```bash
# Example: Ollama serving Hermes
export OPENAI_API_BASE=http://localhost:11434/v1
export OPENAI_MODEL=hermes3
```

If your Mnemosyne build requires an API key placeholder for these paths, set a
dummy value — requests stay on localhost.

## Performance notes

- Release builds use full LTO + `codegen-units = 1` (see `.cargo/config.toml`);
  retrieval is sub-millisecond on warm caches.
- Use `cargo build --profile fast-release` for quicker iteration builds.
- The `sccache` wrapper configured in `.cargo/config.toml` is optional; install
  it or unset `RUSTC_WRAPPER` if you don't use it.

## Troubleshooting

| Symptom | Fix |
|---|---|
| `initialize` gets no response | Ensure you're writing to stdin of `mnemosyne serve` (not `mnemosyne` with no args on older builds) |
| Memories land in the wrong namespace | Check which metadata file exists at the repo root; `CLAUDE.md` wins if multiple exist |
| Build fails with `sccache` not found | `apt install sccache` or `unset RUSTC_WRAPPER` |
| Build fails on `openssl-sys` | Install `pkg-config` and `libssl-dev` |
