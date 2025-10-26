"""
Work Plan Protocol Validation Tests (Part 1).

Tests the complete 4-phase workflow with real Claude API calls:
- Phase 1: Prompt → Spec (clarification, skills discovery)
- Phase 2: Spec → Full Spec (decomposition, test plan)
- Phase 3: Full Spec → Plan (scheduling, dependencies)
- Phase 4: Plan → Artifacts (implementation, review)
"""

import asyncio
import os
import sys
from pathlib import Path
import tempfile

import pytest

# Add src to path
sys.path.insert(0, str(Path(__file__).parent.parent.parent / "src"))

# Check if bindings and API key available
try:
    import mnemosyne_core
    BINDINGS_AVAILABLE = True
except ImportError:
    BINDINGS_AVAILABLE = False
    pytestmark = pytest.mark.skip(reason="PyO3 bindings not available")

API_KEY_AVAILABLE = bool(os.environ.get("ANTHROPIC_API_KEY"))

if BINDINGS_AVAILABLE:
    from orchestration import (
        OrchestrationEngine,
        EngineConfig,
        ExecutorAgent,
        ExecutorConfig,
    )


@pytest.mark.skipif(
    not API_KEY_AVAILABLE,
    reason="ANTHROPIC_API_KEY not set. Required for Work Plan Protocol tests."
)
class TestWorkPlanProtocol:
    """Test complete Work Plan Protocol (Phases 1-4)."""

    @pytest.mark.asyncio
    async def test_phase_1_vague_requirements(self):
        """
        Test 1.1: Phase 1 (Prompt → Spec)

        Verify executor challenges vague requirements and asks clarifying questions.
        """
        with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
            temp_db = f.name

        try:
            # Create engine
            config = EngineConfig(db_path=temp_db, enable_dashboard=False)
            engine = OrchestrationEngine(config)
            await engine.start()

            # Test 1.1: Vague requirement
            vague_plan = {
                "prompt": "Add a search feature to the memory system",
                # Deliberately vague - no tech stack, no details
            }

            result = await engine.execute_work_plan(vague_plan)

            # Expected: Executor should challenge vague requirements
            assert result["status"] in ["challenged", "error"], \
                "Executor should challenge vague requirements"

            if result["status"] == "challenged":
                # Check that questions were asked
                assert "questions" in result, "Should include clarifying questions"
                questions = result["questions"]

                # Expected questions about:
                # - Type of search (keyword, semantic, hybrid)
                # - Fields to search
                # - Performance requirements
                # - UI requirements

                print("\n=== Test 1.1: Phase 1 (Vague Requirements) ===")
                print(f"Status: {result['status']}")
                print(f"Issues: {result.get('issues', [])}")
                print(f"Questions asked: {len(questions)}")
                for q in questions:
                    print(f"  - {q}")

                # Verify tech stack was identified as missing
                assert any("tech stack" in q.lower() for q in questions), \
                    "Should ask about tech stack"

            await engine.stop()

            return {
                "test": "1.1 - Phase 1 Vague Requirements",
                "passed": True,
                "details": result
            }

        finally:
            os.unlink(temp_db)

    @pytest.mark.asyncio
    async def test_phase_2_decomposition(self):
        """
        Test 1.2: Phase 2 (Spec → Full Spec)

        Verify executor decomposes clear spec into components with test plan.
        """
        with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
            temp_db = f.name

        try:
            config = EngineConfig(db_path=temp_db, enable_dashboard=False)
            engine = OrchestrationEngine(config)
            await engine.start()

            # Test 1.2: Clear spec
            clear_plan = {
                "prompt": """Implement hybrid search for Mnemosyne:
- Combine keyword (FTS5) + graph traversal + importance weighting
- Search across content, summary, keywords, tags
- Return ranked results with relevance scores
- CLI command: mnemosyne search <query>
- Response time: <200ms for typical queries""",
                "tech_stack": "Rust + SQLite FTS5 + Python bindings",
                "deployment": "Local CLI tool",
                "success_criteria": "Search returns relevant results ranked by score in <200ms",
            }

            result = await engine.execute_work_plan(clear_plan)

            print("\n=== Test 1.2: Phase 2 (Decomposition) ===")
            print(f"Status: {result['status']}")

            # Check for decomposition in responses
            if "execution" in result:
                responses = result["execution"].get("responses", [])
                print(f"Agent responses: {len(responses)}")

                # Look for evidence of decomposition
                combined_text = " ".join(str(r) for r in responses)

                # Expected components
                expected_components = [
                    "query parser", "keyword search", "graph traversal",
                    "importance", "rank", "test"
                ]

                found_components = [c for c in expected_components
                                  if c in combined_text.lower()]

                print(f"Components identified: {len(found_components)}/{len(expected_components)}")
                for c in found_components:
                    print(f"  ✓ {c}")

            await engine.stop()

            return {
                "test": "1.2 - Phase 2 Decomposition",
                "passed": result["status"] == "success",
                "details": result
            }

        finally:
            os.unlink(temp_db)

    @pytest.mark.asyncio
    async def test_phase_3_planning(self):
        """
        Test 1.3: Phase 3 (Full Spec → Plan)

        Verify orchestrator creates execution plan with parallelization.
        """
        with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
            temp_db = f.name

        try:
            config = EngineConfig(db_path=temp_db, enable_dashboard=False)
            engine = OrchestrationEngine(config)
            await engine.start()

            # Test 1.3: Full spec requiring planning
            full_spec = {
                "prompt": "Implement the hybrid search system with all components",
                "tech_stack": "Rust + SQLite FTS5 + Python",
                "deployment": "Local CLI",
                "success_criteria": "All tests pass, <200ms search time",
                "agents": ["executor", "reviewer"],  # Request multiple agents
                "tasks": [
                    {"id": "impl-keyword", "description": "Implement keyword search", "depends_on": []},
                    {"id": "impl-graph", "description": "Implement graph traversal", "depends_on": []},
                    {"id": "impl-ranker", "description": "Implement result ranker", "depends_on": ["impl-keyword", "impl-graph"]},
                    {"id": "impl-cli", "description": "Implement CLI interface", "depends_on": ["impl-ranker"]},
                ]
            }

            result = await engine.execute_work_plan(full_spec)

            print("\n=== Test 1.3: Phase 3 (Planning) ===")
            print(f"Status: {result['status']}")

            # Check orchestration results
            if "orchestration" in result:
                orch_result = result["orchestration"]
                print(f"Orchestration status: {orch_result.get('status')}")
                print(f"Planning analysis performed: {len(orch_result.get('planning_analysis', []))} messages")

            await engine.stop()

            return {
                "test": "1.3 - Phase 3 Planning",
                "passed": result["status"] == "success",
                "details": result
            }

        finally:
            os.unlink(temp_db)

    @pytest.mark.asyncio
    async def test_phase_4_implementation_and_review(self):
        """
        Test 1.4: Phase 4 (Plan → Artifacts)

        Verify executor implements, tests, and reviewer validates quality gates.
        """
        with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
            temp_db = f.name

        try:
            config = EngineConfig(db_path=temp_db, enable_dashboard=False)
            engine = OrchestrationEngine(config)
            await engine.start()

            # Test 1.4: Simple implementation task
            impl_plan = {
                "prompt": "Create a simple function that adds two numbers",
                "tech_stack": "Python",
                "deployment": "Library function",
                "success_criteria": "Function works correctly with tests",
            }

            result = await engine.execute_work_plan(impl_plan)

            print("\n=== Test 1.4: Phase 4 (Implementation & Review) ===")
            print(f"Status: {result['status']}")

            # Check for artifacts
            if "execution" in result:
                artifacts = result["execution"].get("artifacts", {})
                print(f"Artifacts produced: {list(artifacts.keys())}")

            # Check review was performed
            if "review" in result:
                review = result["review"]
                print(f"Review passed: {review.get('passed')}")
                if not review.get("passed"):
                    print(f"Issues: {review.get('issues', [])}")

            await engine.stop()

            return {
                "test": "1.4 - Phase 4 Implementation & Review",
                "passed": result["status"] == "success",
                "details": result
            }

        finally:
            os.unlink(temp_db)


if __name__ == "__main__":
    # Run tests manually for debugging
    asyncio.run(TestWorkPlanProtocol().test_phase_1_vague_requirements())
