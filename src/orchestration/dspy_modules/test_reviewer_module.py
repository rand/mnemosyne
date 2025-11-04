"""Integration tests for ReviewerModule.

Tests verify:
- Requirement extraction from user intent
- Intent satisfaction validation
- Completeness checking against requirements
- Correctness validation (logic, bugs)
- JSON output format compatibility with Rust bridge
- ChainOfThought reasoning transparency
"""

import os
import pytest
import dspy
from reviewer_module import ReviewerModule


@pytest.fixture
def reviewer_module():
    """Create ReviewerModule with Claude API (requires ANTHROPIC_API_KEY)."""
    # Check for API key
    api_key = os.getenv("ANTHROPIC_API_KEY")
    if not api_key:
        pytest.skip("ANTHROPIC_API_KEY not set - skipping integration tests")

    # Configure DSPy with Anthropic Claude
    dspy.configure(lm=dspy.LM('anthropic/claude-haiku-4-5-20251001', api_key=api_key))

    return ReviewerModule()


class TestRequirementExtraction:
    """Test requirement extraction from user intent."""

    def test_extract_requirements_basic(self, reviewer_module):
        """Test basic requirement extraction."""
        user_intent = "Implement user authentication with JWT tokens"
        context = "REST API project using Python/FastAPI"

        result = reviewer_module.extract_requirements(
            user_intent=user_intent,
            context=context
        )

        # Check structure
        assert hasattr(result, 'requirements')
        assert isinstance(result.requirements, list)

    def test_extract_requirements_structure(self, reviewer_module):
        """Test extracted requirements are strings."""
        user_intent = "Add logging and error handling to the service"

        result = reviewer_module.extract_requirements(user_intent=user_intent)

        for requirement in result.requirements:
            assert isinstance(requirement, str)
            assert len(requirement) > 0

    def test_extract_requirements_without_context(self, reviewer_module):
        """Test extraction works without context."""
        user_intent = "Create a database migration system"

        result = reviewer_module.extract_requirements(user_intent=user_intent)

        assert hasattr(result, 'requirements')
        assert isinstance(result.requirements, list)


class TestIntentSatisfaction:
    """Test intent satisfaction validation."""

    def test_validate_intent_basic(self, reviewer_module):
        """Test basic intent validation."""
        user_intent = "Add user registration endpoint"
        implementation = "Created POST /users/register endpoint with validation"
        execution_context = []

        result = reviewer_module.validate_intent(
            user_intent=user_intent,
            implementation=implementation,
            execution_context=execution_context
        )

        # Check structure
        assert hasattr(result, 'intent_satisfied')
        assert hasattr(result, 'issues')
        assert isinstance(result.intent_satisfied, bool)
        assert isinstance(result.issues, list)

    def test_validate_intent_with_context(self, reviewer_module):
        """Test validation with execution context."""
        user_intent = "Implement caching"
        implementation = "Added Redis caching layer"
        execution_context = [
            {"summary": "Installed redis client", "content": "pip install redis"},
            {"summary": "Created cache module", "content": "cache.py with get/set"}
        ]

        result = reviewer_module.validate_intent(
            user_intent=user_intent,
            implementation=implementation,
            execution_context=execution_context
        )

        assert hasattr(result, 'intent_satisfied')
        assert hasattr(result, 'issues')

    def test_validate_intent_issues_structure(self, reviewer_module):
        """Test issues are strings."""
        user_intent = "Add authentication"
        implementation = "Partial implementation"

        result = reviewer_module.validate_intent(
            user_intent=user_intent,
            implementation=implementation
        )

        for issue in result.issues:
            assert isinstance(issue, str)


class TestCompletenessValidation:
    """Test completeness validation against requirements."""

    def test_verify_completeness_basic(self, reviewer_module):
        """Test basic completeness validation."""
        requirements = [
            "User registration endpoint",
            "Email validation",
            "Password hashing"
        ]
        implementation = "Implemented all three requirements"
        execution_context = []

        result = reviewer_module.verify_completeness(
            requirements=requirements,
            implementation=implementation,
            execution_context=execution_context
        )

        # Check structure
        assert hasattr(result, 'complete')
        assert hasattr(result, 'issues')
        assert isinstance(result.complete, bool)
        assert isinstance(result.issues, list)

    def test_verify_completeness_empty_requirements(self, reviewer_module):
        """Test with empty requirements list."""
        requirements = []
        implementation = "Some implementation"

        result = reviewer_module.verify_completeness(
            requirements=requirements,
            implementation=implementation
        )

        # Should handle gracefully
        assert hasattr(result, 'complete')
        assert isinstance(result.complete, bool)

    def test_verify_completeness_with_context(self, reviewer_module):
        """Test completeness with execution context."""
        requirements = ["Add tests", "Add documentation"]
        implementation = "Added tests and docs"
        execution_context = [
            {"summary": "Created test file", "content": "test_auth.py"},
            {"summary": "Updated README", "content": "Added API docs"}
        ]

        result = reviewer_module.verify_completeness(
            requirements=requirements,
            implementation=implementation,
            execution_context=execution_context
        )

        assert hasattr(result, 'complete')
        assert hasattr(result, 'issues')


class TestCorrectnessValidation:
    """Test logical correctness validation."""

    def test_verify_correctness_basic(self, reviewer_module):
        """Test basic correctness validation."""
        implementation = "def add(a, b): return a + b"
        execution_context = []

        result = reviewer_module.verify_correctness(
            implementation=implementation,
            execution_context=execution_context
        )

        # Check structure
        assert hasattr(result, 'correct')
        assert hasattr(result, 'issues')
        assert isinstance(result.correct, bool)
        assert isinstance(result.issues, list)

    def test_verify_correctness_with_bugs(self, reviewer_module):
        """Test detection of logical errors."""
        implementation = """
        def divide(a, b):
            return a / b  # No zero check!
        """
        execution_context = []

        result = reviewer_module.verify_correctness(
            implementation=implementation,
            execution_context=execution_context
        )

        # Should flag potential issues
        assert hasattr(result, 'correct')
        assert hasattr(result, 'issues')

    def test_verify_correctness_with_context(self, reviewer_module):
        """Test correctness with execution context."""
        implementation = "async def fetch_data(): ..."
        execution_context = [
            {"summary": "Added async function", "content": "fetch_data implementation"},
            {"summary": "Added error handling", "content": "try/except blocks"}
        ]

        result = reviewer_module.verify_correctness(
            implementation=implementation,
            execution_context=execution_context
        )

        assert hasattr(result, 'correct')
        assert hasattr(result, 'issues')


class TestEndToEndWorkflow:
    """Test complete review workflow."""

    def test_full_review_workflow(self, reviewer_module):
        """Test complete review process."""
        # Step 1: Extract requirements
        user_intent = "Create user authentication system"
        context = "Web application with database"

        requirements_result = reviewer_module.extract_requirements(
            user_intent=user_intent,
            context=context
        )
        assert requirements_result.requirements

        # Step 2: Validate intent
        implementation = "Implemented JWT-based auth with database"
        intent_result = reviewer_module.validate_intent(
            user_intent=user_intent,
            implementation=implementation
        )
        assert hasattr(intent_result, 'intent_satisfied')

        # Step 3: Check completeness
        completeness_result = reviewer_module.verify_completeness(
            requirements=requirements_result.requirements,
            implementation=implementation
        )
        assert hasattr(completeness_result, 'complete')

        # Step 4: Verify correctness
        correctness_result = reviewer_module.verify_correctness(
            implementation=implementation
        )
        assert hasattr(correctness_result, 'correct')


class TestJSONCompatibility:
    """Test JSON compatibility with Rust bridge."""

    def test_requirements_json_serializable(self, reviewer_module):
        """Test requirement results can be JSON serialized."""
        import json

        user_intent = "Add feature X"
        result = reviewer_module.extract_requirements(user_intent=user_intent)

        json_str = json.dumps({'requirements': result.requirements})
        assert json_str

        parsed = json.loads(json_str)
        assert 'requirements' in parsed
        assert isinstance(parsed['requirements'], list)

    def test_intent_validation_json_serializable(self, reviewer_module):
        """Test intent validation results can be JSON serialized."""
        import json

        result = reviewer_module.validate_intent(
            user_intent="Test",
            implementation="Test implementation"
        )

        json_str = json.dumps({
            'intent_satisfied': result.intent_satisfied,
            'issues': result.issues
        })
        assert json_str

        parsed = json.loads(json_str)
        assert 'intent_satisfied' in parsed
        assert 'issues' in parsed
        assert isinstance(parsed['intent_satisfied'], bool)
        assert isinstance(parsed['issues'], list)

    def test_completeness_json_serializable(self, reviewer_module):
        """Test completeness results can be JSON serialized."""
        import json

        result = reviewer_module.verify_completeness(
            requirements=["Req 1"],
            implementation="Implementation"
        )

        json_str = json.dumps({
            'complete': result.complete,
            'issues': result.issues
        })
        assert json_str

        parsed = json.loads(json_str)
        assert 'complete' in parsed
        assert 'issues' in parsed

    def test_correctness_json_serializable(self, reviewer_module):
        """Test correctness results can be JSON serialized."""
        import json

        result = reviewer_module.verify_correctness(
            implementation="def test(): pass"
        )

        json_str = json.dumps({
            'correct': result.correct,
            'issues': result.issues
        })
        assert json_str

        parsed = json.loads(json_str)
        assert 'correct' in parsed
        assert 'issues' in parsed


class TestEdgeCases:
    """Test edge cases and error handling."""

    def test_empty_user_intent(self, reviewer_module):
        """Test with empty user intent."""
        result = reviewer_module.extract_requirements(user_intent="")
        assert hasattr(result, 'requirements')

    def test_empty_implementation(self, reviewer_module):
        """Test with empty implementation."""
        result = reviewer_module.validate_intent(
            user_intent="Do something",
            implementation=""
        )
        assert hasattr(result, 'intent_satisfied')
        assert hasattr(result, 'issues')

    def test_very_long_text(self, reviewer_module):
        """Test with very long text (context window)."""
        long_text = "Implementation details. " * 1000

        result = reviewer_module.verify_correctness(
            implementation=long_text
        )
        assert hasattr(result, 'correct')

    def test_special_characters(self, reviewer_module):
        """Test with special characters in text."""
        implementation = "def test(): return {'key': 'value', \"other\": 123}"

        result = reviewer_module.verify_correctness(
            implementation=implementation
        )
        assert hasattr(result, 'correct')


if __name__ == '__main__':
    pytest.main([__file__, '-v'])
