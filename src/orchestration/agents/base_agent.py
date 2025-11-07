"""
Base Agent Mixin - Common interface for all Claude SDK agents.

Provides standard interface for Rust PyO3 bridge integration.
"""

import sys
import os
from typing import Any, Dict, List
from dataclasses import dataclass


def validate_environment() -> None:
    """
    Validate Python environment and dependencies.

    Checks:
    - Python version >= 3.9
    - anthropic package installed
    - ANTHROPIC_API_KEY environment variable set

    Raises:
        RuntimeError: If environment is invalid

    Example:
        ```python
        from base_agent import validate_environment

        # Call at agent initialization
        validate_environment()
        ```
    """
    # Check Python version
    if sys.version_info < (3, 9):
        raise RuntimeError(
            f"Python 3.9+ required, got {sys.version_info.major}.{sys.version_info.minor}. "
            f"Install with: brew install python@3.11 or pyenv install 3.11"
        )

    # Check anthropic package
    try:
        import anthropic
        version = getattr(anthropic, "__version__", "unknown")
        print(f"✓ anthropic SDK installed: {version}")
    except ImportError:
        raise RuntimeError(
            "anthropic package not installed. "
            "Install with: uv pip install anthropic OR pip install anthropic"
        )

    # Check API key (warning only, not fatal)
    api_key = os.getenv("ANTHROPIC_API_KEY")
    if not api_key:
        print(
            "⚠ Warning: ANTHROPIC_API_KEY not set. "
            "Agent initialization will fail without API key. "
            "Get your key from: https://console.anthropic.com/settings/keys"
        )
    else:
        print(f"✓ ANTHROPIC_API_KEY configured ({api_key[:7]}...{api_key[-4:]})")

    # Check Python path includes agents directory
    agents_dir = os.path.dirname(os.path.abspath(__file__))
    if agents_dir not in sys.path:
        print(f"ℹ Adding {agents_dir} to PYTHONPATH")
        sys.path.insert(0, agents_dir)


@dataclass
class WorkItem:
    """Work item structure matching Rust WorkItem."""
    id: str
    description: str
    phase: str
    priority: int
    consolidated_context_id: str | None = None
    review_feedback: List[str] | None = None
    review_attempt: int = 0


@dataclass
class WorkResult:
    """Work result structure matching Rust WorkResult."""
    success: bool
    data: str | None = None
    memory_ids: List[str] = None
    error: str | None = None

    def __post_init__(self):
        if self.memory_ids is None:
            self.memory_ids = []


class AgentExecutionMixin:
    """
    Mixin providing standard execute_work interface for Rust bridge.

    All agents should inherit from this mixin to provide consistent
    interface for PyO3 bridge integration.
    """

    async def execute_work(self, work_dict: Dict[str, Any]) -> Dict[str, Any]:
        """
        Execute work item from Rust bridge.

        This is the standard interface called by claude_agent_bridge.rs.
        Agents should override this method or provide agent-specific
        implementation.

        Args:
            work_dict: Work item dictionary from Rust (converted from WorkItem)

        Returns:
            Work result dictionary (converted to WorkResult in Rust)

        Raises:
            NotImplementedError: If agent doesn't implement execution
        """
        # Convert dict to WorkItem
        work_item = WorkItem(
            id=work_dict["id"],
            description=work_dict["description"],
            phase=work_dict["phase"],
            priority=work_dict["priority"],
            consolidated_context_id=work_dict.get("consolidated_context_id"),
            review_feedback=work_dict.get("review_feedback"),
            review_attempt=work_dict.get("review_attempt", 0)
        )

        # Call agent-specific implementation
        result = await self._execute_work_item(work_item)

        # Convert result to dict for Rust
        return {
            "success": result.success,
            "data": result.data,
            "memory_ids": result.memory_ids,
            "error": result.error
        }

    async def _execute_work_item(self, work_item: WorkItem) -> WorkResult:
        """
        Agent-specific work execution implementation.

        Override this method in agent implementations to provide
        role-specific behavior.

        Args:
            work_item: Structured work item

        Returns:
            Work result

        Raises:
            NotImplementedError: If not overridden
        """
        raise NotImplementedError(
            f"{self.__class__.__name__} must implement _execute_work_item"
        )


# Export for convenience
__all__ = [
    "WorkItem",
    "WorkResult",
    "AgentExecutionMixin"
]
