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
from typing import Dict, List, Optional, Set
from enum import Enum


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
    Quality assurance and validation specialist.

    Enforces quality standards before work completion:
    - All tests passing
    - Documentation complete
    - No anti-patterns
    - Facts verified
    - Intent satisfied
    """

    def __init__(self, config: ReviewerConfig, coordinator, storage):
        """
        Initialize Reviewer agent.

        Args:
            config: Reviewer configuration
            coordinator: PyCoordinator for shared state
            storage: PyStorage for memory operations
        """
        self.config = config
        self.coordinator = coordinator
        self.storage = storage

        # Register with coordinator
        self.coordinator.register_agent(config.agent_id)

        # State
        self._review_count = 0
        self._pass_count = 0
        self._fail_count = 0

    async def review(self, work_artifact: Dict[str, any]) -> ReviewResult:
        """
        Review work artifact against quality gates.

        Args:
            work_artifact: Artifact to review (code, docs, plan, etc.)

        Returns:
            ReviewResult with pass/fail and detailed feedback
        """
        self.coordinator.update_agent_state(self.config.agent_id, "running")
        self._review_count += 1

        try:
            gate_results = {}
            issues = []
            recommendations = []

            # Gate 1: Intent satisfaction
            if QualityGate.INTENT_SATISFIED in self.config.required_gates:
                passed, gate_issues = await self._check_intent(work_artifact)
                gate_results[QualityGate.INTENT_SATISFIED] = passed
                issues.extend(gate_issues)

            # Gate 2: Tests passing
            if QualityGate.TESTS_PASSING in self.config.required_gates:
                passed, gate_issues = await self._check_tests(work_artifact)
                gate_results[QualityGate.TESTS_PASSING] = passed
                issues.extend(gate_issues)

            # Gate 3: Documentation complete
            if QualityGate.DOCUMENTATION_COMPLETE in self.config.required_gates:
                passed, gate_issues = await self._check_documentation(work_artifact)
                gate_results[QualityGate.DOCUMENTATION_COMPLETE] = passed
                issues.extend(gate_issues)

            # Gate 4: No anti-patterns
            if QualityGate.NO_ANTIPATTERNS in self.config.required_gates:
                passed, gate_issues = await self._check_antipatterns(work_artifact)
                gate_results[QualityGate.NO_ANTIPATTERNS] = passed
                issues.extend(gate_issues)

            # Gate 5: Facts verified
            if QualityGate.FACTS_VERIFIED in self.config.required_gates:
                passed, gate_issues = await self._check_facts(work_artifact)
                gate_results[QualityGate.FACTS_VERIFIED] = passed
                issues.extend(gate_issues)

            # Gate 6: Constraints maintained
            if QualityGate.CONSTRAINTS_MAINTAINED in self.config.required_gates:
                passed, gate_issues = await self._check_constraints(work_artifact)
                gate_results[QualityGate.CONSTRAINTS_MAINTAINED] = passed
                issues.extend(gate_issues)

            # Gate 7: No TODOs/stubs
            if QualityGate.NO_TODOS in self.config.required_gates:
                passed, gate_issues = await self._check_todos(work_artifact)
                gate_results[QualityGate.NO_TODOS] = passed
                issues.extend(gate_issues)

            # Determine overall pass/fail
            all_passed = all(gate_results.values())

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

            self.coordinator.update_agent_state(
                self.config.agent_id,
                "complete" if all_passed else "blocked"
            )

            return result

        except Exception as e:
            self.coordinator.update_agent_state(self.config.agent_id, "failed")
            raise RuntimeError(f"Review failed: {e}") from e

    async def _check_intent(self, artifact: Dict) -> tuple[bool, List[str]]:
        """Check if work satisfies original intent."""
        issues = []

        # Check if intent field exists
        if "intent" not in artifact:
            issues.append("No intent specification found")
            return False, issues

        # Check if implementation field exists
        if "implementation" not in artifact:
            issues.append("No implementation found")
            return False, issues

        # Basic intent matching (production: use LLM for semantic matching)
        intent = str(artifact.get("intent", "")).lower()
        implementation = str(artifact.get("implementation", "")).lower()

        # Extract key terms from intent
        intent_terms = set(intent.split())
        impl_terms = set(implementation.split())

        # Check coverage of intent terms
        covered = intent_terms & impl_terms
        coverage = len(covered) / len(intent_terms) if intent_terms else 0

        if coverage < 0.5:
            issues.append(f"Low intent coverage: {coverage:.1%}")
            return False, issues

        return True, []

    async def _check_tests(self, artifact: Dict) -> tuple[bool, List[str]]:
        """Check if tests exist and pass."""
        issues = []

        # Check for test results
        test_results = artifact.get("test_results")
        if not test_results:
            issues.append("No test results found")
            return False, issues

        # Check test coverage
        coverage = test_results.get("coverage", 0.0)
        if coverage < self.config.min_test_coverage:
            issues.append(
                f"Test coverage {coverage:.1%} below minimum {self.config.min_test_coverage:.1%}"
            )
            return False, issues

        # Check if tests passed
        passed = test_results.get("passed", 0)
        failed = test_results.get("failed", 0)

        if failed > 0:
            issues.append(f"{failed} test(s) failed")
            return False, issues

        if passed == 0:
            issues.append("No tests executed")
            return False, issues

        return True, []

    async def _check_documentation(self, artifact: Dict) -> tuple[bool, List[str]]:
        """Check if documentation is complete."""
        issues = []

        # Check for documentation
        docs = artifact.get("documentation")
        if not docs:
            issues.append("No documentation found")
            return False, issues

        # Check required sections
        required_sections = ["overview", "usage", "examples"]
        missing = [s for s in required_sections if s not in docs]

        if missing:
            issues.append(f"Missing documentation sections: {', '.join(missing)}")
            return False, issues

        return True, []

    async def _check_antipatterns(self, artifact: Dict) -> tuple[bool, List[str]]:
        """Check for anti-patterns in code."""
        issues = []

        # Get code content
        code = str(artifact.get("code", ""))

        # Check for anti-pattern markers
        for pattern in self.config.antipattern_patterns:
            if pattern in code:
                issues.append(f"Anti-pattern found: {pattern}")

        return len(issues) == 0, issues

    async def _check_facts(self, artifact: Dict) -> tuple[bool, List[str]]:
        """Verify factual claims and references."""
        issues = []

        # Check for unverified claims
        claims = artifact.get("claims", [])
        for claim in claims:
            if not claim.get("verified"):
                issues.append(f"Unverified claim: {claim.get('text')}")

        # Check for broken references
        references = artifact.get("references", [])
        for ref in references:
            if not ref.get("accessible"):
                issues.append(f"Inaccessible reference: {ref.get('url')}")

        return len(issues) == 0, issues

    async def _check_constraints(self, artifact: Dict) -> tuple[bool, List[str]]:
        """Check if constraints are maintained."""
        issues = []

        # Get constraints
        constraints = artifact.get("constraints", [])
        violations = artifact.get("constraint_violations", [])

        if violations:
            for violation in violations:
                issues.append(f"Constraint violated: {violation}")

        return len(issues) == 0, issues

    async def _check_todos(self, artifact: Dict) -> tuple[bool, List[str]]:
        """Check for TODO/FIXME/stub comments."""
        issues = []

        code = str(artifact.get("code", ""))

        # Check for TODO markers
        for pattern in ["TODO", "FIXME", "HACK", "XXX", "STUB"]:
            if pattern in code:
                # Count occurrences
                count = code.count(pattern)
                issues.append(f"Found {count} {pattern} marker(s)")

        return len(issues) == 0, issues

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
            "strict_mode": self.config.strict_mode
        }
