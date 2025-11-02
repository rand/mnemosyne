---
name: feature-clarify
description: Resolve ambiguities in feature specs through interactive Q&A
---

I will help you clarify ambiguities in a feature specification through structured question and answer sessions.

**Usage**:
- `/feature-clarify <feature-id>` - Start clarification for a spec
- `/feature-clarify --auto <feature-id>` - Auto-detect ambiguities
- `/feature-clarify --show <feature-id>` - Display existing clarifications

**Instructions for me**:

1. **Load feature spec**:
   - Read `.mnemosyne/artifacts/specs/<feature-id>.md`
   - If not found: "Error: Feature spec '<feature-id>' not found. Use /feature-specify first."
   - Parse YAML frontmatter to get spec memory_id

2. **Check for existing clarifications**:
   - Look for `.mnemosyne/artifacts/clarifications/<feature-id>-clarifications.md`
   - If exists: Load and display summary
   - Count pending vs. resolved questions

3. **Auto-detect ambiguities** (if `--auto` flag or no existing clarifications):
   Scan the spec for:
   - **Vague quantifiers**: "fast", "slow", "easy", "hard", "secure", "scalable" without metrics
   - **Missing acceptance criteria**: Scenarios with <3 criteria
   - **Underspecified requirements**: Performance/security requirements without numbers
   - **Unclear dependencies**: References to external systems without details
   - **Open questions**: Explicit "?" or "TBD" markers

   Limit to top 3 most critical ambiguities.

4. **Interactive clarification** (max 3 questions per session):
   For each ambiguity:

   a) **Present question**:
   ```
   Question Q00X: [Clear, specific question]

   Context: [Why this needs clarification]
   Found in: [Spec section]
   Impact: [What this blocks or affects]
   ```

   b) **Gather answer**:
   - Ask user for decision
   - Ask for rationale (why this choice?)
   - Ask which spec sections need updating
   - Validate answer is concrete and measurable

   c) **Record clarification item**:
   ```markdown
   ## Q00X - Question

   [Question text]

   ### Context

   [Background and why this needs clarification]

   ### Decision

   [User's decision]

   **Rationale**: [Why this decision was made]

   **Spec Updates**:
   - [Section to update with new information]
   - [Another section to update]
   ```

5. **Format clarifications document**:
   ```markdown
   ---
   type: clarification
   id: <feature-id>-clarifications
   name: <feature-name> Clarifications
   feature_id: <feature-id>
   version: 1.0.0
   created_at: <ISO 8601 timestamp>
   updated_at: <ISO 8601 timestamp>
   memory_id: <will be filled>
   references: [<spec-memory-id>]
   ---

   # Clarifications: <feature-name>

   **Status**: [X resolved, Y pending]

   ## Q001 - Question

   [Question text]

   ### Context

   [Context]

   ### Decision

   [Decision text or *Pending*]

   **Rationale**: [Rationale if decided]

   **Spec Updates**:
   - [Update 1]
   - [Update 2]

   ## Q002 - Question

   ...
   ```

6. **Write clarifications file**:
   - Create/update `.mnemosyne/artifacts/clarifications/<feature-id>-clarifications.md`
   - Ensure directory exists
   - If updating: Preserve existing questions, add new ones with incremented IDs

7. **Store memory entry**:
   - Use Mnemosyne CLI: `mnemosyne remember`
   - Arguments:
     - Content: "Clarifications for <feature-name>: Q001: <first question summary>, Q002: ..., X resolved, Y pending"
     - Namespace: `project:<project-name>`
     - Importance: 7 (clarifications are important)
     - Type: clarification
     - Tags: clarification,<feature-id>,ambiguities
     - Context: "Clarification questions and answers for <feature-name>"
   - Capture memory_id

8. **Create memory links**:
   - Link clarification → spec with relationship "clarifies"
   - Update clarification's `references` field with spec memory_id
   - Update clarification's `memory_id` field

9. **Update spec with clarification outcomes** (if decisions were made):
   - For each resolved question with spec_updates:
     - Parse spec file
     - Add clarification details to relevant sections
     - Update spec's `updated_at` timestamp
     - Increment spec's patch version (e.g., 1.0.0 → 1.0.1)
   - Write updated spec back to file

10. **Display confirmation**:
    ```
    ✓ Clarifications recorded successfully

    Feature ID: <feature-id>
    Location: .mnemosyne/artifacts/clarifications/<feature-id>-clarifications.md
    Memory ID: <memory-id>

    Questions:
    - Q001: [Resolved] <question summary>
    - Q002: [Resolved] <question summary>
    - Q003: [Pending] <question summary>

    Status: 2 resolved, 1 pending

    Spec Updates Applied:
    - Updated Performance Requirements (Q001)
    - Updated Security Requirements (Q002)

    Next steps:
    - Review clarifications: cat .mnemosyne/artifacts/clarifications/<feature-id>-clarifications.md
    - Review updated spec: cat .mnemosyne/artifacts/specs/<feature-id>.md
    - Create implementation plan: /feature-plan <feature-id>
    - Continue clarifying: /feature-clarify <feature-id> (if pending questions)
    ```

11. **Error handling**:
    - If spec not found: "Error: Feature spec '<feature-id>' not found"
    - If no ambiguities detected: "✓ No ambiguities detected in spec. Spec looks clear!"
    - If clarifications file corrupted: Attempt to parse, warn about errors
    - If user skips question: Mark as "Pending" in clarifications document

**Special behaviors**:
- `--auto`: Automatically scan and detect ambiguities without user input
- `--show`: Display existing clarifications in readable format
- Interactive mode: Ask one question at a time, wait for user response
- Batch mode: Can process up to 3 questions per session (avoid overwhelming user)
- Smart detection: Prioritize blocking ambiguities (P0/P1 scenarios) over P2/P3
- Validation: Ensure decisions are concrete (reject "we'll figure it out later")

**Example ambiguity detection**:
- "The API should be fast" → Q: "What is the target p95 latency for API responses?"
- "Users can upload files" → Q: "What is the maximum file size allowed?"
- "Must be secure" → Q: "What specific security requirements? (auth, encryption, rate limiting?)"
- Scenario with 1 criterion → Q: "This scenario needs more acceptance criteria. What else must be true?"

**Clarification quality gates**:
- Decisions must be actionable (not "TBD" or "later")
- Metrics must be specific numbers (not ranges like "1-10")
- Rationale explains "why" not just "what"
- Spec updates reference exact sections to modify

Please proceed to clarify the feature specification based on user input.
