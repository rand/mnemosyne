---
name: feature-tasks
description: Break down implementation plan into Beads tasks with dependencies
---

I will help you convert a feature implementation plan into concrete, trackable Beads tasks with proper dependencies and estimates.

**Usage**:
- `/feature-tasks <feature-id>` - Create tasks from plan
- `/feature-tasks --show <feature-id>` - Display existing tasks
- `/feature-tasks --sync <feature-id>` - Sync plan changes to tasks

**Instructions for me**:

1. **Load implementation plan**:
   - Read `.mnemosyne/artifacts/plans/<feature-id>-plan.md`
   - If not found: "Error: Plan '<feature-id>' not found. Use /feature-plan first."
   - Parse YAML frontmatter to get plan version, memory_id, feature_id
   - Extract implementation order, parallel streams, dependencies

2. **Check for existing tasks**:
   - Query Beads for tasks with tag `feature:<feature-id>`
   - If exists:
     - If `--show`: Display task summary and exit
     - If `--sync`: Update tasks based on plan changes
     - Otherwise: Ask if user wants to view, sync, or recreate

3. **Analyze plan structure**:
   From plan.md, extract:
   - **Implementation Order**: Ordered list of steps
   - **Parallel Streams**: Independent workstreams
   - **Dependencies**: Task X depends on Task Y relationships
   - **Integration Points** (Typed Holes): Interfaces between components
   - **Testing Requirements**: Test types for each component

4. **Generate Beads task structure**:
   For each plan item:

   **a) Task Title**:
   - Format: `[<feature-id>] <step-name>: <brief-description>`
   - Example: `[jwt-auth] Implement JWT Generation: RS256 signing with refresh tokens`
   - Max 100 characters

   **b) Task Description**:
   ```markdown
   ## Goal
   <What needs to be done>

   ## Technical Details
   <From plan: approach, data models, APIs>

   ## Acceptance Criteria
   - [ ] <criterion 1 from plan>
   - [ ] <criterion 2 from plan>
   - [ ] Tests: <test requirements>

   ## Dependencies
   - Requires: <task-id> (<task-name>)
   - Blocks: <task-id> (<task-name>)

   ## Integration Points
   - <Typed hole to implement or consume>

   ## Estimated Effort
   <hours or days based on plan>

   ## Related Files
   - <files from plan>

   ## Notes
   <Any additional context from plan>
   ```

   **c) Priority**:
   - P0 (critical): On critical path, blocks multiple tasks
   - P1 (high): Important but not blocking
   - P2 (medium): Nice-to-have, can defer
   - P3 (low): Future enhancement

   Determine priority from:
   - Critical path items: P0
   - Parallel stream items: P1
   - Polish/refinement: P2
   - Future work: P3

   **d) Effort Estimate**:
   - Parse from plan's implementation order
   - Default: 4 hours for typical tasks
   - Adjust based on complexity indicators:
     - "Foundation" or "Core": 4-8 hours
     - "Integration": 2-4 hours
     - "Polish" or "Refinement": 1-2 hours
     - "Testing": 2-4 hours

   **e) Type**:
   - feature: New functionality
   - refactor: Code improvement
   - test: Test addition
   - doc: Documentation
   - bug: Fix (if plan mentions fixing issues)

   **f) Tags**:
   - `feature:<feature-id>`
   - `plan:<plan-version>`
   - `<category>` (from plan: auth, api, database, etc.)

5. **Handle dependencies**:
   - Parse "Dependencies" section from plan
   - For "Task X depends on Task Y":
     - Create Task Y first (if not exists)
     - Add `--blocked-by <task-y-id>` to Task X
   - For parallel streams:
     - No blocking relationships within stream
     - May depend on foundation tasks
   - For critical path:
     - Each task blocks next task

6. **Create tasks in Beads** (auto-synced):
   ```bash
   # Foundation task (no dependencies) - Hash ID assigned
   bd create "[jwt-auth] Database Schema: Create users table with auth fields" \
     -d "$(cat task_desc.md)" \
     -t feature -p 0 -l "feature:jwt-auth,plan:1.0.0,database" \
     --json
   # Returns: {"id": "bd-a1b2", ...}

   # Task with dependency (using hash ID)
   bd create "[jwt-auth] JWT Generation: Implement RS256 signing" \
     -d "$(cat task_desc.md)" \
     -t feature -p 0 -l "feature:jwt-auth,plan:1.0.0,auth" \
     --json
   # Returns: {"id": "bd-c3d4", ...}

   # Add dependency using bd dep
   bd dep add bd-c3d4 bd-a1b2 --type blocks

   # Parallel stream task
   bd create "[jwt-auth] Middleware: Token validation for protected routes" \
     -d "$(cat task_desc.md)" \
     -t feature -p 1 -l "feature:jwt-auth,plan:1.0.0,middleware" \
     --json
   # Returns: {"id": "bd-e5f6", ...}

   bd dep add bd-e5f6 bd-c3d4 --type blocks

   # All tasks auto-synced to .beads/issues.jsonl (5-second debounce)
   ```

7. **Handle typed holes**:
   For each integration point (typed hole):
   - Create interface definition task (P0, blocks both sides)
   - Create implementation tasks for each side (depend on interface)
   - Link with bd dep
   - Example:
     ```
     bd create "Define TokenValidator interface" -t feature -p 0 --json
     # Returns: bd-a7b8

     bd create "Implement JWT validation" -t feature -p 0 --json
     # Returns: bd-c9d0

     bd create "Add middleware integration" -t feature -p 0 --json
     # Returns: bd-e1f2

     bd dep add bd-c9d0 bd-a7b8 --type blocks
     bd dep add bd-e1f2 bd-a7b8 --type blocks
     ```

8. **Create test tasks**:
   From plan's Testing Strategy:
   - Unit tests: One task per major component
   - Integration tests: One task for integration points
   - E2E tests: One task for complete workflows
   - Performance tests: One task if specified

   Tag with `test` type, link to corresponding feature tasks

9. **Update plan with task IDs**:
   - Update `.mnemosyne/artifacts/plans/<feature-id>-plan.md`
   - Add Tasks section with hash IDs:
     ```markdown
     ## Beads Tasks

     Generated: <timestamp>
     Total: <count> tasks
     ID Format: Hash-based (bd-a1b2)

     ### Critical Path
     - bd-a1b2: Database Schema (P0, 4h)
     - bd-c3d4: JWT Generation (P0, 6h) [blocks: bd-a1b2]
     - bd-e5f6: Middleware Integration (P0, 4h) [blocks: bd-c3d4]

     ### Parallel Streams
     #### Stream A: Core Auth
     - bd-g7h8: Password Hashing (P1, 2h) [blocks: bd-a1b2]
     - bd-i9j0: Login Endpoint (P1, 3h) [blocks: bd-g7h8]

     #### Stream B: Testing
     - bd-k1l2: Unit Tests (P1, 4h) [blocks: bd-c3d4]
     - bd-m3n4: Integration Tests (P1, 3h) [blocks: bd-e5f6]

     ### Total Estimated Effort
     26 hours (critical path: 14h, parallel: +12h)
     ```

10. **Display summary**:
    ```
    ✓ Beads tasks created successfully (auto-synced)

    Feature ID: <feature-id>
    Plan: .mnemosyne/artifacts/plans/<feature-id>-plan.md
    Total Tasks: <count>
    ID Format: Hash-based (bd-a1b2, bd-c3d4, ...)

    Tasks by Priority:
    - P0 (Critical): <count> tasks (<hours>h)
    - P1 (High): <count> tasks (<hours>h)
    - P2 (Medium): <count> tasks (<hours>h)

    Critical Path:
    1. bd-a1b2: Database Schema (4h)
    2. bd-c3d4: JWT Generation (6h) → blocks: bd-a1b2
    3. bd-e5f6: Middleware Integration (4h) → blocks: bd-c3d4

    Critical Path Duration: 14 hours

    Parallel Streams: 2 streams
    - Stream A (Core Auth): 5 hours
    - Stream B (Testing): 7 hours

    Estimated Total Time: 14h (critical path)
    With parallelization: ~17h (critical + longest parallel stream)

    Auto-Sync: ✓ Tasks synced to .beads/issues.jsonl

    Next steps:
    - Commit: ./scripts/beads-sync.sh commit
    - View tasks: bd list --label feature:<feature-id> --json
    - Check dependencies: bd dep tree bd-a1b2
    - Start work: bd update bd-a1b2 --status in_progress --json
    - Track progress: /feature-checklist <feature-id>
    - See workflow: docs/BEADS_INTEGRATION.md
    ```

11. **Error handling**:
    - If plan not found: "Error: Plan '<feature-id>' not found. Run /feature-plan first."
    - If tasks already exist: Offer to view, sync, or recreate
    - If Beads not available: "Error: Beads CLI not installed. Install:\n  npm install -g @beads/bd\n  OR: curl -fsSL https://raw.githubusercontent.com/steveyegge/beads/main/scripts/install.sh | bash\n  Then run: bd init"
    - If Beads not initialized: "Error: Run 'bd init' first to set up Beads."
    - If dependencies circular: Detect and report circular dependency chain with bd dep tree
    - If estimate missing: Use default 4h and log warning

**Special behaviors**:
- `--show <feature-id>`: Display task summary without creating
- `--sync <feature-id>`: Update tasks based on plan changes (increment plan version)
- Smart defaults: Infer priorities from plan structure
- Dependency validation: Detect and prevent circular dependencies
- Parallel detection: Identify truly independent tasks for parallel execution
- Effort aggregation: Sum estimates for realistic project timeline

**Examples**:
```
/feature-tasks jwt-authentication
/feature-tasks --show jwt-authentication
/feature-tasks --sync jwt-authentication
```

Please proceed to create Beads tasks from the implementation plan.
