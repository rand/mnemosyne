"""DSPy signatures for Mnemosyne agent operations.

This module defines type-safe DSPy signatures for all agent operations.
Signatures specify input/output schemas and enable systematic prompt optimization.

# Signature Format

DSPy signatures use the format:
```
"input_field1: type1, input_field2: type2 -> output_field1: type3, output_field2: type4"
```

# Design Principles

1. **Explicit Types**: All fields have explicit types (str, int, list[str], etc.)
2. **Clear Semantics**: Field names describe their purpose unambiguously
3. **Minimal Context**: Include only necessary fields to reduce token usage
4. **Structured Output**: Use typed fields instead of free-form text

# Usage

```python
import dspy
from mnemosyne.orchestration.dspy_modules.signatures import ExtractRequirements

cot = dspy.ChainOfThought(ExtractRequirements)
result = cot(user_intent="Implement authentication", context="...")
print(result.requirements)  # ['Add login endpoint', 'Hash passwords', ...]
```
"""

import dspy
from typing import List


class ExtractRequirements(dspy.Signature):
    """Extract explicit requirements from user intent.

    Given a user's high-level intent and context, identify all explicit
    requirements that must be satisfied for successful completion.

    Requirements should be:
    - Specific and testable
    - Independent (not overlapping)
    - Complete (cover all aspects of intent)
    - Prioritized (ordered by importance)
    """

    user_intent: str = dspy.InputField(
        desc="User's high-level description of what they want"
    )
    context: str = dspy.InputField(
        desc="Additional context: work item phase, agent, file scope"
    )

    requirements: list[str] = dspy.OutputField(
        desc="List of explicit, testable requirements extracted from intent"
    )
    priorities: list[int] = dspy.OutputField(
        desc="Priority scores (1-10) for each requirement, same order as requirements"
    )


class ValidateIntentSatisfaction(dspy.Signature):
    """Validate that implementation satisfies original user intent.

    Semantic analysis of whether the work done matches what the user requested.
    Goes beyond pattern matching to understand implicit expectations.

    Checks:
    - Does implementation address the core problem?
    - Are user expectations met (explicit and implicit)?
    - Is the solution appropriate for the context?
    """

    user_intent: str = dspy.InputField(
        desc="Original user request describing desired outcome"
    )
    work_item: str = dspy.InputField(
        desc="Description of the work item being validated"
    )
    implementation: str = dspy.InputField(
        desc="Summary of implementation: code changes, files modified, approach taken"
    )
    requirements: list[str] = dspy.InputField(
        desc="Extracted requirements from intent (may be empty)"
    )

    intent_satisfied: bool = dspy.OutputField(
        desc="True if implementation satisfies user intent, False otherwise"
    )
    explanation: str = dspy.OutputField(
        desc="Explanation of why intent is/isn't satisfied, referencing specific requirements"
    )
    missing_aspects: list[str] = dspy.OutputField(
        desc="List of aspects of user intent not addressed by implementation (empty if satisfied)"
    )


class ValidateCompleteness(dspy.Signature):
    """Validate implementation completeness.

    Checks for:
    - Partial implementations (TODOs, FIXMEs, stubs)
    - Unfilled typed holes (interfaces without implementations)
    - Missing error handling
    - Incomplete test coverage
    - Missing documentation

    Completeness means ready for production, not just "it compiles".
    """

    work_item: str = dspy.InputField(
        desc="Description of work item being validated"
    )
    implementation: str = dspy.InputField(
        desc="Summary of implementation with file paths and key changes"
    )
    requirements: list[str] = dspy.InputField(
        desc="Extracted requirements (may be empty)"
    )

    is_complete: bool = dspy.OutputField(
        desc="True if implementation is complete and production-ready"
    )
    incomplete_aspects: list[str] = dspy.OutputField(
        desc="List of incomplete aspects found (TODOs, stubs, missing tests, etc.)"
    )
    typed_holes: list[str] = dspy.OutputField(
        desc="List of typed holes or unfilled interfaces requiring implementation"
    )
    missing_tests: list[str] = dspy.OutputField(
        desc="List of areas lacking test coverage"
    )


class ValidateCorrectness(dspy.Signature):
    """Validate implementation correctness.

    Semantic analysis of logic quality and bug potential.

    Checks for:
    - Logic errors (off-by-one, race conditions, etc.)
    - Error handling gaps (uncaught exceptions, missing validation)
    - Edge case handling (empty inputs, overflow, etc.)
    - Type safety issues (implicit conversions, null derefs)
    - Security vulnerabilities

    Focus on potential runtime failures, not style or conventions.
    """

    work_item: str = dspy.InputField(
        desc="Description of work item"
    )
    implementation: str = dspy.InputField(
        desc="Summary of implementation with code snippets if available"
    )
    test_results: str = dspy.InputField(
        desc="Test execution results: pass/fail counts, error messages"
    )

    is_correct: bool = dspy.OutputField(
        desc="True if implementation is logically sound and bug-free"
    )
    logic_issues: list[str] = dspy.OutputField(
        desc="List of potential logic errors or bugs found"
    )
    error_handling_gaps: list[str] = dspy.OutputField(
        desc="List of error handling gaps (missing try/catch, validation, etc.)"
    )
    edge_cases: list[str] = dspy.OutputField(
        desc="List of unhandled edge cases"
    )


class GenerateImprovementGuidance(dspy.Signature):
    """Generate actionable improvement guidance for failed reviews.

    Provide specific, prioritized recommendations for fixing issues.
    Focus on "what to do next" rather than "what went wrong".

    Guidance should be:
    - Actionable (specific steps, not vague advice)
    - Prioritized (most critical issues first)
    - Referenced (cite specific code/requirements)
    - Constructive (suggest fixes, not just criticism)
    """

    user_intent: str = dspy.InputField(
        desc="Original user intent"
    )
    work_item: str = dspy.InputField(
        desc="Work item description"
    )
    implementation: str = dspy.InputField(
        desc="Implementation summary"
    )
    failed_gates: list[str] = dspy.InputField(
        desc="List of quality gates that failed"
    )
    all_issues: list[str] = dspy.InputField(
        desc="Consolidated list of all issues found across gates"
    )

    guidance: str = dspy.OutputField(
        desc="Actionable improvement guidance with specific steps"
    )
    priority_fixes: list[str] = dspy.OutputField(
        desc="List of highest-priority fixes to address first"
    )
    suggestions: list[str] = dspy.OutputField(
        desc="Additional suggestions for improving quality beyond minimum requirements"
    )


# Orchestrator signatures (placeholder for Phase 2)
class PrioritizeWorkItems(dspy.Signature):
    """Prioritize work items based on dependencies and criticality."""
    work_items: list[str] = dspy.InputField(
        desc="List of work item descriptions"
    )
    dependencies: list[str] = dspy.InputField(
        desc="Dependency relationships between work items"
    )

    prioritized_ids: list[str] = dspy.OutputField(
        desc="Work item IDs in priority order (highest first)"
    )
    reasoning: str = dspy.OutputField(
        desc="Explanation of prioritization logic"
    )


# Optimizer signatures (placeholder for Phase 2)
class DiscoverRelevantSkills(dspy.Signature):
    """Discover relevant skills for a given task."""
    task_description: str = dspy.InputField(
        desc="Description of the task"
    )
    available_skills: list[str] = dspy.InputField(
        desc="List of available skill names"
    )

    relevant_skills: list[str] = dspy.OutputField(
        desc="Skills relevant to the task, ranked by relevance"
    )
    relevance_scores: list[float] = dspy.OutputField(
        desc="Relevance scores (0-1) for each selected skill"
    )


# Executor signatures (placeholder for Phase 2)
class DecomposeTask(dspy.Signature):
    """Decompose high-level task into concrete steps."""
    task: str = dspy.InputField(
        desc="High-level task description"
    )
    context: str = dspy.InputField(
        desc="Project context and constraints"
    )

    steps: list[str] = dspy.OutputField(
        desc="Concrete implementation steps in execution order"
    )
    dependencies: list[str] = dspy.OutputField(
        desc="Dependencies between steps (format: 'step_i depends on step_j')"
    )
