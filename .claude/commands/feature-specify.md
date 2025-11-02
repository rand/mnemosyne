---
name: feature-specify
description: Create feature specification with user scenarios and requirements
---

I will help you create a feature specification following the Spec-Kit workflow. This will capture user scenarios, requirements, and acceptance criteria in a structured format.

**Usage**:
- `/feature-specify <brief description>` - Create new feature spec
- `/feature-specify --from-branch` - Create spec from current git branch
- `/feature-specify --show <feature-id>` - Display existing spec

**Instructions for me**:

1. **Generate feature ID**:
   - If `--from-branch`: Use current git branch name
     ```bash
     git branch --show-current
     ```
   - Otherwise: Convert description to kebab-case (e.g., "JWT Authentication" → "jwt-authentication")
   - Max 50 characters
   - Format: `{category}-{name}` (e.g., `auth-jwt`, `ui-dashboard`, `api-rate-limiting`)

2. **Check for existing spec**:
   - Look for `.mnemosyne/artifacts/specs/<feature-id>.md`
   - If exists: Ask if user wants to update (increment version) or view
   - If updating: Parse current version and increment

3. **Gather feature details** (if creating new):
   Ask the user about:

   **a) Feature Overview**:
   - Feature name (human-readable)
   - Brief description (1-2 sentences)
   - Why this feature? (business value)

   **b) User Scenarios** (prioritized):
   For each scenario (start with P0/P1):
   - Priority: P0 (critical), P1 (high), P2 (medium), P3 (nice-to-have)
   - Actor: "As a [user type]..."
   - Goal: "I want [capability]..."
   - Benefit: "So that [value]..."
   - Acceptance Criteria: 3-7 specific, testable criteria

   **c) Requirements**:
   - Functional requirements
   - Non-functional requirements (performance, security, etc.)
   - Constraints or limitations

   **d) Success Criteria**:
   - How will we know this is successful?
   - What metrics matter?

4. **Check constitution alignment**:
   - Load `.mnemosyne/artifacts/constitution/project-constitution.md`
   - Verify feature aligns with core principles
   - Flag any conflicts (e.g., performance requirement vs. constitution)
   - If misaligned: Ask user to clarify or adjust

5. **Format spec as markdown**:
   ```markdown
   ---
   type: feature_spec
   id: <feature-id>
   name: <feature-name>
   branch: <branch-name>
   version: 1.0.0
   status: draft
   created_at: <ISO 8601 timestamp>
   updated_at: <ISO 8601 timestamp>
   memory_id: <will be filled after storage>
   references: [<constitution-memory-id>]
   ---

   # Feature: <feature-name>

   ## Overview

   <brief description>

   **Business Value**: <why this matters>

   ## User Scenarios (Prioritized)

   ### P0: <Scenario Name>

   **As a** <actor>
   **I want** <goal>
   **So that** <benefit>

   **Acceptance Criteria**:
   - [ ] <criterion 1>
   - [ ] <criterion 2>
   - [ ] <criterion 3>

   ### P1: <Scenario Name>

   ...

   ## Requirements

   ### Functional
   - <requirement>

   ### Non-Functional
   - **Performance**: <requirement>
   - **Security**: <requirement>
   - **Scalability**: <requirement>

   ### Constraints
   - <constraint>

   ## Success Criteria

   - <criterion>

   ## Open Questions

   - [ ] <question needing clarification>

   ## Related

   - Constitution: `.mnemosyne/artifacts/constitution/project-constitution.md`
   - Parent Feature: <if sub-feature>
   ```

6. **Write spec file**:
   - Create `.mnemosyne/artifacts/specs/<feature-id>.md`
   - Ensure `.mnemosyne/artifacts/specs/` directory exists

7. **Store memory entry**:
   - Use Mnemosyne CLI: `mnemosyne remember`
   - Arguments:
     - Content: Feature overview + first P0 scenario + "...see .mnemosyne/artifacts/specs/<feature-id>.md for full spec"
     - Namespace: `project:<project-name>` (detect from git)
     - Importance: 8 (specs are important)
     - Type: feature_spec
     - Tags: spec,feature,<feature-id>
     - Context: "Feature specification for <feature-name>"
   - Capture memory_id from output

8. **Create memory link to constitution**:
   - Get constitution memory_id from `.mnemosyne/artifacts/constitution/project-constitution.md` frontmatter
   - Link spec → constitution with relationship "builds_upon"
   - Update spec's `references` field in frontmatter

9. **Update artifact with memory_id**:
   - Update `memory_id` field in spec's YAML frontmatter
   - Write back to file

10. **Identify ambiguities** (optional auto-clarify):
    - Scan spec for vague terms: "fast", "easy", "secure", "scalable" without metrics
    - Scan for missing details in acceptance criteria
    - If found (max 3): Offer to run `/feature-clarify` immediately

11. **Display confirmation**:
    ```
    ✓ Feature spec created successfully

    Feature ID: <feature-id>
    Name: <feature-name>
    Location: .mnemosyne/artifacts/specs/<feature-id>.md
    Memory ID: <memory-id>
    Branch: <branch-name>

    User Scenarios:
    - P0: <count>
    - P1: <count>
    - P2: <count>

    Constitution Alignment: ✓ Aligned

    Next steps:
    - Review spec: cat .mnemosyne/artifacts/specs/<feature-id>.md
    - Clarify ambiguities: /feature-clarify <feature-id>
    - Create implementation plan: /feature-plan <feature-id>
    - Create git branch: git checkout -b feature/<feature-id>
    ```

12. **Error handling**:
    - If artifacts directory missing: "Error: Run 'mnemosyne artifact init' first"
    - If feature-id already exists: Offer to view, update, or choose new ID
    - If no constitution: Warn "No constitution found. Consider creating one with /project-constitution"
    - If description too vague: Ask for more details

**Special behaviors**:
- `--show <feature-id>`: Display existing spec from file
- Interactive mode: Guide user through each section with examples
- Smart defaults: Pre-fill common patterns based on feature type (auth, API, UI, etc.)
- Validation: Require at least 1 P0 or P1 scenario, 3 acceptance criteria per scenario
- Constitution check: Flag misalignments prominently

**Examples**:
```
/feature-specify JWT authentication for API
/feature-specify Real-time notifications via WebSocket
/feature-specify --from-branch
```

Please proceed to create the feature specification based on user input.
