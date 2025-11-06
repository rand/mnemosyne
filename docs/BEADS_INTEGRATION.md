# Beads Integration with SpecFlow

**Status**: Integration Complete (Updated for Beads v0.20.1+)
**Date**: 2025-11-02
**Updated**: 2025-11-06 (Auto-sync, hash IDs)

---

## Overview

This document describes the integration between Beads task tracking and the SpecFlow specification workflow, enabling seamless conversion from feature specifications to executable, trackable tasks.

**New in v0.20.1+**:
- **Auto-sync**: Automatic export/import with 5-second debounce
- **Hash-based IDs**: Collision-resistant IDs (bd-a1b2 instead of bd-1)
- **bd init**: One-command setup with git hooks
- **Dependency management**: bd dep with 4 relationship types
- **Templates**: Built-in templates for common task types

## Architecture

```
┌─────────────────────┐
│  /feature-specify   │  Create feature spec
└──────────┬──────────┘
           ↓
┌─────────────────────┐
│  /feature-clarify   │  Resolve ambiguities
└──────────┬──────────┘
           ↓
┌─────────────────────┐
│  /feature-plan      │  Generate implementation plan
└──────────┬──────────┘
           ↓
┌─────────────────────┐
│  /feature-tasks     │  Break down into Beads tasks
└──────────┬──────────┘
           ↓
┌─────────────────────┐
│  bd create/update   │  Create tasks (auto-synced)
└──────────┬──────────┘
           ↓
┌─────────────────────┐
│  Auto-Sync (5s)     │  Automatic export to .jsonl
└──────────┬──────────┘
           ↓
┌─────────────────────┐
│  .beads/issues.jsonl│  Persistent task storage
└──────────┬──────────┘
           ↓
┌─────────────────────┐
│  scripts/beads-sync │  Git commit (optional)
└─────────────────────┘
```

## Workflow

### 0. One-Time Setup (First Time Only)

```bash
# Install Beads
npm install -g @beads/bd
# OR: curl -fsSL https://raw.githubusercontent.com/steveyegge/beads/main/scripts/install.sh | bash
# OR: brew tap steveyegge/beads && brew install bd

# Initialize Beads for this project
bd init

# What bd init does:
#  - Creates .beads/ directory
#  - Imports existing issues from .beads/issues.jsonl (if found)
#  - Installs git hooks
#  - Starts auto-sync daemon (5-second debounce)
```

### 1. Specify Feature

```bash
/feature-specify JWT authentication for API endpoints
```

**Output**: `.mnemosyne/artifacts/specs/jwt-auth.md`

### 2. Clarify Ambiguities (Optional)

```bash
/feature-clarify jwt-auth --auto
```

**Output**: Updated spec with resolved ambiguities

### 3. Create Implementation Plan

```bash
/feature-plan jwt-auth
```

**Output**: `.mnemosyne/artifacts/plans/jwt-auth-plan.md`

### 4. Generate Beads Tasks

```bash
/feature-tasks jwt-auth
```

**Output**:
- Beads tasks created via `bd create` (hash IDs: bd-a1b2, bd-c3d4)
- Task IDs appended to plan
- Dependencies configured with `bd dep add`
- **Auto-synced** to `.beads/issues.jsonl` (5-second debounce)
- Summary displayed

### 5. Commit to Git (Optional)

```bash
./scripts/beads-sync.sh commit
```

**Output**:
- Auto-sync already exported to `.beads/issues.jsonl`
- Commits to git with auto-generated summary
- Displays task statistics

**Note**: Manual export/import no longer needed with auto-sync!

## Beads Sync Script

**Location**: `scripts/beads-sync.sh`

### Commands

```bash
# One-time setup (bd init)
./scripts/beads-sync.sh setup

# Force immediate sync (rarely needed with auto-sync)
./scripts/beads-sync.sh sync

# Migrate to hash-based IDs
./scripts/beads-sync.sh migrate

# Commit auto-synced state to git
./scripts/beads-sync.sh commit

# Show sync status and diagnostics
./scripts/beads-sync.sh status
```

### Features

- **Setup**: Guides you through `bd init` one-time initialization
- **Sync**: Forces immediate sync (rarely needed, auto-sync handles it)
- **Migrate**: Converts sequential IDs (bd-1) to hash IDs (bd-a1b2)
- **Commit**: Commits auto-synced `.jsonl` to git with summary
- **Status**: Shows sync state, task summary, ID format, diagnostics

### Example Output

```
$ ./scripts/beads-sync.sh status

Beads Status & Diagnostics

Installation:
Database path: /Users/rand/src/mnemosyne/.beads/mnemosyne.db
Daemon status: running

Sync Status:
✓ .beads/issues.jsonl tracked in git
✓ No uncommitted changes (auto-sync working)

Task Summary (from .beads/issues.jsonl):
  Total: 33 tasks
  Open: 5
  In Progress: 1
  Closed: 27

Recent Activity:
  [in_progress] bd-a3f8: JWT Generation implementation
  [open] bd-c2d4: Create OptimizerModule training data
  [open] bd-e1b9: Implement /feature-checklist
  [open] bd-f7a2: Add dependency visualization
  [open] bd-b4c3: Update documentation

ID Format:
✓ Using hash-based IDs (modern format)
```

## Task Format

Tasks created by `/feature-tasks` follow this structure:

```json
{
  "id": "bd-a3f8e9",
  "title": "[jwt-auth] JWT Generation: RS256 signing with refresh tokens",
  "description": "## Goal\nImplement JWT token generation...\n\n## Acceptance Criteria\n- [ ] Generate RS256 signed tokens...",
  "status": "open",
  "priority": 0,
  "issue_type": "feature",
  "labels": ["feature:jwt-auth", "plan:1.0.0", "auth"],
  "created_at": "2025-11-02T...",
  "updated_at": "2025-11-02T..."
}
```

## Integration Points

### /feature-tasks Enhancement

**At end of task creation**:

```markdown
11. **Auto-sync and commit**:
    - Tasks auto-synced to .beads/issues.jsonl (5-second debounce)
    - Optional: `./scripts/beads-sync.sh commit` to commit to git
    - User sees confirmation with task stats and hash IDs
```

### Session Workflow

**Start of session** (one-time setup):
```bash
# First time only: Install and initialize
npm install -g @beads/bd
bd init
```

**Start of session** (regular):
```bash
# Check status and ready tasks
bd doctor
bd ready --json --limit 5
```

**During session**:
```bash
# Create feature tasks (auto-synced)
/feature-tasks jwt-auth

# Work on tasks (auto-synced after each command)
bd update bd-a1b2 --status in_progress --json
bd update bd-a1b2 --comment "Implemented schema" --json
bd close bd-a1b2 --reason "Complete" --json

# Add dependencies
bd dep add bd-c3d4 bd-a1b2 --type blocks

# Check progress
bd list --label feature:jwt-auth --json
```

**End of session**:
```bash
# Auto-sync already saved to .beads/issues.jsonl
# Just commit to git
./scripts/beads-sync.sh commit

# Or manually:
git add .beads/issues.jsonl
git commit -m "[Beads] Session work complete"
```

## Benefits

### 1. Traceability

```
Feature Spec → Implementation Plan → Beads Tasks → Git History
```

Every task traces back to:
- Original feature spec (via feature-id tag)
- Implementation plan (via plan version tag)
- Git commits (via .beads/issues.jsonl history)

### 2. Collaboration

- `.beads/issues.jsonl` in git enables team collaboration
- Pull/merge resolves task state conflicts
- History shows who worked on what

### 3. Progress Tracking

```bash
# View all tasks for a feature
bd list --tags feature:jwt-auth --json

# View critical path tasks
bd list --priority 0 --status open --json

# View blocked tasks
bd list --status blocked --json
```

### 4. Automation

- Auto-generate commit messages with task summaries
- Track task completion rates
- Measure velocity (tasks/sprint)
- Identify bottlenecks (long-running in_progress)

## Advanced Usage

### Task Dependencies

```bash
# Create foundation task
bd create "Database Schema" -t feature -p 0 --json

# Create dependent task
bd create "JWT Generation" -t feature -p 0 \
  --blocked-by mnemosyne-42 --json
```

### Task Queries

```bash
# Ready to work (no blockers)
bd ready --json --limit 5

# High priority open tasks
bd list --priority-min 0 --priority-max 1 --status open --json

# Filter by labels (AND)
bd list --label feature:jwt-auth,auth --json

# Filter by labels (OR)
bd list --label-any frontend,backend --json

# Date-based queries
bd list --created-after 2024-01-01 --json
bd list --updated-before 2024-12-31 --json

# Unassigned tasks
bd list --no-assignee --json

# Full details of specific task
bd show bd-a1b2
```

### Bulk Updates

```bash
# Close all completed tasks
bd list --status in_progress --json | \
  jq -r '.[] | .id' | \
  xargs -I {} bd close {} --reason "Complete" --json
```

## Troubleshooting

### Issue: Beads not initialized

**Symptom**: Commands fail with "cannot connect to daemon"

**Solution**:
```bash
bd init
# OR: ./scripts/beads-sync.sh setup
```

### Issue: Auto-sync not working

**Symptom**: Changes not appearing in `.beads/issues.jsonl`

**Solution**:
```bash
# Check daemon status
bd info

# Force sync
bd sync

# Restart daemon if needed
pkill -f "bd daemon"
bd doctor
```

### Issue: Uncommitted changes in .jsonl

**Symptom**: Git shows uncommitted changes in `.beads/issues.jsonl`

**Solution** (this is expected with auto-sync):
```bash
# Auto-sync exported changes - just commit them
./scripts/beads-sync.sh commit
```

### Issue: Merge conflicts in .jsonl

**Solution**:
1. Resolve conflict in `.beads/issues.jsonl`
2. Auto-sync will re-import automatically
3. Or force: `bd sync`
4. Commit: `git add .beads/issues.jsonl && git commit`

### Issue: Circular dependencies

**Symptom**: Tasks block each other in a cycle

**Solution**: Review plan dependencies, break cycle by:
1. Identify typed hole (interface)
2. Create interface definition task (no dependencies)
3. Make both sides depend on interface using `bd dep add`

### Issue: Using old sequential IDs

**Symptom**: Task IDs like bd-1, bd-2 instead of bd-a1b2

**Solution**:
```bash
# Migrate to hash-based IDs
./scripts/beads-sync.sh migrate
# OR: bd migrate
```

## Future Enhancements

### Phase 4: Bidirectional Sync (SF-8)

- Real-time sync between Beads and plan
- Update plan when tasks change
- Propagate task completion to spec status
- Track actual vs estimated effort

### Phase 5: Analytics

- Burndown charts (tasks remaining over time)
- Velocity tracking (tasks/week)
- Bottleneck detection (long-running tasks)
- Dependency visualization (task graph)

## References

- [Beads](https://github.com/steveyegge/beads) - Official repository
- [/feature-tasks](../.claude/commands/feature-tasks.md) - Task generation command
- [Work Plan Protocol](../CLAUDE.md) - Development workflow
- [Sprint Planning](./SPRINT_PLANNING.md) - Sprint structure

---

## Changelog

- **2025-11-06**: Updated for Beads v0.20.1+
  - Auto-sync replaces manual export/import
  - Hash-based IDs (bd-a1b2) replace sequential IDs
  - Added bd init one-time setup
  - Added dependency management (bd dep)
  - Added advanced filtering and queries
  - Updated all examples and workflows
- **2025-11-02**: Created Beads integration with sync script
- **TBD**: Add bidirectional sync (SF-8)
- **TBD**: Add analytics dashboard
