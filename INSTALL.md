# Installation Guide

Complete installation instructions for Mnemosyne, the project-aware memory system for Claude Code.

## Table of Contents

- [Quick Install (Recommended)](#quick-install-recommended)
- [Installation Script Options](#installation-script-options)
- [Manual Installation](#manual-installation)
- [Configuration](#configuration)
- [Verification](#verification)
- [Troubleshooting](#troubleshooting)
- [Uninstallation](#uninstallation)

---

## Quick Install (Recommended)

### Prerequisites

Before installing, ensure you have:

```bash
# Rust 1.75+ (required)
rustc --version
# Should show: rustc 1.75.0 or higher

# If not installed:
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Git (required)
git --version

# Anthropic API key (required for LLM features)
# Get from: https://console.anthropic.com/
```

### One-Command Installation

```bash
# Clone the repository
git clone https://github.com/rand/mnemosyne.git
cd mnemosyne

# Run automated installer
./scripts/install/install.sh
```

**What the installer does:**

1. ✅ Builds Rust binary (`cargo build --release`)
2. ✅ Installs to `~/.local/bin/mnemosyne`
3. ✅ Creates database directory (`~/.local/share/mnemosyne/`)
4. ✅ Initializes database (`mnemosyne.db`)
5. ✅ Prompts for API key configuration
6. ✅ Sets up MCP server configuration
7. ✅ Verifies installation

**Time to complete:** 2-3 minutes

**Expected output:**
```
==> Building Mnemosyne (release mode)

This will compile ~150 Rust dependencies plus the main binary.
Expected time: 2-3 minutes on most systems (longer on first build)

Build progress will stream below - this is normal!
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

   Compiling libc v0.2.147
   Compiling cfg-if v1.0.0
   ... (many dependencies) ...
   ⏱  Progress: 50 crates compiled (1m 15s elapsed)
   ... (more dependencies) ...
   ⏱  Progress: 100 crates compiled (2m 5s elapsed)
   ... (more dependencies) ...
   ✓ Dependencies complete! Building main binary... (2m 20s)
   Compiling mnemosyne v2.0.0
   Finished `release` profile [optimized] target(s) in 2m 45s

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✓ Build complete in 2m 45s
✓ Binary size: 48.2 MB
✓ Location: target/release/mnemosyne

==> Installing binary to /Users/you/.local/bin
✓ Installed to /Users/you/.local/bin/mnemosyne

==> Initializing database
✓ Database initialized

==> Configuring Anthropic API key
✓ API key configured

==> Configuring MCP server
✓ MCP config updated: /Users/you/.claude/mcp_config.json

Installation complete!

Next steps:
  1. Test the installation: mnemosyne status
  2. Start using Mnemosyne in Claude Code
  3. See: QUICK_START.md for getting started
```

**What you'll see during the build:**
- **Real-time compilation output** - Every crate being compiled streams to the terminal
- **Progress updates** - Every 10 crates compiled, you'll see a progress indicator with elapsed time
- **Milestone notifications** - When dependency compilation completes and main binary build starts
- **Build summary** - Total build time, binary size, and location after completion

**Build process is working correctly if:**
- ✅ You see "Compiling..." messages streaming
- ✅ Progress indicators appear every 10-30 seconds
- ✅ Build completes with "Finished" message
- ✅ No red error messages appear

**Build might be stuck if:**
- ❌ No output for more than 2 minutes
- ❌ Error messages appear and build stops
- ❌ Process hangs without "Finished" message

See [Troubleshooting](#troubleshooting) section below if build fails.

---

## Installation Script Options

The install script supports several options for customization:

### Basic Options

```bash
# Show help
./scripts/install/install.sh --help

# Skip API key configuration (configure later)
./scripts/install/install.sh --skip-api-key

# Non-interactive mode (answer yes to all prompts)
./scripts/install/install.sh --yes
```

### Custom Paths

```bash
# Install binary to custom directory
./scripts/install/install.sh --bin-dir /usr/local/bin

# Default is: ~/.local/bin
# Ensure custom directory is in your PATH
```

### MCP Configuration Options

```bash
# Install MCP config globally (~/.claude/mcp_config.json)
./scripts/install/install.sh --global-mcp

# Install to project only (.claude/mcp_config.json)
./scripts/install/install.sh
# (Project-level is default)

# Skip MCP configuration entirely
./scripts/install/install.sh --no-mcp
```

### Combined Options

```bash
# Example: CI/CD installation
./scripts/install/install.sh \
  --bin-dir /usr/local/bin \
  --skip-api-key \
  --global-mcp \
  --yes

# Example: Development installation
./scripts/install/install.sh \
  --bin-dir ~/.local/bin \
  --no-mcp
```

---

## Manual Installation

For advanced users or custom setups:

### Step 1: Build from Source

```bash
# Clone repository
git clone https://github.com/rand/mnemosyne.git
cd mnemosyne

# Build release binary
cargo build --release

# Binary location: ./target/release/mnemosyne
```

**Build options:**
```bash
# Debug build (faster compile, slower runtime)
cargo build

# Release build with optimizations
cargo build --release

# Check build without producing binary
cargo check
```

### Step 2: Install Binary

**Option A: Cargo install (recommended)**
```bash
cargo install --path .

# Installs to: ~/.cargo/bin/mnemosyne
# Usually already in PATH
```

**Option B: Manual copy**
```bash
# To user bin directory
mkdir -p ~/.local/bin
cp target/release/mnemosyne ~/.local/bin/

# Ensure ~/.local/bin is in PATH
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Or to system bin directory (requires sudo)
sudo cp target/release/mnemosyne /usr/local/bin/
```

**Verify binary is accessible:**
```bash
which mnemosyne
# Should show: /Users/you/.local/bin/mnemosyne

mnemosyne --help
# Should show usage information
```

### Step 3: Initialize Database

```bash
# Using default path (~/.local/share/mnemosyne/mnemosyne.db)
mnemosyne init

# Or specify custom path
mnemosyne --db-path /path/to/custom/mnemosyne.db init

# Or use environment variable
export MNEMOSYNE_DB_PATH=/path/to/custom/mnemosyne.db
mnemosyne init
```

**Database path priority:**
1. `--db-path` CLI flag (highest priority)
2. `MNEMOSYNE_DB_PATH` environment variable
3. Default: `~/.local/share/mnemosyne/mnemosyne.db`

### Step 4: Configure API Key

See [Configuration](#configuration) section below.

### Step 5: Set Up MCP Server

**For Claude Code integration:**

**Global configuration** (all projects):
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
      "description": "Mnemosyne - Project-aware memory system"
    }
  }
}
EOF
```

**Project-level configuration** (specific project):
```bash
# In your project directory
mkdir -p .claude

cat > .claude/mcp_config.json <<'EOF'
{
  "mcpServers": {
    "mnemosyne": {
      "command": "mnemosyne",
      "args": ["serve"],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
EOF
```

**Merge with existing config:**
```bash
# Use jq to merge
jq -s '.[0] * .[1]' ~/.claude/mcp_config.json new_server.json > merged.json
mv merged.json ~/.claude/mcp_config.json
```

---

## Configuration

### API Key Setup

Mnemosyne requires an Anthropic API key for LLM-powered memory enrichment.

**Three-tier security with priority order:**

#### Option 1: Environment Variable (Highest Priority)

Best for: CI/CD, temporary testing

```bash
# Add to shell profile (~/.bashrc, ~/.zshrc, etc.)
export ANTHROPIC_API_KEY=sk-ant-api03-YOUR_KEY_HERE

# Reload shell
source ~/.zshrc  # or source ~/.bashrc

# Verify
echo $ANTHROPIC_API_KEY
```

#### Option 2: Age-Encrypted File (Recommended)

Best for: Daily development, secure persistent storage

```bash
# Interactive setup
mnemosyne secrets init

# Or set directly
mnemosyne secrets set ANTHROPIC_API_KEY

# When prompted, enter: sk-ant-api03-YOUR_KEY_HERE

# Stored at: ~/.config/mnemosyne/secrets.age
# Encrypted with X25519 + ChaCha20-Poly1305
```

**Benefits:**
- Encrypted at rest
- No keychain prompts
- Cross-platform
- Easy backup/restore

#### Option 3: OS Keychain (Fallback)

Best for: Backward compatibility, OS-managed secrets

The keychain is used automatically as a fallback if no environment variable or age file exists.

**Verify configuration:**
```bash
mnemosyne config show-key
```

**Expected output:**
```
✓ API key is accessible via environment variable
# or
✓ API key is accessible via age encrypted file
# or
✓ API key is accessible via OS keychain
```

### Database Path Configuration

**Default location:**
```
~/.local/share/mnemosyne/mnemosyne.db
```

**Custom location via CLI flag:**
```bash
mnemosyne --db-path /custom/path/mnemosyne.db serve
```

**Custom location via environment variable:**
```bash
export MNEMOSYNE_DB_PATH=/custom/path/mnemosyne.db
mnemosyne serve
```

**Per-project database:**
```bash
# Set in project directory
cd /path/to/myproject
export MNEMOSYNE_DB_PATH=$(pwd)/mnemosyne.db
mnemosyne init
```

### MCP Server Configuration

**Configuration file locations:**

- **Global:** `~/.claude/mcp_config.json`
- **Project:** `.claude/mcp_config.json`

**Configuration options:**

```json
{
  "mcpServers": {
    "mnemosyne": {
      "command": "mnemosyne",
      "args": ["serve"],
      "env": {
        "RUST_LOG": "info",
        "MNEMOSYNE_DB_PATH": "/custom/path/mnemosyne.db"
      },
      "description": "Mnemosyne memory system"
    }
  }
}
```

**Log levels:**
- `error`: Errors only
- `warn`: Warnings and errors
- `info`: Info, warnings, and errors (default)
- `debug`: Verbose debugging output
- `trace`: Very verbose output

**Custom database per MCP server:**
```json
{
  "mcpServers": {
    "mnemosyne-project-a": {
      "command": "mnemosyne",
      "args": ["--db-path", "/projects/a/mnemosyne.db", "serve"]
    },
    "mnemosyne-project-b": {
      "command": "mnemosyne",
      "args": ["--db-path", "/projects/b/mnemosyne.db", "serve"]
    }
  }
}
```

### Hooks Configuration (Optional)

For automatic memory capture in Claude Code, you can configure hooks:

**Automatic Setup** (Recommended):
```bash
# The install.sh script offers to configure hooks automatically
./scripts/install/install.sh
# Answer "yes" when prompted about hooks configuration
```

**Manual Setup** (if needed):
```bash
# Make hooks executable
chmod +x .claude/hooks/*.sh

# Configure in .claude/settings.json with ABSOLUTE paths
# Important: Use absolute paths to prevent issues after context compaction
cat > .claude/settings.json <<EOF
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": ".*",
        "hooks": [
          {
            "type": "command",
            "command": "$(pwd)/.claude/hooks/session-start.sh"
          }
        ]
      }
    ],
    "PreCompact": [
      {
        "matcher": ".*",
        "hooks": [
          {
            "type": "command",
            "command": "$(pwd)/.claude/hooks/pre-compact.sh"
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "^Bash\\\\(git commit.*",
        "hooks": [
          {
            "type": "command",
            "command": "$(pwd)/.claude/hooks/post-commit.sh"
          }
        ]
      }
    ]
  }
}
EOF
```

**What the hooks do**:
- `session-start.sh`: Loads important project memories at session start
- `pre-compact.sh`: Saves context before Claude Code compacts conversation history
- `post-commit.sh`: Captures git commits as memories for traceability

**Important**: Always use absolute paths in hook configurations to prevent "No such file or directory" errors after context compaction.

See [HOOKS_TESTING.md](HOOKS_TESTING.md) for detailed hook testing and validation.

---

## Verification

After installation, verify everything works:

### 1. Binary Accessibility

```bash
which mnemosyne
# Should show path to binary

mnemosyne --help
# Should show usage information
```

### 2. Database Initialization

```bash
# Initialize test database
mnemosyne init

# Check database file exists
ls -lh ~/.local/share/mnemosyne/mnemosyne.db

# Should show file with size > 0
```

### 3. API Key Configuration

```bash
mnemosyne config show-key
# Should show: "✓ API key is accessible via [method]"

# NOT: "Error: No API key found"
```

### 4. MCP Server

```bash
# Test MCP server manually
echo '{"jsonrpc":"2.0","method":"initialize","id":1}' | mnemosyne serve

# Should return JSON response (not error)
# Press Ctrl+C to stop
```

### 5. Store and Retrieve Memory

```bash
# Store a test memory
mnemosyne remember \
  --content "Installation test memory" \
  --importance 5 \
  --format json

# Should return JSON with memory ID

# List memories
mnemosyne list --limit 5 --format json

# Should show the test memory

# Search memories
mnemosyne recall --query "installation" --format json

# Should find the test memory
```

### 6. Claude Code Integration

```bash
# Restart Claude Code completely (quit and relaunch)

# In Claude Code, check MCP servers list
# Should see "mnemosyne" with 8 tools:
#   - mnemosyne.remember
#   - mnemosyne.recall
#   - mnemosyne.list
#   - mnemosyne.graph
#   - mnemosyne.context
#   - mnemosyne.consolidate
#   - mnemosyne.update
#   - mnemosyne.delete

# Try a slash command
/memory-store Test from Claude Code

# Should work without errors
```

---

## Troubleshooting

### Common Installation Issues

#### "mnemosyne: command not found"

**Cause:** Binary not in PATH

**Solution:**
```bash
# Check if binary exists
ls -la ~/.local/bin/mnemosyne

# If exists, add to PATH
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# If missing, reinstall
./scripts/install/install.sh
```

#### Build fails with "linker `cc` not found"

**Cause:** Missing C compiler

**Solution:**
```bash
# macOS
xcode-select --install

# Ubuntu/Debian
sudo apt-get install build-essential

# Fedora/RHEL
sudo dnf groupinstall "Development Tools"

# Then retry build
cargo build --release
```

#### "failed to run custom build command for `libsql`"

**Cause:** Incompatible Rust version

**Solution:**
```bash
# Update Rust
rustup update stable
rustup default stable

# Clean and rebuild
cargo clean
cargo build --release
```

#### "Database initialization failed"

**Cause:** Permission issues or missing parent directory

**Solution:**
```bash
# Create parent directory
mkdir -p ~/.local/share/mnemosyne

# Check permissions
ls -la ~/.local/share/

# Initialize with explicit path
mnemosyne --db-path ~/.local/share/mnemosyne/mnemosyne.db init
```

#### "MCP server failed to start"

**Cause:** Configuration error or binary path wrong

**Solution:**
```bash
# Verify binary location
which mnemosyne

# Test server manually
echo '{"jsonrpc":"2.0","method":"initialize","id":1}' | mnemosyne serve

# Check MCP config
cat ~/.claude/mcp_config.json | jq .

# Ensure command matches binary location
```

### Getting More Help

For detailed troubleshooting, see:
- **[TROUBLESHOOTING.md](TROUBLESHOOTING.md)** - Comprehensive issue resolution
- **[GitHub Issues](https://github.com/rand/mnemosyne/issues)** - Known issues and fixes
- **[GitHub Discussions](https://github.com/rand/mnemosyne/discussions)** - Community support

---

## Uninstallation

### Using Uninstall Script

```bash
# Standard uninstall (preserves data)
./scripts/install/uninstall.sh

# Uninstall with data purge
./scripts/install/uninstall.sh --purge

# Non-interactive mode
./scripts/install/uninstall.sh --yes

# Show help
./scripts/install/uninstall.sh --help
```

**What the uninstaller does:**

**Standard mode (default):**
- ✅ Removes binary
- ✅ Removes MCP config
- ✅ Preserves database
- ✅ Preserves secrets
- ✅ Creates backup of removed files

**Purge mode (`--purge`):**
- ✅ Removes binary
- ✅ Removes MCP config
- ✅ **Deletes database**
- ✅ **Deletes secrets**
- ✅ Removes all data

### Manual Uninstallation

```bash
# Remove binary
rm ~/.local/bin/mnemosyne
# or
rm /usr/local/bin/mnemosyne
# or
cargo uninstall mnemosyne

# Remove MCP config
# Edit and remove mnemosyne section from:
nano ~/.claude/mcp_config.json

# Remove database (optional, loses all memories!)
rm -rf ~/.local/share/mnemosyne/

# Remove secrets (optional)
rm -rf ~/.config/mnemosyne/

# Remove project hooks (if installed)
rm -rf .claude/hooks/
```

### Reinstallation

After uninstallation, you can reinstall cleanly:

```bash
# If you preserved data (standard uninstall):
./scripts/install/install.sh
# Database and secrets will be reused

# If you purged data:
./scripts/install/install.sh
# Fresh installation, configure API key again
```

---

## Advanced Topics

### Multiple Installations

You can install multiple versions side-by-side:

```bash
# Install v1.0 to one location
git checkout v1.0.0
cargo install --path . --root ~/.local/mnemosyne-v1.0

# Install v1.1 to another location
git checkout v1.1.0
cargo install --path . --root ~/.local/mnemosyne-v1.1

# Use specific version
~/.local/mnemosyne-v1.0/bin/mnemosyne --version
~/.local/mnemosyne-v1.1/bin/mnemosyne --version
```

### Upgrading

See [Migration Guide](docs/guides/migration.md) for version-specific upgrade instructions.

**General upgrade process:**

```bash
# Backup database
cp ~/.local/share/mnemosyne/mnemosyne.db \
   ~/.local/share/mnemosyne/mnemosyne-backup-$(date +%Y%m%d).db

# Export memories (optional safety)
mnemosyne export --output memories-backup-$(date +%Y%m%d).md

# Update code
cd /path/to/mnemosyne
git pull origin main

# Rebuild and install
cargo build --release
cargo install --path .

# Restart Claude Code
```

### Development Installation

For contributing to Mnemosyne:

```bash
# Clone repository
git clone https://github.com/rand/mnemosyne.git
cd mnemosyne

# Install development dependencies
cargo build

# For Python orchestration development
python3 -m venv .venv
source .venv/bin/activate
uv pip install maturin pytest
export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1
maturin develop

# Run tests
./test-all.sh

# See CONTRIBUTING.md for full development setup
```

---

## Next Steps

After successful installation:

1. **[Quick Start Guide](QUICK_START.md)** - Store your first memory in 5 minutes
2. **[Common Workflows](docs/guides/workflows.md)** - Learn practical usage patterns
3. **[Hooks Configuration](HOOKS_TESTING.md)** - Set up automatic memory capture
4. **[MCP API Reference](MCP_SERVER.md)** - Explore all available tools

---

**Last Updated**: 2025-10-27
**Version**: 1.0.0
