"""DSPy module for Reviewer agent intelligence.

This module implements systematic prompt optimization for the Reviewer agent's
core responsibilities:

- Requirement extraction from user intent
- Three-pillar validation (intent, completeness, correctness)
- Quality gate validation
- Improvement guidance generation

# Architecture

The ReviewerModule uses ChainOfThought for all operations, enabling:
- Explicit reasoning traces for transparency
- Systematic prompt optimization via teleprompters
- Automatic few-shot example generation
- Metric-driven improvement

# Usage from Rust

```rust
// Via DSpyBridge
let mut inputs = HashMap::new();
inputs.insert("user_intent".to_string(), json!("Implement auth"));
inputs.insert("context".to_string(), json!("Phase: Spec, Agent: Executor"));

let result = bridge.call_agent_module("reviewer", inputs).await?;
let requirements: Vec<String> = serde_json::from_value(result["requirements"])?;
```

# Usage from Python (testing/development)

```python
from mnemosyne.orchestration.dspy_modules.reviewer_module import ReviewerModule

reviewer = ReviewerModule()

# Extract requirements
result = reviewer.extract_requirements(
    user_intent="Implement user authentication",
    context="Phase: Spec, Files: auth.py"
)
print(result.requirements)  # ['Add login endpoint', ...]

# Validate intent
result = reviewer.validate_intent(
    user_intent="Add auth",
    work_item="Implement JWT authentication",
    implementation="Added login endpoint with JWT tokens",
    requirements=['JWT tokens', 'Login endpoint']
)
print(result.intent_satisfied)  # True
print(result.explanation)  # "Implementation addresses..."
```

# Optimization

This module can be optimized using DSPy teleprompters:

```python
from dspy.teleprompt import BootstrapFewShot

# Define metric
def requirement_quality(example, pred, trace=None):
    # Check if requirements are specific and testable
    return all(is_specific(r) for r in pred.requirements)

# Compile optimized module
teleprompter = BootstrapFewShot(metric=requirement_quality)
optimized_reviewer = teleprompter.compile(
    ReviewerModule(),
    trainset=training_examples
)

# Save compiled module
optimized_reviewer.save("reviewer_v1.json")
```
"""

import dspy
from typing import Optional
import logging

from signatures import (
    ExtractRequirements,
    ValidateIntentSatisfaction,
    ValidateCompleteness,
    ValidateCorrectness,
    GenerateImprovementGuidance,
)

logger = logging.getLogger(__name__)


class ReviewerModule(dspy.Module):
    """DSPy module for Reviewer agent operations.

    Implements all Reviewer LLM operations using systematic prompt optimization:
    - Requirement extraction
    - Intent validation
    - Completeness validation
    - Correctness validation
    - Improvement guidance

    All operations use ChainOfThought for transparency and optimization.
    """

    def __init__(self):
        """Initialize Reviewer module with ChainOfThought for all operations."""
        super().__init__()

        # Requirement extraction
        self.extract_reqs = dspy.ChainOfThought(ExtractRequirements)

        # Three-pillar validation (using _ prefix to avoid method name conflicts)
        self._validate_intent_cot = dspy.ChainOfThought(ValidateIntentSatisfaction)
        self._validate_completeness_cot = dspy.ChainOfThought(ValidateCompleteness)
        self._validate_correctness_cot = dspy.ChainOfThought(ValidateCorrectness)

        # Improvement guidance
        self.generate_guidance = dspy.ChainOfThought(GenerateImprovementGuidance)

        logger.info("ReviewerModule initialized with ChainOfThought")

    def _parse_numbered_list(self, text: str) -> list[str]:
        """Parse DSPy's numbered list format into Python list.

        DSPy often returns formatted strings like:
        "1. First item\n2. Second item\n3. Third item"

        This converts to: ["First item", "Second item", "Third item"]
        """
        if isinstance(text, list):
            return text  # Already a list

        if not isinstance(text, str):
            return []

        # Handle string representations of empty lists
        if text.strip() in ['[]', '[ ]', '']:
            return []

        # Split by newlines and parse numbered items
        lines = text.strip().split('\n')
        items = []

        for line in lines:
            line = line.strip()
            if not line:
                continue

            # Match patterns like "1. Item" or "1) Item" or "- Item"
            import re
            match = re.match(r'^(?:\d+[\.\)]\s*|[-*]\s*)(.*)', line)
            if match:
                items.append(match.group(1).strip())
            elif line:
                # If no numbering found, add the whole line
                items.append(line)

        return items

    def _parse_boolean(self, value) -> bool:
        """Parse DSPy's boolean output which may be string 'True'/'False'."""
        if isinstance(value, bool):
            return value
        if isinstance(value, str):
            return value.strip().lower() in ['true', '1', 'yes']
        return False

    def forward(
        self,
        user_intent: str,
        work_item: Optional[str] = None,
        implementation: Optional[str] = None,
        context: Optional[str] = None,
        requirements: Optional[list[str]] = None,
        test_results: Optional[str] = None,
        failed_gates: Optional[list[str]] = None,
        all_issues: Optional[list[str]] = None,
    ):
        """Main forward pass - extracts requirements and validates work.

        This is called when the module is invoked as a function.
        Routes to appropriate submodule based on available inputs.

        Args:
            user_intent: Original user request
            work_item: Work item description (optional)
            implementation: Implementation summary (optional)
            context: Additional context (optional)
            requirements: Pre-extracted requirements (optional)
            test_results: Test execution results (optional)
            failed_gates: List of failed quality gates (optional)
            all_issues: All issues found (optional)

        Returns:
            dspy.Prediction with validation results
        """
        # If we have full validation inputs, run complete review
        if implementation is not None and work_item is not None:
            return self.full_review(
                user_intent=user_intent,
                work_item=work_item,
                implementation=implementation,
                context=context or "",
                test_results=test_results or "Not provided",
            )

        # If we only have intent and context, extract requirements
        if context is not None:
            return self.extract_requirements(
                user_intent=user_intent,
                context=context
            )

        raise ValueError(
            "ReviewerModule requires either:\n"
            "  1. user_intent + work_item + implementation (for full review)\n"
            "  2. user_intent + context (for requirement extraction)"
        )

    def extract_requirements(
        self,
        user_intent: str,
        context: str = "",
    ) -> dspy.Prediction:
        """Extract explicit requirements from user intent.

        Args:
            user_intent: User's high-level description
            context: Work item phase, agent, file scope (optional)

        Returns:
            Prediction with:
                - requirements: List[str] of extracted requirements
                - priorities: List[int] of priority scores
        """
        logger.debug(f"Extracting requirements from intent: {user_intent[:50]}...")

        result = self.extract_reqs(
            user_intent=user_intent,
            context=context or "General development context",
        )

        # Parse DSPy's formatted string output into list
        requirements = self._parse_numbered_list(result.requirements)
        priorities_raw = self._parse_numbered_list(result.priorities) if hasattr(result, 'priorities') else []

        # Extract just the numbers from priorities (e.g., "1. 9 (Critical)" -> 9)
        priorities = []
        for p in priorities_raw:
            try:
                # Extract first number from string like "1. 9 (Critical)"
                parts = p.strip().split()
                if len(parts) >= 2:
                    priorities.append(int(parts[1]))
                else:
                    priorities.append(int(parts[0]))
            except (ValueError, IndexError):
                priorities.append(5)  # Default priority

        logger.info(f"Extracted {len(requirements)} requirements")
        return dspy.Prediction(
            requirements=requirements,
            priorities=priorities,
            **{k: v for k, v in result.items() if k not in ['requirements', 'priorities']}
        )

    def validate_intent_satisfaction(
        self,
        user_intent: str,
        work_item: str,
        implementation: str,
        requirements: list[str],
    ) -> dspy.Prediction:
        """Validate that implementation satisfies user intent.

        Args:
            user_intent: Original user request
            work_item: Work item description
            implementation: Implementation summary
            requirements: Extracted requirements

        Returns:
            Prediction with:
                - intent_satisfied: bool
                - explanation: str
                - missing_aspects: List[str]
        """
        logger.debug("Validating intent satisfaction")

        result = self._validate_intent_cot(
            user_intent=user_intent,
            work_item=work_item,
            implementation=implementation,
            requirements=requirements,
        )

        logger.info(f"Intent satisfied: {result.intent_satisfied}")
        return result

    def validate_implementation_completeness(
        self,
        work_item: str,
        implementation: str,
        requirements: list[str],
    ) -> dspy.Prediction:
        """Validate implementation completeness.

        Args:
            work_item: Work item description
            implementation: Implementation summary
            requirements: Extracted requirements

        Returns:
            Prediction with:
                - is_complete: bool
                - incomplete_aspects: List[str]
                - typed_holes: List[str]
                - missing_tests: List[str]
        """
        logger.debug("Validating completeness")

        result = self._validate_completeness_cot(
            work_item=work_item,
            implementation=implementation,
            requirements=requirements,
        )

        logger.info(f"Implementation complete: {result.is_complete}")
        return result

    def validate_implementation_correctness(
        self,
        work_item: str,
        implementation: str,
        test_results: str,
    ) -> dspy.Prediction:
        """Validate implementation correctness.

        Args:
            work_item: Work item description
            implementation: Implementation summary
            test_results: Test execution results

        Returns:
            Prediction with:
                - is_correct: bool
                - logic_issues: List[str]
                - error_handling_gaps: List[str]
                - edge_cases: List[str]
        """
        logger.debug("Validating correctness")

        result = self._validate_correctness_cot(
            work_item=work_item,
            implementation=implementation,
            test_results=test_results,
        )

        logger.info(f"Implementation correct: {result.is_correct}")
        return result

    def generate_improvement_guidance_for_failed_review(
        self,
        user_intent: str,
        work_item: str,
        implementation: str,
        failed_gates: list[str],
        all_issues: list[str],
    ) -> dspy.Prediction:
        """Generate actionable improvement guidance.

        Args:
            user_intent: Original user intent
            work_item: Work item description
            implementation: Implementation summary
            failed_gates: List of failed quality gates
            all_issues: All issues found

        Returns:
            Prediction with:
                - guidance: str (actionable guidance)
                - priority_fixes: List[str]
                - suggestions: List[str]
        """
        logger.debug("Generating improvement guidance")

        result = self.generate_guidance(
            user_intent=user_intent,
            work_item=work_item,
            implementation=implementation,
            failed_gates=failed_gates,
            all_issues=all_issues,
        )

        logger.info(f"Generated {len(result.priority_fixes)} priority fixes")
        return result

    def full_review(
        self,
        user_intent: str,
        work_item: str,
        implementation: str,
        context: str,
        test_results: str,
    ) -> dspy.Prediction:
        """Perform complete review: extract requirements and validate all pillars.

        Args:
            user_intent: Original user request
            work_item: Work item description
            implementation: Implementation summary
            context: Additional context
            test_results: Test execution results

        Returns:
            Prediction with all validation results combined
        """
        logger.info("Performing full review")

        # Extract requirements
        reqs_result = self.extract_requirements(
            user_intent=user_intent,
            context=context,
        )

        # Validate intent
        intent_result = self.validate_intent_satisfaction(
            user_intent=user_intent,
            work_item=work_item,
            implementation=implementation,
            requirements=reqs_result.requirements,
        )

        # Validate completeness
        completeness_result = self.validate_implementation_completeness(
            work_item=work_item,
            implementation=implementation,
            requirements=reqs_result.requirements,
        )

        # Validate correctness
        correctness_result = self.validate_implementation_correctness(
            work_item=work_item,
            implementation=implementation,
            test_results=test_results,
        )

        # Combine results
        return dspy.Prediction(
            requirements=reqs_result.requirements,
            priorities=reqs_result.priorities,
            intent_satisfied=intent_result.intent_satisfied,
            intent_explanation=intent_result.explanation,
            missing_aspects=intent_result.missing_aspects,
            is_complete=completeness_result.is_complete,
            incomplete_aspects=completeness_result.incomplete_aspects,
            typed_holes=completeness_result.typed_holes,
            missing_tests=completeness_result.missing_tests,
            is_correct=correctness_result.is_correct,
            logic_issues=correctness_result.logic_issues,
            error_handling_gaps=correctness_result.error_handling_gaps,
            edge_cases=correctness_result.edge_cases,
        )

    # =============================================================================
    # Simplified Test API
    # =============================================================================
    # These methods provide a simplified interface for testing, matching the
    # test file expectations with execution_context instead of structured inputs.

    def validate_intent(
        self,
        user_intent: str,
        implementation: str,
        execution_context: list = None,
    ) -> dspy.Prediction:
        """Simplified API for testing intent validation.

        Args:
            user_intent: User's original intent
            implementation: Implementation summary
            execution_context: List of execution memory dicts (optional)

        Returns:
            Prediction with intent_satisfied (bool) and issues (list)
        """
        # Extract work_item from execution context if available
        work_item = ""
        if execution_context:
            work_item = execution_context[0].get("summary", "") if len(execution_context) > 0 else ""

        # Call the full validation method
        result = self.validate_intent_satisfaction(
            user_intent=user_intent,
            work_item=work_item or "General implementation",
            implementation=implementation,
            requirements=[],
        )

        # Map to simpler test-expected format
        issues = self._parse_numbered_list(result.missing_aspects) if hasattr(result, 'missing_aspects') else []
        intent_satisfied = self._parse_boolean(result.intent_satisfied) if hasattr(result, 'intent_satisfied') else False

        return dspy.Prediction(
            intent_satisfied=intent_satisfied,
            issues=issues,
        )

    def verify_completeness(
        self,
        requirements: list[str],
        implementation: str,
        execution_context: list = None,
    ) -> dspy.Prediction:
        """Simplified API for testing completeness validation.

        Args:
            requirements: List of requirements to check
            implementation: Implementation summary
            execution_context: List of execution memory dicts (optional)

        Returns:
            Prediction with complete (bool) and issues (list)
        """
        # Extract work_item from execution context if available
        work_item = ""
        if execution_context:
            work_item = execution_context[0].get("summary", "") if len(execution_context) > 0 else ""

        # Call the full validation method
        result = self.validate_implementation_completeness(
            work_item=work_item or "General implementation",
            implementation=implementation,
            requirements=requirements,
        )

        # Map to simpler test-expected format
        issues = []
        if hasattr(result, 'incomplete_aspects'):
            issues.extend(self._parse_numbered_list(result.incomplete_aspects))
        if hasattr(result, 'typed_holes'):
            issues.extend(self._parse_numbered_list(result.typed_holes))
        if hasattr(result, 'missing_tests'):
            issues.extend(self._parse_numbered_list(result.missing_tests))

        complete = self._parse_boolean(result.is_complete) if hasattr(result, 'is_complete') else False

        return dspy.Prediction(
            complete=complete,
            issues=issues,
        )

    def verify_correctness(
        self,
        implementation: str,
        execution_context: list = None,
    ) -> dspy.Prediction:
        """Simplified API for testing correctness validation.

        Args:
            implementation: Implementation summary
            execution_context: List of execution memory dicts (optional)

        Returns:
            Prediction with correct (bool) and issues (list)
        """
        # Extract work_item and test_results from execution context
        work_item = ""
        test_results = "No test results provided"
        if execution_context:
            work_item = execution_context[0].get("summary", "") if len(execution_context) > 0 else ""
            test_results = execution_context[1].get("content", "No test results") if len(execution_context) > 1 else test_results

        # Call the full validation method
        result = self.validate_implementation_correctness(
            work_item=work_item or "General implementation",
            implementation=implementation,
            test_results=test_results,
        )

        # Map to simpler test-expected format
        issues = []
        if hasattr(result, 'logic_issues'):
            issues.extend(self._parse_numbered_list(result.logic_issues))
        if hasattr(result, 'error_handling_gaps'):
            issues.extend(self._parse_numbered_list(result.error_handling_gaps))
        if hasattr(result, 'edge_cases'):
            issues.extend(self._parse_numbered_list(result.edge_cases))

        correct = self._parse_boolean(result.is_correct) if hasattr(result, 'is_correct') else False

        return dspy.Prediction(
            correct=correct,
            issues=issues,
        )
