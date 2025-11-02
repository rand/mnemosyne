# Specification Artifacts Design

## Overview

This document defines the artifact storage system for specification workflow integration in Mnemosyne, inspired by GitHub's Spec-Kit but adapted for Mnemosyne's memory-first architecture.

## Directory Structure

```
.mnemosyne/
├── artifacts/
│   ├── constitution/
│   │   └── project-constitution.md
│   ├── specs/
│   │   ├── <feature-branch-name>.md
│   │   └── <feature-branch-name>-002.md  # versioning
│   ├── plans/
│   │   └── <feature-branch-name>-plan.md
│   ├── tasks/
│   │   └── <feature-branch-name>-tasks.md
│   ├── checklists/
│   │   └── <feature-branch-name>-checklist.md
│   └── clarifications/
│       └── <feature-branch-name>-clarifications.md
└── db/
    └── project.db  # Memory entries reference artifacts
```

## YAML Frontmatter Format

All artifacts use YAML frontmatter for metadata, followed by markdown content.

### Constitution

```yaml
---
type: constitution
version: 1.2.3  # SemVer: MAJOR.MINOR.PATCH
created_at: 2025-01-15T10:30:00Z
updated_at: 2025-01-15T14:20:00Z
memory_id: mem-abc123  # Link to memory entry
project_name: mnemosyne
branch: main
---

# Project Constitution

## Core Principles

1. **Performance First**: Sub-200ms retrieval latency (p95)
2. **Safety**: Type-safe Rust with comprehensive error handling
3. **Project Awareness**: Auto-detect context from git + CLAUDE.md

## Quality Gates

...
```

### Feature Specification

```yaml
---
type: feature_spec
feature_id: user-auth-jwt
feature_name: JWT Authentication
branch: feature/user-auth-jwt
created_at: 2025-01-15T10:30:00Z
updated_at: 2025-01-15T14:20:00Z
version: 1
status: draft | approved | implemented | deprecated
memory_id: mem-def456
constitution_ref: mem-abc123
parent_spec: null  # For sub-features
---

# Feature: JWT Authentication

## User Scenarios (Prioritized)

### P1: User Login with Token

**As a** developer
**I want** to authenticate with JWT tokens
**So that** I can maintain stateless sessions

**Acceptance Criteria**:
- [ ] Token issued on successful login
- [ ] Token validated on protected endpoints
- [ ] Token expires after 24 hours

...
```

### Implementation Plan

```yaml
---
type: implementation_plan
feature_id: user-auth-jwt
plan_name: JWT Auth Implementation
branch: feature/user-auth-jwt
created_at: 2025-01-15T15:00:00Z
updated_at: 2025-01-15T16:30:00Z
version: 1
memory_id: mem-ghi789
spec_ref: mem-def456
constitution_ref: mem-abc123
---

# Implementation Plan: JWT Authentication

## Technical Approach

### Architecture Decision

Using jsonwebtoken crate with RS256 algorithm...

### Data Models

```rust
struct Claims {
    sub: String,  // User ID
    exp: i64,     // Expiration
    iat: i64,     // Issued at
}
```

...
```

### Task Breakdown

```yaml
---
type: task_breakdown
feature_id: user-auth-jwt
branch: feature/user-auth-jwt
created_at: 2025-01-15T17:00:00Z
updated_at: 2025-01-15T17:30:00Z
version: 1
memory_id: mem-jkl012
plan_ref: mem-ghi789
spec_ref: mem-def456
---

# Task Breakdown: JWT Authentication

## Setup

- [x] [T001] Add jsonwebtoken dependency
- [x] [T002] Create keys directory structure

## P1: User Login with Token

- [ ] [T003] [P] Implement token generation function
- [ ] [T004] [P] Implement token validation middleware
- [ ] [T005] Integrate login endpoint with token generation
- [ ] [T006] Add token refresh endpoint
- [ ] [T007] Write tests for token lifecycle

**Legend**: `[P]` = Parallelizable

...
```

### Quality Checklist

```yaml
---
type: quality_checklist
feature_id: user-auth-jwt
branch: feature/user-auth-jwt
created_at: 2025-01-15T17:30:00Z
updated_at: 2025-01-15T18:00:00Z
version: 1
memory_id: mem-mno345
spec_ref: mem-def456
plan_ref: mem-ghi789
task_ref: mem-jkl012
---

# Quality Checklist: JWT Authentication

## Functional Requirements

- [ ] All P1 user scenarios passing
- [ ] Token generation works correctly
- [ ] Token validation rejects invalid tokens
- [ ] Token expiration enforced

## Non-Functional Requirements

- [ ] Performance: Token validation <10ms (p95)
- [ ] Security: Keys stored securely
- [ ] Security: No secrets in git

## Testing

- [ ] Unit tests: 90%+ coverage
- [ ] Integration tests passing
- [ ] Manual testing complete

...
```

### Clarification

```yaml
---
type: clarification
feature_id: user-auth-jwt
branch: feature/user-auth-jwt
created_at: 2025-01-15T11:00:00Z
updated_at: 2025-01-15T11:30:00Z
version: 1
memory_id: mem-pqr678
spec_ref: mem-def456
question_id: Q001
---

# Clarification: JWT Authentication

## Question

Should we support refresh tokens, or just short-lived access tokens?

## Context

The spec mentions "stateless sessions" but doesn't specify refresh mechanism.

## Decision

Use refresh tokens stored in HTTP-only cookies for better UX.

**Rationale**: Improves security (refresh tokens can be revoked) and UX (no manual re-auth every 24h).

**Updated Spec Section**: Added P2 scenario for refresh token flow.

...
```

## Artifact Lifecycle

### 1. Constitution

```
/project-constitution
  → Create/update constitution
  → Version bump (SemVer)
  → Store in .mnemosyne/artifacts/constitution/
  → Create memory entry with type=Constitution
  → Link to project namespace
```

### 2. Feature Workflow

```
/feature-specify <description>
  → Parse user input
  → Generate feature_id and branch name
  → Create spec in .mnemosyne/artifacts/specs/
  → Create memory entry with type=FeatureSpec
  → Link to constitution memory
  → Optionally trigger /feature-clarify

/feature-clarify
  → Identify ambiguities (max 3)
  → Interactive Q&A
  → Update spec with clarifications
  → Create clarification artifacts

/feature-plan
  → Read spec from artifacts
  → Generate technical plan
  → Create plan in .mnemosyne/artifacts/plans/
  → Create memory entry with type=ImplementationPlan
  → Link to spec memory

/feature-tasks
  → Read plan from artifacts
  → Generate task breakdown
  → Create tasks in .mnemosyne/artifacts/tasks/
  → Create memory entry with type=TaskBreakdown
  → Link to plan memory
  → Optionally export to Beads

/feature-checklist
  → Read spec, plan, tasks
  → Generate quality checklist
  → Create checklist in .mnemosyne/artifacts/checklists/
  → Create memory entry with type=QualityChecklist
  → Link to spec, plan, task memories

/feature-implement
  → Read tasks from artifacts
  → Execute task-by-task with Executor agent
  → Update task status in artifact + memory
  → Link commits to tasks
  → Run checklist validation
```

## Memory Graph Relationships

```
Constitution (mem-abc123)
  ↓ referenced_by
FeatureSpec (mem-def456)
  ↓ builds_upon
ImplementationPlan (mem-ghi789)
  ↓ builds_upon
TaskBreakdown (mem-jkl012)
  ↓ referenced_by
QualityChecklist (mem-mno345)

FeatureSpec (mem-def456)
  ↓ clarified_by
Clarification (mem-pqr678)

TaskBreakdown (mem-jkl012)
  ↓ implements
CodePattern (mem-stu901)  # Actual code
  ↓ tested_by
BugFix (mem-vwx234)  # Test results
```

## Storage Strategy

### Artifact Files
- Human-readable markdown with YAML frontmatter
- Stored in `.mnemosyne/artifacts/`
- Versioned via git
- Can be edited directly or via slash commands

### Memory Entries
- Store metadata + summary in database
- Full content stored in artifact file
- Memory entry has `artifact_path` field
- Enables semantic search across all artifacts
- Graph relationships between memories

### Benefits
1. **Human-Readable**: Markdown files can be read/edited directly
2. **Searchable**: Memory system indexes content semantically
3. **Linked**: Graph relationships show dependencies
4. **Versioned**: Git tracks artifact evolution
5. **Portable**: Can export artifacts without database

## File Naming Conventions

### Constitution
- `project-constitution.md` (always singular)

### Feature Artifacts
- Spec: `{feature-id}.md` or `{feature-id}-v{version}.md`
- Plan: `{feature-id}-plan.md`
- Tasks: `{feature-id}-tasks.md`
- Checklist: `{feature-id}-checklist.md`
- Clarifications: `{feature-id}-clarifications.md`

### Feature ID Format
- Derived from branch name or description
- Format: `{category}-{name}` (e.g., `user-auth-jwt`)
- Max 50 characters
- Kebab-case

## Implementation Notes

### New Storage Module

Create `src/artifacts/` with:
- `mod.rs`: Core types and traits
- `constitution.rs`: Constitution management
- `feature_spec.rs`: Spec management
- `plan.rs`: Plan management
- `tasks.rs`: Task management
- `checklist.rs`: Checklist management
- `clarification.rs`: Clarification management
- `storage.rs`: File I/O and YAML parsing
- `memory_link.rs`: Memory entry creation and linking

### New CLI Subcommands

Add to `src/main.rs`:
```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing commands ...

    #[command(subcommand)]
    Artifact(ArtifactCommands),
}

#[derive(Subcommand)]
enum ArtifactCommands {
    /// Initialize artifact directory structure
    Init,

    /// List all artifacts
    List {
        #[arg(long)]
        type: Option<String>,
    },

    /// Show artifact details
    Show {
        artifact_id: String,
    },

    /// Validate artifact
    Validate {
        artifact_path: String,
    },
}
```

## Testing Strategy

### Unit Tests
- YAML parsing/serialization
- File I/O operations
- Memory linking logic

### Integration Tests
- Create artifact → verify file + memory
- Update artifact → verify version bump
- Link artifacts → verify graph relationships
- Search artifacts → verify semantic retrieval

### E2E Tests
- Complete workflow: constitution → spec → plan → tasks → implement
- Verify traceability: requirement → code → test
- Verify consistency: changes propagate correctly

## Rollout Plan

1. **Phase 1.1**: Artifact storage infrastructure ✅
2. **Phase 1.2**: Memory linking and graph relationships ✅
3. **Phase 1.3**: Directory initialization ✅
4. **Phase 1.4**: Tests and validation ✅
5. **Phase 2.1**: Workflow coordinator and builders ✅
6. **Phase 2.2**: CLI create commands ✅
7. **Phase 2.3**: Database schema updates ✅
8. **Phase 2.4**: End-to-end examples ✅
9. **Phase 2.5**: Markdown parsing for round-trip serialization ✅

### Implementation Status

**Fully Complete** ✅:
- Constitution: Save/load with principles, quality gates, constraints
- FeatureSpec: Save/load with scenarios, requirements, success criteria
- ImplementationPlan: Save/load with approach, architecture decisions, dependencies
- TaskBreakdown: Save/load with phases, tasks (all markers: [P], [story], dependencies)
- QualityChecklist: Save/load with sections, items, completion status, notes
- CLI commands: init, create-constitution, create-feature-spec, list, show, validate
- Working example: examples/specification_workflow.rs
- Round-trip serialization: Tested and verified for all artifact types
- Unit tests: 45 tests covering all artifact parsing and round-trips

**Pending** ⚠️:
- Clarification: Basic structure exists, markdown parsing TODO
- Workflow integration tests: Minimal coverage (needs expansion)
- Phase 2 memory linking: Advanced graph operations (link_artifacts, etc.)

## CLI Usage

### Initialize Artifact Directory

```bash
mnemosyne artifact init
```

Creates the directory structure:
```
.mnemosyne/artifacts/
├── constitution/
├── specs/
├── plans/
├── tasks/
├── checklists/
├── clarifications/
└── README.md
```

### Create Project Constitution

```bash
mnemosyne artifact create-constitution \
  --project "MyProject" \
  --principle "Performance First: Sub-200ms latency" \
  --principle "Type Safety: Leverage strong typing" \
  --quality-gate "90%+ test coverage" \
  --quality-gate "All warnings addressed" \
  --constraint "Rust-only for core logic" \
  --namespace "project:myproject"  # optional, defaults to project:PROJECT_NAME
```

Output:
```
✅ Constitution saved!
   Memory ID: abc-123-...
   File: constitution/project-constitution.md
```

The command will:
1. Build constitution using fluent builder API
2. Save to `.mnemosyne/artifacts/constitution/project-constitution.md` with YAML frontmatter
3. Create memory entry in database with high importance (9)
4. Link memory entry to artifact file
5. Update frontmatter with memory ID

### Create Feature Specification

```bash
mnemosyne artifact create-feature-spec \
  --id "user-authentication" \
  --name "User Authentication" \
  --requirement "Use JWT tokens for authentication" \
  --requirement "Support refresh tokens" \
  --success-criterion "Sub-100ms authentication latency" \
  --success-criterion "Support 10,000 concurrent sessions" \
  --constitution-id "abc-123-..."  # optional, links to constitution
  --namespace "project:myproject"  # optional, inferred from cwd
```

Output:
```
✅ Feature spec saved!
   Memory ID: def-456-...
   File: specs/user-authentication.md
   Linked to constitution: abc-123-...
```

The command will:
1. Build feature spec using fluent builder API
2. Save to `.mnemosyne/artifacts/specs/<feature-id>.md` with YAML frontmatter
3. Create memory entry with importance 8
4. Link to constitution if provided
5. Update frontmatter with memory ID and references

### List Artifacts

```bash
# List all artifacts
mnemosyne artifact list

# List by type
mnemosyne artifact list --artifact-type constitution
mnemosyne artifact list --artifact-type spec
mnemosyne artifact list --artifact-type plan
```

### Show Artifact Details

```bash
# By artifact ID
mnemosyne artifact show constitution
mnemosyne artifact show user-authentication

# By file path
mnemosyne artifact show .mnemosyne/artifacts/specs/user-authentication.md
```

### Validate Artifact

```bash
mnemosyne artifact validate .mnemosyne/artifacts/constitution/project-constitution.md
```

Checks:
- YAML frontmatter structure
- Required fields present
- Valid field types

## Programmatic API

For advanced use cases, use the Rust API directly:

```rust
use mnemosyne_core::artifacts::{Constitution, FeatureSpec, ArtifactWorkflow};
use mnemosyne_core::types::Namespace;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize storage and workflow
    let storage = Arc::new(LibsqlStorage::new_with_validation(...).await?);
    let workflow = ArtifactWorkflow::new(
        ".mnemosyne/artifacts".into(),
        storage
    )?;

    // Create constitution
    let mut constitution = Constitution::builder("MyProject".to_string())
        .principle("Performance First")
        .quality_gate("90%+ coverage")
        .constraint("Rust-only")
        .build();

    let namespace = Namespace::Project { name: "myproject".to_string() };
    let memory_id = workflow.save_constitution(&mut constitution, namespace).await?;

    // Create feature spec
    let mut spec = FeatureSpec::builder(
        "user-auth".to_string(),
        "User Authentication".to_string()
    )
        .requirement("Use JWT tokens")
        .success_criterion("Sub-100ms latency")
        .build();

    let spec_id = workflow.save_feature_spec(
        &mut spec,
        namespace,
        Some(memory_id.to_string())
    ).await?;

    // Load artifacts
    let loaded_constitution = workflow.load_constitution().await?;
    let loaded_spec = workflow.load_feature_spec("user-auth").await?;

    Ok(())
}
```

See `examples/specification_workflow.rs` for complete working example.

---

## References

- GitHub Spec-Kit (inspiration): https://github.com/github/spec-kit
- Mnemosyne Work Plan Protocol: CLAUDE.md
- Memory Types: src/types.rs
- Storage Schema: docs/STORAGE_SCHEMA.md
