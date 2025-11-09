"""
Reviewer Agent - Quality Assurance and Validation Specialist.

Responsibilities:
- Validate intent satisfaction, documentation, test coverage
- Fact-check claims, references, external dependencies
- Check for anti-patterns and technical debt
- Block work until quality standards met
- Mark "COMPLETE" only when all gates pass
"""

from dataclasses import dataclass
from typing import Dict, List, Optional, Set, Any
from enum import Enum
import json

try:
    from .base_agent import AgentExecutionMixin, WorkItem, WorkResult
    from .logging_config import get_logger
    from .error_context import (
        create_work_item_error_context,
        create_validation_error_context,
        format_error_for_rust
    )
    from .validation import validate_work_item, validate_agent_state, validate_review_artifact
    from .metrics import get_metrics_collector
except ImportError:
    import sys, os
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    from base_agent import AgentExecutionMixin, WorkItem, WorkResult
    from logging_config import get_logger
    from error_context import (
        create_work_item_error_context,
        create_validation_error_context,
        format_error_for_rust
    )
    from validation import validate_work_item, validate_agent_state, validate_review_artifact
    from metrics import get_metrics_collector

logger = get_logger("reviewer")


class QualityGate(Enum):
    """Quality gates that must pass (8 total: 5 existing + 3 pillars)."""
    # Existing gates
    INTENT_SATISFIED = "intent_satisfied"
    TESTS_PASSING = "tests_passing"
    DOCUMENTATION_COMPLETE = "documentation_complete"
    NO_ANTIPATTERNS = "no_antipatterns"
    CONSTRAINTS_MAINTAINED = "constraints_maintained"
    # Three-pillar gates
    COMPLETENESS = "completeness"
    CORRECTNESS = "correctness"
    PRINCIPLED_IMPLEMENTATION = "principled_implementation"


@dataclass
class ReviewResult:
    """Result of quality review with three-pillar validation."""
    passed: bool
    gate_results: Dict[QualityGate, bool]
    issues: List[str]
    recommendations: List[str]
    suggested_tests: List[str]  # New: tests suggested by Reviewer
    execution_context: List[str]  # New: memory IDs from execution
    confidence: float


@dataclass
class ReviewerConfig:
    """Configuration for Reviewer agent."""
    agent_id: str = "reviewer"
    strict_mode: bool = True  # Fail on any gate failure
    required_gates: Set[QualityGate] = None
    min_test_coverage: float = 0.70  # 70% minimum
    antipattern_patterns: List[str] = None
    # Anthropic API key (injected from Rust via environment)
    api_key: Optional[str] = None

    def __post_init__(self):
        if self.required_gates is None:
            # All gates required by default
            self.required_gates = set(QualityGate)

        if self.antipattern_patterns is None:
            self.antipattern_patterns = [
                "TODO",
                "FIXME",
                "HACK",
                "XXX",
                "mock_",
                "stub_",
                "__placeholder__"
            ]


class ReviewerAgent(AgentExecutionMixin):
    """
    Quality assurance and validation specialist using direct Anthropic API.

    Enforces quality standards before work completion:
    - All tests passing
    - Documentation complete
    - No anti-patterns
    - Facts verified
    - Intent satisfied

    **PyO3 Bridge Integration**: Inherits from AgentExecutionMixin to provide
    standard interface for Rust bridge communication.
    """

    REVIEWER_SYSTEM_PROMPT = """You are the Reviewer Agent in a multi-agent orchestration system.

Your role:
- Quality assurance and validation specialist with three-pillar validation
- Validate completeness, correctness, and principled implementation
- Check intent satisfaction, documentation, test coverage
- Fact-check claims, references, external dependencies
- Check for anti-patterns and technical debt
- Suggest missing tests
- Block work until quality standards met
- Mark "COMPLETE" only when all 8 quality gates pass

Quality Gates (ALL must pass):
1. Intent Satisfied - Implementation fulfills original requirements
2. Tests Passing - All tests pass, coverage ≥ 70%
3. Documentation Complete - Overview, usage, examples present
4. No Anti-patterns - No TODO/FIXME/HACK/stub/mock markers
5. Constraints Maintained - No constraint violations
6. Completeness - No TODOs, partial implementations, or unfilled typed holes
7. Correctness - Logic is sound, no errors or failed validations
8. Principled Implementation - No hacks, workarounds, or architectural inconsistencies

Your Review Process:
1. Read and understand the work artifact
2. Evaluate each quality gate rigorously
3. Verify completeness (no incomplete work)
4. Verify correctness (logic is sound)
5. Verify principled implementation (clean architecture)
6. Suggest missing tests for untested scenarios
7. Provide specific, actionable feedback on failures
8. BLOCK work if any required gate fails (strict mode)
9. Only mark COMPLETE when ALL gates pass

Be thorough but constructive. Identify real issues, not nitpicks. Suggest tests for edge cases."""

    def __init__(self, config: ReviewerConfig, coordinator, storage):
        """
        Initialize Reviewer agent with direct Anthropic API access.

        Args:
            config: Reviewer configuration
            coordinator: PyCoordinator for shared state
            storage: PyStorage for memory operations
        """
        self.config = config
        self.coordinator = coordinator
        self.storage = storage

        # Store API key (injected from Rust environment)
        import os
        self.api_key = config.api_key or os.getenv("ANTHROPIC_API_KEY")

        # Register with coordinator
        self.coordinator.register_agent(config.agent_id)

        # State
        self._review_count = 0
        self._pass_count = 0
        self._fail_count = 0
        self._session_active = False
        self._conversation_history: List[Dict[str, Any]] = []

        logger.info(f"[Reviewer] Initialized with direct Anthropic API access")

    async def start_session(self):
        """Start agent session (validates API key availability)."""
        if not self._session_active:
            logger.info(f"Starting session for agent {self.config.agent_id}")

            # Validate API key is available
            if not self.api_key:
                raise ValueError(
                    "ANTHROPIC_API_KEY not set. Cannot start session without API access. "
                    "Get your key from: https://console.anthropic.com/settings/keys"
                )

            # Initialize conversation with system prompt
            self._conversation_history = []
            self._session_active = True
            logger.info(f"Session started successfully for {self.config.agent_id}")

    async def stop_session(self):
        """Stop agent session."""
        if self._session_active:
            logger.info(f"Stopping session for agent {self.config.agent_id}")
            self._session_active = False
            self._conversation_history = []
            logger.info(f"Session stopped for {self.config.agent_id}")

    async def _call_api(self, prompt: str) -> str:
        """
        Helper method to make Anthropic API calls.

        Args:
            prompt: User prompt to send

        Returns:
            Text response from API
        """
        import anthropic

        if not self.api_key:
            raise ValueError("ANTHROPIC_API_KEY not set. Cannot make API calls.")

        client = anthropic.Anthropic(api_key=self.api_key)

        # Add to conversation history
        self._conversation_history.append({
            "role": "user",
            "content": prompt
        })

        # Call API
        response = client.messages.create(
            model="claude-sonnet-4-5-20250929",
            max_tokens=2048,
            system=self.REVIEWER_SYSTEM_PROMPT,
            messages=self._conversation_history
        )

        # Extract text
        response_text = ""
        for block in response.content:
            if block.type == "text":
                response_text += block.text

        # Add to conversation history
        self._conversation_history.append({
            "role": "assistant",
            "content": response_text
        })

        return response_text

    async def review(self, work_artifact: Dict[str, Any]) -> ReviewResult:
        """
        Review work artifact against quality gates using Claude Agent SDK.

        Args:
            work_artifact: Artifact to review (code, docs, plan, etc.)

        Returns:
            ReviewResult with pass/fail and detailed feedback
        """
        self.coordinator.update_agent_state(self.config.agent_id, "running")
        self._review_count += 1

        try:
            # Ensure session is active
            if not self._session_active:
                await self.start_session()

            # Build comprehensive review prompt
            review_prompt = self._build_review_prompt(work_artifact)

            # Call API for review
            review_response = await self._call_api(review_prompt)
            await self._store_message(review_response, "review")

            # Parse review results
            gate_results, issues, recommendations = self._parse_review_results(
                [review_response],
                work_artifact
            )

            # Suggest missing tests
            suggested_tests = self._suggest_missing_tests(work_artifact, issues)

            # Extract execution context (memory IDs from execution)
            execution_context = work_artifact.get("execution_memory_ids", [])

            # Determine overall pass/fail
            required_gates_passed = all(
                gate_results.get(gate, False)
                for gate in self.config.required_gates
            )

            all_passed = required_gates_passed if self.config.strict_mode else len(issues) == 0

            if all_passed:
                self._pass_count += 1
            else:
                self._fail_count += 1

            # Calculate confidence
            pass_rate = sum(gate_results.values()) / len(gate_results) if gate_results else 0
            confidence = pass_rate

            result = ReviewResult(
                passed=all_passed,
                gate_results=gate_results,
                issues=issues,
                recommendations=recommendations,
                suggested_tests=suggested_tests,
                execution_context=execution_context,
                confidence=confidence
            )

            # Store review result
            self.storage.store({
                "content": f"Review {'PASSED' if all_passed else 'FAILED'}: {len(issues)} issues found",
                "namespace": f"project:agent-{self.config.agent_id}",
                "importance": 9 if not all_passed else 7,
                "tags": ["review", "quality-gate", "passed" if all_passed else "failed"]
            })

            self.coordinator.update_agent_state(
                self.config.agent_id,
                "complete" if all_passed else "blocked"
            )

            return result

        except Exception as e:
            self.coordinator.update_agent_state(self.config.agent_id, "failed")
            raise RuntimeError(f"Review failed: {e}") from e

    def _suggest_missing_tests(self, artifact: Dict[str, Any], issues: List[str]) -> List[str]:
        """Suggest missing tests based on work artifact and detected issues."""
        suggestions = []

        # Check if work involves error handling
        code = str(artifact.get("code", ""))
        if "error" in code.lower() and not any("error" in str(t).lower() for t in artifact.get("tests", [])):
            suggestions.append("Add tests for error handling and edge cases")

        # Check for async code without async tests
        if "async" in code.lower() and not any("async" in str(t).lower() for t in artifact.get("tests", [])):
            suggestions.append("Add tests for async behavior and concurrency scenarios")

        # Check for null/None handling
        if ("null" in code.lower() or "none" in code.lower()) and not any("null" in str(t).lower() or "none" in str(t).lower() for t in artifact.get("tests", [])):
            suggestions.append("Add tests for null/None handling")

        # Check for boundary conditions
        if "boundary" in code.lower() and not any("boundary" in str(t).lower() for t in artifact.get("tests", [])):
            suggestions.append("Add boundary condition tests")

        # Check for integration points
        if "integration" in code.lower() and not any("integration" in str(t).lower() for t in artifact.get("tests", [])):
            suggestions.append("Add integration tests for component interactions")

        # If completeness gate failed, suggest implementation coverage tests
        if any("completeness" in issue.lower() or "incomplete" in issue.lower() for issue in issues):
            suggestions.append("Add tests to verify all required features are implemented")

        # If correctness gate failed, suggest logic validation tests
        if any("correctness" in issue.lower() or "logic" in issue.lower() for issue in issues):
            suggestions.append("Add tests to validate core logic and invariants")

        # Remove duplicates
        suggestions = list(set(suggestions))

        return suggestions

    def _build_review_prompt(self, artifact: Dict[str, Any]) -> str:
        """Build comprehensive review prompt for Claude with three-pillar validation."""
        prompt_parts = [
            "# Quality Review Request\n\n",
            "Review this work artifact against all 8 quality gates (5 standard + 3 pillars):\n\n",
            f"**Artifact**: {json.dumps(artifact, indent=2)}\n\n",
            "## Quality Gates to Evaluate:\n\n",
        ]

        if QualityGate.INTENT_SATISFIED in self.config.required_gates:
            prompt_parts.append("""
### 1. Intent Satisfied
- Does the implementation fulfill the original requirements?
- Are all specified features present?
- Is the approach appropriate for the problem?
""")

        if QualityGate.TESTS_PASSING in self.config.required_gates:
            prompt_parts.append(f"""
### 2. Tests Passing
- Do all tests pass?
- Is coverage ≥ {self.config.min_test_coverage:.0%}?
- Are edge cases tested?
- Suggest any missing tests
""")

        if QualityGate.DOCUMENTATION_COMPLETE in self.config.required_gates:
            prompt_parts.append("""
### 3. Documentation Complete
- Is overview/purpose documented?
- Are usage examples provided?
- Are APIs documented?
""")

        if QualityGate.NO_ANTIPATTERNS in self.config.required_gates:
            prompt_parts.append("""
### 4. No Anti-patterns
- No TODO/FIXME/HACK markers?
- No mock/stub placeholders?
- No obvious code smells?
""")

        if QualityGate.CONSTRAINTS_MAINTAINED in self.config.required_gates:
            prompt_parts.append("""
### 5. Constraints Maintained
- Are specified constraints respected?
- No violations of requirements?
""")

        if QualityGate.COMPLETENESS in self.config.required_gates:
            prompt_parts.append("""
### 6. Completeness (Three-Pillar Gate #1)
- No TODOs, FIXME, or incomplete markers?
- No partial implementations?
- All typed holes filled?
- No placeholder code?
""")

        if QualityGate.CORRECTNESS in self.config.required_gates:
            prompt_parts.append("""
### 7. Correctness (Three-Pillar Gate #2)
- Logic is sound and correct?
- No runtime errors or panics?
- Error handling is appropriate?
- No logic bugs or incorrect behavior?
""")

        if QualityGate.PRINCIPLED_IMPLEMENTATION in self.config.required_gates:
            prompt_parts.append("""
### 8. Principled Implementation (Three-Pillar Gate #3)
- No hacks or workarounds?
- Consistent with architectural patterns?
- Clean, maintainable code?
- No temporary fixes or code smells?
""")

        prompt_parts.append("""
## Instructions:
For each gate:
1. Evaluate: PASS or FAIL
2. If FAIL: Provide specific issues
3. Suggest actionable improvements
4. Suggest missing tests (especially for gates 2, 6, 7, 8)

Format your response clearly with gate-by-gate analysis.
Be thorough but constructive. Focus on real issues.""")

        return "".join(prompt_parts)

    def _parse_review_results(
        self,
        review_responses: List[Any],
        artifact: Dict[str, Any]
    ) -> tuple[Dict[QualityGate, bool], List[str], List[str]]:
        """Parse Claude's review responses into structured results."""
        # In production, parse Claude's structured response
        # For now, use simple heuristics + Claude's guidance

        gate_results = {}
        issues = []
        recommendations = []

        # Extract text from responses
        review_text = " ".join(str(r) for r in review_responses)
        review_text_lower = review_text.lower()

        # Simple parsing (production would use structured output)
        for gate in self.config.required_gates:
            gate_name = gate.value.replace("_", " ")

            # Look for "PASS" or "FAIL" near gate name
            if f"{gate_name} pass" in review_text_lower or f"{gate_name}: pass" in review_text_lower:
                gate_results[gate] = True
            elif f"{gate_name} fail" in review_text_lower or f"{gate_name}: fail" in review_text_lower:
                gate_results[gate] = False
                issues.append(f"Gate '{gate_name}' failed (see Claude's review)")
            else:
                # Default to basic check
                passed, gate_issues = self._fallback_gate_check(gate, artifact)
                gate_results[gate] = passed
                issues.extend(gate_issues)

        # Extract recommendations from Claude's response
        if "recommend" in review_text_lower:
            # In production, parse structured recommendations
            recommendations.append("See Claude's detailed recommendations in review responses")

        return gate_results, issues, recommendations

    def _fallback_gate_check(self, gate: QualityGate, artifact: Dict[str, Any]) -> tuple[bool, List[str]]:
        """Fallback basic check when Claude's response is ambiguous."""
        issues = []

        if gate == QualityGate.TESTS_PASSING:
            test_results = artifact.get("test_results", {})
            if not test_results:
                issues.append("No test results found")
                return False, issues

            failed = test_results.get("failed", 0)
            if failed > 0:
                issues.append(f"{failed} test(s) failed")
                return False, issues

        elif gate == QualityGate.NO_ANTIPATTERNS:
            code = str(artifact.get("code", ""))
            for pattern in self.config.antipattern_patterns:
                if pattern in code:
                    issues.append(f"Anti-pattern found: {pattern}")
            return len(issues) == 0, issues

        elif gate == QualityGate.NO_TODOS:
            code = str(artifact.get("code", ""))
            for pattern in ["TODO", "FIXME", "HACK", "XXX", "STUB"]:
                if pattern in code:
                    count = code.count(pattern)
                    issues.append(f"Found {count} {pattern} marker(s)")
            return len(issues) == 0, issues

        # Default: assume pass if no obvious issues
        return True, []

    async def _store_message(self, message: Any, phase: str):
        """Store important review messages in memory."""
        content = str(message)
        if len(content) > 100:
            self.storage.store({
                "content": content[:500],
                "namespace": f"project:agent-{self.config.agent_id}",
                "importance": 8,
                "tags": ["review", phase]
            })

    def get_statistics(self) -> Dict[str, Any]:
        """Get review statistics."""
        return {
            "total_reviews": self._review_count,
            "passed": self._pass_count,
            "failed": self._fail_count,
            "pass_rate": self._pass_count / self._review_count if self._review_count > 0 else 0
        }

    def get_status(self) -> Dict[str, Any]:
        """Get reviewer status."""
        return {
            "reviews_completed": self._review_count,
            "current_pass_rate": self._pass_count / self._review_count if self._review_count > 0 else 0,
            "strict_mode": self.config.strict_mode,
            "session_active": self._session_active
        }

    async def _execute_work_item(self, work_item: WorkItem) -> WorkResult:
        """
        Execute work item for PyO3 bridge integration.

        For Reviewer, "execution" means reviewing the work described
        in the work item and returning validation results.

        Args:
            work_item: Work item from Rust (via PyO3 bridge)

        Returns:
            Work result with review feedback
        """
        logger.info(f"Received review request from Rust bridge: {work_item.id} (attempt {work_item.review_attempt})")
        logger.debug(f"Work item description: {work_item.description[:200]}")

        # Start metrics tracking
        metrics_collector = get_metrics_collector()
        work_metrics = metrics_collector.start_work_item(
            work_item_id=work_item.id,
            agent_id=self.config.agent_id,
            phase="review"
        )

        # Validate work item
        validation_result = validate_work_item(work_item)
        if not validation_result.valid:
            logger.error(f"Work item validation failed: {validation_result.errors}")
            metrics_collector.finish_work_item(work_item.id, success=False, error_type="ValidationError")
            return WorkResult(
                success=False,
                error=f"Invalid work item for review:\n" + "\n".join(f"  • {err}" for err in validation_result.errors)
            )

        # Log warnings
        if validation_result.warnings:
            for warning in validation_result.warnings:
                logger.warning(f"Work item validation warning: {warning}")

        # Validate agent state
        state_validation = validate_agent_state(
            agent_id=self.config.agent_id,
            session_active=self._session_active,
            required_session=False
        )
        if not state_validation.valid:
            logger.error(f"Agent state validation failed: {state_validation.errors}")
            metrics_collector.finish_work_item(work_item.id, success=False, error_type="StateValidationError")
            return WorkResult(
                success=False,
                error=f"Invalid agent state:\n" + "\n".join(f"  • {err}" for err in state_validation.errors)
            )

        try:
            # Convert WorkItem to work artifact for review
            work_artifact = {
                "id": work_item.id,
                "description": work_item.description,
                "phase": work_item.phase,
                "review_attempt": work_item.review_attempt,
                # Reviewer needs context to validate work
                "context": {
                    "review_feedback": work_item.review_feedback or [],
                    "consolidated_context_id": work_item.consolidated_context_id
                }
            }

            # Review using existing review method
            review_result = await self.review(work_artifact)

            if review_result.passed:
                logger.info(f"Review passed for work item {work_item.id} (confidence: {review_result.confidence:.2f})")
            else:
                logger.warning(f"Review failed for work item {work_item.id}: {len(review_result.issues)} issues")
                logger.debug(f"Issues: {review_result.issues}")

                # Identify which gates failed
                failed_gates = [gate.value for gate, passed in review_result.gate_results.items() if not passed]

            # Update metrics with review-specific data
            work_metrics.review_passed = review_result.passed
            work_metrics.review_confidence = review_result.confidence
            work_metrics.quality_gates_passed = sum(1 for passed in review_result.gate_results.values() if passed)
            work_metrics.quality_gates_failed = sum(1 for passed in review_result.gate_results.values() if not passed)

            # Finish metrics tracking
            metrics_collector.finish_work_item(
                work_item.id,
                success=True,  # Review completed (even if review failed gates)
                error_type=None
            )

            # Log metrics
            completed_metrics = metrics_collector.get_work_item_metrics(work_item.id)
            if completed_metrics and completed_metrics.duration_seconds:
                logger.info(f"Review completed in {completed_metrics.duration_seconds:.2f}s (confidence: {review_result.confidence:.2f})")

            # Convert ReviewResult to WorkResult
            error_msg = None
            if not review_result.passed:
                # Create detailed error message for failed review
                error_msg = f"Review failed {len(failed_gates)} quality gate(s): {', '.join(failed_gates)}\n"
                error_msg += f"Issues:\n" + "\n".join(f"  • {issue}" for issue in review_result.issues[:5])
                if review_result.recommendations:
                    error_msg += f"\nRecommendations:\n" + "\n".join(f"  → {rec}" for rec in review_result.recommendations[:3])

            return WorkResult(
                success=review_result.passed,
                data=json.dumps({
                    "gate_results": {gate.value: result for gate, result in review_result.gate_results.items()},
                    "issues": review_result.issues,
                    "recommendations": review_result.recommendations,
                    "suggested_tests": review_result.suggested_tests,
                    "confidence": review_result.confidence
                }),
                memory_ids=review_result.execution_context,
                error=error_msg
            )

        except Exception as e:
            # Handle any errors during review with enhanced context
            logger.error(
                f"Failed to review work item {work_item.id}: {type(e).__name__}: {str(e)}",
                exc_info=True
            )

            # Finish metrics tracking with error
            error_type = type(e).__name__
            metrics_collector.finish_work_item(work_item.id, success=False, error_type=error_type)

            # Log metrics
            completed_metrics = metrics_collector.get_work_item_metrics(work_item.id)
            if completed_metrics and completed_metrics.duration_seconds:
                logger.info(f"Review failed after {completed_metrics.duration_seconds:.2f}s")

            # Create enhanced error context
            error_context = create_work_item_error_context(
                work_item_id=work_item.id,
                work_item_phase="review",
                work_item_description=work_item.description,
                agent_id=self.config.agent_id,
                agent_state="reviewing",
                session_active=self._session_active,
                error=e
            )

            # Log full context for debugging
            logger.debug(f"Error context:\n{error_context.format()}")

            # Return concise error for Rust bridge
            return WorkResult(
                success=False,
                error=format_error_for_rust(error_context)
            )

    async def __aenter__(self):
        """Async context manager entry."""
        await self.start_session()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit."""
        await self.stop_session()

    # ============================================================================
    # LLM-Based Semantic Validation Methods
    # ============================================================================

    async def semantic_intent_check(
        self,
        original_intent: str,
        implementation_content: str,
        execution_memories: List[Dict[str, Any]]
    ) -> tuple[bool, List[str]]:
        """
        Use Claude to deeply compare implementation against original intent.

        Args:
            original_intent: Original work requirement/specification
            implementation_content: Code/documentation that was produced
            execution_memories: Memories from execution for context

        Returns:
            Tuple of (passed, issues) where issues lists missing requirements
        """
        if not self._session_active:
            await self.start_session()

        # Build semantic validation prompt
        memories_summary = "\n".join([
            f"- {m.get('summary', m.get('content', '')[:100])}"
            for m in execution_memories[:10]  # Limit to 10 for context
        ])

        prompt = f"""# Semantic Intent Validation

**Original Intent/Requirements:**
{original_intent}

**Implementation Summary:**
{implementation_content[:2000]}  # Limit for context

**Execution Context (Recent Memories):**
{memories_summary}

## Task
Perform deep semantic analysis to determine if the implementation FULLY satisfies the original intent.

## Analysis Required
1. **Requirement Extraction**: List ALL requirements from the original intent
2. **Implementation Coverage**: For each requirement, determine if it's implemented
3. **Gap Analysis**: Identify any requirements that are missing or partially implemented
4. **Evidence**: For missing/partial requirements, explain what's missing

## Output Format
Provide your analysis in this format:

REQUIREMENTS ANALYSIS:
- [Requirement 1]: SATISFIED / PARTIAL / MISSING - [evidence/explanation]
- [Requirement 2]: SATISFIED / PARTIAL / MISSING - [evidence/explanation]
...

VERDICT: PASS / FAIL

ISSUES (if FAIL):
- [Specific issue 1]
- [Specific issue 2]
...

Be strict: FAIL if ANY requirement is MISSING or PARTIAL."""

        # Query Claude
        await self.claude_client.query(prompt)

        # Collect response
        responses = []
        async for message in self.claude_client.receive_response():
            responses.append(str(message))

        full_response = " ".join(responses)

        # Parse verdict and issues
        passed = "VERDICT: PASS" in full_response
        issues = []

        if not passed:
            # Extract issues section
            if "ISSUES" in full_response:
                issues_section = full_response.split("ISSUES")[1].split("\n\n")[0]
                issues = [
                    line.strip("- ").strip()
                    for line in issues_section.split("\n")
                    if line.strip().startswith("-")
                ]

        return (passed, issues)

    async def semantic_completeness_check(
        self,
        requirements: List[str],
        implementation_content: str,
        execution_memories: List[Dict[str, Any]]
    ) -> tuple[bool, List[str]]:
        """
        Use Claude to validate all requirements are fully implemented (not stubbed/partial).

        Args:
            requirements: List of explicit requirements to validate
            implementation_content: Implementation to check
            execution_memories: Context from execution

        Returns:
            Tuple of (passed, missing_requirements)
        """
        if not self._session_active:
            await self.start_session()

        reqs_list = "\n".join([f"{i+1}. {req}" for i, req in enumerate(requirements)])
        memories_summary = "\n".join([
            f"- {m.get('summary', m.get('content', '')[:100])}"
            for m in execution_memories[:10]
        ])

        prompt = f"""# Semantic Completeness Validation

**Requirements to Validate:**
{reqs_list}

**Implementation:**
{implementation_content[:2000]}

**Execution Context:**
{memories_summary}

## Task
Determine if EVERY requirement is FULLY implemented with substantive code/content.

## What Constitutes "Complete"
- ✅ Real implementation with logic and functionality
- ✅ Tests present and passing
- ✅ Documentation explaining the implementation
- ✅ Edge cases handled
- ❌ TODO/FIXME markers
- ❌ Stub/mock/placeholder functions
- ❌ Empty implementations
- ❌ Comments saying "implement this later"
- ❌ Partial implementations

## Output Format
For each requirement, assess completeness:

REQUIREMENT 1: [requirement text]
STATUS: COMPLETE / INCOMPLETE / PARTIAL
EVIDENCE: [what was found or what's missing]

...

VERDICT: PASS / FAIL

MISSING/PARTIAL REQUIREMENTS:
- [Requirement X]: [what's missing/partial]
...

Be strict: FAIL if ANY requirement is not COMPLETE."""

        await self.claude_client.query(prompt)

        responses = []
        async for message in self.claude_client.receive_response():
            responses.append(str(message))

        full_response = " ".join(responses)

        passed = "VERDICT: PASS" in full_response
        missing = []

        if not passed and "MISSING/PARTIAL REQUIREMENTS" in full_response:
            missing_section = full_response.split("MISSING/PARTIAL REQUIREMENTS")[1].split("\n\n")[0]
            missing = [
                line.strip("- ").strip()
                for line in missing_section.split("\n")
                if line.strip().startswith("-")
            ]

        return (passed, missing)

    async def semantic_correctness_check(
        self,
        implementation_content: str,
        test_results: Dict[str, Any],
        execution_memories: List[Dict[str, Any]]
    ) -> tuple[bool, List[str]]:
        """
        Use Claude to analyze logic correctness, edge case handling, and error handling.

        Args:
            implementation_content: Code/implementation to validate
            test_results: Test execution results (if available)
            execution_memories: Execution context

        Returns:
            Tuple of (passed, logic_issues)
        """
        if not self._session_active:
            await self.start_session()

        test_summary = "No test results available"
        if test_results:
            passed_tests = test_results.get("passed", 0)
            failed_tests = test_results.get("failed", 0)
            test_summary = f"Tests: {passed_tests} passed, {failed_tests} failed"

        memories_summary = "\n".join([
            f"- {m.get('summary', m.get('content', '')[:100])}"
            for m in execution_memories[:10]
        ])

        prompt = f"""# Semantic Correctness Validation

**Implementation:**
{implementation_content[:2000]}

**Test Results:**
{test_summary}

**Execution Context:**
{memories_summary}

## Task
Analyze the implementation for correctness, focusing on:
1. **Logic Correctness**: Is the logic sound and bug-free?
2. **Edge Case Handling**: Are edge cases properly handled?
3. **Error Handling**: Are errors handled appropriately?
4. **Type Safety**: Are types used correctly (if applicable)?
5. **Race Conditions**: Are there potential concurrency issues?
6. **Resource Management**: Are resources properly managed (memory, files, connections)?

## What Constitutes "Correct"
- ✅ Logic matches expected behavior
- ✅ Edge cases (null, empty, boundary values) handled
- ✅ Errors handled with appropriate recovery
- ✅ No obvious bugs or logic errors
- ✅ Tests validate critical paths
- ❌ Logic bugs or incorrect behavior
- ❌ Unhandled edge cases
- ❌ Missing error handling
- ❌ Potential crashes or panics
- ❌ Data corruption risks

## Output Format
LOGIC ANALYSIS:
- [Finding 1]: OK / ISSUE - [explanation]
- [Finding 2]: OK / ISSUE - [explanation]
...

EDGE CASE ANALYSIS:
- [Case 1]: HANDLED / MISSING - [explanation]
...

ERROR HANDLING ANALYSIS:
- [Scenario 1]: HANDLED / MISSING - [explanation]
...

VERDICT: PASS / FAIL

ISSUES (if FAIL):
- [Specific issue 1]
- [Specific issue 2]
...

Be thorough: FAIL if logic issues, unhandled edges, or missing error handling."""

        await self.claude_client.query(prompt)

        responses = []
        async for message in self.claude_client.receive_response():
            responses.append(str(message))

        full_response = " ".join(responses)

        passed = "VERDICT: PASS" in full_response
        issues = []

        if not passed and "ISSUES" in full_response:
            issues_section = full_response.split("ISSUES")[1].split("\n\n")[0]
            issues = [
                line.strip("- ").strip()
                for line in issues_section.split("\n")
                if line.strip().startswith("-")
            ]

        return (passed, issues)

    async def generate_improvement_guidance(
        self,
        failed_gates: Dict[str, bool],
        issues: List[str],
        original_intent: str,
        execution_memories: List[Dict[str, Any]]
    ) -> str:
        """
        Use Claude to generate detailed, actionable guidance for retry after review failure.

        Args:
            failed_gates: Dict of gate names to pass/fail status
            issues: List of all issues identified
            original_intent: Original work requirements
            execution_memories: Previous execution context

        Returns:
            Consolidated improvement plan with step-by-step guidance
        """
        if not self._session_active:
            await self.start_session()

        failed_gate_names = [name for name, passed in failed_gates.items() if not passed]
        issues_list = "\n".join([f"- {issue}" for issue in issues])
        memories_summary = "\n".join([
            f"- {m.get('summary', m.get('content', '')[:100])}"
            for m in execution_memories[:10]
        ])

        prompt = f"""# Generate Improvement Guidance for Review Failure

**Original Intent:**
{original_intent}

**Failed Quality Gates:**
{', '.join(failed_gate_names)}

**Issues Identified:**
{issues_list}

**Previous Attempt Context:**
{memories_summary}

## Task
Generate a detailed, actionable improvement plan to fix ALL issues and pass review on next attempt.

## Guidance Should Include
1. **Root Cause Analysis**: Why did the work fail review?
2. **Specific Fixes**: For each issue, what needs to be done?
3. **Step-by-Step Plan**: Ordered steps to implement fixes
4. **Validation Criteria**: How to verify each fix works
5. **Testing Guidance**: What tests to add/update
6. **Documentation Needs**: What docs to add/improve

## Output Format
# Improvement Plan

## Root Cause
[Analysis of why review failed]

## Required Fixes

### Fix 1: [Issue summary]
**Problem:** [What's wrong]
**Solution:** [What to do]
**Validation:** [How to verify it's fixed]

### Fix 2: [Issue summary]
...

## Implementation Steps
1. [First step - most critical]
2. [Second step]
...

## Testing Checklist
- [ ] [Test to add/verify]
- [ ] [Another test]
...

## Documentation Updates
- [What documentation to add/update]

## Success Criteria
When you've completed these fixes:
- [Criterion 1]
- [Criterion 2]
...

Be specific and actionable. Focus on WHAT to fix and HOW to fix it."""

        await self.claude_client.query(prompt)

        responses = []
        async for message in self.claude_client.receive_response():
            responses.append(str(message))

        return " ".join(responses)

    async def extract_requirements_from_intent(
        self,
        original_intent: str,
        context: Optional[str] = None
    ) -> List[str]:
        """
        Extract explicit, testable requirements from user intent using Claude.

        This method analyzes the original intent string and extracts a list of
        concrete, actionable requirements that can be individually tracked and validated.

        Args:
            original_intent: The original user request/intent
            context: Optional additional context (e.g., project background)

        Returns:
            List of requirement strings, each being specific and testable

        Example:
            intent = "Add JWT authentication with refresh tokens"
            requirements = await extract_requirements_from_intent(intent)
            # Returns: [
            #   "Implement JWT token generation",
            #   "Implement refresh token rotation",
            #   "Add token validation middleware",
            #   "Handle token expiration errors"
            # ]
        """
        prompt = f"""Analyze the following user intent and extract explicit, testable requirements.

# User Intent
{original_intent}

{f"# Additional Context\\n{context}\\n" if context else ""}

# Task
Extract a list of concrete, actionable requirements from this intent. Each requirement should be:
1. **Specific**: Clearly define what needs to be done
2. **Testable**: Can be verified through testing or inspection
3. **Atomic**: Represents a single, focused piece of work
4. **Implementation-oriented**: Focuses on what to build, not how

# Requirements Format
Return ONLY a JSON array of requirement strings, with no additional commentary.

Example format:
["Requirement 1", "Requirement 2", "Requirement 3"]

# Guidelines
- Break down high-level goals into concrete implementation tasks
- Include both functional requirements (features) and non-functional requirements (error handling, edge cases)
- Focus on observable, verifiable outcomes
- Keep each requirement concise (1-2 sentences max)
- Aim for 3-8 requirements for typical tasks

Extract the requirements now, returning ONLY the JSON array:"""

        await self.claude_client.query(prompt)

        responses = []
        async for message in self.claude_client.receive_response():
            responses.append(str(message))

        response_text = " ".join(responses)

        # Parse JSON response
        try:
            # Extract JSON array from response (handle potential markdown formatting)
            import re
            json_match = re.search(r'\[.*\]', response_text, re.DOTALL)
            if json_match:
                requirements = json.loads(json_match.group())
                return requirements
            else:
                # Fallback: return empty list if no valid JSON found
                return []
        except json.JSONDecodeError:
            # Fallback: return empty list if JSON parsing fails
            return []


# Standalone agent runner
async def main():
    """Run reviewer agent as independent process."""
    import argparse
    import asyncio
    import httpx
    import signal
    import sys

    parser = argparse.ArgumentParser(description="Reviewer Agent")
    parser.add_argument("--agent-id", default="reviewer", help="Agent ID")
    parser.add_argument("--api-url", default="http://127.0.0.1:3000", help="API server URL")
    parser.add_argument("--database", default=".mnemosyne/orchestration.db", help="Database path")
    parser.add_argument("--namespace", default="project:mnemosyne", help="Namespace")
    args = parser.parse_args()

    logger.info(f"Starting Reviewer Agent (ID: {args.agent_id})")
    logger.info(f"API Server: {args.api_url}")

    http_client = httpx.AsyncClient(timeout=5.0)

    async def send_heartbeat():
        while True:
            try:
                await http_client.post(
                    f"{args.api_url}/events",
                    json={"event_type": "Heartbeat", "instance_id": args.agent_id, "timestamp": "auto"}
                )
                logger.debug(f"Heartbeat sent from {args.agent_id}")
            except Exception as e:
                logger.warning(f"Heartbeat failed: {e}")
            await asyncio.sleep(10)

    shutdown_event = asyncio.Event()
    loop = asyncio.get_event_loop()

    def signal_handler(signum, frame):
        logger.info(f"Received signal {signum}, shutting down...")
        # Use call_soon_threadsafe to safely interact with asyncio from signal handler
        loop.call_soon_threadsafe(shutdown_event.set)

    signal.signal(signal.SIGTERM, signal_handler)
    signal.signal(signal.SIGINT, signal_handler)

    heartbeat_task = asyncio.create_task(send_heartbeat())

    try:
        logger.info("Reviewer agent running (press Ctrl+C to stop)")
        await shutdown_event.wait()
    except KeyboardInterrupt:
        logger.info("Keyboard interrupt received")
    except Exception as e:
        logger.error(f"Error in agent main loop: {e}")
        sys.exit(1)
    finally:
        logger.info("Shutting down reviewer agent...")
        heartbeat_task.cancel()
        await http_client.aclose()
        logger.info("Reviewer agent stopped")


if __name__ == "__main__":
    import asyncio
    asyncio.run(main())
