"""
Enhanced error handling with actionable context.

Provides structured error messages with:
- Work item details
- Agent state information
- Environment diagnostics
- Troubleshooting hints
- Recovery suggestions
"""

import sys
import os
from typing import Optional, Dict, Any, List
from dataclasses import dataclass


@dataclass
class ErrorContext:
    """Structured error context for production debugging."""
    error_type: str
    error_message: str
    work_item_id: Optional[str] = None
    work_item_phase: Optional[str] = None
    work_item_description: Optional[str] = None
    agent_id: Optional[str] = None
    agent_state: Optional[str] = None
    session_active: Optional[bool] = None
    troubleshooting_hints: Optional[List[str]] = None
    recovery_suggestions: Optional[List[str]] = None
    environment_info: Optional[Dict[str, str]] = None

    def format(self) -> str:
        """Format error context as human-readable string."""
        parts = []

        # Error header
        parts.append(f"╔══════════════════════════════════════════════════════════════")
        parts.append(f"║ {self.error_type}: {self.error_message}")
        parts.append(f"╠══════════════════════════════════════════════════════════════")

        # Work item details
        if self.work_item_id:
            parts.append(f"║ Work Item:")
            parts.append(f"║   ID: {self.work_item_id}")
            if self.work_item_phase:
                parts.append(f"║   Phase: {self.work_item_phase}")
            if self.work_item_description:
                desc = self.work_item_description[:100]
                parts.append(f"║   Description: {desc}...")

        # Agent state
        if self.agent_id:
            parts.append(f"║ Agent:")
            parts.append(f"║   ID: {self.agent_id}")
            if self.agent_state:
                parts.append(f"║   State: {self.agent_state}")
            if self.session_active is not None:
                parts.append(f"║   Session Active: {self.session_active}")

        # Environment info
        if self.environment_info:
            parts.append(f"║ Environment:")
            for key, value in self.environment_info.items():
                parts.append(f"║   {key}: {value}")

        # Troubleshooting hints
        if self.troubleshooting_hints:
            parts.append(f"╠══════════════════════════════════════════════════════════════")
            parts.append(f"║ Troubleshooting:")
            for hint in self.troubleshooting_hints:
                parts.append(f"║   • {hint}")

        # Recovery suggestions
        if self.recovery_suggestions:
            parts.append(f"╠══════════════════════════════════════════════════════════════")
            parts.append(f"║ Recovery:")
            for suggestion in self.recovery_suggestions:
                parts.append(f"║   → {suggestion}")

        parts.append(f"╚══════════════════════════════════════════════════════════════")

        return "\n".join(parts)


def get_environment_info() -> Dict[str, str]:
    """Get environment information for diagnostics."""
    info = {
        "Python": f"{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}",
        "Platform": sys.platform,
    }

    # Check for API key
    if "ANTHROPIC_API_KEY" in os.environ:
        key = os.environ["ANTHROPIC_API_KEY"]
        info["API Key"] = f"Configured ({key[:7]}...{key[-4:]})"
    else:
        info["API Key"] = "❌ NOT CONFIGURED"

    # Check for Claude SDK
    try:
        import anthropic
        info["anthropic SDK"] = f"✓ Installed ({anthropic.__version__ if hasattr(anthropic, '__version__') else 'unknown'})"
    except ImportError:
        info["anthropic SDK"] = "❌ Not installed"

    # Check for PyO3 bindings
    try:
        import mnemosyne_core
        info["mnemosyne_core"] = "✓ Available"
    except ImportError:
        info["mnemosyne_core"] = "❌ Not built"

    return info


def create_session_error_context(
    agent_id: str,
    error: Exception,
    session_active: bool
) -> ErrorContext:
    """Create error context for session failures."""
    troubleshooting = []
    recovery = []

    # Analyze error type
    error_type = type(error).__name__
    error_msg = str(error)

    if "API" in error_msg or "key" in error_msg.lower():
        troubleshooting.append("API key may be missing or invalid")
        troubleshooting.append("Check: mnemosyne config show-key")
        recovery.append("Set API key: export ANTHROPIC_API_KEY='sk-ant-...'")
        recovery.append("Or configure: mnemosyne secrets init")

    if "module" in error_msg.lower() or "import" in error_msg.lower():
        troubleshooting.append("Python package may be missing")
        troubleshooting.append("Check: pip list | grep anthropic")
        recovery.append("Install SDK: uv pip install anthropic")
        recovery.append("Or: pip install anthropic")

    if "connection" in error_msg.lower() or "network" in error_msg.lower():
        troubleshooting.append("Network connectivity issue")
        troubleshooting.append("Check internet connection")
        recovery.append("Verify network: curl https://api.anthropic.com")
        recovery.append("Check firewall settings")

    return ErrorContext(
        error_type=error_type,
        error_message=error_msg,
        agent_id=agent_id,
        session_active=session_active,
        troubleshooting_hints=troubleshooting,
        recovery_suggestions=recovery,
        environment_info=get_environment_info()
    )


def create_work_item_error_context(
    work_item_id: str,
    work_item_phase: str,
    work_item_description: str,
    agent_id: str,
    agent_state: str,
    session_active: bool,
    error: Exception
) -> ErrorContext:
    """Create error context for work item execution failures."""
    troubleshooting = []
    recovery = []

    error_type = type(error).__name__
    error_msg = str(error)

    # Generic troubleshooting
    troubleshooting.append(f"Work item failed during {work_item_phase} phase")
    troubleshooting.append(f"Agent state: {agent_state}, Session: {'active' if session_active else 'inactive'}")

    # Phase-specific hints
    if work_item_phase == "planning":
        troubleshooting.append("Check if requirements are clear and complete")
        recovery.append("Review work item description for ambiguities")
        recovery.append("Add missing context or constraints")

    elif work_item_phase == "implementation":
        troubleshooting.append("Implementation may have encountered code errors")
        recovery.append("Check logs for compilation or runtime errors")
        recovery.append("Verify all dependencies are available")

    elif work_item_phase == "review":
        troubleshooting.append("Quality gates may not be satisfied")
        recovery.append("Check quality gate results in review feedback")
        recovery.append("Address failing gates before retrying")

    # Session-specific hints
    if not session_active:
        troubleshooting.append("Claude session is not active")
        recovery.append("Agent will attempt to restart session automatically")
        recovery.append("Check API key configuration if restart fails")

    # Error-specific hints
    if "timeout" in error_msg.lower():
        troubleshooting.append("Operation timed out (Claude API or network issue)")
        recovery.append("Retry with simpler/shorter work item")
        recovery.append("Check Claude API status: https://status.anthropic.com")

    if "rate limit" in error_msg.lower():
        troubleshooting.append("API rate limit exceeded")
        recovery.append("Wait 60 seconds before retrying")
        recovery.append("Consider upgrading API tier for higher limits")

    return ErrorContext(
        error_type=error_type,
        error_message=error_msg,
        work_item_id=work_item_id,
        work_item_phase=work_item_phase,
        work_item_description=work_item_description,
        agent_id=agent_id,
        agent_state=agent_state,
        session_active=session_active,
        troubleshooting_hints=troubleshooting,
        recovery_suggestions=recovery,
        environment_info=get_environment_info()
    )


def create_validation_error_context(
    work_item_id: str,
    agent_id: str,
    validation_issues: List[str],
    validation_questions: List[str]
) -> ErrorContext:
    """Create error context for validation failures."""
    troubleshooting = [
        "Work item validation failed",
        f"Found {len(validation_issues)} issue(s)",
        "Review questions below to resolve ambiguities"
    ]

    recovery = []
    for question in validation_questions[:5]:  # Limit to 5 questions
        recovery.append(f"Answer: {question}")

    return ErrorContext(
        error_type="ValidationError",
        error_message=f"Work item failed validation with {len(validation_issues)} issues",
        work_item_id=work_item_id,
        agent_id=agent_id,
        troubleshooting_hints=troubleshooting + validation_issues[:5],
        recovery_suggestions=recovery,
        environment_info=None  # Not needed for validation errors
    )


def format_error_for_rust(error_context: ErrorContext) -> str:
    """
    Format error for Rust bridge WorkResult.

    Returns concise but actionable error message suitable for
    transmission back to Rust via PyO3.
    """
    parts = [f"{error_context.error_type}: {error_context.error_message}"]

    if error_context.work_item_id:
        parts.append(f"Work Item: {error_context.work_item_id} ({error_context.work_item_phase})")

    if error_context.agent_id:
        parts.append(f"Agent: {error_context.agent_id}")

    if error_context.troubleshooting_hints:
        parts.append("Troubleshooting:")
        for hint in error_context.troubleshooting_hints[:3]:  # Limit to 3 hints
            parts.append(f"  • {hint}")

    if error_context.recovery_suggestions:
        parts.append("Recovery:")
        for suggestion in error_context.recovery_suggestions[:2]:  # Limit to 2 suggestions
            parts.append(f"  → {suggestion}")

    return "\n".join(parts)
