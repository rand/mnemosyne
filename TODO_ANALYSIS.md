# TODO/FIXME Analysis for v2.0 Release

**Generated**: 2025-10-29
**Total Count**: 54 TODO/FIXME comments in source code
**Status**: Categorized into Critical/Important/Nice-to-Have

---

## Critical (Blocks v2.0 Release) - 15 items

Must be implemented or resolved before release.

### ICS Integration (5 TODOs) - `src/ics/app.rs`

**Location**: Lines 96, 104, 106, 108
**Impact**: ICS app cannot function without these
**Priority**: P0 - Critical

```rust
// Line 96
memories: Vec::new(), // TODO: fetch from storage

// Line 104
agents: Vec::new(), // TODO: track active agents

// Line 106
attributions: Vec::new(), // TODO: extract from CRDT

// Line 108
proposals: Vec::new(), // TODO: agent proposals
```

**Required Action**: Wire ICS app.rs to LibsqlStorage and agent tracking system
**Estimated Effort**: 3-4 hours (covered in Phase 2.1-2.2)

---

### CRDT Undo/Redo (2 TODOs) - `src/ics/editor/crdt_buffer.rs`

**Location**: Lines 266, 273
**Impact**: Core editing functionality incomplete
**Priority**: P0 - Critical

```rust
// Line 266
// TODO: Implement proper undo with Automerge

// Line 273
// TODO: Implement proper redo with Automerge
```

**Required Action**: Implement Automerge transaction history for undo/redo
**Estimated Effort**: 2-3 hours (covered in Phase 2.1)

---

### Actor System (3 TODOs)

#### Orchestrator Deadlock Resolution - `src/orchestration/actors/orchestrator.rs:238`

**Impact**: P0 - Critical for reliability
**Priority**: High

```rust
// TODO: Implement deadlock resolution strategy
```

**Required Action**: Implement timeout-based deadlock detection and resolution
**Estimated Effort**: 2 hours

#### Executor Sub-Agent Spawning - `src/orchestration/actors/executor.rs:156`

**Impact**: P0 - Critical for parallelization
**Priority**: High

```rust
// TODO: Actually spawn a sub-agent actor
```

**Required Action**: Implement ractor sub-agent spawning with proper message passing
**Estimated Effort**: 2 hours

#### Reviewer Validation - `src/orchestration/actors/reviewer.rs:111,115,118,163`

**Impact**: P1 - Important for quality gates
**Priority**: High

```rust
// Line 111: TODO: Implement actual test verification
// Line 115: TODO: Check for anti-patterns
// Line 118: TODO: Verify constraints
// Line 163: TODO: Check if all work items in current phase are complete
```

**Required Action**: Implement validation logic for quality gates
**Estimated Effort**: 3 hours

---

### Evolution System (2 TODOs)

#### Consolidation - `src/evolution/consolidation.rs:617`

**Impact**: P1 - Data integrity for memory consolidation
**Priority**: High

```rust
// TODO: Execute actual supersede operation in database
```

**Required Action**: Implement database supersede operation (mark old memory as superseded)
**Estimated Effort**: 1 hour

#### Importance Scoring - `src/evolution/importance.rs:150`

**Impact**: P1 - Accurate importance calculation
**Priority**: Medium

```rust
incoming_links_count: 0, // TODO: Count incoming links from database
```

**Required Action**: Query link table for incoming link count
**Estimated Effort**: 1 hour

---

### Branch Coordination (2 TODOs) - `src/orchestration/`

**Location**: `cli.rs:145,252` and `status_line.rs:163,166`
**Impact**: P1 - Branch isolation and conflict tracking
**Priority**: High

```rust
// cli.rs:145 - TODO: Get all branches from registry
// cli.rs:252 - TODO: Get conflicts from file tracker via coordinator
// status_line.rs:163 - TODO: Get conflict count from file tracker
// status_line.rs:166 - TODO: Check if blocked
```

**Required Action**: Wire branch registry and file tracker to CLI/status line
**Estimated Effort**: 2 hours

---

## Important (Should Fix for v2.0) - 20 items

Impacts functionality but has workarounds or limited scope.

### Evaluation System (14 TODOs)

#### Feature Extractor - `src/evaluation/feature_extractor.rs`

**Lines**: 102, 229, 234, 245, 256, 293, 303, 334, 343
**Impact**: P2 - Evaluation accuracy affected
**Priority**: Medium

```rust
// Line 102: TODO: Check if context namespace matches task namespace
// Line 229: TODO: Implement memory lookup
// Line 234: TODO: Implement file stat lookup
// Line 245: TODO: Fetch memory access count and age, compute frequency
// Line 256: TODO: Fetch last_accessed_at from memory
// Line 293: TODO: Implement historical query
// Line 303: TODO: Implement co-occurrence tracking
// Line 334: TODO: Implement database insert
// Line 343: TODO: Implement database query
```

**Required Action**: Connect feature extractor to storage backend for metrics
**Estimated Effort**: 4-5 hours

#### Relevance Scorer - `src/evaluation/relevance_scorer.rs`

**Lines**: 310, 330, 392, 401, 415
**Impact**: P2 - Relevance scoring incomplete
**Priority**: Medium

```rust
// Line 310: TODO: Implement database query
// Line 330: TODO: Implement full update logic
// Line 392: TODO: Implement hierarchical propagation
// Line 401: TODO: Implement database upsert
// Line 415: TODO: Implement metrics calculation
```

**Required Action**: Implement persistence layer for relevance scores
**Estimated Effort**: 3 hours

---

### Access Control (3 TODOs) - `src/agents/access_control.rs`

**Lines**: 499, 518, 527
**Impact**: P2 - Audit trail not persisted
**Priority**: Medium

```rust
// Line 499: TODO: Implement audit trail storage when storage backend supports it
// Line 518: TODO: Implement when storage backend supports audit trail queries
// Line 527: TODO: Implement when storage backend supports stats queries
```

**Required Action**: Add audit trail table to LibSQL schema and implement queries
**Estimated Effort**: 2 hours

---

### Evolution Links - `src/evolution/links.rs:104`

**Impact**: P2 - Link metadata incomplete
**Priority**: Low

```rust
last_traversed_at: None, // TODO: Get from database when available
```

**Required Action**: Add last_traversed_at field to links table
**Estimated Effort**: 30 minutes

---

### MCP Tools - `src/mcp/tools.rs:667`

**Impact**: P2 - Vector search accuracy after memory updates
**Priority**: Medium

```rust
// TODO: Re-generate embedding
```

**Required Action**: Trigger embedding regeneration when memory content changes
**Estimated Effort**: 1 hour

---

## Nice-to-Have (Defer to v2.1+) - 19 items

Can be deferred without impacting core functionality.

### DSPy LLM Service - `src/services/dspy_llm.rs:98-100`

**Impact**: P3 - Advanced LLM features
**Priority**: Low
**Reason**: Basic LLM integration works via reqwest; DSPy is enhancement

```rust
// TODO: Implement DSPy signature and module
// TODO: Call Anthropic API via DSPy
// TODO: Parse structured response
```

**Estimated Effort**: 4 hours
**Defer Reason**: Alternative LLM integration works

---

### ICS Editor Enhancements

#### Syntax Highlighting - `src/ics/editor/highlight.rs:103`

**Impact**: P3 - UX enhancement
**Priority**: Low

```rust
// TODO: Add more language support
```

**Defer Reason**: Markdown support already implemented

#### Movement Commands - `src/ics/editor/buffer.rs:280`

**Impact**: P3 - UX enhancement
**Priority**: Low

```rust
// TODO: Implement other movement commands
```

**Defer Reason**: Basic navigation works

---

### Orchestration Enhancements

#### Optimizer Skill Discovery - `src/orchestration/actors/optimizer.rs:95`

**Impact**: P3 - Context optimization
**Priority**: Low

```rust
// TODO: Implement skill discovery from filesystem
```

**Defer Reason**: Manual skill loading works

#### Notification Task - `src/orchestration/notification_task.rs:136`

**Impact**: P3 - Optional feature
**Priority**: Low

```rust
// TODO: Implement actual notification delivery
```

**Defer Reason**: Not critical for v2.0

---

### Python Bindings - `src/python_bindings/evaluation.rs`

**Lines**: 160, 212, 231
**Impact**: P3 - Python API enhancements
**Priority**: Low

```rust
// Line 160: TODO: Implement feature extraction
// Line 212: TODO: Implement actual weight lookup
// Line 231: TODO: Implement weight update
```

**Defer Reason**: Python bindings are optional feature

---

### Network Router - `src/orchestration/network/router.rs:77`

**Impact**: P3 - Distributed features
**Priority**: Low

```rust
// TODO: Implement remote routing via Iroh
```

**Defer Reason**: Phase 4.3 (P2P) feature, can be completed in Phase 3.3

---

### Main.rs Work Plan Processing - `src/main.rs:821`

**Impact**: P3 - CLI enhancement
**Priority**: Low

```rust
// TODO: Process structured work plan when ready
```

**Defer Reason**: Basic work plan handling works

---

### TUI Metrics - `src/tui/views.rs:125`

**Impact**: P3 - UX enhancement
**Priority**: Low

```rust
// TODO: Gather real metrics
```

**Defer Reason**: Not critical for v2.0

---

## Test-Only TODOs - Not Counted

These are in test files and represent test enhancements, not production code gaps:

- `tests/ics_e2e/human_workflows.rs`: Test case expansions (lines 38-40)
- `tests/ics_e2e/helpers/*.rs`: Test fixtures and assertions
- `tests/ics_integration_test.rs`: Integration test enhancements
- `tests/ics_full_integration/llm_integration.rs`: LLM test scenarios

**Note**: These are test enhancements, not blockers for release.

---

## Summary

| Category | Count | Estimated Effort | Phase Coverage |
|----------|-------|------------------|----------------|
| **Critical** | 15 | 19-23 hours | Phase 2 (ICS), Phase 3 (Orchestration), Phase 5 (Tech Debt) |
| **Important** | 20 | 12-15 hours | Phase 5 (Tech Debt) |
| **Nice-to-Have** | 19 | 15-20 hours | Defer to v2.1+ |
| **Test-Only** | ~20 | N/A | Not blocking |
| **TOTAL** | **54** | **46-58 hours** | 3-4 weeks at 15h/week |

---

## Action Plan

### Phase 1.2 (Current) - Categorization ✅

- [x] Grep all TODO/FIXME comments
- [x] Categorize by priority (Critical/Important/Nice-to-Have)
- [x] Document location, impact, and effort estimates
- [x] Create this analysis document

### Phase 1.2 (Next) - Beads Issue Creation

Create Beads issues for:
- **Critical TODOs**: All 15 items (P0-P1)
- **Important TODOs**: All 20 items (P2)
- **Nice-to-Have TODOs**: Create placeholder issues for tracking (P3)

### Phase 2-5 - Implementation

- **Phase 2**: ICS Integration (5 Critical TODOs)
- **Phase 3**: Orchestration Phase 4 (indirectly addresses actor TODOs)
- **Phase 5**: Technical Debt Resolution (remaining Critical + Important TODOs)

---

## Recommendations

1. **Focus on Critical TODOs first** (15 items, ~20 hours)
   - These block core functionality and v2.0 release

2. **Address Important TODOs in Phase 5** (20 items, ~15 hours)
   - These improve evaluation accuracy and feature completeness

3. **Defer Nice-to-Have TODOs to v2.1+** (19 items)
   - These are enhancements, not blockers
   - Can be addressed post-release based on user feedback

4. **Test-Only TODOs**
   - Not counted as technical debt
   - Address opportunistically during relevant phases

---

**Status**: Categorization complete, ready for Beads issue creation.
