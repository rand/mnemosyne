# Common Workflows

This guide covers practical usage patterns for Mnemosyne in real-world development scenarios.

## Table of Contents

- [Daily Development Session](#daily-development-session)
- [Debugging Recurring Issues](#debugging-recurring-issues)
- [Team Knowledge Sharing](#team-knowledge-sharing)
- [Refactoring with Context](#refactoring-with-context)
- [CI/CD Integration](#cicd-integration)
- [Code Review Enhancement](#code-review-enhancement)
- [Architecture Decision Tracking](#architecture-decision-tracking)

---

## Daily Development Session

### Workflow Overview

A typical development session with automatic memory capture.

### Step 1: Session Start (Automatic)

When you open Claude Code in a project, the `session-start` hook automatically loads context:

```bash
# Hook runs automatically:
# .claude/hooks/session-start.sh

# Loads recent important memories for your project
# Example output in Claude Code:
```

**Claude Code will show:**
```markdown
# Project Memory Context

**Project**: mnemosyne
**Namespace**: project:mnemosyne

## Storage decision: LibSQL for vector search
**Importance**: 8/10
**Tags**: storage, libsql, architecture

Decided to use LibSQL instead of plain SQLite for native vector support...

---

## PyO3 bindings provide 10-20x speedup
**Importance**: 9/10
**Tags**: performance, pyo3, rust

Implemented PyO3 bindings for critical operations...
```

### Step 2: Work on Features

As you develop, store key decisions:

```
/memory-store Implemented rate limiting using token bucket algorithm.
Max 100 requests per minute per user.
```

Or let agents store automatically:
```
You: "Why did we choose LibSQL over PostgreSQL?"
Claude: [Searches memories, finds decision, explains rationale]
```

### Step 3: Commit Code (Automatic Link)

When you commit, the `post-commit` hook automatically:
- Detects architectural commits (keywords: architecture, implement, refactor, migrate)
- Creates memory linking commit to decisions
- Stores context for future reference

```bash
git add .
git commit -m "Implement rate limiting with token bucket algorithm"

# Hook automatically creates memory linking commit to rate limiting decision
```

### Step 4: Session End Review

Before ending your session, consolidate memories:

```
/memory-consolidate
```

This reviews similar memories and suggests merges or supersedes to avoid duplication.

---

## Debugging Recurring Issues

###  Workflow: Track and Prevent Bug Recurrence

### Problem

You encounter the same bug pattern across multiple features.

### Solution Workflow

#### 1. Store Initial Bug Discovery

```bash
mnemosyne remember \
  --content "Bug: Race condition when updating user profile concurrently.
             Root cause: Missing transaction isolation.
             Fix: Wrap update in serializable transaction." \
  --importance 8 \
  --namespace "project:myapp" \
  --tags "bug,concurrency,database" \
  --format json
```

#### 2. Store the Pattern

```bash
mnemosyne remember \
  --content "Pattern: All concurrent updates to shared state need transaction isolation.
             Always wrap in BEGIN TRANSACTION SERIALIZABLE...COMMIT." \
  --importance 9 \
  --namespace "project:myapp" \
  --tags "pattern,concurrency,best-practice" \
  --format json
```

#### 3. Link Related Memories

The LLM will automatically link these memories. Verify:

```bash
mnemosyne graph --memory-id "mem_bug_id" --depth 2 --format json
```

#### 4. Future Prevention

Next time someone works on concurrent updates:

```
Developer: "I need to update user profiles from multiple threads"
Claude: [Searches memories, finds pattern]
Claude: "Based on past experience (memory link), you should wrap this in
        a serializable transaction to avoid the race condition we hit before."
```

#### 5. Document Resolution

After fixing:

```bash
mnemosyne update \
  --id "mem_bug_id" \
  --metadata '{"status": "resolved", "fix_commit": "abc123"}' \
  --format json
```

---

## Team Knowledge Sharing

### Workflow: Onboard New Team Members

### Export Project Knowledge

```bash
# Export all project memories to markdown
mnemosyne export \
  --output onboarding-guide.md \
  --namespace "project:myapp" \
  --format markdown

# Filter by importance for essentials only
mnemosyne recall \
  --query "architecture decision" \
  --namespace "project:myapp" \
  --min-importance 8 \
  --limit 20 \
  --format json > key-decisions.json
```

### Share Context Across Team

**Option A: Shared Database (Turso)**
```bash
# Use remote Turso database for team sharing
# Configure in .env:
TURSO_DATABASE_URL=libsql://your-db.turso.io
TURSO_AUTH_TOKEN=your-token

# All team members query same knowledge base
```

**Option B: Export/Import**
```bash
# Team lead exports memories
mnemosyne export --output team-context.md --namespace "project:myapp"

# New team member imports context
# (Import feature planned for v2.0)
```

### Create Shared Patterns

```bash
# Team agrees on patterns, stores them
mnemosyne remember \
  --content "Team Convention: All API endpoints must have rate limiting.
             Use token bucket algorithm with 100 req/min default." \
  --importance 9 \
  --namespace "project:myapp" \
  --tags "convention,api,rate-limiting" \
  --format json
```

Now all team members' AI assistants know the convention.

---

## Refactoring with Context

### Workflow: Safe Refactoring Using Historical Context

### Scenario

You need to refactor the authentication system.

### Step 1: Recall Past Decisions

```bash
mnemosyne recall \
  --query "authentication decision security" \
  --namespace "project:myapp" \
  --min-importance 7 \
  --format json
```

**Results might show:**
- Why JWT was chosen over sessions
- Security constraints (HTTPS only, short expiry)
- Known pitfalls (token refresh edge cases)

### Step 2: Check Related Systems

```bash
# Get full context graph
mnemosyne context \
  --namespace "project:myapp" \
  --keywords "authentication authorization api" \
  --limit 15 \
  --format json
```

This shows:
- Which APIs depend on current auth
- Integration points
- Past bugs related to auth

### Step 3: Store Refactoring Plan

```bash
mnemosyne remember \
  --content "Refactoring Plan: Migrate from JWT to session-based auth.
             Rationale: Simplify token refresh complexity.
             Migration: Add session table, gradual rollout with feature flag.
             Risk: Ensure CSRF protection added." \
  --importance 9 \
  --namespace "project:myapp" \
  --tags "refactoring,authentication,plan" \
  --format json
```

### Step 4: Document Progress

As you refactor, update memories:

```bash
mnemosyne update \
  --id "mem_refactor_plan_id" \
  --content "Status: 60% complete. Session table added, API migrated,
             frontend still using JWT." \
  --format json
```

### Step 5: Link Commit to Decision

```bash
git commit -m "Migrate authentication to session-based approach

Refs: mem_refactor_plan_id
Risk mitigation: Added CSRF protection, gradual rollout via feature flag"

# Post-commit hook automatically links this commit to the refactoring plan memory
```

---

## CI/CD Integration

### Workflow: Automated Testing with Memory Context

### Setup Environment Variables

```yaml
# .github/workflows/test.yml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Build Mnemosyne
        run: cargo build --release

      - name: Run Tests
        env:
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
          MNEMOSYNE_DB_PATH: /tmp/mnemosyne-test.db
        run: |
          ./target/release/mnemosyne init
          cargo test
```

### Store Build Failures

```bash
# In CI script, capture failures
if ! cargo test; then
  # Store failure context
  ./target/release/mnemosyne remember \
    --content "Build failure on commit $GITHUB_SHA.
               Branch: $GITHUB_REF. Test failures: $(cat test-output.log)" \
    --importance 7 \
    --namespace "ci:myapp" \
    --tags "ci,failure,test" \
    --format json
fi
```

### Track Deployment Decisions

```bash
# After successful deployment
mnemosyne remember \
  --content "Deployed v2.3.0 to production.
             New features: Rate limiting, async processing.
             Rollback plan: Revert to v2.2.5 if errors spike." \
  --importance 8 \
  --namespace "deployments:myapp" \
  --tags "deployment,production,v2.3.0" \
  --format json
```

### Query Deployment History

```bash
# Find last successful deployment
mnemosyne recall \
  --query "deployed production successful" \
  --namespace "deployments:myapp" \
  --limit 5 \
  --format json
```

---

## Code Review Enhancement

### Workflow: Context-Aware Code Reviews

### Before Review: Load Context

```bash
# Reviewer loads project context
mnemosyne context \
  --namespace "project:myapp" \
  --keywords "architecture conventions patterns" \
  --limit 10 \
  --format json
```

### During Review: Check Against Patterns

```
Reviewer (using Claude): "Does this PR follow our error handling conventions?"
Claude: [Searches memories for "error handling convention"]
Claude: "According to team pattern (mem_xyz), all errors should be wrapped in
         Result<T, AppError>. This PR uses unwrap() in 3 places."
```

### Store Review Insights

```bash
mnemosyne remember \
  --content "Code Review Insight: PR #123 revealed we're inconsistent with
             async/await patterns. Some functions use blocking I/O.
             Action: Create linting rule to prevent blocking calls in async context." \
  --importance 7 \
  --namespace "project:myapp" \
  --tags "code-review,async,quality" \
  --format json
```

---

## Architecture Decision Tracking

### Workflow: Document and Track ADRs

### Store Architecture Decision

```bash
mnemosyne remember \
  --content "ADR-005: Use Event Sourcing for Order Management

## Context
Need audit trail and ability to replay events for debugging.

## Decision
Implement event sourcing with PostgreSQL event store.

## Consequences
Positive: Complete audit trail, easy debugging, temporal queries
Negative: Higher complexity, eventual consistency challenges
Mitigation: Use sagas for cross-aggregate transactions

## Alternatives Considered
1. CRUD with audit log - Rejected: Limited replay capability
2. Change Data Capture - Rejected: Vendor lock-in to specific DB" \
  --importance 10 \
  --namespace "project:myapp" \
  --tags "adr,architecture,event-sourcing,decision" \
  --format json
```

### Link to Implementation

```bash
git commit -m "Implement event sourcing for orders

Implements ADR-005: Event sourcing for order management
See memory mem_adr_005 for full decision context"

# Post-commit hook links commit to ADR memory
```

### Review Past ADRs

```bash
# List all architecture decisions
mnemosyne recall \
  --query "ADR architecture decision" \
  --namespace "project:myapp" \
  --min-importance 9 \
  --format json

# Get specific ADR
mnemosyne recall \
  --query "ADR-005 event sourcing" \
  --namespace "project:myapp" \
  --format json
```

### Update ADR Status

```bash
mnemosyne update \
  --id "mem_adr_005" \
  --content "ADR-005: Use Event Sourcing (SUPERSEDED by ADR-012)

Status: Superseded
Reason: Event sourcing added too much complexity for our scale.
New approach: CRUD with change data capture (ADR-012)" \
  --format json
```

---

## Best Practices

### Memory Hygiene

1. **Be Specific**: Good memories have concrete details
   ```bash
   # Good
   "Rate limiting: 100 req/min using token bucket. Rejected sliding window due to memory overhead."

   # Bad
   "Added rate limiting"
   ```

2. **Set Appropriate Importance**:
   - 9-10: Critical architecture decisions
   - 7-8: Important patterns and conventions
   - 5-6: Useful context and discoveries
   - 3-4: Minor decisions and notes
   - 1-2: Temporary reminders

3. **Use Meaningful Tags**:
   ```bash
   # Good tags
   --tags "performance,caching,redis,optimization"

   # Bad tags
   --tags "stuff,important,todo"
   ```

4. **Review and Consolidate Weekly**:
   ```bash
   # Check for duplicate or outdated memories
   mnemosyne consolidate --namespace "project:myapp"
   ```

### Namespace Strategy

```bash
# Global conventions (cross-project)
--namespace "global"

# Project-specific (auto-detected from git)
--namespace "project:myapp"

# Feature-specific (temporary, delete after shipped)
--namespace "feature:new-auth-system"

# CI/CD and deployments
--namespace "ci:myapp"
--namespace "deployments:myapp"
```

### Integration Tips

1. **Train Your Team**: Share this guide, establish conventions
2. **Use Hooks**: Enable automatic capture for zero-friction memory
3. **Regular Reviews**: Weekly consolidation prevents memory bloat
4. **Export Important Decisions**: Keep markdown backups
5. **Link Commits**: Reference memory IDs in commit messages

---

## Workflow Templates

### Bug Report Template

```bash
mnemosyne remember \
  --content "Bug: [Title]

Symptom: [What went wrong]
Root Cause: [Why it happened]
Fix: [How it was resolved]
Prevention: [How to avoid in future]
Related: [Links to similar bugs]" \
  --importance 7 \
  --tags "bug,[feature],[subsystem]" \
  --format json
```

### Pattern Template

```bash
mnemosyne remember \
  --content "Pattern: [Name]

Problem: [What problem does this solve]
Solution: [How to apply this pattern]
Example: [Code example or scenario]
When to Use: [Applicability]
When Not to Use: [Anti-patterns]" \
  --importance 8 \
  --tags "pattern,[domain]" \
  --format json
```

### Decision Template

```bash
mnemosyne remember \
  --content "Decision: [Title]

Context: [Why we needed to decide]
Decision: [What we chose]
Rationale: [Why we chose it]
Alternatives: [What else we considered]
Consequences: [Trade-offs]
Review Date: [When to revisit]" \
  --importance 9 \
  --tags "decision,architecture" \
  --format json
```

---

## Troubleshooting Workflows

### "I can't find a memory I know I stored"

```bash
# Search without namespace filter
mnemosyne recall --query "your search" --format json

# List all namespaces
sqlite3 ~/.local/share/mnemosyne/mnemosyne.db \
  "SELECT DISTINCT namespace, COUNT(*) FROM memories GROUP BY namespace;"

# Search specific namespace
mnemosyne recall --query "your search" --namespace "project:myapp" --format json
```

### "Too many irrelevant search results"

```bash
# Increase minimum importance
mnemosyne recall --query "your search" --min-importance 7

# Use more specific query
mnemosyne recall --query "specific technical terms" --limit 5

# Filter by namespace
mnemosyne recall --query "your search" --namespace "project:myapp"
```

### "Memories are getting stale"

```bash
# Review and update old memories
mnemosyne list --namespace "project:myapp" --sort importance --limit 20 | \
  jq -r '.results[] | select(.importance < 5) | .id'

# Delete low-importance old memories
# (Manual review recommended)
```

---

## Additional Resources

- [MCP API Reference](../../MCP_SERVER.md) - Complete tool documentation
- [Hooks Testing Guide](../../HOOKS_TESTING.md) - Automatic memory capture
- [Troubleshooting Guide](../../TROUBLESHOOTING.md) - Common issues
- [Architecture Documentation](../../ARCHITECTURE.md) - System internals

---

**Last Updated**: 2025-10-27
**Version**: 1.0.0
