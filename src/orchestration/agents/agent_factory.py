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


def create_agent(role: str, config: Optional[Dict[str, Any]] = None) -> Any:
    """
    Create an agent instance based on role.

    Args:
        role: Agent role ("orchestrator", "optimizer", "reviewer", "executor")
        config: Optional configuration dict

    Returns:
        Agent instance with Claude SDK client initialized

    Raises:
        ValueError: If role is unknown
    """
    config = config or {}

    if role == "orchestrator":
        agent_config = OrchestratorConfig(
            agent_id="orchestrator",
            **config
        )
        # For now, pass None for dependencies - will be injected later
        return OrchestratorAgent(
            config=agent_config,
            coordinator=None,
            storage=None,
            context_monitor=None
        )

    elif role == "optimizer":
        agent_config = OptimizerConfig(
            agent_id="optimizer",
            **config
        )
        return OptimizerAgent(
            config=agent_config,
            skills_directory=config.get("skills_directory", "skills"),
            storage=None
        )

    elif role == "reviewer":
        agent_config = ReviewerConfig(
            agent_id="reviewer",
            **config
        )
        return ReviewerAgent(
            config=agent_config,
            storage=None
        )

    elif role == "executor":
        agent_config = ExecutorConfig(
            agent_id="executor",
            **config
        )
        return ExecutorAgent(
            config=agent_config,
            storage=None,
            skills_cache=None
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
