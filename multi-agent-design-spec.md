# Multi-Agent Orchestration System - Technical Specification

**Version**: 2.0
**Date**: 2025-01-26
**Status**: Complete and Operational

## Executive Summary

This specification defines a multi-agent orchestration system for Claude Code that coordinates four specialized agents (Orchestrator, Optimizer, Reviewer, Executor) to maximize productivity, maintain context quality, and ensure high-quality output through structured workflows and enforcement mechanisms.

**Key Innovation**: The system is operational through specifications, skills, commands, and hooks—without requiring full Python implementations. This "specification-driven" approach means the behavior is defined in documentation that Claude interprets, rather than requiring compiled code.

---

## Table of Contents

1. [Goals & Principles](#1-goals--principles)
2. [Architecture Overview](#2-architecture-overview)
3. [Component Specifications](#3-component-specifications)
4. [Skills Library Specification](#4-skills-library-specification)
5. [Slash Commands Specification](#5-slash-commands-specification)
6. [Hooks Specification](#6-hooks-specification)
7. [CLAUDE.md Specification](#7-claudemd-specification)
8. [Implementation Guide](#8-implementation-guide)
9. [Validation & Testing](#9-validation--testing)
10. [Success Metrics](#10-success-metrics)

---

## 1. Goals & Principles

### 1.1 Primary Goals

1. **Enable Multi-Agent Coordination**: Four specialized agents working in concert
2. **Preserve Critical Thinking**: Enforce minimum clarifying questions, assumption detection
3. **Optimize Context**: Prevent context collapse and brevity bias
4. **Enforce Quality**: Block poor practices (testing uncommitted code, vague requirements)
5. **Enable Parallel Work**: Safe sub-agent spawning for independent tasks
6. **Support Multi-Session Work**: State tracking via beads across sessions
7. **Maintain Modularity**: Skills, commands, hooks as separate, composable units

### 1.2 Design Principles

**Specification-Driven Architecture**:
- Behavior defined in markdown/YAML specifications
- Claude interprets specifications to exhibit agent behaviors
- Minimal compiled code required (only enforcement hooks in JavaScript)
- Python implementations optional, provided as stubs

**Modular Composition**:
- Skills are atomic, focused, composable units (<500 lines guideline)
- Commands trigger workflows by loading skills and providing prompts
- Hooks enforce protocols deterministically
- CLAUDE.md ties everything together with cross-references

**ACE-Inspired Context Management**:
- Treat context as evolving playbook, not static documentation
- Structured accumulation with clear organization
- Strategy preservation in persistent playbooks
- Reflection loops for quality improvement
- Reference grounding to prevent drift

**Critical Thinking First**:
- Minimum 2-3 clarifying questions for non-trivial work (Phase 1)
- Challenge vague requirements proactively
- Detect assumptions and flag them
- Suggest alternatives constructively
- Block on critical unknowns

### 1.3 Non-Goals

- ❌ Full Python implementation of all agent logic (stubs sufficient)
- ❌ Real-time agent process orchestration (declarative, not imperative)
- ❌ Complex distributed systems infrastructure
- ❌ Machine learning models for agent behavior
- ❌ GUI or web interface (CLI and Claude Code integration only)

---

## 2. Architecture Overview

### 2.1 System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                          CLAUDE.md                              │
│              (Master Specification & Orchestration)             │
└─────────────────────────────────────────────────────────────────┘
                                 │
                 ┌───────────────┼───────────────┐
                 │               │               │
                 ▼               ▼               ▼
        ┌─────────────┐  ┌─────────────┐  ┌─────────────┐
        │   Skills    │  │  Commands   │  │   Hooks     │
        │  (.claude/  │  │  (.claude/  │  │ (.claude/   │
        │   skills/)  │  │  commands/) │  │  hooks/)    │
        └─────────────┘  └─────────────┘  └─────────────┘
               │                │                 │
               │                │                 │
               └────────┬───────┴─────────┬───────┘
                        │                 │
                        ▼                 ▼
              ┌──────────────────────────────────┐
              │     Claude Code Runtime          │
              │  - Interprets specifications     │
              │  - Executes hooks               │
              │  - Loads skills as needed       │
              │  - Triggers slash commands      │
              └──────────────────────────────────┘
                        │
                        ▼
              ┌──────────────────────────────────┐
              │    External Integrations         │
              │  - Beads (bd) for work tracking  │
              │  - Git for version control       │
              │  - File system for artifacts     │
              └──────────────────────────────────┘
```

### 2.2 Four-Agent Model

The system models four conceptual agents that operate continuously:

1. **Orchestrator**: Coordinates workflow, manages dependencies, prevents deadlocks
2. **Optimizer**: Manages context using ACE principles, prevents collapse
3. **Reviewer**: Validates quality, enforces critical thinking, blocks violations
4. **Executor**: Executes work, spawns sub-agents, implements solutions

**Key Insight**: These agents are not separate processes. They are behavioral patterns specified in CLAUDE.md and reinforced through skills, commands, and hooks. Claude Code exhibits these behaviors by following the specifications.

### 2.3 Component Interaction Flow

```
User Request → CLAUDE.md loads relevant context
                     ↓
              Activates agents conceptually
              (Orchestrator, Optimizer, Reviewer, Executor)
                     ↓
         Optimizer loads relevant skills automatically
                     ↓
         Executor follows Work Plan Protocol
         (asks clarifying questions, creates specs)
                     ↓
         Reviewer validates completeness
         (checks exit criteria, enforces quality gates)
                     ↓
         Orchestrator coordinates phase transitions
         (ensures dependencies met, manages state)
                     ↓
         Hooks enforce protocols at runtime
         (block tests on uncommitted code, etc.)
                     ↓
              Output produced with quality guarantees
```

### 2.4 State Management

**Session State**:
- Work items tracked in `.beads/issues.jsonl` (via beads/bd tool)
- Context snapshots in `.claude/context-snapshots/` (manual or hook-triggered)
- Git commits for all artifacts (specs, plans, code)

**Context State**:
- Managed by Optimizer agent according to ACE principles
- Budget allocation: 40% active work, 20% critical state, 15% working memory, 15% reference, 10% history
- Proactive preservation at 75% utilization
- Recovery via snapshots + git history + beads state

**Work State**:
- Current phase of Work Plan Protocol (1-4)
- Exit criteria completion status
- Active beads (in_progress, blocked, completed)
- Typed holes (integration points waiting to be filled)
- Constraints and invariants

---

## 3. Component Specifications

### 3.1 Skills Library

**Location**: `.claude/skills/[skill-name]/SKILL.md`

**Purpose**: Modular, focused knowledge units that agents can load and apply

**Structure**:
```yaml
---
name: skill-name
description: >
  Clear description of what skill does and when to use it.
  This is critical for auto-discovery. Should be 1-3 sentences.
allowed-tools: [Tool1, Tool2, ...]
version: X.Y
---

# Skill Title

[Detailed skill content in markdown]
- Practical guidance
- Examples
- Templates
- Checklists
- Anti-patterns
- Quick reference
```

**Requirements**:
- YAML frontmatter with name, description, allowed-tools, version
- Name must be lowercase-with-hyphens
- Description must clearly state WHAT and WHEN
- Content should be practical and actionable
- Size guideline: <500 lines (preferred), <800 lines (max)
- Include examples and templates where applicable
- End with Quick Reference section

**Discovery**:
- Auto-loaded by Optimizer based on description matching task context
- Can be explicitly loaded by name reference in CLAUDE.md or commands
- Skills compose (can reference other skills)

### 3.2 Slash Commands

**Location**: `.claude/commands/[command-name].md`

**Purpose**: Workflow automation - trigger specific agent behaviors with single command

**Structure**:
```yaml
---
description: Brief description of what command does
argument-hint: "[arg1] [arg2]"  # optional
---

# Command Title

Context about what this command does and when to use it.

## Actions

1. First action
2. Second action
...

## Expected Behavior

What Claude should do when this command is invoked.

## References

- Skill: skill-name
- Other commands: /related-command
```

**Requirements**:
- YAML frontmatter with description (required) and argument-hint (optional)
- Clear action steps
- Reference to skills that should be loaded
- Can use `$ARGUMENTS` placeholder for user-provided arguments
- Should be focused on single workflow or analysis

**Execution**:
- User types `/command-name` in Claude Code
- Command file content is injected as system prompt
- Claude executes based on instructions in command
- Can load skills, run bash commands, create files, etc.

### 3.3 Hooks

**Location**: `.claude/hooks/[hook-name].js`

**Purpose**: Deterministic enforcement of protocols at specific lifecycle points

**Structure**:
```javascript
/**
 * Hook Name
 *
 * Description of what this hook enforces
 */

export async function hook(context) {
  const { tool, parameters, prompt, ... } = context;

  // Hook logic here
  // Can use: context.exec(command) to run bash commands
  // Can inspect: tool, parameters, prompt, etc.

  // To block:
  return {
    block: true,
    message: "Reason for blocking with guidance"
  };

  // To allow:
  return {
    block: false,
    message: "Optional info message"  // optional
  };
}
```

**Available Hooks**:
- `pre-tool-use.js` - Before any tool invocation
- `post-tool-use.js` - After tool completes
- `user-prompt-submit.js` - When user submits prompt
- `session-start.js` - At session start
- `session-end.js` - At session end
- `pre-compact.js` - Before context compaction
- `notification.js` - When notification sent
- `stop.js` - When Claude stops responding

**Requirements**:
- Must export async function named `hook`
- Must accept `context` parameter with relevant data
- Must return object with `block` and optional `message`
- Should be deterministic (same input → same output)
- Should provide clear, actionable feedback if blocking
- Can execute bash commands via `context.exec()`

### 3.4 CLAUDE.md

**Location**: Project root or `~/.claude/CLAUDE.md`

**Purpose**: Master specification that defines agent behaviors and ties system together

**Structure**:
```markdown
# Title

> Core principles

## 1. Multi-Agent Orchestration System
- Agent 1: Orchestrator (role, responsibilities, protocols)
- Agent 2: Optimizer (role, responsibilities, protocols)
- Agent 3: Reviewer (role, responsibilities, protocols)
- Agent 4: Executor (role, responsibilities, protocols)
- Coordination patterns
- Context management
- Hooks integration

## 2. Critical Thinking & Pushback
- Philosophy
- Core behaviors
- Required questions
- Templates by domain
- Pushback patterns
- Integration with agents

## 3. Work Plan Protocol
- Phase 1: Prompt → Spec
- Phase 2: Spec → Full Spec
- Phase 3: Full Spec → Execution Plan
- Phase 4: Execution Plan → Artifacts
- Enforcement rules

## 4. Integration & Workflows
- Beads integration
- Testing discipline
- Version control

## 5. Skills & Commands Reference
- Available skills (list with references)
- Slash commands (list with references)
- CLI tools

## 6. Quick Reference
- Decision tree
- Command quick reference
- Enforcement checklist
- Anti-patterns
```

**Requirements**:
- Keep agent architecture specifications complete (don't abbreviate)
- Keep critical thinking framework complete (full behavioral patterns)
- Reference external skills/commands (don't inline full content)
- Cross-reference implementation files (`.claude/skills/skill-name/SKILL.md`)
- Include clear decision trees and quick references
- Size: Aim for <1000 lines by using modular skills
- End with implementation status section

---

## 4. Skills Library Specification

### 4.1 Required Skills (Minimum 8)

#### Skill 1: critical-thinking-framework

**File**: `.claude/skills/critical-thinking-framework/SKILL.md`

**Size**: ~400 lines

**Frontmatter**:
```yaml
---
name: critical-thinking-framework
description: >
  Deep critical thinking and pushback framework for requirements analysis,
  design validation, and risk assessment. Includes question templates,
  challenge patterns, assumption detection, edge case identification,
  and constructive pushback strategies. Use during Phase 1 and when
  validating any significant technical decision.
allowed-tools: [Read, Grep, WebSearch]
version: 2.0
---
```

**Content Sections**:
1. **Philosophy**: Why critical thinking matters, balance between constructive and obstructionist
2. **Core Behaviors**: Always ask "why", challenge vague specs, question assumptions, identify edge cases, point out risks, suggest alternatives, refuse to proceed without clarity
3. **Required Questions**: Minimum 2-3 for non-trivial work (problem definition, success criteria, constraints, edge cases, risks)
4. **Question Templates by Domain**: Security, Performance, Data, UX, Integration, Testing (5-7 questions each)
5. **Challenge Patterns**: Vague requirements, unstated assumptions, technical risks, missing edge cases (with examples)
6. **Assumption Detection Heuristics**: Red flags (undefined pronouns, modal verbs, implied requirements, missing details, absolutes)
7. **Edge Case Identification Strategies**: Boundary conditions, error conditions, special values, temporal, user behavior
8. **Risk Assessment Framework**: Technical risk, business risk, mitigation strategies
9. **Constructive Pushback Patterns**: When to warn vs. block (with templates)
10. **Socratic Questioning Techniques**: Lead with questions rather than statements
11. **Domain-Specific Validation Checklists**: Web app, API, database, distributed system
12. **Anti-Patterns**: Rubber stamping, blind compliance, unhelpful negativity, over-engineering, analysis paralysis
13. **Examples**: Good vs. bad critical thinking (3-5 concrete examples)
14. **Integration with Multi-Agent System**: Executor asks questions, Reviewer enforces, Orchestrator flags blockers
15. **Metrics**: Questions asked, assumptions flagged, edge cases identified, rework rate
16. **Quick Reference Card**: 7-point checklist

#### Skill 2: work-plan-protocol

**File**: `.claude/skills/work-plan-protocol/SKILL.md`

**Size**: ~350 lines

**Frontmatter**:
```yaml
---
name: work-plan-protocol
description: >
  Apply structured 4-phase development protocol:
  Phase 1 (Prompt→Spec): Clarify requirements, apply critical thinking
  Phase 2 (Spec→Full Spec): Decompose and plan tests
  Phase 3 (Full Spec→Execution Plan): Identify parallelization
  Phase 4 (Execution Plan→Artifacts): Generate beads/holes/docs
  Use for all non-trivial development work.
allowed-tools: [Read, Write, Edit, Grep, Glob, Bash]
version: 2.0
---
```

**Content Sections**:
1. **Overview**: Mandatory application, critical flags, multi-agent integration
2. **Phase 1: Prompt → Specification**:
   - Agent responsibilities (Executor, Optimizer, Orchestrator, Reviewer)
   - Actions (parse intent, load context/skills, ask 2-3+ questions, produce spec)
   - Exit criteria (user confirms, ambiguities resolved, skills loaded, spec created, reviewer validated, questions asked)
   - Verification command
3. **Phase 2: Specification → Full Specification**:
   - Dependency on Phase 1
   - Actions (master spec, decompose, map dependencies, define holes, constraints, test plan)
   - Exit criteria (all specs exist, dependency graph, holes/constraints documented, test plan, reviewer validated)
   - Verification command
4. **Phase 3: Full Specification → Execution Plan**:
   - Dependency on Phase 2
   - Actions (atomic steps, annotate with dependencies/complexity/risk/parallelization, identify critical path, sequence)
   - Exit criteria (execution plan exists, dependencies clear, parallelization marked, critical path, risks assessed, reviewer validated)
   - Verification command
5. **Phase 4: Execution Plan → Artifacts**:
   - Dependency on Phase 3
   - Actions (generate beads, document holes, create docs, maintain structure, establish traceability)
   - Exit criteria (beads exported, holes documented, docs created, structure clean, traceability, indexes updated, reviewer validated)
   - Verification command
6. **Enforcement Rules**: Never proceed without exit criteria, stop and clarify if unclear, restart from Phase 1 on feedback
7. **Version Control**: Archive, increment, changelog, lineage
8. **Integration with Critical Thinking**: Phase 1 requires it, all phases benefit
9. **Integration with Beads**: Phase 4 creates beads, during execution claim/complete beads
10. **Quick Reference**: Phase checklist, decision flow
11. **Common Mistakes**: Skipping questions, not loading skills, proceeding without exit criteria, etc.
12. **Success Indicators**: User confirms, specs detailed, test plan comprehensive, etc.

#### Skill 3: multi-agent-coordination

**File**: `.claude/skills/multi-agent-coordination/SKILL.md`

**Size**: ~400 lines

**Frontmatter**:
```yaml
---
name: multi-agent-coordination
description: >
  Patterns and protocols for coordinating multiple agents in parallel.
  Includes sub-agent spawning criteria, coordination patterns (fan-out/fan-in,
  pipeline, recursive decomposition, map-reduce), safety checks, conflict
  prevention, and integration strategies. Use when decomposing complex work
  for parallel execution.
allowed-tools: [Task, Read, Write]
version: 2.0
---
```

**Content Sections**:
1. **Overview**: Parallel execution via Task tool
2. **Sub-Agent Spawning Criteria**: When to spawn (≥2 independent units, clear interfaces, ≥30% time savings, no shared state, clear dependencies, acceptable risk)
3. **Sub-Agent Types**: Implementation, Test, Documentation, Research, Refactor, Integration (with examples)
4. **Coordination Pattern 1: Fan-Out/Fan-In**: Description, when to use, flow diagram, example, Task tool usage
5. **Coordination Pattern 2: Pipeline**: Description, when to use, flow diagram, example, handoff protocol
6. **Coordination Pattern 3: Recursive Decomposition**: Description, when to use, flow diagram, example, complexity threshold
7. **Coordination Pattern 4: Map-Reduce**: Description, when to use, flow diagram, example, batch sizing
8. **Safe Parallelization Conditions**: All prerequisites (file independence, no shared state, clear interfaces, explicit dependencies, sufficient context, defined integration, acceptable risk)
9. **Unsafe Parallelization Scenarios**: Shared mutable state, circular dependencies, unclear integration, high merge conflict risk, insufficient context
10. **Conflict Prevention Protocol**: Before spawning (analyze graph, identify conflicts, assign non-overlapping work, establish merge strategy), during execution (monitor, detect, serialize, update graph), after completion (sequence merges, handle conflicts, validate, commit)
11. **Integration Strategies**: Sequential, batch, incremental, feature branch
12. **Communication Protocol**: Task assignment, work handoff, progress monitoring
13. **Performance Optimization**: Minimize coordination overhead, maximize parallelism, context efficiency
14. **Example: Full Workflow**: Blog platform with dependency analysis, parallelization plan, execution
15. **Anti-Patterns**: Too many agents, trivial tasks, unclear deliverables, ignoring dependencies, poor context, no integration plan, risky parallelization
16. **Success Metrics**: 30%+ time savings, successful integration, no conflicts, clear handoffs, reviewer approval

#### Skill 4: context-optimization

**File**: `.claude/skills/context-optimization/SKILL.md`

**Size**: ~300 lines

**Frontmatter**:
```yaml
---
name: context-optimization
description: >
  ACE-inspired context management strategies including structured accumulation,
  strategy preservation, semantic compression, and recovery protocols.
  Prevents context collapse and brevity bias. Use when context approaches
  75% utilization or before major operations.
allowed-tools: [Read, Write]
version: 2.0
---
```

**Content Sections**:
1. **Overview**: ACE principles, context as evolving playbook
2. **Context Budget Allocation**: 40/20/15/15/10 split, reallocation triggers
3. **Proactive Context Preservation**: Continuous monitoring (thresholds: 50/75/90/95%), preservation triggers (at 75%: compress, snapshot, document, update index; before phase transitions; during compaction; after compaction)
4. **ACE-Inspired Strategies**:
   - Structured Accumulation: Build incrementally with organization
   - Strategy Preservation: Maintain successful patterns in playbooks
   - Reflection Loops: Periodically review quality
   - Modular Organization: Separate concerns
   - Reference Grounding: Include source quotes
5. **Optimization Metrics**: Context density, coverage, precision, efficiency, recovery rate (with targets)
6. **Context Payload Construction**: For sub-agents (relevance scoring, extraction, compression, validation phases)
7. **Context Recovery Protocol**: If loss detected (load snapshot, replay log, reconstruct from git, import beads, verify, resume)
8. **Preventing Context Collapse**: Symptoms, prevention strategies (never compress critical, tiered compression, maintain anchors, regular validation)
9. **Preventing Brevity Bias**: Symptoms, prevention strategies (preserve non-obvious insights, keep "why", document gotchas, maintain glossary)
10. **Best Practices**: 10 key practices
11. **Anti-Patterns**: 8 things never to do
12. **Quick Reference**: Context at 75%, before major operation, during compression, after compression

#### Skill 5: testing-discipline

**File**: `.claude/skills/testing-discipline/SKILL.md`

**Size**: ~200 lines

**Frontmatter**:
```yaml
---
name: testing-discipline
description: >
  Commit-first testing protocol, TDD workflow, coverage targets, and test types.
  Prevents hours of debugging stale code by enforcing git commit before running tests.
  Use when implementing features or fixing bugs.
allowed-tools: [Bash, Read, Write]
version: 2.0
---
```

**Content Sections**:
1. **Critical Rule**: Commit first, then test. NEVER run tests on uncommitted code.
2. **Why Commit-First Matters**: The problem (wrong flow leads to hours wasted), the solution (correct flow)
3. **Testing Protocol (MANDATORY)**: 5-step process with bash commands
4. **Automation via Hook**: Reference to pre-tool-use.js
5. **Test Types**: Unit (purpose, scope, coverage target, characteristics, example), Integration (same structure), E2E (same), Playwright (same), Property (same), Performance (same)
6. **Coverage Targets**: By code type (critical 90%, business logic 80%, UI 60%, overall 70%)
7. **TDD Workflow**: Red-Green-Refactor cycle (9 steps with examples)
8. **Test Organization**: Directory structure, naming conventions
9. **Anti-Patterns**: 7 things never to do (with explanations)
10. **Quick Reference Card**: Protocol checklist, if tests fail, coverage check, test types by speed

#### Skill 6: beads-integration

**File**: `.claude/skills/beads-integration/SKILL.md`

**Size**: ~250 lines

**Frontmatter**:
```yaml
---
name: beads-integration
description: >
  Session protocols, state management patterns, and multi-session workflows
  using beads (bd) for work tracking. Includes session start/end procedures,
  beads workflow, context management strategies. Use for all multi-session
  or complex work.
allowed-tools: [Bash, Read, Write]
version: 2.0
---
```

**Content Sections**:
1. **Overview**: Framework URL, purpose, integration with Work Plan Protocol
2. **Session Start Protocol (MANDATORY)**: 4 commands
3. **Core Workflow Pattern**: Flow diagram
4. **Common Commands**: Create, add dependencies, update status, close, list/view, export
5. **Integration with Phases**: Phase 4 creates beads from execution plan
6. **Context Management Strategies**: Strategic /context (when), strategic /compact (when), multi-agent coordination
7. **Non-Negotiable Rules**: 5 rules (never leave TODO/mocks, always --json, always export, always commit, always submit to reviewer)
8. **Session End Protocol (MANDATORY)**: 4 steps
9. **Multi-Session Patterns**: Resume work, parallel work streams, dependency chains
10. **Beads with Git Workflow**: Recommended flow (7 steps)
11. **Troubleshooting**: 3 common issues with solutions
12. **Quick Reference**: Session start, during work, session end, close issue

#### Skill 7: language-stacks

**File**: `.claude/skills/language-stacks/SKILL.md`

**Size**: ~150 lines

**Frontmatter**:
```yaml
---
name: language-stacks
description: >
  Language-specific best practices, tooling, and patterns for Python (uv),
  Zig, Rust, Go, TypeScript. Includes package managers, build systems, testing
  frameworks, and cloud deployment patterns. Use when working with these languages.
allowed-tools: [Bash, Read, Write]
version: 2.0
---
```

**Content Sections**:
1. **Python (Primary) - ALWAYS use uv**: Setup & commands, best practices
2. **Zig**: Commands, best practices
3. **Rust**: Commands, best practices
4. **Go**: Commands, best practices
5. **TypeScript**: Commands, configuration, best practices
6. **Cloud Platforms**: Modal.com, Cloudflare Workers, Heroku (commands and use cases)

#### Skill 8: frontend-development

**File**: `.claude/skills/frontend-development/SKILL.md`

**Size**: ~100 lines

**Frontmatter**:
```yaml
---
name: frontend-development
description: >
  Frontend development workflow with shadcn/ui blocks-first approach.
  Includes component patterns, responsive design, accessibility, and
  state management. Use when building web UIs.
allowed-tools: [Bash, Read, Write, WebFetch]
version: 2.0
---
```

**Content Sections**:
1. **shadcn/ui Blocks-First Workflow**: Setup, development process, key practices
2. **Component Patterns**: Loading states, error states, empty states
3. **Responsive Design**: Example
4. **Themes**: URL reference

---

## 5. Slash Commands Specification

### 5.1 Required Commands (Minimum 11)

#### Command 1: work-plan

**File**: `.claude/commands/work-plan.md`

**Purpose**: Initialize Work Plan Protocol for new development work

**Frontmatter**:
```yaml
---
description: Initialize Work Plan Protocol for new development work
argument-hint: "[brief-description]"
---
```

**Content**:
- Title: "# Initialize Work Plan Protocol"
- Context: "You are starting Phase 1 of the Work Plan Protocol for: $ARGUMENTS"
- Required Actions:
  1. Load Skills: work-plan-protocol, critical-thinking-framework, domain-specific
  2. Apply Critical Thinking: Ask 2-3+ questions using templates
  3. Produce Initial Spec: Create specs/[project]-spec-v1.md with required sections
- Exit Criteria for Phase 1: Checklist
- Reminder: "Do NOT proceed to Phase 2 until all exit criteria are met."

#### Command 2: challenge

**File**: `.claude/commands/challenge.md`

**Purpose**: Activate critical thinking framework to challenge current approach

**Frontmatter**:
```yaml
---
description: Activate critical thinking framework to challenge current approach
argument-hint: "[aspect-to-challenge]"
---
```

**Content**:
- Title: "# Critical Challenge Mode"
- Context: "Perform deep critical analysis of: $ARGUMENTS"
- Load Framework: Reference critical-thinking-framework skill
- Analysis Required: 5 sections (assumption detection, edge cases, risks, alternatives, domain validation)
- Output: Detailed challenge report with identified issues and recommendations

#### Command 3-11: Similar Structure

Follow the same pattern for:
- `beads-start.md` - Initialize beads workflow
- `beads-end.md` - Complete beads workflow
- `test-protocol.md` - Enforce commit-first testing
- `review.md` - Comprehensive review checklist
- `spawn-agents.md` - Spawn sub-agents for parallel work
- `optimize-context.md` - Audit and optimize context
- `validate-requirements.md` - Deep requirements validation
- `assume-check.md` - Detect assumptions
- `dependency-graph.md` - Visualize dependencies

Each command should:
1. Have clear frontmatter with description
2. Load relevant skills
3. Provide step-by-step instructions
4. Produce specific outputs
5. Reference related commands/skills

---

## 6. Hooks Specification

### 6.1 Required Hooks (Minimum 5)

#### Hook 1: pre-tool-use.js

**Purpose**: Block test execution on uncommitted code

**Logic**:
1. Check if tool is test-related (Bash with pytest/jest/etc. patterns)
2. If test tool, run `git status --porcelain`
3. If uncommitted changes exist, return `{block: true, message: "..."}`
4. Else return `{block: false}`

**Message Template**:
```
⛔ BLOCKED: Test execution on uncommitted code

CRITICAL VIOLATION: You attempted to run tests on uncommitted code.

Git Status:
[output]

REQUIRED ACTION:
1. Commit your changes first:
   git add .
   git commit -m "[descriptive message]"

2. Then run tests

WHY: Running tests on uncommitted code leads to hours of debugging confusion.

See: .claude/skills/testing-discipline/SKILL.md
```

#### Hook 2: session-start.js

**Purpose**: Initialize agents, load beads state, show ready work

**Logic**:
1. Log "Multi-agent system: ACTIVATING" with agent status
2. Check if bd (beads) is installed
3. If installed, import .beads/issues.jsonl and show ready work count
4. Check for context snapshots and log latest
5. Return info message with status

#### Hook 3: session-end.js

**Purpose**: Export beads state, log metrics, prompt commit

**Logic**:
1. Log "Multi-agent system: DEACTIVATING"
2. Export beads state to .beads/issues.jsonl
3. Count completed work items
4. Check if .beads/issues.jsonl has uncommitted changes
5. If uncommitted, warn and provide commit command
6. Return info message with summary

#### Hook 4: user-prompt-submit.js

**Purpose**: Detect vague requirements, suggest critical thinking

**Logic**:
1. Check prompt for vague terms (user-friendly, fast, scalable, etc.)
2. If found, return message suggesting clarification and /challenge command
3. Check if prompt is new project request with <100 characters
4. If yes, suggest /work-plan command
5. Else return empty object

#### Hook 5: pre-compact.js

**Purpose**: Preserve state before context compaction

**Logic**:
1. Log "Context compaction initiated - Preserving state..."
2. Export beads state
3. Create .claude/context-snapshots/ directory
4. Log timestamp
5. Return message indicating state preserved and recovery available

---

## 7. CLAUDE.md Specification

### 7.1 Structure Requirements

**File**: `CLAUDE.md` (or `CLAUDE-optimized.md` during development)

**Size**: Target 600-1000 lines (vs. original 1600+)

**Required Sections**:

1. **Header** (~50 lines)
   - Title: "# Claude Development Guidelines"
   - Core principles (2 quote blocks)
   - Table of Contents (6 sections)

2. **Section 1: Multi-Agent Orchestration System** (~200 lines)
   - Foundation statement (ACE principles)
   - Implementation references
   - Four agent descriptions (roles, responsibilities, protocols)
   - Coordination patterns (overview + skill reference)
   - Context management strategy (overview + skill reference)
   - Hooks & enforcement (list + references)

3. **Section 2: Critical Thinking & Pushback** (~200 lines)
   - Philosophy statement
   - Skill reference to full framework
   - Commands reference
   - Implementation reference
   - Core behaviors (always/never lists)
   - Required questions (5 types)
   - Question templates by domain (overview, full in skill)
   - Constructive pushback patterns (when to warn/block)
   - Multi-agent integration
   - Metrics

4. **Section 3: Work Plan Protocol** (~150 lines)
   - Mandatory statement
   - Skill reference
   - Command reference
   - Integration note
   - Phase 1-4 (each with: agent roles, actions, exit criteria, verification)
   - Enforcement rules
   - Version control note

5. **Section 4: Integration & Workflows** (~100 lines)
   - Beads integration (skill reference, commands, workflow)
   - Testing discipline (skill reference, command, protocol, hook)
   - Version control (branch strategy, commit guidelines)

6. **Section 5: Skills & Commands Reference** (~50 lines)
   - Skills list (8 skills with brief descriptions and references)
   - Commands list (11 commands with brief descriptions)
   - CLI tools

7. **Section 6: Quick Reference** (~100 lines)
   - Decision tree (visual flow)
   - Command quick reference (bash snippets)
   - Enforcement checklist
   - Anti-patterns
   - Conclusion

8. **Implementation Status** (~50 lines)
   - Operational now (checklist)
   - Stubs for future (checklist)
   - Note about system being operational via specifications

**Cross-Reference Pattern**:
```markdown
**Skill Reference**: `.claude/skills/skill-name/SKILL.md`
**Command**: `/command-name`
**Implementation**: `.claude/agents/agent-name/component.py`
**Hook**: `.claude/hooks/hook-name.js`
```

**Key Principles**:
- Keep agent architecture complete (don't abbreviate roles/responsibilities)
- Keep critical thinking framework complete (don't abbreviate behaviors)
- Reference external skills for detailed implementations
- Use consistent formatting for cross-references
- Include decision trees and quick references
- End with clear implementation status

---

## 8. Implementation Guide

### 8.1 Directory Structure

Create this exact structure:

```
project-root/
├── CLAUDE.md (or CLAUDE-optimized.md)
├── CLAUDE-original.md (backup if optimizing existing)
├── README.md
├── SPECIFICATION.md (this document)
├── IMPLEMENTATION_SUMMARY.md
└── .claude/
    ├── skills/
    │   ├── critical-thinking-framework/
    │   │   └── SKILL.md
    │   ├── work-plan-protocol/
    │   │   └── SKILL.md
    │   ├── multi-agent-coordination/
    │   │   └── SKILL.md
    │   ├── context-optimization/
    │   │   └── SKILL.md
    │   ├── testing-discipline/
    │   │   └── SKILL.md
    │   ├── beads-integration/
    │   │   └── SKILL.md
    │   ├── language-stacks/
    │   │   └── SKILL.md
    │   └── frontend-development/
    │       └── SKILL.md
    ├── commands/
    │   ├── work-plan.md
    │   ├── challenge.md
    │   ├── beads-start.md
    │   ├── beads-end.md
    │   ├── test-protocol.md
    │   ├── review.md
    │   ├── spawn-agents.md
    │   ├── optimize-context.md
    │   ├── validate-requirements.md
    │   ├── assume-check.md
    │   └── dependency-graph.md
    ├── hooks/
    │   ├── pre-tool-use.js
    │   ├── session-start.js
    │   ├── session-end.js
    │   ├── user-prompt-submit.js
    │   └── pre-compact.js
    ├── agents/
    │   ├── README.md
    │   ├── orchestrator/
    │   ├── optimizer/
    │   ├── reviewer/
    │   └── executor/
    ├── system/
    │   ├── README.md
    │   ├── state/
    │   ├── models/
    │   ├── context/
    │   └── metrics/
    ├── bin/
    │   └── claude-agent
    ├── tests/
    │   └── scenarios/
    ├── mcp-server/
    └── tools/
```

### 8.2 Implementation Order

**Phase 1: Skills (Week 1)**
1. Create directory structure
2. Implement critical-thinking-framework skill (400 lines)
3. Implement work-plan-protocol skill (350 lines)
4. Implement multi-agent-coordination skill (400 lines)
5. Implement context-optimization skill (300 lines)
6. Implement testing-discipline skill (200 lines)
7. Implement beads-integration skill (250 lines)
8. Implement language-stacks skill (150 lines)
9. Implement frontend-development skill (100 lines)
10. Validate: Each has YAML frontmatter, follows structure, <500 lines

**Phase 2: Commands (Week 1)**
1. Implement work-plan command
2. Implement challenge command
3. Implement beads-start command
4. Implement beads-end command
5. Implement test-protocol command
6. Implement review command
7. Implement spawn-agents command
8. Implement optimize-context command
9. Implement validate-requirements command
10. Implement assume-check command
11. Implement dependency-graph command
12. Validate: Each has frontmatter, loads skills, provides instructions

**Phase 3: Hooks (Week 1)**
1. Implement pre-tool-use.js (test blocking)
2. Implement session-start.js (agent initialization)
3. Implement session-end.js (state export)
4. Implement user-prompt-submit.js (vague detection)
5. Implement pre-compact.js (state preservation)
6. Validate: Each exports hook function, returns correct structure

**Phase 4: CLAUDE.md (Week 2)**
1. Create optimized structure (6 sections)
2. Write Section 1: Multi-Agent System (200 lines)
3. Write Section 2: Critical Thinking (200 lines)
4. Write Section 3: Work Plan Protocol (150 lines)
5. Write Section 4: Integration & Workflows (100 lines)
6. Write Section 5: Skills & Commands Reference (50 lines)
7. Write Section 6: Quick Reference (100 lines)
8. Add Implementation Status section (50 lines)
9. Add cross-references throughout
10. Validate: <1000 lines, all critical content retained

**Phase 5: Documentation & Tools (Week 2)**
1. Write README.md (comprehensive overview)
2. Write IMPLEMENTATION_SUMMARY.md (what was built)
3. Write SPECIFICATION.md (this document)
4. Create .claude/agents/README.md (stub status)
5. Create .claude/system/README.md (stub status)
6. Create .claude/bin/claude-agent CLI tool
7. Validate: All docs complete, CLI functional

### 8.3 Validation Checklist

After implementation, verify:

**Skills**:
- [ ] 8 skills created with YAML frontmatter
- [ ] Each skill has name, description, allowed-tools, version
- [ ] Each skill follows structure guidelines
- [ ] Each skill has practical examples
- [ ] Each skill has quick reference section
- [ ] Each skill is <500 lines (preferred) or <800 lines (max)

**Commands**:
- [ ] 11 commands created with frontmatter
- [ ] Each command has description
- [ ] Each command loads relevant skills
- [ ] Each command provides clear instructions
- [ ] Each command references related commands/skills

**Hooks**:
- [ ] 5 hooks created in JavaScript
- [ ] Each hook exports async function named 'hook'
- [ ] Each hook accepts context parameter
- [ ] Each hook returns {block, message} structure
- [ ] pre-tool-use.js blocks tests on uncommitted code
- [ ] session-start.js initializes agents
- [ ] session-end.js exports beads state
- [ ] user-prompt-submit.js detects vague terms
- [ ] pre-compact.js preserves state

**CLAUDE.md**:
- [ ] 6 main sections present
- [ ] Multi-agent architecture complete
- [ ] Critical thinking framework complete
- [ ] Work Plan Protocol complete
- [ ] Cross-references to skills/commands/hooks throughout
- [ ] Quick reference section with decision tree
- [ ] Implementation status section
- [ ] Size: 600-1000 lines

**Documentation**:
- [ ] README.md explains system overview
- [ ] IMPLEMENTATION_SUMMARY.md lists what was built
- [ ] SPECIFICATION.md (this document) complete
- [ ] Agent and system READMEs present

**Tools**:
- [ ] claude-agent CLI created and executable
- [ ] Shows status, skills, commands, hooks

**Integration**:
- [ ] Directory structure matches specification
- [ ] All files in correct locations
- [ ] No broken references in CLAUDE.md
- [ ] System is testable (can invoke commands)

---

## 9. Validation & Testing

### 9.1 Functional Testing

**Test 1: Slash Commands**
```
Action: Type /work-plan test-project in Claude Code
Expected: Claude loads work-plan-protocol and critical-thinking-framework skills,
          asks 2-3+ clarifying questions, creates initial spec
Validation: ✅ Skills loaded, ✅ Questions asked, ✅ Spec created
```

**Test 2: Critical Thinking Enforcement**
```
Action: Submit vague prompt "Make it user-friendly"
Expected: user-prompt-submit.js hook triggers, suggests /challenge
Validation: ✅ Hook triggers, ✅ Vague terms detected, ✅ Suggestion provided
```

**Test 3: Testing Protocol Enforcement**
```
Action: Attempt to run pytest without committing changes
Expected: pre-tool-use.js hook blocks with clear error message
Validation: ✅ Test blocked, ✅ Git status shown, ✅ Instructions provided
```

**Test 4: Beads Integration**
```
Action: Run /beads-start
Expected: Claude installs/verifies bd, imports state, shows ready work
Validation: ✅ bd installed, ✅ State imported, ✅ Ready work shown
```

**Test 5: Context Optimization**
```
Action: Run /optimize-context when context >75%
Expected: Claude audits context, identifies issues, applies optimization
Validation: ✅ Audit performed, ✅ Issues identified, ✅ Actions taken
```

**Test 6: Agent Coordination**
```
Action: Run /spawn-agents with 3 independent tasks
Expected: Claude validates safety, spawns 3 sub-agents, integrates results
Validation: ✅ Safety checks passed, ✅ Sub-agents spawned, ✅ Integration successful
```

### 9.2 Integration Testing

**Test 1: Work Plan Protocol Flow**
```
Scenario: New project from start to artifact generation
Steps:
  1. /work-plan Build user authentication
  2. Answer clarifying questions (Phase 1)
  3. Review initial spec
  4. Proceed through Phase 2 (full spec)
  5. Proceed through Phase 3 (execution plan)
  6. Proceed through Phase 4 (beads generation)
Expected: All phases complete with exit criteria met, beads created
Validation: ✅ All phases completed, ✅ All artifacts created, ✅ Exit criteria met
```

**Test 2: Multi-Session Workflow**
```
Scenario: Start work, end session, resume later
Steps:
  1. Create beads for work
  2. Claim bead and start work
  3. /beads-end (export state)
  4. End session
  5. New session: /beads-start (import state)
  6. Resume work on bead
Expected: State preserved and restored correctly
Validation: ✅ State exported, ✅ State imported, ✅ Work resumable
```

**Test 3: Testing Discipline Workflow**
```
Scenario: Implement feature with testing
Steps:
  1. Implement feature
  2. Attempt tests without committing (should block)
  3. Commit changes
  4. Run tests (should allow)
Expected: Hook blocks uncommitted tests, allows committed tests
Validation: ✅ Block on uncommitted, ✅ Allow on committed
```

### 9.3 Quality Metrics

**Metric 1: Context Efficiency**
- Measure: CLAUDE.md line count
- Target: <1000 lines (vs. >1500 original)
- Validation: Count lines in CLAUDE.md

**Metric 2: Modularity**
- Measure: Skills are separate, reusable files
- Target: 8 skills with YAML frontmatter
- Validation: Count .claude/skills/*/SKILL.md files

**Metric 3: Enforcement**
- Measure: Hooks block violations deterministically
- Target: 100% block rate for violations
- Validation: Test pre-tool-use.js blocks tests without commit

**Metric 4: Completeness**
- Measure: All critical thinking logic retained
- Target: 100% retention
- Validation: Compare original vs. optimized critical thinking sections

**Metric 5: Usability**
- Measure: Slash commands functional
- Target: 11 commands work correctly
- Validation: Test each command

---

## 10. Success Metrics

### 10.1 Implementation Success

System is considered successfully implemented when:

✅ **All 8 skills created** with YAML frontmatter and following structure
✅ **All 11 commands created** with frontmatter and clear instructions
✅ **All 5 hooks created** and enforce protocols correctly
✅ **CLAUDE.md optimized** to <1000 lines with all critical content retained
✅ **Documentation complete** (README, SPEC, SUMMARY)
✅ **CLI tools functional** (claude-agent works)
✅ **Directory structure matches** specification exactly
✅ **All tests pass** (functional, integration, quality)

### 10.2 Operational Success

System is considered operationally successful when:

✅ **Critical thinking enforced** - Minimum 2-3 questions asked in Phase 1
✅ **Testing discipline enforced** - Tests blocked on uncommitted code
✅ **Work Plan Protocol followed** - All phases complete with exit criteria
✅ **Context optimized** - Density >60%, preserved at 75% utilization
✅ **Multi-session support** - State preserved across sessions via beads
✅ **Parallel work functional** - Sub-agents spawnable with safety checks
✅ **Quality maintained** - Reviewer validates all work before completion

### 10.3 User Success

System is considered successful from user perspective when:

✅ **Slash commands work** - User can type /work-plan and get guided workflow
✅ **Hooks provide value** - User prevented from common mistakes
✅ **Skills auto-load** - User doesn't manually load, Optimizer does it
✅ **Documentation clear** - User can understand system from README
✅ **System feels cohesive** - Skills, commands, hooks work together seamlessly
✅ **Productivity increased** - User completes work faster with fewer mistakes
✅ **Quality improved** - User produces better output with critical thinking

### 10.4 Metrics Dashboard

Track these metrics over time:

**Context Metrics**:
- Context density: >60%
- Context utilization: Peak at 75% before optimization
- Recovery success rate: 100%

**Critical Thinking Metrics**:
- Questions asked per Phase 1: ≥2-3
- Assumptions flagged: Track count
- Vague requirements detected: Track count

**Quality Metrics**:
- Reviewer approval rate: >80%
- Tests blocked on uncommitted code: 100%
- Exit criteria met before phase transition: 100%

**Efficiency Metrics**:
- Parallelization usage: 50%+ of eligible work
- Time savings from parallelization: ≥30%
- Multi-session work resumption: 100% success

---

## Appendix A: File Templates

### A.1 Skill Template

```markdown
---
name: skill-name
description: >
  One to three sentence description of what this skill does and when to use it.
  This is critical for auto-discovery by the Optimizer agent.
allowed-tools: [Tool1, Tool2]
version: 1.0
---

# Skill Title

## Overview

Brief introduction to the skill.

## Section 1

Content...

## Section 2

Content...

## Quick Reference

- Key point 1
- Key point 2
```

### A.2 Command Template

```markdown
---
description: Brief description of what command does
argument-hint: "[arg1] [arg2]"
---

# Command Title

Context about what this command does.

## Load Skills

- skill-name-1
- skill-name-2

## Actions

1. First action
2. Second action

## Expected Output

What Claude should produce.
```

### A.3 Hook Template

```javascript
/**
 * Hook Name
 *
 * Description of what this hook enforces
 */

export async function hook(context) {
  const { tool, parameters } = context;

  // Logic here

  return {
    block: false,  // or true
    message: "Optional message"
  };
}
```

---

## Appendix B: Reference Implementation

See actual implementation in this repository:

- **Skills**: `.claude/skills/*/SKILL.md`
- **Commands**: `.claude/commands/*.md`
- **Hooks**: `.claude/hooks/*.js`
- **CLAUDE.md**: `CLAUDE-optimized.md`
- **Documentation**: `README.md`, `IMPLEMENTATION_SUMMARY.md`

Total implementation: ~4,000 lines of code across 27+ files.

---

## Appendix C: Troubleshooting

**Issue**: Skills not loading automatically
**Solution**: Verify YAML frontmatter has clear description field

**Issue**: Slash commands not working
**Solution**: Check commands are in `.claude/commands/` with `.md` extension

**Issue**: Hooks not enforcing
**Solution**: Verify hooks are in `.claude/hooks/` with `.js` extension and export `hook` function

**Issue**: Beads state lost between sessions
**Solution**: Ensure `/beads-end` is called to export state, verify `.beads/issues.jsonl` committed

**Issue**: Context loss at 90%
**Solution**: Implement proactive preservation at 75% as specified in context-optimization skill

---

## Document History

- **v1.0** (2025-01-26): Initial specification extracted from implementation
- **v2.0** (2025-01-26): Comprehensive specification with all components detailed

---

## End of Specification

This specification is complete and sufficient to recreate the multi-agent orchestration system from scratch. Follow the implementation guide in Section 8, validate using Section 9, and measure success using Section 10.
