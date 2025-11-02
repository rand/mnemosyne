---
name: feature-checklist
description: Interactive task execution workflow for feature implementation
---

I will guide you through executing tasks for a feature implementation plan interactively, tracking progress in Beads and updating the plan as tasks are completed.

**Usage**:
- `/feature-checklist <feature-id>` - Start interactive checklist for feature
- `/feature-checklist <feature-id> --status` - Show current progress only
- `/feature-checklist <feature-id> --resume` - Resume from last checkpoint

**Instructions for me**:

## 1. Load Feature Context

**Load plan and tasks**:
```bash
# Read implementation plan
cat .mnemosyne/artifacts/plans/<feature-id>-plan.md

# Get Beads tasks for feature
bd list --tags feature:<feature-id> --json
```

**Validate**:
- If plan not found: "Error: Plan '<feature-id>' not found. Run /feature-plan first."
- If no Beads tasks: "Error: No tasks found for '<feature-id>'. Run /feature-tasks first."
- If both exist: Proceed to display

## 2. Display Interactive Checklist

**Format**:
```
Feature: <feature-name>
Plan: .mnemosyne/artifacts/plans/<feature-id>-plan.md
Tasks: <total> (<completed> complete, <in-progress> in progress, <open> ready)

Critical Path:
  ✅ bd-42: Database Schema (4h) - COMPLETE
  🔄 bd-43: JWT Generation (6h) - IN PROGRESS
  ⏸️  bd-44: Middleware Integration (4h) - BLOCKED by bd-43
  ⬜ bd-45: Testing (3h) - READY

Parallel Streams:
  Stream A (Core Auth):
    ✅ bd-46: Password Hashing (2h) - COMPLETE
    🔄 bd-47: Login Endpoint (3h) - IN PROGRESS

  Stream B (Documentation):
    ⬜ bd-48: API Documentation (2h) - READY
    ⬜ bd-49: Integration Guide (1h) - BLOCKED by bd-48

Progress: ████████░░░░░░░░ 40% (2/5 critical path)
Estimated Time Remaining: 12h (critical path), 6h (parallel)

Next Ready Tasks:
  1. bd-43 (IN PROGRESS): JWT Generation (6h)
  2. bd-48 (READY): API Documentation (2h)

What would you like to do?
  [c] Continue current task (bd-43)
  [s] Start next ready task (bd-48)
  [v] View task details (enter task ID)
  [p] Show full progress
  [q] Quit checklist mode
```

**Interactive Mode**:
- User selects action
- I respond accordingly
- Update Beads state as tasks progress
- Track actual vs estimated time

## 3. Task Execution Flow

**Continue current task** (`[c]`):
```
Continuing: bd-43 - JWT Generation

From plan:
  Goal: Implement RS256 JWT token generation with refresh tokens

  Technical Details:
  - Use `jsonwebtoken` crate for RS256 signing
  - Generate keypair: openssl genrsa -out private.pem 2048
  - Public key for validation: openssl rsa -in private.pem -pubout -out public.pem
  - Token claims: user_id, email, exp (15 min), iat, jti
  - Refresh tokens: 7 day expiry, stored in database

  Acceptance Criteria:
  - [ ] Generate RS256 signed access tokens (15 min TTL)
  - [ ] Generate refresh tokens (7 day TTL)
  - [ ] Store refresh tokens in database
  - [ ] Include user_id, email in claims
  - [ ] Tests: Token generation, expiry validation, refresh flow

  Integration Points:
  - Consumes: Database schema (bd-42)
  - Provides: TokenValidator interface for middleware (bd-44)

Proceeding with implementation...
[I implement the code, write tests, commit]

Task complete! Updating Beads...
bd close bd-43 --reason "Complete" --json

✅ bd-43 COMPLETE (actual: 5.5h vs estimated: 6h)

Next ready tasks:
  1. bd-44: Middleware Integration (4h) - NOW UNBLOCKED
  2. bd-48: API Documentation (2h)

Continue? [y/n]
```

**Start next ready task** (`[s]`):
```
Starting: bd-48 - API Documentation

From plan:
  Goal: Document JWT authentication endpoints and flows
  [... similar detail display ...]

Proceeding with implementation...
```

**View task details** (`[v]`):
```
Enter task ID: bd-44

Task: bd-44 - Middleware Integration
Status: BLOCKED (waiting for bd-43)
Priority: P0 (Critical Path)
Estimated: 4h
Tags: feature:jwt-auth, plan:1.0.0, middleware

Goal:
  Create Express/Axum middleware for JWT validation on protected routes

Technical Details:
  - Extract token from Authorization header (Bearer <token>)
  - Validate signature using public key
  - Check expiration (exp claim)
  - Add user context to request
  - Return 401 for invalid/expired tokens

Dependencies:
  - Requires: bd-43 (JWT Generation) - IN PROGRESS
  - Blocks: bd-50 (Protected Endpoints)

Integration Points:
  - Consumes: TokenValidator interface from bd-43
  - Provides: AuthMiddleware for route protection

Acceptance Criteria:
  - [ ] Validate Bearer token from header
  - [ ] Check signature and expiration
  - [ ] Add user to request context
  - [ ] Return 401 for auth failures
  - [ ] Tests: Valid token, expired token, invalid signature, missing token

This task will become READY when bd-43 is complete.

[Back to checklist]
```

## 4. Progress Tracking

**Show full progress** (`[p]`):
```
Feature: jwt-authentication
Plan Version: 1.0.0
Started: 2025-11-02 14:30
Elapsed: 9.5h

Task Breakdown:
  Total: 8 tasks
  Complete: 3 (38%)
  In Progress: 2 (25%)
  Ready: 2 (25%)
  Blocked: 1 (12%)

Critical Path Progress:
  ✅ bd-42: Database Schema (4h actual vs 4h est)
  ✅ bd-43: JWT Generation (5.5h actual vs 6h est)
  🔄 bd-44: Middleware Integration (0h / 4h est)
  ⬜ bd-45: Testing (3h est)

  Critical Path: 40% complete (9.5h / 17h)

Parallel Streams Progress:
  Stream A (Core Auth): 50% (1/2 complete)
  Stream B (Documentation): 0% (0/2 complete)

Velocity:
  Estimated: 17h total
  Actual: 9.5h spent (56% of estimate)
  Remaining: ~7.5h (based on current velocity: 1.05x estimate)
  Projected Completion: 2025-11-03 10:00 (tomorrow)

Time Tracking:
  bd-42: 4h (100% of 4h est) - efficient
  bd-43: 5.5h (92% of 6h est) - faster than expected
  bd-47: 2.5h (in progress, 50% of 5h est)

[Back to checklist]
```

## 5. Task State Management

**Update task status in Beads**:
```bash
# Mark task in progress
bd update <task-id> --status in_progress --json

# Add progress comment
bd update <task-id> --comment "Implemented core logic, writing tests" --json

# Mark complete
bd close <task-id> --reason "Complete" --json

# Track actual time
bd update <task-id> --comment "Completed in 5.5h (estimated 6h)" --json
```

**Update plan with actual times**:
- Append "Actual: Xh" to task entries in plan.md
- Update progress percentages
- Record velocity metrics

## 6. Completion Criteria

**Task complete when**:
- [ ] All acceptance criteria checked
- [ ] Tests written and passing
- [ ] Code committed
- [ ] Integration points satisfied (typed holes filled)
- [ ] Beads task closed

**Feature complete when**:
- [ ] All critical path tasks complete
- [ ] All parallel stream tasks complete
- [ ] Integration tests passing
- [ ] Documentation complete
- [ ] Plan updated with actuals

**On feature completion**:
```
🎉 Feature 'jwt-authentication' COMPLETE!

Summary:
  Total Time: 16.5h (estimated 17h, 97% accurate)
  Tasks: 8/8 complete
  Critical Path: 100%
  Parallel Streams: 100%

Velocity Metrics:
  Average: 0.97x (slightly faster than estimated)
  Range: 0.85x - 1.1x
  Most accurate: Database tasks
  Least accurate: Testing tasks (1.3x estimate)

Next Steps:
  - Export Beads state: ./scripts/beads-sync.sh commit
  - Update spec status: echo "Status: COMPLETE" >> .mnemosyne/artifacts/specs/<feature-id>.md
  - Create PR: /create-pr <feature-id>
  - Archive plan: git add .mnemosyne/artifacts/plans/<feature-id>-plan.md

Lessons Learned (for future estimates):
  - JWT implementation faster than expected (92% of estimate)
  - Testing took longer (130% of estimate)
  - Database work highly predictable
```

## 7. Checkpoint and Resume

**Checkpoint state** (auto-save every 30 min):
```bash
# Save checkpoint to plan frontmatter
cat >> .mnemosyne/artifacts/plans/<feature-id>-plan.md <<EOF
---
checkpoint:
  timestamp: 2025-11-02T16:30:00
  current_task: bd-43
  elapsed: 9.5h
  completed: [bd-42, bd-46]
  in_progress: [bd-43, bd-47]
---
EOF
```

**Resume from checkpoint** (`--resume`):
```
Resuming feature 'jwt-authentication'

Last checkpoint: 2025-11-02 16:30 (30 minutes ago)
Elapsed: 9.5h
Current task: bd-43 (JWT Generation)

Progress since checkpoint:
  - bd-43: Still in progress (no new commits)
  - bd-47: Still in progress (1 new commit)

Would you like to:
  [c] Continue where you left off
  [s] Start a different ready task
  [p] Show current progress
```

## 8. Error Handling

**Circular dependencies detected**:
```
⚠️  Warning: Circular dependency detected!

  bd-44 depends on bd-43
  bd-43 depends on bd-50
  bd-50 depends on bd-44

This is likely a typed hole that needs an interface definition.

Suggested fix:
  1. Create interface task: bd-51 "Define TokenValidator interface"
  2. Make bd-43 and bd-44 both depend on bd-51
  3. bd-51 should have no dependencies

Run: /fix-circular-dependency <feature-id>
```

**Task failing repeatedly**:
```
⚠️  Task bd-43 marked in_progress for 4 hours with no completion

Possible issues:
  - Estimated time too low (6h, consider 8-10h)
  - Blocking issue not captured
  - Scope creep

Actions:
  [r] Revise estimate
  [b] Mark as blocked (add blocker)
  [s] Split into subtasks
  [c] Continue (it's almost done)
```

**Dependencies out of sync**:
```
⚠️  bd-44 marked READY but depends on bd-43 (still in progress)

Beads dependency graph differs from plan.

Fix:
  bd update bd-44 --blocked-by bd-43 --json
```

## 9. Reporting

**Generate completion report** (on feature complete):
```markdown
# Feature Implementation Report: jwt-authentication

**Status**: COMPLETE
**Completed**: 2025-11-03 10:15
**Duration**: 16.5 hours (estimated 17h)
**Accuracy**: 97%

## Task Breakdown

| Task | Estimated | Actual | Variance | Status |
|------|-----------|--------|----------|--------|
| bd-42: Database Schema | 4h | 4h | 100% | ✅ |
| bd-43: JWT Generation | 6h | 5.5h | 92% | ✅ |
| bd-44: Middleware | 4h | 4.5h | 112% | ✅ |
| bd-45: Testing | 3h | 3.9h | 130% | ✅ |
| bd-46: Password Hash | 2h | 2h | 100% | ✅ |
| bd-47: Login Endpoint | 3h | 3.5h | 117% | ✅ |
| bd-48: Docs | 2h | 1.5h | 75% | ✅ |
| bd-49: Integration Guide | 1h | 1.1h | 110% | ✅ |

## Velocity Analysis

**Average**: 0.97x (3% faster than estimates)
**Critical Path**: 1.05x (5% slower)
**Parallel Streams**: 0.88x (12% faster)

**Most Accurate**: Database tasks (100%)
**Least Accurate**: Testing (+30% overrun)

## Recommendations

1. **Testing Estimates**: Increase by 20-30% for authentication features
2. **Documentation**: Decrease by 10-15% (faster than expected)
3. **Critical Path Buffer**: Add 10% contingency for dependencies

## Artifacts

- Plan: `.mnemosyne/artifacts/plans/jwt-auth-plan.md`
- Spec: `.mnemosyne/artifacts/specs/jwt-auth.md`
- Tasks: 8 Beads tasks (all closed)
- Commits: 12 commits on `feature/jwt-authentication`

---
Generated by /feature-checklist
</markdown>

Save to: `.mnemosyne/artifacts/reports/<feature-id>-report.md`
```

## 10. Advanced Features

**Parallel execution** (when safe):
```
Multiple ready tasks detected:
  1. bd-48: API Documentation (Stream B)
  2. bd-50: Rate Limiting (Stream C)

These tasks have no dependencies on each other.

Execute in parallel?
  [y] Yes - I'll work on both simultaneously
  [n] No - Sequential execution
  [1/2] Choose which to start first

[If parallel]
I'll track both tasks as "in_progress" and context-switch
between them, marking progress on each.
```

**Smart suggestions** (ML-driven, future):
```
💡 Suggestion: Start bd-48 (Documentation) while waiting for CI on bd-43

Reasoning:
  - bd-43 tests running (ETA 5 min)
  - bd-48 is independent (Stream B)
  - bd-48 is quick (2h estimate)
  - Maximizes parallelization

Accept suggestion? [y/n]
```

## Usage Examples

```bash
# Start interactive checklist
/feature-checklist jwt-authentication

# Just show status
/feature-checklist jwt-authentication --status

# Resume from checkpoint
/feature-checklist jwt-authentication --resume
```

## Integration with Other Commands

- **Before**: `/feature-specify` → `/feature-clarify` → `/feature-plan` → `/feature-tasks`
- **During**: `/feature-checklist` (this command)
- **After**: `/create-pr`, `./scripts/beads-sync.sh commit`

---

**Key Behaviors**:
- Interactive, step-by-step guidance
- Track actual vs estimated time
- Update Beads state in real-time
- Display progress visually
- Handle errors gracefully
- Generate completion reports
- Support checkpointing and resume

Please start the interactive checklist for the specified feature.
