# Slash Commands Reference

Quick reference for Mnemosyne slash commands in Claude Code.

## Table of Contents

- [Setup](#setup)
- [Basic Commands](#basic-commands)
- [Advanced Commands](#advanced-commands)
- [Command Composition](#command-composition)
- [Custom Slash Commands](#custom-slash-commands)
- [Best Practices](#best-practices)

---

## Setup

Slash commands are automatically available in Claude Code when Mnemosyne MCP server is configured.

**Verify slash commands work:**

```
/memory-store Test memory for slash commands
```

If you see an error, check [Claude Code Setup](claude-code-setup.md) first.

---

## Basic Commands

### `/memory-store` - Store a Memory

Store important information for future recall.

**Syntax:**
```
/memory-store <content> [--importance N] [--namespace NS] [--tags tag1,tag2]
```

**Examples:**

```
/memory-store We decided to use PostgreSQL for the main database

/memory-store Authentication uses JWT tokens with 1-hour expiry --importance 8

/memory-store Bug fix: Race condition in cache invalidation --tags bug,cache,fix

/memory-store Project uses React 18 with TypeScript --namespace project:myapp
```

**Options:**
- `--importance N`: Importance score 1-10 (default: 5)
- `--namespace NS`: Namespace for organization (default: current project)
- `--tags tag1,tag2`: Comma-separated tags

---

### `/memory-recall` - Search Memories

Search for relevant memories using natural language.

**Syntax:**
```
/memory-recall <query> [--limit N] [--min-importance N] [--namespace NS]
```

**Examples:**

```
/memory-recall database decisions

/memory-recall authentication bug --min-importance 7

/memory-recall architecture decisions --namespace project:myapp --limit 5

/memory-recall what did we decide about caching?
```

**Options:**
- `--limit N`: Maximum results (default: 10)
- `--min-importance N`: Filter by importance (1-10)
- `--namespace NS`: Search specific namespace
- `--all-namespaces`: Search across all namespaces

---

### `/memory-update` - Update a Memory

Modify an existing memory.

**Syntax:**
```
/memory-update <memory-id> <new-content> [--importance N] [--tags tag1,tag2]
```

**Examples:**

```
/memory-update mem_abc123 Updated decision: Using PostgreSQL 15 with connection pooling

/memory-update mem_xyz789 --importance 9

/memory-update mem_def456 --tags critical,security,auth
```

**Finding memory IDs:**
```
/memory-recall recent decisions --format json
# Copy the "id" field from results
```

---

### `/memory-delete` - Delete a Memory

Remove a memory from the database.

**Syntax:**
```
/memory-delete <memory-id>
```

**Example:**

```
/memory-delete mem_abc123
```

**⚠️ Warning:** This action cannot be undone. Consider exporting first.

---

## Advanced Commands

### `/memory-graph` - Traverse Memory Graph

Explore related memories using semantic links.

**Syntax:**
```
/memory-graph <memory-id> [--depth N] [--min-importance N]
```

**Examples:**

```
/memory-graph mem_abc123

/memory-graph mem_abc123 --depth 2

/memory-graph mem_abc123 --min-importance 6 --depth 3
```

**Options:**
- `--depth N`: Traversal depth (default: 1)
- `--min-importance N`: Filter nodes by importance

**Use cases:**
- Find all decisions related to authentication
- Explore bug fix history
- Trace architectural evolution

---

### `/memory-export` - Export Memories

Export memories to a file for documentation or sharing.

**Syntax:**
```
/memory-export <output-file> [--namespace NS] [--min-importance N] [--format FORMAT]
```

**Examples:**

```
/memory-export architecture-decisions.md --min-importance 7

/memory-export team-knowledge.md --namespace project:myapp

/memory-export all-memories.json --format json

/memory-export high-priority.md --min-importance 8 --namespace global
```

**Options:**
- `--namespace NS`: Export specific namespace
- `--min-importance N`: Filter by importance
- `--format FORMAT`: Output format (markdown, json)

---

### `/memory-import` - Import Memories

Import memories from a file (typically from export).

**Syntax:**
```
/memory-import <input-file> [--namespace NS] [--merge]
```

**Examples:**

```
/memory-import team-knowledge.json

/memory-import backup.json --namespace project:newapp

/memory-import shared-context.json --merge
```

**Options:**
- `--namespace NS`: Import into specific namespace
- `--merge`: Merge with existing memories (default: overwrite duplicates)

---

### `/memory-stats` - View Statistics

Display database statistics and insights.

**Syntax:**
```
/memory-stats [--namespace NS]
```

**Examples:**

```
/memory-stats

/memory-stats --namespace project:myapp
```

**Shows:**
- Total memories
- Memories by type (decision, pattern, bug, context)
- Average importance
- Most common tags
- Storage size

---

## Command Composition

Combine slash commands to create powerful workflows.

### Example 1: Store and Link

```
# Store a decision
/memory-store We're migrating from REST to GraphQL --importance 8 --tags architecture,api

# Find the memory ID
/memory-recall GraphQL migration --format json

# Store a related implementation note
/memory-store GraphQL schema uses code-first approach with TypeGraphQL --related-to mem_abc123
```

### Example 2: Search, Review, Update

```
# Find outdated decisions
/memory-recall database --min-importance 7

# Review the results, find ID

# Update with new information
/memory-update mem_xyz789 Updated: Now using PostgreSQL 15 with pgvector extension
```

### Example 3: Export, Share, Import

```
# Export project knowledge
/memory-export team-onboarding.md --namespace project:myapp --min-importance 6

# (Share team-onboarding.md with team)

# Team member imports into their database
/memory-import team-onboarding.md --namespace project:myapp
```

---

## Custom Slash Commands

Create custom slash commands for common workflows.

### Setup

Create `.claude/commands/` directory:

```bash
mkdir -p .claude/commands
```

### Example 1: Daily Standup

Create `.claude/commands/standup.md`:

```markdown
Recall yesterday's work and store today's plan:

/memory-recall work progress implementation --min-importance 5 --limit 10

What did I accomplish yesterday? Store today's plan.
```

**Usage:**
```
/standup
```

### Example 2: Architecture Review

Create `.claude/commands/arch-review.md`:

```markdown
Review all architecture decisions:

/memory-recall architecture decision --min-importance 7 --limit 20

Summarize the architectural decisions and identify any conflicts or outdated decisions.
```

**Usage:**
```
/arch-review
```

### Example 3: Bug Context

Create `.claude/commands/bug-context.md`:

```markdown
Load context for debugging:

/memory-recall bug fix error --namespace project:myapp --min-importance 6

What are the most common bugs we've encountered? Any patterns I should know about?
```

**Usage:**
```
/bug-context
```

### Example 4: Commit Memory

Create `.claude/commands/commit-memory.md`:

```markdown
Store memory about the latest commit:

/memory-store {{prompt:Enter commit summary}} --importance {{prompt:Importance (1-10)}} --tags commit,{{prompt:Additional tags}}

Stored commit context.
```

**Usage:**
```
/commit-memory
```

---

## Best Practices

### 1. Use Descriptive Content

**❌ Bad:**
```
/memory-store changed the auth
```

**✅ Good:**
```
/memory-store Changed authentication from session cookies to JWT tokens.
Reasons: Need stateless auth for microservices, better mobile support.
Trade-off: Slightly more complex token refresh logic.
```

### 2. Set Appropriate Importance

**Importance scale:**
- **9-10**: Critical architecture decisions, major incidents
- **7-8**: Important features, significant bugs, key patterns
- **5-6**: Useful context, minor decisions, tips
- **3-4**: Nice-to-know information
- **1-2**: Temporary notes, debugging scratchpad

### 3. Use Namespaces Effectively

**Namespace patterns:**
- `global`: Cross-project knowledge
- `project:name`: Project-specific memories
- `session:date`: Temporary session context

**Example:**
```
# Global pattern
/memory-store Always use parameterized queries to prevent SQL injection --namespace global --importance 9

# Project-specific
/memory-store API rate limit is 100 req/min per user --namespace project:myapp --importance 7
```

### 4. Tag Consistently

**Use tags for:**
- Type: `decision`, `pattern`, `bug`, `tip`
- Component: `auth`, `database`, `api`, `frontend`
- Status: `resolved`, `active`, `deprecated`
- Priority: `critical`, `important`, `nice-to-have`

**Example:**
```
/memory-store Fixed race condition in cache invalidation --tags bug,cache,resolved,critical
```

### 5. Regular Exports

Export important knowledge regularly:

```
# Weekly export
/memory-export weekly-backup-$(date +%Y%m%d).json --format json

# Project documentation
/memory-export project-decisions.md --namespace project:myapp --min-importance 7
```

### 6. Clean Up Old Memories

Review and delete outdated memories:

```
# Find old, low-importance memories
/memory-recall old decisions --min-importance 1

# Delete obsolete memories
/memory-delete mem_old123
```

### 7. Use Graph Traversal for Context

When working on a feature, load related context:

```
# Find authentication decision
/memory-recall authentication decision

# Load all related memories
/memory-graph mem_auth789 --depth 2 --min-importance 6
```

---

## Common Patterns

### Pattern 1: Decision Documentation

```
/memory-store Decision: [What was decided]

Rationale:
- [Reason 1]
- [Reason 2]

Trade-offs:
- [Pro 1]
- [Con 1]

Alternatives considered:
- [Alternative 1]: [Why rejected]

--importance 8 --tags decision,architecture
```

### Pattern 2: Bug Tracking

```
# When bug discovered
/memory-store Bug: [Description] --importance 8 --tags bug,active

# Root cause found
/memory-update mem_bug123 Root cause: [Explanation] --tags bug,diagnosed

# Bug fixed
/memory-update mem_bug123 Fixed in commit abc123. Solution: [Explanation] --tags bug,resolved
```

### Pattern 3: Pattern Library

```
/memory-store Pattern: [Pattern name]

Problem: [What problem it solves]
Solution: [How to apply it]
When to use: [Conditions]
When not to use: [Contraindications]

--importance 8 --tags pattern,best-practice --namespace global
```

---

## Troubleshooting

### Slash Command Not Found

**Symptom:** `/memory-store` returns "Unknown command"

**Solutions:**
1. Check MCP server is running: See [Claude Code Setup](claude-code-setup.md)
2. Restart Claude Code
3. Verify `.claude/mcp_config.json` exists

### Command Returns Error

**Symptom:** Slash command executes but returns an error

**Solutions:**
1. Check command syntax (see examples above)
2. Verify API key: `mnemosyne secrets get ANTHROPIC_API_KEY`
3. Test CLI directly: `mnemosyne remember --content "test"`

### No Results from Recall

**Symptom:** `/memory-recall` returns empty results

**Solutions:**
1. Check namespace: `/memory-recall query --all-namespaces`
2. Lower importance filter: `/memory-recall query --min-importance 1`
3. Verify memories exist: `mnemosyne recall --query "query"`

---

## Next Steps

- **[Claude Code Setup](claude-code-setup.md)** - Configure MCP integration
- **[Common Workflows](../workflows/README.md)** - Usage patterns
- **[MCP API Reference](../../MCP_SERVER.md)** - Detailed tool documentation

---

**Version**: 1.0.0
**Last Updated**: 2025-10-27
