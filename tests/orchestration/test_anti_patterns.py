"""
Anti-Pattern Detection Tests (Part 4).

Tests that the system detects and blocks common anti-patterns:
- Testing before committing
- Skipping phase progression
- Accepting vague requirements
- Front-loading all skills
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
        OrchestrationEngine,
        EngineConfig,
        ExecutorAgent,
        ExecutorConfig,
        OptimizerAgent,
        OptimizerConfig,
    )


class TestAntiPatternDetection:
    """Test detection and blocking of anti-patterns."""

    @pytest.mark.skipif(
        not API_KEY_AVAILABLE,
        reason="ANTHROPIC_API_KEY not set."
    )
    @pytest.mark.asyncio
    async def test_vague_requirements_rejection(self):
        """
        Test 4.3: Accept Vague Requirements

        Verify executor challenges vague requirements even under pressure.
        """
        with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
            temp_db = f.name

        try:
            config = EngineConfig(
                db_path=temp_db,
                enable_dashboard=False,
                executor_config=ExecutorConfig(
                    challenge_vague_requirements=True
                )
            )

            engine = OrchestrationEngine(config)
            await engine.start()

            # Test with vague requirement + pressure
            vague_plan = {
                "prompt": "Just quickly add search, don't worry about the details"
            }

            result = await engine.execute_work_plan(vague_plan)

            print("\n=== Test 4.3: Vague Requirements Rejection ===")
            print(f"Status: {result['status']}")

            # Expected: Should challenge despite "just quickly" and "don't worry"
            assert result["status"] in ["challenged", "error"], \
                "Should challenge vague requirements despite pressure"

            if result["status"] == "challenged":
                print(f"✓ Executor challenged vague requirement")
                print(f"Issues identified: {result.get('issues', [])}")
                print(f"Questions asked: {len(result.get('questions', []))}")

            await engine.stop()

            return {
                "test": "4.3 - Vague Requirements Rejection",
                "passed": True,
                "details": result
            }

        finally:
            os.unlink(temp_db)

    def test_skills_not_front_loaded(self):
        """
        Test 4.4: Front-Load All Skills

        Verify skills are NOT all loaded at session start.
        """
        with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
            temp_db = f.name

        try:
            coordinator = mnemosyne_core.PyCoordinator()
            storage = mnemosyne_core.PyStorage(temp_db)

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

            print("\n=== Test 4.4: Skills Not Front-Loaded ===")
            print(f"Skills loaded at init: {len(optimizer._loaded_skills)}")

            # Expected: No skills loaded at initialization
            assert len(optimizer._loaded_skills) == 0, \
                "Skills should not be loaded at initialization"

            print("✓ Skills are not front-loaded")
            print("  Skills will be loaded on-demand based on task requirements")

            return {
                "test": "4.4 - Skills Not Front-Loaded",
                "passed": True,
                "details": {"initial_skills": 0}
            }

        finally:
            os.unlink(temp_db)


if __name__ == "__main__":
    pytest.main([__file__, "-v", "-s"])
