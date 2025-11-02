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

6. **Create tasks in Beads**:
   ```bash
   # Foundation task (no dependencies)
   bd create "[jwt-auth] Database Schema: Create users table with auth fields" \
     -t feature -p 0 --json \
     --description "$(cat task_desc.md)" \
     --tags "feature:jwt-auth,plan:1.0.0,database"

   # Task with dependency
   bd create "[jwt-auth] JWT Generation: Implement RS256 signing" \
     -t feature -p 0 --json \
     --blocked-by bd-42 \
     --description "$(cat task_desc.md)" \
     --tags "feature:jwt-auth,plan:1.0.0,auth"

   # Parallel stream task
   bd create "[jwt-auth] Middleware: Token validation for protected routes" \
     -t feature -p 1 --json \
     --blocked-by bd-43 \
     --description "$(cat task_desc.md)" \
     --tags "feature:jwt-auth,plan:1.0.0,middleware"
   ```

7. **Handle typed holes**:
   For each integration point (typed hole):
   - Create interface definition task (P0, blocks both sides)
   - Create implementation tasks for each side (depend on interface)
   - Example:
     ```
     bd-50: Define TokenValidator interface (P0)
       ├─> bd-51: Implement JWT validation (depends on bd-50)
       └─> bd-52: Add middleware integration (depends on bd-50)
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
   - Add Tasks section:
     ```markdown
     ## Beads Tasks

     Generated: <timestamp>
     Total: <count> tasks

     ### Critical Path
     - bd-42: Database Schema (P0, 4h)
     - bd-43: JWT Generation (P0, 6h) [depends: bd-42]
     - bd-44: Middleware Integration (P0, 4h) [depends: bd-43]

     ### Parallel Streams
     #### Stream A: Core Auth
     - bd-45: Password Hashing (P1, 2h) [depends: bd-42]
     - bd-46: Login Endpoint (P1, 3h) [depends: bd-45]

     #### Stream B: Testing
     - bd-47: Unit Tests (P1, 4h) [depends: bd-43]
     - bd-48: Integration Tests (P1, 3h) [depends: bd-44]

     ### Total Estimated Effort
     26 hours (critical path: 14h, parallel: +12h)
     ```

10. **Display summary**:
    ```
    ✓ Beads tasks created successfully

    Feature ID: <feature-id>
    Plan: .mnemosyne/artifacts/plans/<feature-id>-plan.md
    Total Tasks: <count>

    Tasks by Priority:
    - P0 (Critical): <count> tasks (<hours>h)
    - P1 (High): <count> tasks (<hours>h)
    - P2 (Medium): <count> tasks (<hours>h)

    Critical Path:
    1. bd-42: Database Schema (4h)
    2. bd-43: JWT Generation (6h) → depends on bd-42
    3. bd-44: Middleware Integration (4h) → depends on bd-43

    Critical Path Duration: 14 hours

    Parallel Streams: 2 streams
    - Stream A (Core Auth): 5 hours
    - Stream B (Testing): 7 hours

    Estimated Total Time: 14h (critical path)
    With parallelization: ~17h (critical + longest parallel stream)

    Next steps:
    - View tasks: bd list --tags feature:<feature-id> --json
    - Start work: bd update bd-42 --status in_progress --json
    - Track progress: /feature-checklist <feature-id>
    - Export state: bd export -o .beads/issues.jsonl
    ```

11. **Error handling**:
    - If plan not found: "Error: Plan '<feature-id>' not found. Run /feature-plan first."
    - If tasks already exist: Offer to view, sync, or recreate
    - If Beads not available: "Error: Beads CLI not installed. Install: go install github.com/steveyegge/beads/cmd/bd@latest"
    - If dependencies circular: Detect and report circular dependency chain
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
