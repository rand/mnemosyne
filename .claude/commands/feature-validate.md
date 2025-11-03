---
name: feature-validate
description: Validate feature specification quality using DSPy ReviewerModule
---

I will help you validate a feature specification using AI-powered semantic analysis.

**Usage**:
- `/feature-validate <feature-id>` - Validate a specific feature spec
- `/feature-validate --all` - Validate all specs in artifacts/specs/
- `/feature-validate <feature-id> --fix` - Validate and suggest specific fixes

**Instructions for me**:

1. **Load feature spec**:
   - Read `.mnemosyne/artifacts/specs/<feature-id>.md`
   - If `--all`: Glob `.mnemosyne/artifacts/specs/*.md`
   - If not found: "Error: Feature spec '<feature-id>' not found"
   - Parse YAML frontmatter for spec metadata

2. **Run DSPy validation**:
   - Execute validation using optimized ReviewerModule v1:
     ```bash
     cd src/orchestration/dspy_modules
     uv run python3 specflow_integration.py ../../.mnemosyne/artifacts/specs/<feature-id>.md --json
     ```
   - Parse JSON output for validation results:
     - `is_valid`: Overall validation status (bool)
     - `issues`: List of specific problems found
     - `suggestions`: Actionable improvement recommendations
     - `requirements`: LLM-extracted requirements from spec
     - `ambiguities`: Detected vague terms and missing metrics
     - `completeness_score`: 0.0-1.0 quality score

3. **Interpret results**:
   - **Excellent** (score ≥ 0.9): "✓ Spec quality: Excellent"
   - **Good** (score ≥ 0.8): "✓ Spec quality: Good"
   - **Fair** (score ≥ 0.7): "⚠️ Spec quality: Fair - improvements recommended"
   - **Poor** (score < 0.7): "✗ Spec quality: Poor - significant issues found"

4. **Display validation report**:
   ```
   ✓ Validation complete

   Feature ID: <feature-id>
   Feature Name: <feature-name>
   Spec Location: .mnemosyne/artifacts/specs/<feature-id>.md
   Spec Version: <version>

   Validation Method: DSPy ReviewerModule v1 (semantic analysis)

   == QUALITY ASSESSMENT ==

   Completeness Score: <score>% (<rating>)
   Requirements Extracted: <count>
   Issues Found: <count>
   Ambiguities Detected: <count>
   Validation Status: <✓ Pass | ⚠️ Warning | ✗ Fail>

   == EXTRACTED REQUIREMENTS ==

   [First 5 requirements extracted by LLM:]
   1. <requirement>
   2. <requirement>
   3. <requirement>
   4. <requirement>
   5. <requirement>

   [If more than 5:]
   ... and <N> more requirements

   == ISSUES ==

   [If issues found:]
   ✗ <issue 1>
   ✗ <issue 2>
   ✗ <issue 3>

   [If no issues:]
   ✓ No issues detected

   == AMBIGUITIES ==

   [If ambiguities found:]
   🔍 <location>: <term>
      Question: <clarifying question>
      Impact: <why this matters>

   [If no ambiguities:]
   ✓ No ambiguities detected

   == SUGGESTIONS ==

   [If suggestions available:]
   💡 <suggestion 1>
   💡 <suggestion 2>
   💡 <suggestion 3>

   [If no suggestions:]
   ✓ Spec meets quality standards

   == NEXT STEPS ==

   [If score >= 0.8:]
   - Review spec: cat .mnemosyne/artifacts/specs/<feature-id>.md
   - Create implementation plan: /feature-plan <feature-id>

   [If score < 0.8:]
   - Address issues above (priority: high)
   - Clarify ambiguities: /feature-clarify <feature-id>
   - Re-validate: /feature-validate <feature-id>
   - After fixes, create plan: /feature-plan <feature-id>
   ```

5. **Detailed fix suggestions** (if `--fix` flag):
   For each issue/ambiguity, provide:
   - **Location**: Exact section/line in spec
   - **Problem**: What's wrong
   - **Fix**: Specific text to add/change
   - **Example**: Show before/after

   Format:
   ```
   == FIX #1: <issue summary> ==

   Location: <section> - <line range>
   Problem: <specific issue>

   Suggested Fix:
   Replace: "<current text>"
   With: "<improved text>"

   Example:
   Before: "API must be fast"
   After: "API must respond within 200ms (p95 latency) under normal load (1000 req/s)"

   Rationale: <why this fix improves spec quality>
   ```

6. **Validation for --all flag**:
   - Run validation on each spec sequentially
   - Display summary table:
     ```
     == BATCH VALIDATION RESULTS ==

     | Feature ID         | Score | Status   | Issues | Ambiguities |
     |--------------------|-------|----------|--------|-------------|
     | jwt-auth           | 92%   | ✓ Pass   | 0      | 0           |
     | api-rate-limiting  | 78%   | ⚠️ Warn  | 2      | 3           |
     | user-dashboard     | 65%   | ✗ Fail   | 5      | 7           |

     Summary:
     - Total Specs: 3
     - Passed (≥80%): 1
     - Warning (70-79%): 1
     - Failed (<70%): 1

     Average Score: 78%

     Recommended Actions:
     - Fix critical issues in: user-dashboard
     - Review and improve: api-rate-limiting
     ```

7. **Store validation history** (optional):
   - Append validation result to `.mnemosyne/artifacts/validation-history.jsonl`:
     ```json
     {"feature_id":"jwt-auth","timestamp":"2025-11-03T14:30:00Z","score":0.92,"issues":0,"ambiguities":0,"validator":"dspy-v1"}
     ```
   - Enables tracking spec quality over time

8. **Error handling**:
   - If spec not found: "Error: Feature spec '<feature-id>' not found. Use /feature-specify first."
   - If DSPy validation fails: Fall back to pattern-based validation, warn "DSPy unavailable, using pattern matching only"
   - If specflow_integration.py not found: "Error: Validation module not found at src/orchestration/dspy_modules/specflow_integration.py"
   - If spec file corrupted: "Error: Failed to parse spec file. Check YAML frontmatter format."
   - If no specs found (with --all): "Error: No feature specs found in .mnemosyne/artifacts/specs/"

**Special behaviors**:
- `--all`: Batch validate all specs in artifacts directory
- `--fix`: Provide detailed, actionable fix suggestions for each issue
- `--json`: Output validation results in JSON format (for scripting)
- `--quiet`: Only show summary, suppress detailed report
- Smart prioritization: Show P0/P1 scenario issues before P2/P3
- Historical tracking: Optionally log validation results for trend analysis

**Quality standards**:
- **Pass (≥80%)**: Spec ready for implementation planning
- **Warning (70-79%)**: Spec acceptable but improvements recommended
- **Fail (<70%)**: Spec needs significant work before implementation

**Example validation scores**:
- 95%: Excellent spec with clear requirements, quantified metrics, comprehensive scenarios
- 85%: Good spec with minor ambiguities in non-critical areas
- 75%: Fair spec with some vague terms or missing acceptance criteria
- 65%: Poor spec with multiple underspecified requirements, needs clarification

**Integration points**:
- Use after `/feature-specify` to verify spec quality before planning
- Use before `/feature-plan` to ensure clean input
- Use with `/feature-clarify` to resolve detected ambiguities
- Use periodically to validate specs haven't degraded

Please proceed to validate the feature specification.
