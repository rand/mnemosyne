# Mnemosyne Installation Guide

## Prerequisites

- Rust 1.75 or higher
- Anthropic API key (for LLM features)
- Claude Code CLI

## Installation Steps

### 1. Build Mnemosyne

```bash
cd /path/to/mnemosyne
cargo build --release
```

### 2. Install Binary

```bash
# Install to cargo bin directory (in PATH)
cargo install --path .

# Or copy manually
cp target/release/mnemosyne ~/.local/bin/
# OR
cp target/release/mnemosyne /usr/local/bin/
```

### 3. Configure API Key

Choose one of the following methods:

**Option A: Interactive Setup (Recommended)**
```bash
mnemosyne config set-key
```

**Option B: Command Line**
```bash
mnemosyne config set-key sk-ant-api03-...
```

**Option C: Environment Variable**
```bash
# Add to ~/.bashrc, ~/.zshrc, or ~/.profile
export ANTHROPIC_API_KEY=sk-ant-api03-...
```

### 4. Initialize Database

```bash
# Initialize SQLite database (creates mnemosyne.db)
mnemosyne init
```

### 5. Configure Claude Code

**Option A: Project-Level Configuration**

Copy the MCP configuration to your project:

```bash
mkdir -p .claude
cp /path/to/mnemosyne/.claude/mcp_config.json .claude/
```

**Option B: Global Configuration**

For use across all projects:

```bash
mkdir -p ~/.claude
cat > ~/.claude/mcp_config.json <<'EOF'
{
  "mcpServers": {
    "mnemosyne": {
      "command": "mnemosyne",
      "args": ["serve"],
      "env": {
        "RUST_LOG": "info"
      },
      "description": "Mnemosyne - Project-aware agentic memory system"
    }
  }
}
EOF
```

### 6. Verify Installation

```bash
# Check status
mnemosyne status

# Check API key configuration
mnemosyne config show-key

# Test MCP server
echo '{"jsonrpc":"2.0","method":"initialize","id":1}' | mnemosyne serve
# Should return JSON with server info
```

## Usage

### MCP Server (for Claude Code)

The MCP server runs automatically when Claude Code starts. Claude will use these tools:

**OBSERVE:**
- `mnemosyne.recall` - Search memories
- `mnemosyne.list` - List recent memories

**ORIENT:**
- `mnemosyne.graph` - Get memory graph
- `mnemosyne.context` - Get full context

**DECIDE:**
- `mnemosyne.remember` - Store new memory
- `mnemosyne.consolidate` - Merge memories

**ACT:**
- `mnemosyne.update` - Update memory
- `mnemosyne.delete` - Archive memory

### Manual Testing

Test individual tools:

```bash
# Store a memory
echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"mnemosyne.remember","arguments":{"content":"Decided to use PostgreSQL for user data","context":"myproject"}},"id":1}' | mnemosyne serve

# Recall memories (when hybrid search is implemented)
echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"mnemosyne.recall","arguments":{"query":"database decisions"}},"id":2}' | mnemosyne serve
```

## Troubleshooting

### API Key Issues

```bash
# Check if key is configured
mnemosyne config show-key

# Reconfigure
mnemosyne config delete-key
mnemosyne config set-key
```

### Database Issues

```bash
# Reinitialize database (WARNING: deletes existing data)
rm mnemosyne.db
mnemosyne init
```

### MCP Server Issues

```bash
# Check logs (goes to stderr)
echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | mnemosyne serve 2>&1 | grep -E "(ERROR|WARN)"

# Increase logging
RUST_LOG=debug echo '{"jsonrpc":"2.0","method":"initialize","id":1}' | mnemosyne serve
```

## Uninstallation

```bash
# Remove binary
cargo uninstall mnemosyne
# OR
rm ~/.local/bin/mnemosyne

# Remove API key from keychain
mnemosyne config delete-key

# Remove database
rm mnemosyne.db

# Remove configuration
rm -rf .claude/mcp_config.json
rm -rf ~/.claude/mcp_config.json
```

## Security Notes

- API keys are stored securely in OS keychains:
  - **macOS**: Keychain
  - **Windows**: Credential Manager
  - **Linux**: Secret Service (libsecret)
- Keys are never written to disk in plaintext
- Environment variables (`ANTHROPIC_API_KEY`) take precedence over keychain
- Database file (`mnemosyne.db`) contains memory content - protect accordingly

## What's Next?

See [MCP_SERVER.md](MCP_SERVER.md) for API documentation and examples.
