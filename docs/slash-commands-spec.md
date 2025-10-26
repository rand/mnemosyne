# Slash Commands Specification - Mnemosyne

**Phase**: 5 - Multi-Agent Integration
**Status**: Design Phase
**Date**: 2025-10-26

## Overview

Slash commands provide convenient, high-level interfaces for Claude Code users to interact with Mnemosyne without directly calling MCP tools. They translate user-friendly commands into MCP tool invocations.

## Requirements

### User Goals
1. **Quick memory storage** - Store important information without verbose MCP syntax
2. **Easy memory retrieval** - Search and display memories in readable format
3. **Context loading** - Load relevant project context at session start
4. **Memory review** - Export/view memories for review and maintenance

### Integration Points
- Slash commands invoke MCP tools (`mnemosyne.*`)
- Support both interactive and non-interactive modes
- Namespace auto-detection from current project
- Clear, formatted output for readability

## Slash Command Definitions

### 1. `/memory-store`

**Purpose**: Store a new memory with minimal syntax

**Syntax**:
```
/memory-store <content>
/memory-store --importance <1-10> <content>
/memory-store --context <context> <content>
```

**Examples**:
```
/memory-store Decided to use PostgreSQL for better ACID guarantees

/memory-store --importance 9 Critical: All API responses must be idempotent

/memory-store --context "authentication refactor" Switched from JWT to session-based auth for better security
```

**Behavior**:
1. Auto-detect namespace from current project (git root + CLAUDE.md)
2. Call `mnemosyne.remember` MCP tool
3. Display confirmation with memory ID, summary, and tags
4. Default importance: 5 (medium)

**Output Format**:
```
✓ Memory stored successfully

ID: mem_a8f3b2c1...
Summary: Decided to use PostgreSQL for better ACID compliance
Tags: database, postgresql, architecture
Importance: 7/10
```

---

### 2. `/memory-search`

**Purpose**: Search memories and display results

**Syntax**:
```
/memory-search <query>
/memory-search --namespace <ns> <query>
/memory-search --min-importance <1-10> <query>
/memory-search --limit <N> <query>
```

**Examples**:
```
/memory-search database decisions

/memory-search --namespace project:ecommerce payment processing

/memory-search --min-importance 8 critical bugs

/memory-search --limit 5 authentication patterns
```

**Behavior**:
1. Call `mnemosyne.recall` MCP tool with hybrid search
2. Format results with importance indicators
3. Display match reasons (keyword/graph/importance)
4. Highlight relevant excerpts

**Output Format**:
```
Found 3 memories matching "database decisions":

1. [⭐⭐⭐⭐⭐⭐⭐⭐⭐ 9/10] Architecture Decision
   Created: 2025-10-15
   Summary: Chose PostgreSQL over MongoDB for user data
   Match: keyword_match (score: 0.92)

   Rationale: Need strong ACID guarantees for financial transactions...
   Tags: database, postgresql, architecture

2. [⭐⭐⭐⭐⭐⭐⭐ 7/10] Constraint
   Created: 2025-10-12
   Summary: Database queries must complete in <200ms
   Match: graph_expansion (score: 0.65)

   Performance requirement for user-facing APIs...
   Tags: database, performance, constraint

3. [⭐⭐⭐⭐⭐⭐ 6/10] Configuration
   Created: 2025-10-10
   Summary: Database connection pool size set to 20
   Match: keyword_match (score: 0.58)

   Based on load testing results...
   Tags: database, configuration, performance
```

---

### 3. `/memory-context`

**Purpose**: Load relevant project context at session start

**Syntax**:
```
/memory-context
/memory-context <project-name>
/memory-context --recent <days>
/memory-context --important
```

**Examples**:
```
/memory-context

/memory-context ecommerce

/memory-context --recent 7

/memory-context --important
```

**Behavior**:
1. Auto-detect current project namespace
2. Call `mnemosyne.context` with project memories
3. Prioritize: recent (last 7 days) + high importance (8+)
4. Build memory graph from key decisions
5. Display structured summary

**Output Format**:
```
📚 Loading context for project: ecommerce

Recent Activity (last 7 days):
- 2025-10-25: Implemented stripe webhook handler
- 2025-10-24: Fixed race condition in order processing
- 2025-10-23: Added idempotency keys to payment API

Critical Decisions (importance 8+):
- Event-driven architecture for order processing
- PostgreSQL for transactional data
- Redis for session storage
- Stripe for payment processing

Active Constraints:
- All events must be idempotent
- API response time <200ms p95
- PCI compliance required for payment data

Related Files:
- src/orders/processor.rs
- src/payments/stripe.rs
- src/api/routes.rs

Memory Graph: 23 memories, 45 links
Most connected: "event-driven architecture" (12 links)
```

---

### 4. `/memory-list`

**Purpose**: List and browse memories

**Syntax**:
```
/memory-list
/memory-list --sort <recent|importance|access>
/memory-list --type <type>
/memory-list --namespace <ns>
/memory-list --limit <N>
```

**Examples**:
```
/memory-list

/memory-list --sort importance

/memory-list --type architecture_decision

/memory-list --namespace project:ecommerce --limit 10
```

**Behavior**:
1. Call `mnemosyne.list` MCP tool
2. Default: recent, limit 20
3. Display in table format

**Output Format**:
```
Recent Memories (20 total):

 # | Date       | Type                | Imp | Summary
---|------------|---------------------|-----|----------------------------------
 1 | 2025-10-25 | Bug Fix            | 6   | Fixed race condition in orders
 2 | 2025-10-24 | Configuration      | 5   | Updated stripe webhook endpoint
 3 | 2025-10-23 | Code Pattern       | 7   | Added retry logic with exp backoff
 4 | 2025-10-22 | Architecture       | 9   | Event-driven order processing
 5 | 2025-10-21 | Constraint         | 8   | All events must be idempotent
...

Use /memory-search <query> to find specific memories
Use /memory-export to export all memories
```

---

### 5. `/memory-export`

**Purpose**: Export memories to markdown for review

**Syntax**:
```
/memory-export
/memory-export --namespace <ns>
/memory-export --output <file>
/memory-export --format <markdown|json>
```

**Examples**:
```
/memory-export

/memory-export --namespace project:ecommerce

/memory-export --output memories-export.md

/memory-export --format json
```

**Behavior**:
1. Fetch all memories in namespace
2. Generate markdown document
3. Organize by type and importance
4. Include metadata and links
5. Write to file or display

**Output Format** (Markdown):
```markdown
# Memory Export - Project: ecommerce
Generated: 2025-10-26

## Architecture Decisions (5)

### [9/10] Event-Driven Architecture for Order Processing
**Date**: 2025-10-22
**Tags**: architecture, events, orders
**Context**: Order processing redesign

Decided to use event-driven architecture for order processing.

**Rationale**:
- Decouples payment from fulfillment
- Enables async processing
- Improves resilience and scalability

**Constraints**:
- All events must be idempotent
- Event schema versioning required

**Related Memories**:
- Idempotency requirement (constraint)
- Event schema design (code_pattern)
- Retry logic implementation (code_pattern)

---

## Code Patterns (8)

### [7/10] Exponential Backoff Retry Logic
...

## Bug Fixes (12)

### [6/10] Race Condition in Order Processing
...
```

---

### 6. `/memory-consolidate`

**Purpose**: Review and consolidate duplicate/similar memories

**Syntax**:
```
/memory-consolidate
/memory-consolidate --auto
/memory-consolidate <id1> <id2>
```

**Examples**:
```
/memory-consolidate

/memory-consolidate --auto

/memory-consolidate mem_a8f3b2 mem_x9k1m4
```

**Behavior**:
1. Find consolidation candidates (similar memories)
2. Use LLM to recommend merge/supersede/keep
3. Display recommendations
4. Optionally auto-apply

**Output Format**:
```
🔍 Scanning for consolidation candidates...

Found 3 candidate pairs:

1. Merge Recommended
   A: [8/10] "Use PostgreSQL for user data" (2025-10-15)
   B: [7/10] "PostgreSQL chosen for transactions" (2025-10-16)

   Reason: Very similar content, B adds transaction detail
   Action: Merge B into A, update content

   [m] Merge  [s] Skip  [v] View details

2. Supersede Recommended
   A: [6/10] "API rate limit: 100 req/min" (2025-09-01)
   B: [8/10] "API rate limit increased to 500 req/min" (2025-10-20)

   Reason: B contains updated information
   Action: Keep B, archive A

   [s] Supersede  [k] Keep both  [v] View details

3. Keep Both
   A: [9/10] "Event-driven architecture"
   B: [8/10] "Microservices architecture"

   Reason: Distinct architectural patterns
   Action: No consolidation needed

---

Commands:
- Type letter + number to apply action (e.g., 'm1' to merge pair 1)
- /memory-consolidate --auto to apply all recommendations
- 'q' to quit
```

## Implementation Notes

### File Locations
```
.claude/
└── commands/
    ├── memory-store.md
    ├── memory-search.md
    ├── memory-context.md
    ├── memory-list.md
    ├── memory-export.md
    └── memory-consolidate.md
```

### Command File Format
Each command file contains the prompt that Claude Code will execute:

```markdown
---
name: memory-store
description: Store a new memory in Mnemosyne
---

I will help you store a memory in Mnemosyne.

[Instructions for Claude to parse args and call mnemosyne.remember MCP tool]
[Output formatting instructions]
```

### Namespace Detection
Commands auto-detect namespace using:
1. Check for git root
2. Parse `.claude/CLAUDE.md` or `CLAUDE.md`
3. Default to `global` if no project detected

### Error Handling
- Clear error messages for missing API key
- Helpful suggestions for fixing issues
- Graceful degradation (e.g., if LLM unavailable, skip enrichment)

### Dependencies
- MCP server must be running
- Mnemosyne binary in PATH
- API key configured (for LLM features)

## Success Criteria

- [ ] All 6 slash commands implemented
- [ ] Auto namespace detection works
- [ ] Output formatting is clear and readable
- [ ] Error messages are helpful
- [ ] Commands work in both global and project contexts
- [ ] Documentation updated with examples
- [ ] Manual testing with real usage scenarios

## Testing Plan

### Unit Tests
- Namespace detection logic
- Argument parsing
- Output formatting

### Integration Tests
1. Store memory → verify via search
2. Search → verify results match expectations
3. Context loading → verify relevant memories loaded
4. List → verify sorting and filtering
5. Export → verify markdown format
6. Consolidate → verify recommendations make sense

### E2E Tests
1. Full workflow: store → search → export
2. Multi-project scenario
3. Error recovery scenarios

## Future Enhancements

- Interactive consolidation with approval prompts
- Memory statistics and analytics
- Bulk import from markdown
- Memory expiration/archival automation
- Integration with git hooks (auto-store on commit)
