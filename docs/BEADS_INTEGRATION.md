# Beads Integration with SpecFlow

**Status**: Integration Complete
**Date**: 2025-11-02

---

## Overview

This document describes the integration between Beads task tracking and the SpecFlow specification workflow, enabling seamless conversion from feature specifications to executable, trackable tasks.

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
│  scripts/beads-sync │  Export & commit state
└──────────┬──────────┘
           ↓
┌─────────────────────┐
│  .beads/issues.jsonl│  Persistent task storage
└─────────────────────┘
```

## Workflow

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
- Beads tasks created via `bd create`
- Task IDs appended to plan
- Dependencies configured with `--blocked-by`
- Summary displayed

### 5. Export and Commit

```bash
./scripts/beads-sync.sh commit
```

**Output**:
- Exports to `.beads/issues.jsonl`
- Commits to git with summary
- Displays task statistics

## Beads Sync Script

**Location**: `scripts/beads-sync.sh`

### Commands

```bash
# Export current state
./scripts/beads-sync.sh export

# Import from .jsonl
./scripts/beads-sync.sh import

# Export and commit to git
./scripts/beads-sync.sh commit

# Show sync status
./scripts/beads-sync.sh status
```

### Features

- **Export**: Saves Beads memory to `.beads/issues.jsonl`
- **Import**: Loads tasks from `.jsonl` into Beads
- **Commit**: Exports and commits with auto-generated message
- **Status**: Shows sync state and task summary

### Example Output

```
$ ./scripts/beads-sync.sh status

Beads Sync Status

✓ .beads/issues.jsonl tracked in git
✓ No uncommitted changes

Task Summary:
  Total: 33 tasks
  Open: 5
  In Progress: 1
  Closed: 27

Recent Activity:
  [in_progress] SF-7: Beads export integration
  [open] DS-3: Create OptimizerModule training data
  [open] SF-6: Implement /feature-checklist
```

## Task Format

Tasks created by `/feature-tasks` follow this structure:

```json
{
  "id": "mnemosyne-29",
  "title": "[jwt-auth] JWT Generation: RS256 signing with refresh tokens",
  "description": "## Goal\nImplement JWT token generation...\n\n## Acceptance Criteria\n- [ ] Generate RS256 signed tokens...",
  "status": "open",
  "priority": 0,
  "issue_type": "feature",
  "created_at": "2025-11-02T...",
  "updated_at": "2025-11-02T..."
}
```

## Integration Points

### /feature-tasks Enhancement

**At end of task creation**:

```markdown
11. **Export and commit**:
    - Run: `./scripts/beads-sync.sh commit`
    - This exports state and commits to git
    - User sees confirmation with task stats
```

### Session Workflow

**Start of session**:
```bash
# Import existing tasks
bd import -i .beads/issues.jsonl
```

**During session**:
```bash
# Create feature tasks
/feature-tasks jwt-auth

# Work on tasks
bd update bd-42 --status in_progress --json
bd update bd-42 --comment "Implemented schema" --json
bd close bd-42 --reason "Complete" --json
```

**End of session**:
```bash
# Export and commit
./scripts/beads-sync.sh commit
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

# Blocked by specific task
bd list --blocked-by mnemosyne-42 --json

# High priority open tasks
bd list --priority 0,1 --status open --json
```

### Bulk Updates

```bash
# Close all completed tasks
bd list --status in_progress --json | \
  jq -r '.[] | .id' | \
  xargs -I {} bd close {} --reason "Complete" --json
```

## Troubleshooting

### Issue: Tasks not appearing in Beads

**Solution**:
```bash
bd import -i .beads/issues.jsonl
```

### Issue: Uncommitted changes in .jsonl

**Solution**:
```bash
./scripts/beads-sync.sh commit
```

### Issue: Merge conflicts in .jsonl

**Solution**:
1. Resolve conflict in `.beads/issues.jsonl`
2. Import: `bd import -i .beads/issues.jsonl`
3. Export: `bd export -o .beads/issues.jsonl`
4. Commit: `git add .beads/issues.jsonl && git commit`

### Issue: Circular dependencies

**Symptom**: Tasks block each other in a cycle

**Solution**: Review plan dependencies, break cycle by:
1. Identify typed hole (interface)
2. Create interface definition task (no dependencies)
3. Make both sides depend on interface

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

- **2025-11-02**: Created Beads integration with sync script
- **TBD**: Add bidirectional sync (SF-8)
- **TBD**: Add analytics dashboard
