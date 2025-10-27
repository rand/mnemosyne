# Migration Guide

This guide helps you upgrade Mnemosyne between versions safely.

## Table of Contents

- [v0.1 → v1.0](#v01--v10)
- [v1.0 → v1.1](#v10--v11)
- [v1.x → v2.0 (Future)](#v1x--v20-future)
- [General Upgrade Process](#general-upgrade-process)
- [Rollback Procedures](#rollback-procedures)

---

## v0.1 → v1.0

### Overview

Version 1.0 is a major release with several breaking changes and improvements:
- SQLite → LibSQL migration (automatic)
- Keychain-only → Secure secrets system (age encryption)
- Fixed database path → Configurable XDG path
- Manual hooks → Improved hook binary detection

**Upgrade Difficulty**: Medium
**Data Migration**: Automatic
**Downtime**: < 5 minutes

### Breaking Changes

| Component | v0.1 | v1.0 | Migration Required |
|-----------|------|------|-------------------|
| Database | SQLite | LibSQL | Automatic |
| Secrets | Keychain only | Age→Keychain | Manual config |
| DB Path | `./mnemosyne.db` | `~/.local/share/mnemosyne/` | Automatic |
| Hooks | Basic | Installed binary detection | Automatic |

### Pre-Migration Checklist

```bash
# 1. Backup your data
cp mnemosyne.db mnemosyne-v0.1-backup.db

# 2. Export memories to markdown (optional but recommended)
mnemosyne export --output memories-backup-$(date +%Y%m%d).md

# 3. Note your current configuration
mnemosyne config show-key

# 4. Stop any running mnemosyne processes
pkill -f mnemosyne
```

### Migration Steps

#### Step 1: Update Code

```bash
cd /path/to/mnemosyne
git fetch origin
git checkout v1.0.0

# Or pull latest main
git pull origin main
```

#### Step 2: Rebuild

```bash
cargo clean
cargo build --release
```

#### Step 3: Migrate Database (Automatic)

The v1.0 LibSQL storage is backward compatible with v0.1 SQLite databases.

```bash
# If you had database at ./mnemosyne.db, move it to new location
mkdir -p ~/.local/share/mnemosyne
cp ./mnemosyne.db ~/.local/share/mnemosyne/mnemosyne.db

# Or use environment variable to specify custom path
export MNEMOSYNE_DB_PATH=./mnemosyne.db
```

LibSQL will automatically handle the SQLite database format.

#### Step 4: Migrate Secrets

**Old approach (v0.1)**: Direct keychain access only

**New approach (v1.0)**: Three-tier priority
1. Environment variable (highest priority)
2. Age-encrypted file (`~/.config/mnemosyne/secrets.age`)
3. OS keychain (fallback)

**Migration options:**

**Option A: Use Environment Variable (CI/CD, simplest)**
```bash
# Add to ~/.bashrc or ~/.zshrc
export ANTHROPIC_API_KEY=sk-ant-api03-YOUR_KEY

source ~/.bashrc
```

**Option B: Use Age Encryption (Recommended)**
```bash
# Initialize secrets system
mnemosyne secrets init

# When prompted, enter your API key
# This creates encrypted file: ~/.config/mnemosyne/secrets.age

# Remove old keychain entry (optional, will still work as fallback)
mnemosyne secrets delete ANTHROPIC_API_KEY --keychain-only
```

**Option C: Keep Using Keychain (Legacy)**
```bash
# No migration needed, keychain still works as fallback
# v1.0 will use existing keychain entry if no env var or age file exists
```

#### Step 5: Update Configuration Files

If you have custom configurations:

```bash
# Update MCP config (run installer to merge safely)
./scripts/install/install.sh --global-mcp

# This will merge your existing config, not overwrite
```

#### Step 6: Update Hooks

If you copied hooks to other projects:

```bash
# Hooks now auto-detect installed vs local binary
# Copy updated hooks from mnemosyne repo
cp /path/to/mnemosyne/.claude/hooks/* /path/to/your/project/.claude/hooks/

# Make executable
chmod +x /path/to/your/project/.claude/hooks/*.sh
```

#### Step 7: Verify Migration

```bash
# 1. Check binary works
mnemosyne --help

# 2. Check API key accessible
mnemosyne config show-key

# 3. List existing memories
mnemosyne list --limit 5 --format json

# 4. Test search
mnemosyne recall --query "test" --format json

# 5. Store new memory
mnemosyne remember --content "Migration to v1.0 successful" --importance 8 --format json

# 6. Test MCP server
echo '{"jsonrpc":"2.0","method":"initialize","id":1}' | mnemosyne serve
```

### Post-Migration Cleanup

```bash
# Optional: Remove old database from project root if moved
rm ./mnemosyne.db

# Optional: Remove old v0.1 build artifacts
rm -rf target/
cargo build --release
```

### Rollback to v0.1

If you encounter issues:

```bash
# 1. Stop v1.0 processes
pkill -f mnemosyne

# 2. Checkout v0.1
git checkout v0.1.0

# 3. Rebuild
cargo clean
cargo build --release

# 4. Restore database
cp mnemosyne-v0.1-backup.db ./mnemosyne.db

# 5. Secrets will continue working via keychain
```

---

## v1.0 → v1.1

### Overview

Version 1.1 is a cleanup release with no breaking changes.

**Upgrade Difficulty**: Easy
**Data Migration**: None
**Downtime**: < 1 minute

### Changes

- **P1-001**: Database path now configurable via CLI flag and env var
- **P2-001**: Removed old SQLite implementation (internal cleanup)
- **P2-004**: Improved hooks binary detection (no action needed)
- **P2-005**: Cleaned up .gitignore (no action needed)
- **P2-003**: Updated test documentation (no action needed)
- **P2-009**: Added .editorconfig (no action needed)

### Migration Steps

#### Step 1: Update Code

```bash
cd /path/to/mnemosyne
git pull origin main
```

#### Step 2: Rebuild

```bash
cargo build --release
```

#### Step 3: Install (Optional)

```bash
cargo install --path .
# Or
./scripts/install/install.sh
```

#### Step 4: Verify

```bash
# Verify new CLI flag works
mnemosyne --db-path /custom/path/mnemosyne.db init

# Verify environment variable works
export MNEMOSYNE_DB_PATH=/custom/path/mnemosyne.db
mnemosyne init
```

### New Features Available

```bash
# Custom database path via CLI
mnemosyne --db-path ~/my-project/mnemosyne.db serve

# Custom database path via environment variable
export MNEMOSYNE_DB_PATH=~/my-project/mnemosyne.db
mnemosyne serve

# Default remains XDG standard
# ~/.local/share/mnemosyne/mnemosyne.db
```

---

## v1.x → v2.0 (Future)

### Planned Changes

Version 2.0 will include major new features. This section will be updated as v2.0 development progresses.

### Expected Features

1. **Vector Similarity Search**
   - Remote embeddings via Voyage AI or Anthropic
   - Native vector search with sqlite-vec
   - Hybrid ranking (vector + keyword + graph)

2. **Background Memory Evolution**
   - Automatic consolidation (daily)
   - Link strength decay (weekly)
   - Importance recalibration (weekly)
   - Automatic archival (monthly)

3. **Advanced Agent Features**
   - Role-based memory views per agent
   - Agent-specific importance scoring
   - Memory prefetching for performance
   - Custom consolidation strategies

4. **VSCode Extension**
   - Memory browser UI
   - Visual graph exploration
   - Quick memory capture
   - Search integration

### Expected Migration Complexity

**Upgrade Difficulty**: Medium
**Data Migration**: Automatic (database schema updates)
**Downtime**: < 5 minutes

### Migration Plan (Tentative)

```bash
# 1. Backup
./scripts/backup.sh  # (New script in v2.0)

# 2. Update
git checkout v2.0.0

# 3. Rebuild
cargo build --release

# 4. Run migrations
mnemosyne migrate --from v1.x --to v2.0

# 5. Verify
mnemosyne verify
```

More details will be added as v2.0 development progresses. See [ROADMAP.md](../../ROADMAP.md) for timeline.

---

## General Upgrade Process

### Standard Upgrade Steps

For any version upgrade:

1. **Backup First**
   ```bash
   # Backup database
   cp ~/.local/share/mnemosyne/mnemosyne.db \
      ~/.local/share/mnemosyne/mnemosyne-backup-$(date +%Y%m%d).db

   # Export memories
   mnemosyne export --output memories-backup-$(date +%Y%m%d).md
   ```

2. **Read Changelog**
   ```bash
   cat CHANGELOG.md | less
   # Look for BREAKING CHANGES
   ```

3. **Check Migration Guide**
   ```bash
   # This file!
   cat docs/guides/migration.md | less
   ```

4. **Update Code**
   ```bash
   git fetch origin
   git checkout vX.Y.Z  # Or git pull origin main
   ```

5. **Rebuild**
   ```bash
   cargo clean
   cargo build --release
   ```

6. **Run Migrations** (if any)
   ```bash
   # Version-specific, see sections above
   ```

7. **Verify**
   ```bash
   mnemosyne --help
   mnemosyne config show-key
   mnemosyne list --limit 5 --format json
   ```

8. **Test Critical Paths**
   ```bash
   # Store
   mnemosyne remember --content "Upgrade test" --format json

   # Retrieve
   mnemosyne recall --query "upgrade" --format json

   # MCP
   echo '{"jsonrpc":"2.0","method":"initialize","id":1}' | mnemosyne serve
   ```

---

## Rollback Procedures

### Immediate Rollback

If you encounter critical issues right after upgrade:

```bash
# 1. Stop all mnemosyne processes
pkill -f mnemosyne

# 2. Checkout previous version
git checkout vOLD_VERSION

# 3. Rebuild
cargo clean
cargo build --release

# 4. Restore database backup
cp ~/.local/share/mnemosyne/mnemosyne-backup-YYYYMMDD.db \
   ~/.local/share/mnemosyne/mnemosyne.db

# 5. Reinstall
cargo install --path .

# 6. Verify
mnemosyne list --limit 5 --format json
```

### Gradual Rollback

If you discover issues later:

```bash
# 1. Export current state first (may have new data)
mnemosyne export --output memories-before-rollback-$(date +%Y%m%d).md

# 2. Merge with backup if needed
# (Manual process, compare timestamps)

# 3. Proceed with immediate rollback steps above
```

### Recovery from Failed Migration

If migration fails mid-process:

```bash
# 1. Don't panic - backups exist

# 2. Check what failed
cat migration-error.log

# 3. Restore from backup
cp ~/.local/share/mnemosyne/mnemosyne-backup-YYYYMMDD.db \
   ~/.local/share/mnemosyne/mnemosyne.db

# 4. Rollback to previous version (see above)

# 5. Report issue
# https://github.com/rand/mnemosyne/issues
```

---

## Database Compatibility Matrix

| Version | Database Format | Compatible With |
|---------|----------------|-----------------|
| v0.1.0  | SQLite 3.x     | v0.1.x, v1.x    |
| v1.0.0  | LibSQL (SQLite) | v0.1.x, v1.x    |
| v1.1.0  | LibSQL (SQLite) | v0.1.x, v1.x    |
| v2.0.0  | LibSQL + Vectors | v2.x (migration from v1.x available) |

### Forward Compatibility

- v0.1 databases work with v1.x (LibSQL is backward compatible)
- v1.x databases work with v1.x (same format)
- v2.x migration will be provided for v1.x databases

### Backward Compatibility

- v1.x can read v0.1 databases
- v2.x will be able to read v1.x databases (via migration)
- Downgrading may lose new features but preserves core data

---

## Secrets Migration Details

### Keychain → Age Encryption

If you want to migrate from keychain to age encryption:

```bash
# 1. Check current key storage
mnemosyne config show-key

# 2. Export key from keychain
export ANTHROPIC_API_KEY=$(security find-generic-password \
  -s "mnemosyne-memory-system" \
  -a "anthropic-api-key" \
  -w 2>/dev/null)

# 3. Initialize age encryption
mnemosyne secrets init
# When prompted, enter $ANTHROPIC_API_KEY value

# 4. Verify new system works
unset ANTHROPIC_API_KEY
mnemosyne config show-key
# Should show: "✓ API key accessible via age encrypted file"

# 5. Delete keychain entry (optional)
security delete-generic-password \
  -s "mnemosyne-memory-system" \
  -a "anthropic-api-key"
```

### Age Encryption → Environment Variable

```bash
# 1. Get key from age file
mnemosyne secrets get ANTHROPIC_API_KEY

# 2. Add to shell profile
echo 'export ANTHROPIC_API_KEY=sk-ant-api03-YOUR_KEY' >> ~/.zshrc

# 3. Reload
source ~/.zshrc

# 4. Verify priority
mnemosyne config show-key
# Should show: "✓ API key set via environment variable"

# 5. Delete age file (optional)
mnemosyne secrets delete ANTHROPIC_API_KEY
```

---

## Configuration File Migrations

### MCP Config Merging

The install script safely merges MCP configurations:

```bash
# Safe merge (preserves existing servers)
./scripts/install/install.sh --global-mcp

# Manual merge if needed
cat ~/.claude/mcp_config.json | jq '.mcpServers.mnemosyne = {
  "command": "mnemosyne",
  "args": ["serve"],
  "env": {"RUST_LOG": "info"}
}' > ~/.claude/mcp_config.json.new

mv ~/.claude/mcp_config.json.new ~/.claude/mcp_config.json
```

### Hooks Updates

Hooks are backward compatible. Just copy new versions:

```bash
# Backup old hooks
cp -r .claude/hooks .claude/hooks.backup

# Copy new hooks
cp /path/to/mnemosyne/.claude/hooks/* .claude/hooks/

# Make executable
chmod +x .claude/hooks/*.sh

# Test
.claude/hooks/session-start.sh
```

---

## Troubleshooting Migrations

### "Database is locked" after upgrade

```bash
# Kill old processes
pkill -f mnemosyne

# Wait a moment
sleep 2

# Retry
mnemosyne list --format json
```

### "No such table: memories" after migration

```bash
# Database not initialized or corrupted
# Restore backup
cp ~/.local/share/mnemosyne/mnemosyne-backup-YYYYMMDD.db \
   ~/.local/share/mnemosyne/mnemosyne.db

# Or re-initialize (loses data!)
rm ~/.local/share/mnemosyne/mnemosyne.db
mnemosyne init
```

### "API key not found" after secrets migration

```bash
# Check secrets system status
mnemosyne secrets list

# Reconfigure
mnemosyne secrets set ANTHROPIC_API_KEY

# Or use environment variable
export ANTHROPIC_API_KEY=sk-ant-api03-YOUR_KEY
```

### Version mismatch errors

```bash
# Ensure binary and library match
cargo clean
cargo build --release
cargo install --path .

# Verify installation
which mnemosyne
mnemosyne --version  # (if implemented)
```

---

## Getting Help with Migrations

If you encounter issues:

1. **Check Changelog**: [CHANGELOG.md](../../CHANGELOG.md)
2. **Search Issues**: [GitHub Issues](https://github.com/rand/mnemosyne/issues?q=migration)
3. **Ask Community**: [GitHub Discussions](https://github.com/rand/mnemosyne/discussions)
4. **File Bug**: [New Issue](https://github.com/rand/mnemosyne/issues/new) with:
   - Source version
   - Target version
   - Error message
   - Steps attempted
   - Database size and memory count

---

**Last Updated**: 2025-10-27
**Version**: 1.0.0
