"""
Agent Coordination Validation Tests (Part 2).

Tests agent-specific behaviors:
- Orchestrator: Context preservation, dependency detection
- Optimizer: Skills discovery, context budget management
- Reviewer: Quality gates enforcement
- Executor: Sub-agent spawning
"""

import asyncio
import os
import sys
from pathlib import Path
import tempfile

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent.parent / "src"))

try:
    import mnemosyne_core
    BINDINGS_AVAILABLE = True
except ImportError:
    BINDINGS_AVAILABLE = False
    pytestmark = pytest.mark.skip(reason="PyO3 bindings not available")

API_KEY_AVAILABLE = bool(os.environ.get("ANTHROPIC_API_KEY"))

if BINDINGS_AVAILABLE:
    from orchestration import (
        OrchestratorAgent, OrchestratorConfig,
        OptimizerAgent, OptimizerConfig,
        ReviewerAgent, ReviewerConfig, ReviewResult, QualityGate,
        ExecutorAgent, ExecutorConfig, WorkTask,
        LowLatencyContextMonitor,
        ParallelExecutor,
    )


@pytest.mark.skipif(
    not API_KEY_AVAILABLE,
    reason="ANTHROPIC_API_KEY not set. Required for coordination tests."
)
class TestAgentCoordination:
    """Test agent-specific coordination behaviors."""

    @pytest.fixture
    def coordinator(self):
        """Create test coordinator."""
        return mnemosyne_core.PyCoordinator()

    @pytest.fixture
    def storage(self):
        """Create test storage."""
        with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
            temp_db = f.name
        storage = mnemosyne_core.PyStorage(temp_db)
        yield storage
        os.unlink(temp_db)

    @pytest.mark.asyncio
    async def test_orchestrator_circular_dependency_detection(self, coordinator, storage):
        """
        Test 2.2: Orchestrator - Dependency-Aware Scheduling

        Verify orchestrator detects circular dependencies.
        """
        context_monitor = LowLatencyContextMonitor(
            coordinator=coordinator,
            polling_interval=0.01,
            preservation_threshold=0.75,
            critical_threshold=0.90
        )

        config = OrchestratorConfig(agent_id="test_orch")
        orchestrator = OrchestratorAgent(
            config=config,
            coordinator=coordinator,
            storage=storage,
            context_monitor=context_monitor
        )

        # Create circular dependency
        orchestrator._work_graph = {
            "task_a": ["task_b"],
            "task_b": ["task_c"],
            "task_c": ["task_a"]  # Circular!
        }

        has_circular = orchestrator._has_circular_dependencies()

        print("\n=== Test 2.2: Circular Dependency Detection ===")
        print(f"Work graph: {orchestrator._work_graph}")
        print(f"Circular dependency detected: {has_circular}")

        assert has_circular, "Should detect circular dependency"

        return {
            "test": "2.2 - Circular Dependency Detection",
            "passed": True,
            "details": {"circular_detected": has_circular}
        }

    @pytest.mark.asyncio
    async def test_optimizer_skills_discovery(self, coordinator, storage):
        """
        Test 2.3: Optimizer - Skills Discovery

        Verify optimizer discovers relevant skills from filesystem.
        """
        config = OptimizerConfig(
            agent_id="test_optimizer",
            skills_dir="~/.claude/skills",
            max_skills_loaded=7
        )

        optimizer = OptimizerAgent(
            config=config,
            coordinator=coordinator,
            storage=storage
        )

        async with optimizer:
            # Test with task requiring multiple domain skills
            task_description = "Build an authenticated REST API with PostgreSQL backend and Docker deployment"

            skills = await optimizer._discover_skills(task_description)

            print("\n=== Test 2.3: Skills Discovery ===")
            print(f"Task: {task_description}")
            print(f"Skills discovered: {len(skills)}")
            for skill in skills:
                print(f"  - {Path(skill.skill_path).stem}: {skill.relevance_score:.2f}")

            # Expected: Should find skills related to API, auth, postgres, docker
            expected_keywords = ["api", "auth", "postgres", "docker", "database"]
            found_keywords = []

            for skill in skills:
                skill_name = Path(skill.skill_path).stem.lower()
                for keyword in expected_keywords:
                    if keyword in skill_name:
                        found_keywords.append(keyword)

            print(f"Expected keywords found: {set(found_keywords)}")

            # Should find at least some relevant skills
            assert len(skills) > 0, "Should discover some relevant skills"
            assert len(found_keywords) > 0, "Should find skills matching keywords"

        return {
            "test": "2.3 - Skills Discovery",
            "passed": True,
            "details": {
                "skills_found": len(skills),
                "keywords_matched": list(set(found_keywords))
            }
        }

    @pytest.mark.asyncio
    async def test_optimizer_context_budget_allocation(self, coordinator, storage):
        """
        Test 2.4: Optimizer - Context Budget Management

        Verify optimizer allocates context budget correctly.
        """
        config = OptimizerConfig(
            agent_id="test_optimizer",
            context_budget_critical=0.40,
            context_budget_skills=0.30,
            context_budget_project=0.20,
            context_budget_general=0.10
        )

        optimizer = OptimizerAgent(
            config=config,
            coordinator=coordinator,
            storage=storage
        )

        async with optimizer:
            task_description = "Simple task for budget testing"
            current_context = {
                "available_tokens": 100000,
                "utilization": 0.25
            }

            # Discover skills first (needed for budget allocation)
            skills = await optimizer._discover_skills(task_description)

            # Allocate budget
            allocation = await optimizer._allocate_budget(
                current_context, skills, task_description
            )

            print("\n=== Test 2.4: Context Budget Allocation ===")
            print(f"Total budget: {sum(allocation.values()):,} tokens")
            print("Allocation:")
            for category, tokens in allocation.items():
                percentage = (tokens / sum(allocation.values())) * 100
                print(f"  - {category}: {tokens:,} tokens ({percentage:.0f}%)")

            # Verify allocations match configuration
            total = sum(allocation.values())
            assert allocation["critical"] / total == pytest.approx(0.40, rel=0.01)
            assert allocation["skills"] / total == pytest.approx(0.30, rel=0.01)
            assert allocation["project"] / total == pytest.approx(0.20, rel=0.01)
            assert allocation["general"] / total == pytest.approx(0.10, rel=0.01)

        return {
            "test": "2.4 - Context Budget Allocation",
            "passed": True,
            "details": {"allocation": allocation}
        }

    @pytest.mark.asyncio
    async def test_reviewer_quality_gates_enforcement(self, coordinator, storage):
        """
        Test 2.5: Reviewer - Quality Gates

        Verify reviewer enforces quality gates and blocks incomplete work.
        """
        config = ReviewerConfig(
            agent_id="test_reviewer",
            strict_mode=True
        )

        reviewer = ReviewerAgent(
            config=config,
            coordinator=coordinator,
            storage=storage
        )

        async with reviewer:
            # Test with incomplete work artifact
            incomplete_artifact = {
                "code": """
def calculate(x, y):
    # TODO: implement this
    return x + y
""",
                "test_results": {
                    "passed": 0,
                    "failed": 0  # No tests!
                },
                "documentation": ""  # No docs!
            }

            result = await reviewer.review(incomplete_artifact)

            print("\n=== Test 2.5: Quality Gates Enforcement ===")
            print(f"Review passed: {result.passed}")
            print(f"Confidence: {result.confidence:.0%}")
            print(f"Issues found: {len(result.issues)}")
            for issue in result.issues:
                print(f"  ✗ {issue}")

            # Expected: Should FAIL review due to TODOs, no tests, no docs
            assert not result.passed, "Should fail review for incomplete work"
            assert len(result.issues) > 0, "Should identify specific issues"

            # Check specific gate failures
            print("\nGate Results:")
            for gate, passed in result.gate_results.items():
                status = "✓" if passed else "✗"
                print(f"  {status} {gate.value}")

            # Should fail NO_TODOS gate (has TODO comment)
            assert not result.gate_results.get(QualityGate.NO_TODOS, True), \
                "Should fail NO_TODOS gate"

        return {
            "test": "2.5 - Quality Gates Enforcement",
            "passed": True,
            "details": {
                "review_passed": result.passed,
                "issues": result.issues,
                "gates_failed": [g.value for g, p in result.gate_results.items() if not p]
            }
        }

    @pytest.mark.asyncio
    async def test_executor_subagent_safety_checks(self, coordinator, storage):
        """
        Test 2.6: Executor - Sub-Agent Spawning

        Verify executor enforces safety checks before spawning sub-agents.
        """
        parallel_executor = ParallelExecutor(
            coordinator=coordinator,
            storage=storage,
            max_concurrent=4,
            spawn_timeout=30.0
        )

        config = ExecutorConfig(
            agent_id="test_executor",
            max_subagents=2  # Limit to 2 for testing
        )

        executor = ExecutorAgent(
            config=config,
            coordinator=coordinator,
            storage=storage,
            parallel_executor=parallel_executor
        )

        # Test safety check: context budget
        # Simulate high context utilization
        coordinator.set_metric("context_utilization", 0.80)  # 80% - over 75% threshold

        print("\n=== Test 2.6: Sub-Agent Safety Checks ===")
        print(f"Current context utilization: 80%")
        print(f"Max subagents allowed: {config.max_subagents}")

        # Attempt to spawn sub-agent with high context usage
        try:
            task = WorkTask(
                id="test_task",
                description="Test task",
                phase=1,
                requirements=[],
                deliverables=[],
                constraints=[]
            )

            subagent_id = await executor.spawn_subagent(task)
            print(f"✗ Sub-agent spawned despite high context: {subagent_id}")
            assert False, "Should not spawn sub-agent when context > 75%"

        except RuntimeError as e:
            print(f"✓ Sub-agent spawn blocked: {e}")
            assert "context budget" in str(e).lower(), \
                "Should mention context budget in error"

        return {
            "test": "2.6 - Sub-Agent Safety Checks",
            "passed": True,
            "details": {"safety_check_enforced": True}
        }


if __name__ == "__main__":
    # Run tests manually
    import pytest
    pytest.main([__file__, "-v", "-s"])
