"""Integration tests for OptimizerModule.

Tests verify:
- Context consolidation with progressive modes
- Skills discovery for tasks
- Context budget optimization
- JSON output format compatibility with Rust bridge
- ChainOfThought reasoning transparency
"""

import os
import pytest
import dspy
from optimizer_module import OptimizerModule


@pytest.fixture
def optimizer_module():
    """Create OptimizerModule with Claude API (requires ANTHROPIC_API_KEY)."""
    # Check for API key
    api_key = os.getenv("ANTHROPIC_API_KEY")
    if not api_key:
        pytest.skip("ANTHROPIC_API_KEY not set - skipping integration tests")

    # Configure DSPy with Anthropic Claude
    dspy.configure(lm=dspy.LM('anthropic/claude-3-5-haiku-20241022', api_key=api_key))

    return OptimizerModule()


class TestContextConsolidation:
    """Test context consolidation with progressive modes."""

    def test_consolidate_context_detailed(self, optimizer_module):
        """Test detailed consolidation mode (attempt 1)."""
        result = optimizer_module.consolidate_context(
            original_intent="Implement user authentication",
            execution_summaries=[
                "Created auth module",
                "Added JWT token generation"
            ],
            review_feedback=[
                "Missing password hashing",
                "No error handling for invalid tokens"
            ],
            suggested_tests=[
                "Test token expiration",
                "Test invalid credentials"
            ],
            review_attempt=1,
            consolidation_mode="detailed"
        )

        # Check structure
        assert hasattr(result, 'consolidated_content')
        assert hasattr(result, 'key_issues')
        assert hasattr(result, 'strategic_guidance')
        assert hasattr(result, 'estimated_tokens')

    def test_consolidate_context_summary(self, optimizer_module):
        """Test summary consolidation mode (attempts 2-3)."""
        result = optimizer_module.consolidate_context(
            original_intent="Add caching layer",
            execution_summaries=["Implemented Redis caching"],
            review_feedback=[
                "Missing cache invalidation",
                "No TTL configuration",
                "Cache keys not namespaced"
            ],
            suggested_tests=["Test cache expiration"],
            review_attempt=2,
            consolidation_mode="summary"
        )

        assert hasattr(result, 'consolidated_content')
        assert isinstance(result.key_issues, str) or isinstance(result.key_issues, list)

    def test_consolidate_context_compressed(self, optimizer_module):
        """Test compressed consolidation mode (attempt 4+)."""
        result = optimizer_module.consolidate_context(
            original_intent="Fix database queries",
            execution_summaries=["Optimized slow queries"],
            review_feedback=[
                "Critical: N+1 query issue remains",
                "Missing index on user_id",
                "Query timeout not handled"
            ],
            suggested_tests=[],
            review_attempt=4,
            consolidation_mode="compressed"
        )

        assert hasattr(result, 'consolidated_content')
        assert hasattr(result, 'strategic_guidance')


class TestSkillsDiscovery:
    """Test skills discovery for tasks."""

    def test_discover_skills_basic(self, optimizer_module):
        """Test basic skills discovery."""
        skills = [
            {"name": "rust-async", "description": "Async Rust programming", "keywords": ["async", "tokio"], "domains": ["rust"]},
            {"name": "python-fastapi", "description": "FastAPI web framework", "keywords": ["fastapi", "web"], "domains": ["python"]},
            {"name": "database-postgres", "description": "PostgreSQL database", "keywords": ["postgres", "sql"], "domains": ["database"]},
        ]

        result = optimizer_module.discover_skills_for_task(
            task_description="Build async REST API with database",
            available_skills=skills,
            max_skills=2,
            current_context_usage=0.5
        )

        # Check structure
        assert hasattr(result, 'selected_skills')
        assert hasattr(result, 'relevance_scores')
        assert hasattr(result, 'reasoning')

    def test_discover_skills_with_context_constraint(self, optimizer_module):
        """Test skills discovery with high context usage."""
        skills = [
            {"name": "skill-1", "description": "Skill 1", "keywords": ["test"], "domains": ["test"]},
            {"name": "skill-2", "description": "Skill 2", "keywords": ["test"], "domains": ["test"]},
        ]

        result = optimizer_module.discover_skills_for_task(
            task_description="Simple task",
            available_skills=skills,
            max_skills=5,
            current_context_usage=0.85  # High usage - should recommend fewer
        )

        assert hasattr(result, 'selected_skills')
        assert hasattr(result, 'reasoning')


class TestContextBudgetOptimization:
    """Test context budget optimization."""

    def test_optimize_context_budget_basic(self, optimizer_module):
        """Test basic context budget optimization."""
        current_usage = {
            "critical_pct": 0.40,
            "skills_pct": 0.30,
            "project_pct": 0.20,
            "general_pct": 0.10,
            "total_pct": 1.0
        }

        loaded_resources = {
            "skill_names": ["rust-async", "python-fastapi", "database-postgres"],
            "memory_ids": ["mem-1", "mem-2", "mem-3", "mem-4", "mem-5"],
            "memory_summaries": ["Summary 1", "Summary 2", "Summary 3", "Summary 4", "Summary 5"]
        }

        result = optimizer_module.optimize_context_allocation(
            current_usage=current_usage,
            loaded_resources=loaded_resources,
            target_pct=0.75,
            work_priority=8
        )

        # Check structure
        assert hasattr(result, 'unload_skills')
        assert hasattr(result, 'unload_memory_ids')
        assert hasattr(result, 'optimization_rationale')

    def test_optimize_context_budget_high_priority(self, optimizer_module):
        """Test optimization for high priority work."""
        current_usage = {
            "critical_pct": 0.40,
            "skills_pct": 0.35,
            "project_pct": 0.25,
            "general_pct": 0.10,
            "total_pct": 1.10
        }

        loaded_resources = {
            "skill_names": ["skill-1", "skill-2"],
            "memory_ids": ["mem-1"],
            "memory_summaries": ["Summary 1"]
        }

        result = optimizer_module.optimize_context_allocation(
            current_usage=current_usage,
            loaded_resources=loaded_resources,
            target_pct=0.75,
            work_priority=10  # Critical work
        )

        assert hasattr(result, 'optimization_rationale')


class TestJSONCompatibility:
    """Test JSON compatibility with Rust bridge."""

    def test_consolidation_json_serializable(self, optimizer_module):
        """Test consolidation results can be JSON serialized."""
        import json

        result = optimizer_module.consolidate_context(
            original_intent="Test",
            execution_summaries=["Summary"],
            review_feedback=["Feedback"],
            suggested_tests=["Test"],
            review_attempt=1,
            consolidation_mode="detailed"
        )

        # Extract fields - DSPy predictions have attributes
        json_data = {
            'consolidated_content': getattr(result, 'consolidated_content', ''),
            'key_issues': getattr(result, 'key_issues', ''),
            'strategic_guidance': getattr(result, 'strategic_guidance', ''),
            'estimated_tokens': getattr(result, 'estimated_tokens', '0')
        }

        json_str = json.dumps(json_data)
        assert json_str

        parsed = json.loads(json_str)
        assert 'consolidated_content' in parsed
        assert 'key_issues' in parsed
        assert 'strategic_guidance' in parsed
        assert 'estimated_tokens' in parsed


class TestEdgeCases:
    """Test edge cases and error handling."""

    def test_empty_feedback(self, optimizer_module):
        """Test with empty review feedback."""
        result = optimizer_module.consolidate_context(
            original_intent="Test",
            execution_summaries=[],
            review_feedback=[],
            suggested_tests=[],
            review_attempt=1,
            consolidation_mode="detailed"
        )

        assert hasattr(result, 'consolidated_content')

    def test_very_long_context(self, optimizer_module):
        """Test with very long context."""
        long_feedback = ["Issue " + str(i) for i in range(50)]

        result = optimizer_module.consolidate_context(
            original_intent="Complex task",
            execution_summaries=["Summary " * 100],
            review_feedback=long_feedback,
            suggested_tests=[],
            review_attempt=1,
            consolidation_mode="detailed"
        )

        assert hasattr(result, 'estimated_tokens')


if __name__ == '__main__':
    pytest.main([__file__, '-v'])
