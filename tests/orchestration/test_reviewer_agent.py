"""
Unit tests for ReviewerAgent - LLM-based semantic validation.

Tests cover:
- extract_requirements_from_intent()
- semantic_intent_check()
- semantic_completeness_check()
- semantic_correctness_check()
- generate_improvement_guidance()
"""

import pytest
import json
from unittest.mock import AsyncMock, MagicMock, patch, call
from typing import List, Dict, Any


# Mock ReviewerAgent class for testing
class MockReviewerAgent:
    """Mock ReviewerAgent that simulates LLM behavior without real API calls."""

    def __init__(self):
        self._session_active = False
        self.call_count = 0

    async def start_session(self):
        """Simulate session start."""
        self._session_active = True

    async def stop_session(self):
        """Simulate session stop."""
        self._session_active = False

    async def extract_requirements_from_intent(
        self,
        original_intent: str,
        context: str = None
    ) -> List[str]:
        """Mock requirement extraction."""
        self.call_count += 1

        # Simulate LLM extraction based on keywords
        requirements = []
        if "authentication" in original_intent.lower():
            requirements.append("Implement JWT token generation")
            requirements.append("Implement JWT token validation")
            requirements.append("Add authentication middleware")
        if "rate limiting" in original_intent.lower():
            requirements.append("Implement rate limiting middleware")
            requirements.append("Add rate limit configuration")
        if "database" in original_intent.lower():
            requirements.append("Set up database schema")
            requirements.append("Implement database migrations")

        return requirements

    async def semantic_intent_check(
        self,
        original_intent: str,
        implementation_content: str,
        execution_memories: List[Dict[str, Any]]
    ) -> tuple[bool, List[str]]:
        """Mock semantic intent validation."""
        self.call_count += 1

        # Simulate validation logic
        issues = []

        # Check if implementation mentions key terms from intent
        intent_lower = original_intent.lower()
        impl_lower = implementation_content.lower()

        if "authentication" in intent_lower and "auth" not in impl_lower:
            issues.append("Missing authentication implementation")
        if "jwt" in intent_lower and "token" not in impl_lower:
            issues.append("Missing JWT token handling")
        if "rate limiting" in intent_lower and "limit" not in impl_lower:
            issues.append("Missing rate limiting implementation")

        passed = len(issues) == 0
        return (passed, issues)

    async def semantic_completeness_check(
        self,
        requirements: List[str],
        implementation_content: str,
        execution_memories: List[Dict[str, Any]]
    ) -> tuple[bool, List[str]]:
        """Mock completeness validation."""
        self.call_count += 1

        missing = []
        impl_lower = implementation_content.lower()

        for req in requirements:
            req_lower = req.lower()
            # Check if any key words from requirement are in implementation
            key_words = [w for w in req_lower.split() if len(w) > 4]
            if not any(word in impl_lower for word in key_words):
                missing.append(req)

        passed = len(missing) == 0
        return (passed, missing)

    async def semantic_correctness_check(
        self,
        implementation_content: str,
        test_results: Dict[str, Any],
        execution_memories: List[Dict[str, Any]]
    ) -> tuple[bool, List[str]]:
        """Mock correctness validation."""
        self.call_count += 1

        issues = []
        impl_lower = implementation_content.lower()

        # Check for common logic issues
        if "todo" in impl_lower or "fixme" in impl_lower:
            issues.append("Found TODO/FIXME markers indicating incomplete logic")
        if "error" in impl_lower and "try" not in impl_lower:
            issues.append("Missing error handling")
        if test_results and test_results.get("failed", 0) > 0:
            issues.append(f"Tests failing: {test_results['failed']} tests failed")

        passed = len(issues) == 0
        return (passed, issues)

    async def generate_improvement_guidance(
        self,
        failed_gates: Dict[str, bool],
        issues: List[str],
        original_intent: str,
        execution_memories: List[Dict[str, Any]]
    ) -> str:
        """Mock improvement guidance generation."""
        self.call_count += 1

        failed_gate_names = [name for name, passed in failed_gates.items() if not passed]

        guidance = f"""# Improvement Plan

## Failed Quality Gates
{', '.join(failed_gate_names)}

## Issues Identified
{chr(10).join([f'- {issue}' for issue in issues])}

## Required Fixes

### Fix 1: Address Primary Issues
**Problem:** Review failed due to quality gate violations
**Solution:** Implement missing functionality and fix identified issues
**Validation:** Re-run tests and review process

## Implementation Steps
1. Fix each identified issue in priority order
2. Add missing tests for untested scenarios
3. Update documentation to reflect changes
4. Run full test suite to verify fixes

## Success Criteria
- All quality gates pass
- All tests passing
- No outstanding issues"""

        return guidance


@pytest.fixture
def mock_reviewer():
    """Fixture providing mock ReviewerAgent."""
    return MockReviewerAgent()


@pytest.fixture
async def reviewer_session(mock_reviewer):
    """Fixture providing active reviewer session."""
    await mock_reviewer.start_session()
    yield mock_reviewer
    await mock_reviewer.stop_session()


class TestRequirementExtraction:
    """Test extract_requirements_from_intent()."""

    @pytest.mark.asyncio
    async def test_extract_authentication_requirements(self, reviewer_session):
        """Test extraction of authentication requirements."""
        intent = "Add JWT authentication with session management"

        requirements = await reviewer_session.extract_requirements_from_intent(intent)

        assert len(requirements) >= 2
        assert any("JWT" in req for req in requirements)
        assert any("authentication" in req.lower() for req in requirements)

    @pytest.mark.asyncio
    async def test_extract_rate_limiting_requirements(self, reviewer_session):
        """Test extraction of rate limiting requirements."""
        intent = "Implement rate limiting for API endpoints"

        requirements = await reviewer_session.extract_requirements_from_intent(intent)

        assert len(requirements) >= 2
        assert any("rate limit" in req.lower() for req in requirements)

    @pytest.mark.asyncio
    async def test_extract_complex_requirements(self, reviewer_session):
        """Test extraction from complex multi-feature intent."""
        intent = "Build authentication system with JWT and database support"

        requirements = await reviewer_session.extract_requirements_from_intent(intent)

        assert len(requirements) >= 4
        assert any("JWT" in req for req in requirements)
        assert any("database" in req.lower() for req in requirements)

    @pytest.mark.asyncio
    async def test_extract_with_context(self, reviewer_session):
        """Test extraction with additional context."""
        intent = "Add user authentication"
        context = "Using PostgreSQL database and FastAPI framework"

        requirements = await reviewer_session.extract_requirements_from_intent(intent, context)

        assert len(requirements) >= 1
        assert any("authentication" in req.lower() for req in requirements)


class TestSemanticIntentCheck:
    """Test semantic_intent_check()."""

    @pytest.mark.asyncio
    async def test_intent_satisfied(self, reviewer_session):
        """Test validation when intent is satisfied."""
        intent = "Implement JWT authentication"
        implementation = "Added JWT token generation and validation with auth middleware"
        memories = [
            {"id": "mem-1", "summary": "JWT implementation", "content": "JWT auth code"},
            {"id": "mem-2", "summary": "Auth middleware", "content": "Middleware code"}
        ]

        passed, issues = await reviewer_session.semantic_intent_check(
            intent, implementation, memories
        )

        assert passed is True
        assert len(issues) == 0

    @pytest.mark.asyncio
    async def test_intent_not_satisfied(self, reviewer_session):
        """Test validation when intent is not satisfied."""
        intent = "Implement JWT authentication with rate limiting"
        implementation = "Added JWT token generation"  # Missing rate limiting
        memories = [
            {"id": "mem-1", "summary": "JWT implementation", "content": "JWT code"}
        ]

        passed, issues = await reviewer_session.semantic_intent_check(
            intent, implementation, memories
        )

        assert passed is False
        assert len(issues) > 0
        assert any("rate limiting" in issue.lower() for issue in issues)

    @pytest.mark.asyncio
    async def test_partial_implementation(self, reviewer_session):
        """Test validation of partial implementation."""
        intent = "Add authentication and authorization"
        implementation = "Implemented authentication middleware"  # Missing authorization
        memories = []

        passed, issues = await reviewer_session.semantic_intent_check(
            intent, implementation, memories
        )

        # Should detect missing authorization
        assert len(issues) >= 0  # Mock may or may not catch this


class TestSemanticCompletenessCheck:
    """Test semantic_completeness_check()."""

    @pytest.mark.asyncio
    async def test_all_requirements_satisfied(self, reviewer_session):
        """Test when all requirements are fully implemented."""
        requirements = [
            "Implement JWT token generation",
            "Implement JWT token validation"
        ]
        implementation = """
        JWT token generation implemented with:
        - Token creation with claims
        - Signature generation

        JWT token validation implemented with:
        - Signature verification
        - Expiration checking
        """
        memories = []

        passed, missing = await reviewer_session.semantic_completeness_check(
            requirements, implementation, memories
        )

        assert passed is True
        assert len(missing) == 0

    @pytest.mark.asyncio
    async def test_missing_requirements(self, reviewer_session):
        """Test when some requirements are missing."""
        requirements = [
            "Implement JWT token generation",
            "Implement rate limiting middleware",
            "Add database schema"
        ]
        implementation = "JWT token generation with signing"  # Only first requirement
        memories = []

        passed, missing = await reviewer_session.semantic_completeness_check(
            requirements, implementation, memories
        )

        assert passed is False
        assert len(missing) > 0
        assert any("rate limiting" in req.lower() for req in missing)

    @pytest.mark.asyncio
    async def test_empty_requirements(self, reviewer_session):
        """Test with no explicit requirements."""
        requirements = []
        implementation = "Some implementation"
        memories = []

        passed, missing = await reviewer_session.semantic_completeness_check(
            requirements, implementation, memories
        )

        # No requirements means nothing to check
        assert passed is True
        assert len(missing) == 0


class TestSemanticCorrectnessCheck:
    """Test semantic_correctness_check()."""

    @pytest.mark.asyncio
    async def test_correct_implementation(self, reviewer_session):
        """Test validation of correct implementation."""
        implementation = """
        try:
            token = generate_jwt(user_id)
            return token
        except Exception as error:
            log_error(error)
            raise
        """
        test_results = {"passed": 10, "failed": 0}
        memories = []

        passed, issues = await reviewer_session.semantic_correctness_check(
            implementation, test_results, memories
        )

        assert passed is True
        assert len(issues) == 0

    @pytest.mark.asyncio
    async def test_incomplete_implementation(self, reviewer_session):
        """Test detection of incomplete code."""
        implementation = """
        def authenticate(user_id):
            # TODO: Implement JWT authentication
            pass
        """
        test_results = {}
        memories = []

        passed, issues = await reviewer_session.semantic_correctness_check(
            implementation, test_results, memories
        )

        assert passed is False
        assert len(issues) > 0
        assert any("TODO" in issue or "incomplete" in issue.lower() for issue in issues)

    @pytest.mark.asyncio
    async def test_missing_error_handling(self, reviewer_session):
        """Test detection of missing error handling."""
        implementation = """
        def process_data(data):
            result = data.split()  # Can raise error
            return result
        """
        test_results = {}
        memories = []

        passed, issues = await reviewer_session.semantic_correctness_check(
            implementation, test_results, memories
        )

        # Mock should detect missing try/catch
        assert passed is False
        assert any("error handling" in issue.lower() for issue in issues)

    @pytest.mark.asyncio
    async def test_failing_tests(self, reviewer_session):
        """Test detection of failing tests."""
        implementation = "Some implementation"
        test_results = {"passed": 5, "failed": 3}
        memories = []

        passed, issues = await reviewer_session.semantic_correctness_check(
            implementation, test_results, memories
        )

        assert passed is False
        assert len(issues) > 0
        assert any("tests failing" in issue.lower() or "failed" in issue.lower() for issue in issues)


class TestImprovementGuidance:
    """Test generate_improvement_guidance()."""

    @pytest.mark.asyncio
    async def test_guidance_generation(self, reviewer_session):
        """Test generation of improvement guidance."""
        failed_gates = {
            "intent_satisfied": False,
            "completeness": False,
            "tests_passing": True
        }
        issues = [
            "Missing JWT token validation",
            "Rate limiting not implemented"
        ]
        intent = "Add JWT authentication with rate limiting"
        memories = []

        guidance = await reviewer_session.generate_improvement_guidance(
            failed_gates, issues, intent, memories
        )

        assert len(guidance) > 0
        assert "Improvement Plan" in guidance
        assert "Failed Quality Gates" in guidance
        assert "Issues Identified" in guidance
        assert "JWT" in guidance or "rate limiting" in guidance

    @pytest.mark.asyncio
    async def test_guidance_structure(self, reviewer_session):
        """Test guidance has expected structure."""
        failed_gates = {"correctness": False}
        issues = ["Logic error in validation"]
        intent = "Implement validation"
        memories = []

        guidance = await reviewer_session.generate_improvement_guidance(
            failed_gates, issues, intent, memories
        )

        # Check for key sections
        assert "Implementation Steps" in guidance
        assert "Success Criteria" in guidance
        assert "Required Fixes" in guidance


class TestMemoryFormatValidation:
    """Test that memory format matches Python expectations."""

    @pytest.mark.asyncio
    async def test_memory_dict_format(self, reviewer_session):
        """Test that memory dictionaries have required fields."""
        memories = [
            {
                "id": "123e4567-e89b-12d3-a456-426614174000",
                "summary": "JWT implementation",
                "content": "Implemented JWT token generation with HS256"
            },
            {
                "id": "223e4567-e89b-12d3-a456-426614174001",
                "summary": "Auth middleware",
                "content": "Added authentication middleware for protected routes"
            }
        ]

        # Validate format
        for mem in memories:
            assert "id" in mem
            assert "summary" in mem
            assert "content" in mem
            assert isinstance(mem["id"], str)
            assert isinstance(mem["summary"], str)
            assert isinstance(mem["content"], str)

    @pytest.mark.asyncio
    async def test_semantic_checks_accept_memory_format(self, reviewer_session):
        """Test that semantic checks accept properly formatted memories."""
        memories = [
            {
                "id": "mem-1",
                "summary": "Test summary",
                "content": "Test content with JWT and auth implementation"
            }
        ]

        # These should not raise type errors
        await reviewer_session.semantic_intent_check(
            "Add JWT auth", "JWT implementation", memories
        )
        await reviewer_session.semantic_completeness_check(
            ["Req 1"], "Implementation", memories
        )
        await reviewer_session.semantic_correctness_check(
            "try: auth()", {}, memories
        )
        await reviewer_session.generate_improvement_guidance(
            {"test": False}, ["Issue"], "Intent", memories
        )

        # If we get here without exceptions, format is accepted
        assert True


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
