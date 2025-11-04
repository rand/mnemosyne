---
name: project-constitution
description: Create or update project constitution defining principles and quality gates
---

I will help you create or update your project's constitution. This establishes the core principles, quality gates, and constraints that guide all development work.

**Usage**:
- `/project-constitution` - Create new or update existing constitution
- `/project-constitution --show` - Display current constitution

**Instructions for me**:

1. **Check for existing constitution**:
   - Look for `.mnemosyne/artifacts/constitution/project-constitution.md`
   - If exists, parse YAML frontmatter to get current version
   - If updating, increment version appropriately (major for breaking changes, minor for additions, patch for fixes)

2. **Gather constitution content** (if creating new):
   - Ask the user about:
     - **Core Principles**: 3-7 fundamental principles (e.g., "Performance First", "Type Safety", "Developer Experience")
     - **Quality Gates**: Specific, measurable standards (e.g., "90%+ test coverage on critical path", "Sub-200ms p95 latency")
     - **Constraints**: Technical or business constraints (e.g., "Rust-only for performance-critical code", "No external API dependencies")
     - **Architecture Decisions**: Key technical choices (e.g., "Event-sourced for audit trail", "PostgreSQL for primary storage")

3. **Format constitution as markdown**:
   ```markdown
   ---
   type: constitution
   id: constitution
   name: Project Constitution
   version: <semver>
   created_at: <ISO 8601 timestamp>
   updated_at: <ISO 8601 timestamp>
   memory_id: <will be filled after storage>
   references: []
   ---

   # Project Constitution

   ## Core Principles

   1. **[Principle Name]**: [Description]
   2. **[Principle Name]**: [Description]
   ...

   ## Quality Gates

   ### Testing
   - [ ] Critical path: 90%+ coverage
   - [ ] Integration tests for all public APIs
   - [ ] E2E tests for user workflows

   ### Performance
   - [ ] Sub-200ms p95 latency for queries
   - [ ] Memory usage under 100MB baseline

   ### Code Quality
   - [ ] All compiler warnings addressed
   - [ ] No TODO/FIXME in committed code
   - [ ] Type-safe error handling

   ## Constraints

   - **[Constraint Category]**: [Description]
   ...

   ## Architecture Decisions

   - **[Decision Area]**: [Choice and rationale]
   ...
   ```

4. **Write constitution file**:
   - Create/update `.mnemosyne/artifacts/constitution/project-constitution.md`
   - Ensure `.mnemosyne/artifacts/constitution/` directory exists

5. **Store memory entry**:
   - Use Mnemosyne CLI: `mnemosyne remember`
   - Arguments:
     - Content: First 500 chars of constitution + "...see .mnemosyne/artifacts/constitution/project-constitution.md for full text"
     - Namespace: `project:<project-name>` (detect from git root directory name)
     - Importance: 9 (constitutions are critical)
     - Type: constitution
     - Tags: constitution,principles,quality-gates
     - Context: "Project constitution defining core principles and quality gates"
   - Capture memory_id from output

6. **Update artifact with memory_id**:
   - Read the artifact file
   - Update `memory_id` field in YAML frontmatter
   - Write back to file

7. **Display confirmation**:
   ```
   ✓ Constitution created/updated successfully

   Version: <version>
   Location: .mnemosyne/artifacts/constitution/project-constitution.md
   Memory ID: <memory_id>

   Core Principles: <count>
   Quality Gates: <count>
   Constraints: <count>
   Architecture Decisions: <count>

   Next steps:
   - Review with team: cat .mnemosyne/artifacts/constitution/project-constitution.md
   - Create feature spec: /feature-specify <description>
   - Update constitution: /project-constitution
   ```

8. **Error handling**:
   - If `.mnemosyne/artifacts/` doesn't exist: "Error: Artifact directory not initialized. Run 'mnemosyne artifact init' first"
   - If user cancels: "Constitution creation cancelled"
   - If Mnemosyne not available: "Error: Mnemosyne not available. Check installation."

**Special behaviors**:
- If `--show` flag: Display current constitution from file (if exists) or "No constitution found"
- Interactive mode: Ask questions one at a time to build constitution
- Validation: Ensure at least 3 principles, 3 quality gates before saving
- Version bumping: Explain what changed and why version was incremented

Please proceed to create or update the constitution based on user input.
