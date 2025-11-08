"""
Agent Factory - Creates Claude SDK agent instances based on role.

This module provides a factory function for creating specialized agents
(Orchestrator, Optimizer, Reviewer, Executor) with Claude SDK integration.

Used by the Rust PyO3 bridge (claude_agent_bridge.rs) to spawn Python agents.
"""

from typing import Any, Dict, Optional
import asyncio

# Import agent implementations
from .orchestrator import OrchestratorAgent, OrchestratorConfig
from .optimizer import OptimizerAgent, OptimizerConfig
from .reviewer import ReviewerAgent, ReviewerConfig
from .executor import ExecutorAgent, ExecutorConfig


class MockCoordinator:
    """Mock coordinator for testing/standalone agent usage."""

    def __init__(self):
        self._agents = {}

    def register_agent(self, agent_id: str):
        """Register an agent."""
        self._agents[agent_id] = "idle"

    def update_agent_state(self, agent_id: str, state: str):
        """Update agent state."""
        if agent_id in self._agents:
            self._agents[agent_id] = state

    def get_context_utilization(self) -> float:
        """Get context utilization (mock returns 0.5)."""
        return 0.5


class MockStorage:
    """Mock storage for testing/standalone agent usage."""

    def store(self, memory: Dict[str, Any]):
        """Store memory (no-op for testing)."""
        pass


class MockParallelExecutor:
    """Mock parallel executor for testing/standalone agent usage."""

    def __init__(self):
        pass


class MockContextMonitor:
    """Mock context monitor for testing/standalone agent usage."""

    def set_preservation_callback(self, callback):
        """Set preservation callback (no-op for testing)."""
        pass


def create_agent(role: str, config: Optional[Dict[str, Any]] = None) -> Any:
    """
    Create an agent instance based on role.

    Args:
        role: Agent role ("orchestrator", "optimizer", "reviewer", "executor")
        config: Optional configuration dict (may include 'anthropic_api_key')

    Returns:
        Agent instance with Claude SDK client initialized

    Raises:
        ValueError: If role is unknown
    """
    import os

    config = config or {}

    # If API key is provided in config, set it as environment variable
    # This ensures Python agents can access it via os.getenv()
    if "anthropic_api_key" in config:
        os.environ["ANTHROPIC_API_KEY"] = config["anthropic_api_key"]
        # Remove from config dict since it's now in environment
        del config["anthropic_api_key"]

    # Create mock dependencies for standalone/testing usage
    mock_coordinator = MockCoordinator()
    mock_storage = MockStorage()
    mock_parallel_executor = MockParallelExecutor()
    mock_context_monitor = MockContextMonitor()

    if role == "orchestrator":
        agent_config = OrchestratorConfig(
            agent_id="orchestrator",
            **config
        )
        return OrchestratorAgent(
            config=agent_config,
            coordinator=mock_coordinator,
            storage=mock_storage,
            context_monitor=mock_context_monitor
        )

    elif role == "optimizer":
        agent_config = OptimizerConfig(
            agent_id="optimizer",
            **config
        )
        return OptimizerAgent(
            config=agent_config,
            skills_directory=config.get("skills_directory", "skills"),
            storage=mock_storage
        )

    elif role == "reviewer":
        agent_config = ReviewerConfig(
            agent_id="reviewer",
            **config
        )
        return ReviewerAgent(
            config=agent_config,
            storage=mock_storage
        )

    elif role == "executor":
        agent_config = ExecutorConfig(
            agent_id="executor",
            **config
        )
        return ExecutorAgent(
            config=agent_config,
            coordinator=mock_coordinator,
            storage=mock_storage,
            parallel_executor=mock_parallel_executor
        )

    else:
        raise ValueError(f"Unknown agent role: {role}. Must be one of: orchestrator, optimizer, reviewer, executor")


async def create_agent_async(role: str, config: Optional[Dict[str, Any]] = None) -> Any:
    """
    Create an agent instance asynchronously and start session.

    Args:
        role: Agent role ("orchestrator", "optimizer", "reviewer", "executor")
        config: Optional configuration dict

    Returns:
        Agent instance with active Claude SDK session

    Raises:
        ValueError: If role is unknown
    """
    agent = create_agent(role, config)
    await agent.start_session()
    return agent
