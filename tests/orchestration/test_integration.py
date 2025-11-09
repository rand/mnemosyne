"""
Integration Tests for Multi-Agent Orchestration with Direct Anthropic API.

Tests the complete multi-agent workflow:
- Orchestrator coordinating work distribution
- Optimizer discovering skills and allocating context
- Executor executing work plans using direct Anthropic API
- Reviewer validating quality gates

NOTE: These tests require ANTHROPIC_API_KEY to be set for full integration testing.
Use pytest markers to separate unit tests from integration tests.

Usage:
    # Run unit tests only (mocked)
    pytest tests/orchestration/test_integration.py -v -m "not integration"

    # Run integration tests (requires API key)
    pytest tests/orchestration/test_integration.py -v -m integration

    # Run all tests
    pytest tests/orchestration/test_integration.py -v
"""

import asyncio
import os
import sys
from pathlib import Path
from typing import Dict, Any
import tempfile

import pytest

# Add src to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent.parent / "src"))

# Check if PyO3 bindings available
try:
    import mnemosyne_core
    BINDINGS_AVAILABLE = True
except ImportError:
    BINDINGS_AVAILABLE = False
    pytestmark = pytest.mark.skip(reason="PyO3 bindings not available. Run: maturin develop --features python")

if BINDINGS_AVAILABLE:
    from orchestration import (
        OrchestrationEngine,
        EngineConfig,
        OrchestratorAgent,
        OrchestratorConfig,
        OptimizerAgent,
        OptimizerConfig,
        ReviewerAgent,
        ReviewerConfig,
        ExecutorAgent,
        ExecutorConfig,
        LowLatencyContextMonitor,
        ParallelExecutor,
    )

# Check if API key available (from secure storage)
def _check_api_key_available():
    """Check if API key is available via secure storage."""
    import subprocess
    try:
        result = subprocess.run(
            ["mnemosyne", "config", "show-key"],
            capture_output=True,
            text=True,
            timeout=5
        )
        if result.returncode == 0:
            for line in result.stdout.split('\n'):
                if 'API key configured:' in line:
                    return True
        return False
    except Exception:
        return False

API_KEY_AVAILABLE = _check_api_key_available()


# ============================================================================
# Fixtures
# ============================================================================

@pytest.fixture
def temp_db():
    """Create temporary database path for testing (file does not exist initially)."""
    import time
    # Generate path but don't create file (mnemosyne init needs to create it)
    db_path = f"/tmp/test_mnemosyne_{int(time.time() * 1000000)}.db"
    yield db_path
    # Cleanup
    if os.path.exists(db_path):
        os.unlink(db_path)


@pytest.fixture
def coordinator():
    """Create test coordinator instance."""
    return mnemosyne_core.PyCoordinator()


@pytest.fixture
def api_key():
    """Get API key from secure storage (environment variable or keyring)."""
    import subprocess

    # Try environment variable first
    api_key = os.environ.get("ANTHROPIC_API_KEY")
    if api_key:
        return api_key

    # Try keyring (macOS keychain)
    try:
        import keyring
        key = keyring.get_password("mnemosyne-memory-system", "anthropic-api-key")
        if key:
            return key
    except ImportError:
        pass
    except Exception:
        pass

    pytest.skip("API key not configured. Set ANTHROPIC_API_KEY or run: mnemosyne config set-key")


@pytest.fixture
def storage(temp_db):
    """Create test storage instance with initialized database."""
    from pathlib import Path
    import subprocess

    # Ensure database directory exists
    Path(temp_db).parent.mkdir(parents=True, exist_ok=True)

    # Initialize database using installed mnemosyne CLI
    try:
        result = subprocess.run(
            ["mnemosyne", "init", "--database", temp_db],
            capture_output=True,
            text=True,
            timeout=5
        )
        if result.returncode != 0:
            print(f"Database init warning: {result.stderr}")
    except Exception as e:
        print(f"Database init skipped: {e}")

    # Create storage instance (schema should now exist)
    return mnemosyne_core.PyStorage(temp_db)


@pytest.fixture
def context_monitor(coordinator):
    """Create test context monitor."""
    return LowLatencyContextMonitor(
        coordinator=coordinator,
        polling_interval=0.01,
        preservation_threshold=0.75,
        critical_threshold=0.90
    )


@pytest.fixture
def parallel_executor(coordinator, storage):
    """Create test parallel executor."""
    return ParallelExecutor(
        coordinator=coordinator,
        storage=storage,
        max_concurrent=4,
        spawn_timeout=30.0
    )


# ============================================================================
# Unit Tests (No API Key Required)
# ============================================================================

class TestAgentInitialization:
    """Test that agents initialize correctly without API calls."""

    @pytest.mark.asyncio
    async def test_executor_initialization(self, coordinator, storage, parallel_executor, api_key):
        """Test ExecutorAgent initializes with direct Anthropic API access."""
        config = ExecutorConfig(
            agent_id="test_executor",
            api_key=api_key
        )

        executor = ExecutorAgent(
            config=config,
            coordinator=coordinator,
            storage=storage,
            parallel_executor=parallel_executor
        )

        # Check initialization
        assert executor.config.agent_id == "test_executor"
        assert executor.api_key is not None  # Should get from secure storage
        assert executor.api_key == api_key
        assert not executor._session_active
        assert executor._current_phase.value == "idle"

        # Check status before session
        status = executor.get_status()
        assert status["session_active"] is False
        assert status["checkpoints"] == 0

    @pytest.mark.asyncio
    async def test_orchestrator_initialization(self, coordinator, storage, context_monitor, api_key):
        """Test OrchestratorAgent initializes with direct Anthropic API access."""
        config = OrchestratorConfig(
            agent_id="test_orchestrator",
            max_parallel_agents=4,
            api_key=api_key
        )

        orchestrator = OrchestratorAgent(
            config=config,
            coordinator=coordinator,
            storage=storage,
            context_monitor=context_monitor
        )

        # Check initialization
        assert orchestrator.config.agent_id == "test_orchestrator"
        assert orchestrator.api_key is not None  # Should get from secure storage
        assert orchestrator.api_key == api_key
        assert not orchestrator._session_active
        assert orchestrator._phase.value == "idle"

        # Check circular dependency detection
        orchestrator._work_graph = {
            "task_a": ["task_b"],
            "task_b": ["task_a"]  # Circular!
        }
        assert orchestrator._has_circular_dependencies() is True

    @pytest.mark.asyncio
    async def test_optimizer_initialization(self, coordinator, storage, api_key):
        """Test OptimizerAgent initializes with direct Anthropic API access."""
        config = OptimizerConfig(
            agent_id="test_optimizer",
            max_skills_loaded=5,
            api_key=api_key
        )

        optimizer = OptimizerAgent(
            config=config,
            coordinator=coordinator,
            storage=storage
        )

        # Check initialization
        assert optimizer.config.agent_id == "test_optimizer"
        assert optimizer.api_key is not None  # Should get from secure storage
        assert optimizer.api_key == api_key
        assert not optimizer._session_active
        assert len(optimizer._loaded_skills) == 0

    @pytest.mark.asyncio
    async def test_reviewer_initialization(self, coordinator, storage, api_key):
        """Test ReviewerAgent initializes with direct Anthropic API access."""
        config = ReviewerConfig(
            agent_id="test_reviewer",
            strict_mode=True,
            api_key=api_key
        )

        reviewer = ReviewerAgent(
            config=config,
            coordinator=coordinator,
            storage=storage
        )

        # Check initialization
        assert reviewer.config.agent_id == "test_reviewer"
        assert reviewer.api_key is not None  # Should get from secure storage
        assert reviewer.api_key == api_key
        assert not reviewer._session_active
        assert reviewer._review_count == 0


class TestEngineConfiguration:
    """Test orchestration engine configuration."""

    @pytest.mark.asyncio
    async def test_engine_initialization(self, temp_db):
        """Test that engine initializes all components correctly."""
        import subprocess

        # Initialize database
        subprocess.run(
            ["mnemosyne", "init", "--database", temp_db],
            capture_output=True,
            timeout=5
        )

        config = EngineConfig(
            db_path=temp_db,
            polling_interval=0.01,
            max_concurrent=4,
            enable_dashboard=False
        )

        engine = OrchestrationEngine(config)

        # Check all agents initialized
        assert engine.orchestrator is not None
        assert engine.optimizer is not None
        assert engine.reviewer is not None
        assert engine.executor is not None

        # Check monitoring components
        assert engine.context_monitor is not None
        assert engine.parallel_executor is not None

        # Check storage
        assert engine.storage is not None

        # Not yet started
        assert not engine._monitoring_active

    @pytest.mark.asyncio
    async def test_engine_start_stop(self, temp_db):
        """Test engine lifecycle management."""
        import subprocess

        # Initialize database
        subprocess.run(
            ["mnemosyne", "init", "--database", temp_db],
            capture_output=True,
            timeout=5
        )

        config = EngineConfig(
            db_path=temp_db,
            enable_dashboard=False
        )

        engine = OrchestrationEngine(config)

        # Start engine
        await engine.start()
        assert engine._monitoring_active is True

        # Get status
        status = engine.get_status()
        assert status["monitoring_active"] is True
        assert "context" in status
        assert "agents" in status

        # Stop engine
        await engine.stop()
        assert engine._monitoring_active is False


# ============================================================================
# Integration Tests (Require API Key)
# ============================================================================

@pytest.mark.integration
@pytest.mark.skipif(
    not API_KEY_AVAILABLE,
    reason="API key not configured. Run: mnemosyne config set-key"
)
class TestAgentSDKIntegration:
    """Test actual Claude Agent SDK integration (requires API key)."""

    @pytest.mark.asyncio
    async def test_executor_session_lifecycle(self, coordinator, storage, parallel_executor, api_key):
        """Test Executor can start and stop Claude agent sessions."""
        config = ExecutorConfig(agent_id="test_executor", api_key=api_key)

        executor = ExecutorAgent(
            config=config,
            coordinator=coordinator,
            storage=storage,
            parallel_executor=parallel_executor
        )

        # Start session
        await executor.start_session()
        assert executor._session_active is True

        # Check status
        status = executor.get_status()
        assert status["session_active"] is True

        # Stop session
        await executor.stop_session()
        assert executor._session_active is False

    @pytest.mark.asyncio
    async def test_executor_context_manager(self, coordinator, storage, parallel_executor, api_key):
        """Test Executor async context manager."""
        config = ExecutorConfig(agent_id="test_executor", api_key=api_key)

        executor = ExecutorAgent(
            config=config,
            coordinator=coordinator,
            storage=storage,
            parallel_executor=parallel_executor
        )

        # Use context manager
        async with executor:
            assert executor._session_active is True

        # Session should be stopped
        assert executor._session_active is False

    @pytest.mark.asyncio
    async def test_optimizer_skill_discovery(self, coordinator, storage, api_key):
        """Test Optimizer can discover skills with Claude."""
        config = OptimizerConfig(
            agent_id="test_optimizer",
            api_key=api_key
        )

        optimizer = OptimizerAgent(
            config=config,
            coordinator=coordinator,
            storage=storage
        )

        # Optimize context for a Rust task
        async with optimizer:
            result = await optimizer.optimize_context(
                task_description="Write a Rust function to parse JSON",
                current_context={"available_tokens": 200000}
            )

            # Should have context allocation
            assert "allocation" in result
            assert "skills" in result
            assert "total_budget" in result

            # Budget should be allocated
            allocation = result["allocation"]
            assert "critical" in allocation
            assert "skills" in allocation
            assert allocation["critical"] > 0

    @pytest.mark.asyncio
    async def test_reviewer_quality_gates(self, coordinator, storage, api_key):
        """Test Reviewer can evaluate quality gates with Claude."""
        config = ReviewerConfig(
            agent_id="test_reviewer",
            strict_mode=True,
            api_key=api_key
        )

        reviewer = ReviewerAgent(
            config=config,
            coordinator=coordinator,
            storage=storage
        )

        # Create test artifact
        artifact = {
            "code": "def hello(): return 'world'",
            "documentation": {
                "overview": "Simple hello function",
                "usage": "Call hello()",
                "examples": "hello() -> 'world'"
            },
            "test_results": {
                "passed": 5,
                "failed": 0,
                "coverage": 0.85
            }
        }

        async with reviewer:
            result = await reviewer.review(artifact)

            # Should have review result
            assert result is not None
            assert hasattr(result, 'passed')
            assert hasattr(result, 'gate_results')
            assert hasattr(result, 'confidence')

            # Statistics should update
            stats = reviewer.get_statistics()
            assert stats["total_reviews"] == 1


@pytest.mark.integration
@pytest.mark.skipif(
    not API_KEY_AVAILABLE,
    reason="API key not configured. Run: mnemosyne config set-key"
)
class TestEndToEndWorkflow:
    """Test complete multi-agent workflow (requires API key)."""

    @pytest.mark.asyncio
    async def test_simple_work_plan_execution(self, temp_db, api_key):
        """Test executing a simple work plan through the engine."""
        import subprocess

        # Initialize database
        subprocess.run(
            ["mnemosyne", "init", "--database", temp_db],
            capture_output=True,
            timeout=5
        )

        config = EngineConfig(
            db_path=temp_db,
            enable_dashboard=False,
            orchestrator_config=OrchestratorConfig(api_key=api_key),
            optimizer_config=OptimizerConfig(api_key=api_key),
            reviewer_config=ReviewerConfig(api_key=api_key),
            executor_config=ExecutorConfig(api_key=api_key)
        )

        engine = OrchestrationEngine(config)
        await engine.start()

        try:
            # Define work plan with clear requirements
            work_plan = {
                "prompt": "Create a Python function that generates a list of integers from 1 to 5 using a range-based approach for numerical sequence generation",
                "tech_stack": "Python 3.11+",
                "success_criteria": "Function returns list [1, 2, 3, 4, 5] and includes type hints"
            }

            # Execute work plan
            result = await engine.execute_work_plan(work_plan)

            # Check result structure
            assert "status" in result
            assert "execution" in result or "review" in result

            # Engine should track execution
            status = engine.get_status()
            assert status is not None

        finally:
            await engine.stop()

    @pytest.mark.asyncio
    async def test_work_plan_with_validation(self, temp_db, api_key):
        """Test work plan that gets validated by Reviewer."""
        import subprocess

        # Initialize database
        subprocess.run(
            ["mnemosyne", "init", "--database", temp_db],
            capture_output=True,
            timeout=5
        )

        config = EngineConfig(
            db_path=temp_db,
            enable_dashboard=False,
            orchestrator_config=OrchestratorConfig(api_key=api_key),
            optimizer_config=OptimizerConfig(api_key=api_key),
            reviewer_config=ReviewerConfig(api_key=api_key),
            executor_config=ExecutorConfig(api_key=api_key)
        )

        engine = OrchestrationEngine(config)
        await engine.start()

        try:
            # Work plan with clear success criteria and detailed requirements
            work_plan = {
                "prompt": "Implement a Python function for calculating the factorial of a given integer using recursive approach with base case handling and input validation",
                "tech_stack": "Python 3.11+ with type hints",
                "success_criteria": "Function is documented with docstrings, includes unit tests with edge cases, and handles invalid inputs gracefully",
                "constraints": ["No external dependencies", "Must include type hints", "Must handle negative numbers"]
            }

            # Execute work plan
            result = await engine.execute_work_plan(work_plan)

            # Should have status
            assert "status" in result

            # Check statistics
            stats = result.get("statistics", {})
            assert "reviewer" in stats or "execution" in result

        finally:
            await engine.stop()


# ============================================================================
# Test Utilities
# ============================================================================

def test_bindings_available():
    """Verify PyO3 bindings are available."""
    assert BINDINGS_AVAILABLE, "PyO3 bindings not available"
    assert mnemosyne_core is not None


def test_anthropic_sdk_importable():
    """Verify Anthropic SDK can be imported."""
    try:
        import anthropic
        assert True
    except ImportError:
        assert False, "anthropic SDK not installed"


def test_api_key_info():
    """Show API key availability (for debugging test runs)."""
    if API_KEY_AVAILABLE:
        print("\n✓ API key configured (secure storage) - integration tests will run")
    else:
        print("\n✗ API key not configured - integration tests will be skipped")
        print("  Configure API key to run full integration tests:")
        print("  mnemosyne config set-key sk-ant-...")
        print("  OR: mnemosyne secrets init")


if __name__ == "__main__":
    pytest.main([__file__, "-v", "-s"])
