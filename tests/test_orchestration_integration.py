"""
Comprehensive integration tests for multi-agent orchestration system.

Tests the complete flow from work submission through execution,
including tool use, circuit breaker behavior, and error handling.
"""

import pytest
import asyncio
import os
import tempfile
import shutil
from pathlib import Path


@pytest.fixture
def temp_workspace():
    """Create temporary workspace for test files."""
    workspace = tempfile.mkdtemp(prefix="mnemosyne_test_")
    yield workspace
    shutil.rmtree(workspace, ignore_errors=True)


@pytest.fixture
def api_key():
    """Check if API key is available for integration tests."""
    key = os.getenv("ANTHROPIC_API_KEY")
    if not key:
        pytest.skip("ANTHROPIC_API_KEY not set - skipping integration tests")
    return key


class TestWorkSubmission:
    """Test work submission and queuing."""

    def test_work_submission_basic(self):
        """Test basic work submission to orchestrator."""
        # This will be implemented with mnemosyne CLI
        pass

    def test_work_submission_with_dependencies(self):
        """Test work submission with dependencies."""
        pass


class TestToolExecution:
    """Test tool execution capabilities."""

    @pytest.mark.asyncio
    async def test_create_file_tool(self, temp_workspace, api_key):
        """Test create_file tool execution."""
        # Import executor
        import sys
        sys.path.insert(0, 'src/orchestration/agents')
        
        from executor import ExecutorAgent, ExecutorConfig
        
        # Mock coordinator, storage, parallel_executor
        class MockCoordinator:
            def register_agent(self, agent_id): pass
            def update_agent_state(self, agent_id, state): pass
        
        class MockStorage:
            pass
        
        class MockParallelExecutor:
            pass
        
        # Create executor
        config = ExecutorConfig(agent_id="test_executor")
        executor = ExecutorAgent(
            config=config,
            coordinator=MockCoordinator(),
            storage=MockStorage(),
            parallel_executor=MockParallelExecutor()
        )
        
        # Test tool directly
        test_file = os.path.join(temp_workspace, "test.txt")
        result = await executor._execute_tool(
            "create_file",
            {
                "file_path": test_file,
                "content": "Test content from integration test"
            }
        )
        
        assert result["success"] is True
        assert os.path.exists(test_file)
        
        with open(test_file, 'r') as f:
            content = f.read()
        assert content == "Test content from integration test"

    @pytest.mark.asyncio
    async def test_read_file_tool(self, temp_workspace, api_key):
        """Test read_file tool execution."""
        # Create test file
        test_file = os.path.join(temp_workspace, "read_test.txt")
        with open(test_file, 'w') as f:
            f.write("Content to read")
        
        # Import and test
        import sys
        sys.path.insert(0, 'src/orchestration/agents')
        from executor import ExecutorAgent, ExecutorConfig
        
        class MockCoordinator:
            def register_agent(self, agent_id): pass
            def update_agent_state(self, agent_id, state): pass
        
        config = ExecutorConfig(agent_id="test_executor")
        executor = ExecutorAgent(
            config=config,
            coordinator=MockCoordinator(),
            storage=None,
            parallel_executor=None
        )
        
        result = await executor._execute_tool(
            "read_file",
            {"file_path": test_file}
        )
        
        assert result["success"] is True
        assert result["content"] == "Content to read"

    @pytest.mark.asyncio
    async def test_edit_file_tool(self, temp_workspace, api_key):
        """Test edit_file tool execution."""
        # Create test file
        test_file = os.path.join(temp_workspace, "edit_test.txt")
        with open(test_file, 'w') as f:
            f.write("Original content")
        
        # Import and test
        import sys
        sys.path.insert(0, 'src/orchestration/agents')
        from executor import ExecutorAgent, ExecutorConfig
        
        class MockCoordinator:
            def register_agent(self, agent_id): pass
            def update_agent_state(self, agent_id, state): pass
        
        config = ExecutorConfig(agent_id="test_executor")
        executor = ExecutorAgent(
            config=config,
            coordinator=MockCoordinator(),
            storage=None,
            parallel_executor=None
        )
        
        result = await executor._execute_tool(
            "edit_file",
            {
                "file_path": test_file,
                "old_text": "Original",
                "new_text": "Modified"
            }
        )
        
        assert result["success"] is True
        
        with open(test_file, 'r') as f:
            content = f.read()
        assert content == "Modified content"

    @pytest.mark.asyncio
    async def test_run_command_tool(self, temp_workspace, api_key):
        """Test run_command tool execution."""
        import sys
        sys.path.insert(0, 'src/orchestration/agents')
        from executor import ExecutorAgent, ExecutorConfig
        
        class MockCoordinator:
            def register_agent(self, agent_id): pass
            def update_agent_state(self, agent_id, state): pass
        
        config = ExecutorConfig(agent_id="test_executor")
        executor = ExecutorAgent(
            config=config,
            coordinator=MockCoordinator(),
            storage=None,
            parallel_executor=None
        )
        
        result = await executor._execute_tool(
            "run_command",
            {
                "command": "echo 'test output'",
                "working_dir": temp_workspace
            }
        )
        
        assert result["success"] is True
        assert "test output" in result["stdout"]


class TestCircuitBreaker:
    """Test circuit breaker functionality."""

    def test_circuit_breaker_initialization(self):
        """Test circuit breaker initializes in CLOSED state."""
        import sys
        sys.path.insert(0, 'src/orchestration/agents')
        from executor import CircuitBreaker, CircuitState
        
        cb = CircuitBreaker(failure_threshold=3, cooldown_seconds=60.0)
        
        assert cb.state == CircuitState.CLOSED
        assert cb.failure_count == 0
        assert cb.can_attempt() is True

    def test_circuit_breaker_opens_after_failures(self):
        """Test circuit opens after threshold failures."""
        import sys
        sys.path.insert(0, 'src/orchestration/agents')
        from executor import CircuitBreaker, CircuitState
        
        cb = CircuitBreaker(failure_threshold=3, cooldown_seconds=60.0)
        
        # Record failures
        cb.record_failure()
        assert cb.state == CircuitState.CLOSED
        assert cb.can_attempt() is True
        
        cb.record_failure()
        assert cb.state == CircuitState.CLOSED
        
        cb.record_failure()
        assert cb.state == CircuitState.OPEN
        assert cb.can_attempt() is False

    def test_circuit_breaker_half_open_transition(self):
        """Test circuit transitions to HALF_OPEN after cooldown."""
        import sys
        sys.path.insert(0, 'src/orchestration/agents')
        from executor import CircuitBreaker, CircuitState
        import time
        
        cb = CircuitBreaker(failure_threshold=2, cooldown_seconds=0.1)
        
        # Open circuit
        cb.record_failure()
        cb.record_failure()
        assert cb.state == CircuitState.OPEN
        
        # Wait for cooldown
        time.sleep(0.15)
        
        # Should transition to HALF_OPEN on next check
        assert cb.can_attempt() is True
        assert cb.state == CircuitState.HALF_OPEN

    def test_circuit_breaker_closes_after_success(self):
        """Test circuit closes after successful call in HALF_OPEN."""
        import sys
        sys.path.insert(0, 'src/orchestration/agents')
        from executor import CircuitBreaker, CircuitState
        import time
        
        cb = CircuitBreaker(failure_threshold=2, cooldown_seconds=0.1, half_open_attempts=1)
        
        # Open circuit
        cb.record_failure()
        cb.record_failure()
        assert cb.state == CircuitState.OPEN
        
        # Wait and transition to HALF_OPEN
        time.sleep(0.15)
        cb.can_attempt()
        assert cb.state == CircuitState.HALF_OPEN
        
        # Record success - should close
        cb.record_success()
        assert cb.state == CircuitState.CLOSED
        assert cb.failure_count == 0

    def test_circuit_breaker_reopens_on_half_open_failure(self):
        """Test circuit reopens if HALF_OPEN attempt fails."""
        import sys
        sys.path.insert(0, 'src/orchestration/agents')
        from executor import CircuitBreaker, CircuitState
        import time
        
        cb = CircuitBreaker(failure_threshold=2, cooldown_seconds=0.1)
        
        # Open circuit
        cb.record_failure()
        cb.record_failure()
        
        # Wait and transition to HALF_OPEN
        time.sleep(0.15)
        cb.can_attempt()
        assert cb.state == CircuitState.HALF_OPEN
        
        # Failure in HALF_OPEN - should reopen
        cb.record_failure()
        assert cb.state == CircuitState.OPEN
        assert cb.can_attempt() is False


class TestEndToEnd:
    """End-to-end integration tests."""

    @pytest.mark.skip(reason="Requires full system integration - manual testing")
    def test_full_work_execution_flow(self):
        """Test complete flow from work submission to completion."""
        # This test would require:
        # 1. Starting orchestration engine
        # 2. Submitting work
        # 3. Verifying executor processes it
        # 4. Checking tool execution results
        # 5. Verifying completion
        pass


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
