# Troubleshooting Guide

This guide covers common issues and their solutions when installing, configuring, and using Mnemosyne.

## Table of Contents

- [Installation Issues](#installation-issues)
- [Runtime Issues](#runtime-issues)
- [Development Issues](#development-issues)
- [Common Error Messages](#common-error-messages)
- [Debugging Tools](#debugging-tools)
- [Getting Help](#getting-help)

---

## Installation Issues

### "mnemosyne: command not found"

**Cause**: Binary not in PATH or not installed correctly.

**Solution**:
```bash
# Check if binary exists
ls -la ~/.local/bin/mnemosyne

# If missing, reinstall
./scripts/install/install.sh

# Or install manually
cargo install --path .

# Verify PATH includes install directory
echo $PATH | grep -o "$HOME/.local/bin"

# If not in PATH, add to ~/.bashrc or ~/.zshrc:
export PATH="$HOME/.local/bin:$PATH"
source ~/.bashrc  # or source ~/.zshrc
```

### Build Failures

#### "error: failed to run custom build command for `libsql`"

**Cause**: Missing system dependencies or incompatible Rust version.

**Solution**:
```bash
# Update Rust to latest version
rustup update stable
rustup default stable

# Verify Rust version (need 1.75+)
rustc --version

# Clean and rebuild
cargo clean
cargo build --release
```

#### "error: linker `cc` not found"

**Cause**: Missing C compiler (required for some dependencies).

**Solution**:
```bash
# macOS
xcode-select --install

# Ubuntu/Debian
sudo apt-get install build-essential

# Fedora/RHEL
sudo dnf groupinstall "Development Tools"
```

**Note**: The installation script now provides platform-specific instructions automatically when this error occurs.

#### Build Appears Stuck or Hung

**Symptoms**: No output for extended period during installation build.

**Normal behavior**:
- Build takes 2-3 minutes on most systems
- Progress indicators appear every 10 crates or 30 seconds
- Some large dependencies (like `libsql`) can take 30-60 seconds to compile
- You should see "Compiling..." messages streaming

**If truly stuck** (no output for >2 minutes):
1. Check your internet connection (dependencies download from crates.io)
2. Press Ctrl+C to cancel
3. Clean build cache: `cargo clean`
4. Retry installation: `./scripts/install/install.sh`

**Check build logs**:
```bash
# If build fails, error log is saved to:
ls -lt /tmp/mnemosyne-build-error-*.log | head -1

# View the most recent error log:
cat $(ls -t /tmp/mnemosyne-build-error-*.log | head -1)
```

#### Understanding Build Progress Indicators

The enhanced installation script provides real-time progress feedback:

**Progress messages**:
- `⏱  Progress: N crates compiled (Xm Ys elapsed)` - Appears every 10 crates
- `✓ Dependencies complete! Building main binary...` - All deps compiled, building mnemosyne
- `✓ Build complete in Xm Ys` - Build finished successfully

**If you see these, the build is working correctly:**
```
   Compiling libc v0.2.147
   Compiling cfg-if v1.0.0
   ⏱  Progress: 10 crates compiled (0m 25s elapsed)
   ...
   ✓ Dependencies complete! Building main binary... (2m 15s)
   Compiling mnemosyne v2.0.0
   ✓ Build complete in 2m 45s
```

**Build time variations**:
- **First build**: 2-3 minutes (downloads all dependencies)
- **Incremental build**: 10-30 seconds (reuses cached dependencies)
- **After `cargo clean`**: 2-3 minutes (full rebuild)

### Database Initialization Errors

#### "Error: failed to initialize database: unable to open database file"

**Cause**: Permission issues or parent directory doesn't exist.

**Solution**:
```bash
# Check database path
echo $MNEMOSYNE_DB_PATH

# Create parent directory if needed
mkdir -p ~/.local/share/mnemosyne

# Initialize with explicit path
mnemosyne --db-path ~/.local/share/mnemosyne/mnemosyne.db init

# Check permissions
ls -la ~/.local/share/mnemosyne/
```

#### "Error: database is locked"

**Cause**: Another mnemosyne process is using the database.

**Solution**:
```bash
# Find and kill existing processes
pkill -f mnemosyne

# Or find specific PID
ps aux | grep mnemosyne
kill <PID>

# Wait a moment, then retry
sleep 2
mnemosyne init
```

### MCP Configuration Not Detected

#### Claude Code doesn't show Mnemosyne tools

**Cause**: MCP configuration file not in correct location or has syntax errors.

**Solution**:
```bash
# Check if MCP config exists
cat ~/.claude/mcp_config.json

# Verify JSON is valid
cat ~/.claude/mcp_config.json | jq .

# If missing or invalid, reinstall MCP config
./scripts/install/install.sh --global-mcp

# For project-level config
cat .claude/mcp_config.json

# Restart Claude Code completely
# (quit and relaunch, not just close window)
```

#### "MCP server mnemosyne failed to start"

**Cause**: Binary path incorrect or binary not executable.

**Solution**:
```bash
# Test server manually
echo '{"jsonrpc":"2.0","method":"initialize","id":1}' | mnemosyne serve

# Check binary is executable
chmod +x ~/.local/bin/mnemosyne

# Verify binary location in MCP config
cat ~/.claude/mcp_config.json | jq '.mcpServers.mnemosyne.command'

# Should show full path or "mnemosyne" if in PATH
```

---

## Runtime Issues

### API Key Errors

#### "Error: No API key found"

**Cause**: Anthropic API key not configured.

**Solution**:
```bash
# Check key status
mnemosyne config show-key

# Configure via environment variable (highest priority)
export ANTHROPIC_API_KEY=sk-ant-api03-...

# Or use encrypted age file (recommended)
mnemosyne secrets set ANTHROPIC_API_KEY

# Or use OS keychain (fallback)
# Will be prompted automatically when needed

# Verify configuration
if [ -n "$ANTHROPIC_API_KEY" ]; then
  echo "✓ API key set via environment variable"
else
  mnemosyne config show-key
fi
```

#### "Error: Invalid API key"

**Cause**: API key is incorrect or expired.

**Solution**:
```bash
# Delete old key
mnemosyne secrets delete ANTHROPIC_API_KEY

# Set new key
mnemosyne secrets set ANTHROPIC_API_KEY

# Or update environment variable
export ANTHROPIC_API_KEY=sk-ant-api03-YOUR_NEW_KEY

# Test with simple operation
mnemosyne remember --content "Test memory" --format json
```

#### Repeated keychain prompts on macOS

**Cause**: Using keychain fallback instead of age encryption.

**Solution**:
```bash
# Migrate to age-encrypted storage (recommended)
mnemosyne secrets set ANTHROPIC_API_KEY

# This will prompt once, then store encrypted

# Or use environment variable to avoid prompts entirely
# Add to ~/.zshrc or ~/.bashrc:
export ANTHROPIC_API_KEY=sk-ant-api03-...
```

### Memory Retrieval Issues

#### "No memories found" when you know they exist

**Cause**: Namespace mismatch or incorrect database path.

**Solution**:
```bash
# Check current namespace
cd /path/to/your/project
basename $(pwd)  # This is your project namespace

# List all memories without namespace filter
mnemosyne recall --query "test" --limit 10 --format json

# List memories for specific namespace
mnemosyne recall --query "test" --namespace "project:myproject" --format json

# Check database location
echo $MNEMOSYNE_DB_PATH

# If using multiple databases, ensure you're using the right one
```

#### Search returns irrelevant results

**Cause**: Query too broad or importance threshold too low.

**Solution**:
```bash
# Use more specific query terms
mnemosyne recall --query "authentication JWT implementation" --limit 5

# Filter by minimum importance
mnemosyne recall --query "architecture" --min-importance 7 --limit 5

# Search within specific namespace
mnemosyne recall --query "bug" --namespace "project:myproject" --limit 5

# Combine filters
mnemosyne recall \
  --query "database migration" \
  --namespace "project:myproject" \
  --min-importance 6 \
  --limit 3 \
  --format json
```

### Performance Issues

#### Slow query responses (>5 seconds)

**Cause**: Large database or inefficient query.

**Solution**:
```bash
# Check database size
ls -lh ~/.local/share/mnemosyne/mnemosyne.db

# Run VACUUM to optimize
sqlite3 ~/.local/share/mnemosyne/mnemosyne.db "VACUUM;"

# Consider archiving old memories
mnemosyne recall --query "old" --format json | \
  jq -r '.results[] | select(.importance < 5) | .id' | \
  while read id; do
    mnemosyne delete --id "$id" --format json
  done

# For very large databases, consider using Turso remote database
# See ARCHITECTURE.md for migration guide
```

#### High memory usage (>500MB)

**Cause**: Long-running MCP server with memory leaks (rare) or very large result sets.

**Solution**:
```bash
# Restart MCP server (restart Claude Code)

# Limit result set sizes
mnemosyne recall --query "test" --limit 10  # Instead of default 50

# Check for runaway processes
ps aux | grep mnemosyne | grep -v grep

# If memory continues growing, file a bug report with:
# - Database size
# - Number of memories
# - Query patterns
```

### Hook Execution Failures

#### Hooks not running

**Cause**: Hooks not configured in Claude Code settings or binary not found.

**Solution**:
```bash
# Check hook configuration
cat .claude/settings.json | jq '.hooks'

# Verify hook scripts exist and are executable
ls -la .claude/hooks/
chmod +x .claude/hooks/*.sh

# Test hook manually
.claude/hooks/session-start.sh

# Check Claude Code picked up hook config
# (Restart Claude Code if hooks.json was just added)
```

#### "Mnemosyne not installed" error in hooks

**Cause**: Hook can't find mnemosyne binary.

**Solution**:
```bash
# Verify mnemosyne is in PATH
which mnemosyne

# If not found, hooks will look for local build
ls -la target/release/mnemosyne
ls -la target/debug/mnemosyne

# Ensure at least one exists, or install globally
./scripts/install/install.sh

# Test hook can find binary
cd /your/project
.claude/hooks/session-start.sh 2>&1 | head -20
```

---

## Development Issues

### PyO3 Build Failures

#### "error: failed to run custom build command for `pyo3-ffi`"

**Cause**: Python development headers not installed or wrong Python version.

**Solution**:
```bash
# Verify Python version (need 3.10+)
python3 --version

# macOS: Install Python dev tools
brew install python@3.11

# Ubuntu/Debian
sudo apt-get install python3-dev python3.11-dev

# Set PYO3 environment variable
export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1

# Rebuild
maturin develop
```

#### "ImportError: mnemosyne_core module not found"

**Cause**: PyO3 bindings not built or wrong Python environment.

**Solution**:
```bash
# Ensure you're in a virtual environment
python3 -m venv .venv
source .venv/bin/activate  # or .venv/Scripts/activate on Windows

# Install maturin
uv pip install maturin

# Build PyO3 bindings
export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1
maturin develop

# Verify installation
python3 -c "import mnemosyne_core; print('✓ PyO3 bindings loaded')"

# Run orchestration tests
pytest tests/orchestration/ -v
```

### Test Failures

#### Rust unit tests failing

**Cause**: Database state from previous tests or missing test data.

**Solution**:
```bash
# Clean test databases
rm -f test_*.db

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_memory_creation -- --nocapture

# Run with debug logging
RUST_LOG=debug cargo test
```

#### LLM integration tests failing

**Cause**: API key not configured or rate limiting.

**Solution**:
```bash
# These tests require API key and are marked #[ignore]
export ANTHROPIC_API_KEY=sk-ant-api03-...

# Run LLM tests explicitly
cargo test -- --ignored

# Or use test script
./test-all.sh --llm-only

# If rate limited, wait and retry
sleep 60
cargo test -- --ignored
```

#### Python orchestration tests failing

**Cause**: PyO3 bindings not built or API key missing.

**Solution**:
```bash
# Activate virtual environment
source .venv/bin/activate

# Rebuild PyO3 bindings
export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1
maturin develop

# Set API key for integration tests
export ANTHROPIC_API_KEY=sk-ant-api03-...

# Run non-API tests only
pytest tests/orchestration/ -v -m "not integration"

# Run all tests including API
pytest tests/orchestration/ -v
```

### Maturin Development Mode Issues

#### "error: no such command: develop"

**Cause**: Maturin not installed or old version.

**Solution**:
```bash
# Install/update maturin via uv (recommended)
uv pip install --upgrade maturin

# Or via pip
pip install --upgrade maturin

# Verify installation
maturin --version  # Should be 1.0+
```

#### Changes not reflected after rebuild

**Cause**: Python caching or virtual environment issues.

**Solution**:
```bash
# Force rebuild
maturin develop --force

# Or recreate virtual environment
deactivate
rm -rf .venv
python3 -m venv .venv
source .venv/bin/activate
uv pip install maturin pytest
export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1
maturin develop
```

---

## Common Error Messages

### "Error: Memory not found: mem_abc123"

**Meaning**: Trying to access a memory that doesn't exist or was deleted.

**Solution**: Verify ID is correct and memory wasn't archived:
```bash
mnemosyne list --namespace "project:myproject" --format json | \
  jq -r '.results[] | .id'  # List all valid IDs
```

### "Error: Namespace detection failed"

**Meaning**: Not in a git repository and no explicit namespace provided.

**Solution**: Either run from git repository or specify namespace:
```bash
cd /path/to/git/repo
# OR
mnemosyne remember --content "Test" --namespace "global" --format json
```

### "Error: LLM enrichment failed: rate limit exceeded"

**Meaning**: Hit Anthropic API rate limit.

**Solution**: Wait and retry, or temporarily disable LLM features:
```bash
# Wait for rate limit reset (usually 60 seconds)
sleep 60

# Retry operation
mnemosyne remember --content "Test memory" --format json

# For bulk operations, add delays between requests
```

### "Error: Failed to parse MCP response"

**Meaning**: Communication issue between Claude Code and Mnemosyne MCP server.

**Solution**: Restart both components:
```bash
# Kill existing mnemosyne processes
pkill -f mnemosyne

# Restart Claude Code completely

# Test MCP server manually
echo '{"jsonrpc":"2.0","method":"initialize","id":1}' | mnemosyne serve

# Should return valid JSON, not an error
```

---

## Debugging Tools

### Enable Debug Logging

```bash
# Set log level via environment variable
export RUST_LOG=debug

# Or use CLI flag
mnemosyne --log-level debug serve

# For specific modules only
export RUST_LOG=mnemosyne_core::storage=debug,mnemosyne_core::llm=debug

# View logs in real-time
mnemosyne --log-level debug serve 2>&1 | tee mnemosyne-debug.log
```

### Inspect Database Directly

```bash
# Open database in SQLite
sqlite3 ~/.local/share/mnemosyne/mnemosyne.db

# Useful queries:
.tables                                    # List all tables
SELECT COUNT(*) FROM memories;             # Count memories
SELECT * FROM memories LIMIT 5;            # View recent memories
SELECT namespace, COUNT(*) FROM memories   # Memories per namespace
  GROUP BY namespace;

# Check FTS5 index
SELECT * FROM memories_fts WHERE memories_fts MATCH 'test';

# Exit sqlite
.quit
```

### Test MCP Protocol Manually

```bash
# Test initialize
echo '{"jsonrpc":"2.0","method":"initialize","id":1}' | mnemosyne serve

# Test remember tool
echo '{
  "jsonrpc":"2.0",
  "method":"tools/call",
  "params":{
    "name":"mnemosyne.remember",
    "arguments":{
      "content":"Test memory",
      "namespace":"global"
    }
  },
  "id":2
}' | mnemosyne serve | jq .

# Test recall tool
echo '{
  "jsonrpc":"2.0",
  "method":"tools/call",
  "params":{
    "name":"mnemosyne.recall",
    "arguments":{
      "query":"test",
      "limit":5
    }
  },
  "id":3
}' | mnemosyne serve | jq .
```

### Check System Status

```bash
# Verify installation
which mnemosyne
mnemosyne --version  # (if implemented)

# Check configuration
mnemosyne config show-key  # Verify API key accessible

# Check database
ls -lh ~/.local/share/mnemosyne/mnemosyne.db
sqlite3 ~/.local/share/mnemosyne/mnemosyne.db "SELECT COUNT(*) FROM memories;"

# Check MCP config
cat ~/.claude/mcp_config.json | jq .

# List running processes
ps aux | grep mnemosyne
```

### Collect Diagnostic Information

When filing a bug report, include:

```bash
# System information
uname -a
rustc --version
python3 --version

# Mnemosyne version
git -C /path/to/mnemosyne log -1 --oneline

# Database info
ls -lh ~/.local/share/mnemosyne/mnemosyne.db
sqlite3 ~/.local/share/mnemosyne/mnemosyne.db \
  "SELECT COUNT(*) as total_memories,
          COUNT(DISTINCT namespace) as namespaces,
          MAX(created_at) as latest_memory
   FROM memories;"

# Environment
echo "RUST_LOG=$RUST_LOG"
echo "ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY:+SET}"
echo "MNEMOSYNE_DB_PATH=$MNEMOSYNE_DB_PATH"
echo "PATH=$PATH"

# Recent logs (if available)
tail -100 mnemosyne-debug.log
```

---

## Getting Help

If you can't resolve the issue with this guide:

### 1. Search Existing Issues
Check if someone else has encountered this problem:
- [GitHub Issues](https://github.com/rand/mnemosyne/issues)
- Search closed issues too (may already be fixed)

### 2. Check Documentation
- [Installation Guide](INSTALL.md) - Setup instructions
- [Architecture](ARCHITECTURE.md) - System internals
- [MCP Server API](MCP_SERVER.md) - Tool documentation
- [Secrets Management](SECRETS_MANAGEMENT.md) - API key setup

### 3. Ask the Community
- [GitHub Discussions](https://github.com/rand/mnemosyne/discussions)
- Provide diagnostic information (see above)
- Describe what you tried
- Include error messages (full output)

### 4. File a Bug Report
If you've found a bug:
- [Create an Issue](https://github.com/rand/mnemosyne/issues/new)
- Use the bug report template
- Include diagnostic information
- Describe expected vs actual behavior
- Provide steps to reproduce

### 5. Contact Maintainers
For sensitive issues or private questions:
- Email: rand.arete@gmail.com
- Response time: Usually within 48 hours

---

## Quick Reference

### Reset Everything
```bash
# Stop all mnemosyne processes
pkill -f mnemosyne

# Remove database
rm ~/.local/share/mnemosyne/mnemosyne.db

# Remove secrets
rm ~/.config/mnemosyne/secrets.age

# Remove MCP config
rm ~/.claude/mcp_config.json

# Reinstall from scratch
./scripts/install/install.sh
```

### Clean Reinstall
```bash
# Uninstall (preserves data by default)
./scripts/install/uninstall.sh

# Or uninstall with data purge
./scripts/install/uninstall.sh --purge

# Reinstall
./scripts/install/install.sh
```

### Test Installation
```bash
# 1. Binary works
mnemosyne --help

# 2. Database accessible
mnemosyne init
mnemosyne list --format json

# 3. API key configured
mnemosyne config show-key

# 4. MCP server starts
echo '{"jsonrpc":"2.0","method":"initialize","id":1}' | mnemosyne serve

# 5. Can store and retrieve
mnemosyne remember --content "Install test" --namespace "global" --format json
mnemosyne recall --query "install" --format json
```

---

**Last Updated**: 2025-10-27
**Version**: 1.0.0
