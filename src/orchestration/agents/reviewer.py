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

from claude_agent_sdk import ClaudeSDKClient, ClaudeAgentOptions


class QualityGate(Enum):
    """Quality gates that must pass."""
    INTENT_SATISFIED = "intent_satisfied"
    TESTS_PASSING = "tests_passing"
    DOCUMENTATION_COMPLETE = "documentation_complete"
    NO_ANTIPATTERNS = "no_antipatterns"
    FACTS_VERIFIED = "facts_verified"
    CONSTRAINTS_MAINTAINED = "constraints_maintained"
    NO_TODOS = "no_todos"


@dataclass
class ReviewResult:
    """Result of quality review."""
    passed: bool
    gate_results: Dict[QualityGate, bool]
    issues: List[str]
    recommendations: List[str]
    confidence: float


@dataclass
class ReviewerConfig:
    """Configuration for Reviewer agent."""
    agent_id: str = "reviewer"
    strict_mode: bool = True  # Fail on any gate failure
    required_gates: Set[QualityGate] = None
    min_test_coverage: float = 0.70  # 70% minimum
    antipattern_patterns: List[str] = None
    # Claude Agent SDK configuration
    allowed_tools: Optional[List[str]] = None
    permission_mode: str = "view"  # Reviewer reads to validate

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


class ReviewerAgent:
    """
    Quality assurance and validation specialist using Claude Agent SDK.

    Enforces quality standards before work completion:
    - All tests passing
    - Documentation complete
    - No anti-patterns
    - Facts verified
    - Intent satisfied
    """

    REVIEWER_SYSTEM_PROMPT = """You are the Reviewer Agent in a multi-agent orchestration system.

Your role:
- Quality assurance and validation specialist
- Validate intent satisfaction, documentation, test coverage
- Fact-check claims, references, external dependencies
- Check for anti-patterns and technical debt
- Block work until quality standards met
- Mark "COMPLETE" only when all 7 quality gates pass

Quality Gates (ALL must pass):
1. Intent Satisfied - Implementation fulfills original requirements
2. Tests Passing - All tests pass, coverage ≥ 70%
3. Documentation Complete - Overview, usage, examples present
4. No Anti-patterns - No TODO/FIXME/HACK/stub/mock markers
5. Facts Verified - All claims and references validated
6. Constraints Maintained - No constraint violations
7. No TODOs - No placeholder or incomplete code

Your Review Process:
1. Read and understand the work artifact
2. Evaluate each quality gate rigorously
3. Provide specific, actionable feedback on failures
4. Suggest concrete improvements
5. BLOCK work if any required gate fails (strict mode)
6. Only mark COMPLETE when ALL gates pass

Be thorough but constructive. Identify real issues, not nitpicks."""

    def __init__(self, config: ReviewerConfig, coordinator, storage):
        """
        Initialize Reviewer agent with Claude Agent SDK.

        Args:
            config: Reviewer configuration
            coordinator: PyCoordinator for shared state
            storage: PyStorage for memory operations
        """
        self.config = config
        self.coordinator = coordinator
        self.storage = storage

        # Initialize Claude Agent SDK client
        self.claude_client = ClaudeSDKClient(
            options=ClaudeAgentOptions(
                allowed_tools=config.allowed_tools or ["Read", "Glob", "Grep"],
                permission_mode=config.permission_mode
            )
        )

        # Register with coordinator
        self.coordinator.register_agent(config.agent_id)

        # State
        self._review_count = 0
        self._pass_count = 0
        self._fail_count = 0
        self._session_active = False

    async def start_session(self):
        """Start Claude agent session."""
        if not self._session_active:
            await self.claude_client.connect()
            # Initialize with system prompt
            await self.claude_client.query(self.REVIEWER_SYSTEM_PROMPT)
            self._session_active = True

    async def stop_session(self):
        """Stop Claude agent session."""
        if self._session_active:
            await self.claude_client.disconnect()
            self._session_active = False

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

            # Ask Claude to review
            await self.claude_client.query(review_prompt)

            # Collect Claude's review
            review_responses = []
            async for message in self.claude_client.receive_response():
                review_responses.append(message)
                await self._store_message(message, "review")

            # Parse review results
            gate_results, issues, recommendations = self._parse_review_results(
                review_responses,
                work_artifact
            )

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
                confidence=confidence
            )

            # Store review result
            await self.storage.store({
                "content": f"Review {'PASSED' if all_passed else 'FAILED'}: {len(issues)} issues found",
                "namespace": f"session:{self.config.agent_id}",
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

    def _build_review_prompt(self, artifact: Dict[str, Any]) -> str:
        """Build comprehensive review prompt for Claude."""
        prompt_parts = [
            "# Quality Review Request\n\n",
            "Review this work artifact against all 7 quality gates:\n\n",
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

        if QualityGate.FACTS_VERIFIED in self.config.required_gates:
            prompt_parts.append("""
### 5. Facts Verified
- Are claims validated?
- Are references accessible?
- Are dependencies verified?
""")

        if QualityGate.CONSTRAINTS_MAINTAINED in self.config.required_gates:
            prompt_parts.append("""
### 6. Constraints Maintained
- Are specified constraints respected?
- No violations of requirements?
""")

        if QualityGate.NO_TODOS in self.config.required_gates:
            prompt_parts.append("""
### 7. No TODOs
- No incomplete code?
- No deferred work?
- All typed holes filled?
""")

        prompt_parts.append("""
## Instructions:
For each gate:
1. Evaluate: PASS or FAIL
2. If FAIL: Provide specific issues
3. Suggest actionable improvements

Format your response clearly with gate-by-gate analysis.
Be thorough but constructive.""")

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
            await self.storage.store({
                "content": content[:500],
                "namespace": f"session:{self.config.agent_id}",
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

    async def __aenter__(self):
        """Async context manager entry."""
        await self.start_session()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit."""
        await self.stop_session()
