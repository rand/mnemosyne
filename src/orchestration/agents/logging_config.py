"""
Logging configuration for Python agents.

Provides structured logging with proper levels and formatting for integration
with Rust's tracing infrastructure via PyO3.
"""

import logging
import sys
from typing import Optional


def configure_agent_logging(
    agent_name: str,
    level: str = "INFO",
    log_file: Optional[str] = None
) -> logging.Logger:
    """
    Configure structured logging for a Python agent.

    Args:
        agent_name: Name of the agent (e.g., "executor", "reviewer")
        level: Logging level (DEBUG, INFO, WARNING, ERROR, CRITICAL)
        log_file: Optional file path for logging (defaults to stderr)

    Returns:
        Configured logger instance

    Example:
        >>> logger = configure_agent_logging("executor", "INFO")
        >>> logger.info("Agent started")
        >>> logger.error("Failed to process work item", exc_info=True)
    """
    logger = logging.getLogger(f"mnemosyne.orchestration.{agent_name}")

    # Avoid duplicate handlers if already configured
    if logger.handlers:
        return logger

    logger.setLevel(getattr(logging, level.upper(), logging.INFO))

    # Create formatter with structured output
    formatter = logging.Formatter(
        fmt="%(asctime)s [%(levelname)8s] [%(name)s] %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S"
    )

    # Console handler (stderr for integration with Rust)
    console_handler = logging.StreamHandler(sys.stderr)
    console_handler.setLevel(logging.DEBUG)
    console_handler.setFormatter(formatter)
    logger.addHandler(console_handler)

    # Optional file handler
    if log_file:
        file_handler = logging.FileHandler(log_file)
        file_handler.setLevel(logging.DEBUG)
        file_handler.setFormatter(formatter)
        logger.addHandler(file_handler)

    # Don't propagate to root logger (avoid duplicate messages)
    logger.propagate = False

    return logger


def get_logger(agent_name: str) -> logging.Logger:
    """
    Get or create a logger for an agent.

    Args:
        agent_name: Name of the agent

    Returns:
        Logger instance (auto-configured if not already set up)
    """
    logger = logging.getLogger(f"mnemosyne.orchestration.{agent_name}")

    # Auto-configure if not already set up
    if not logger.handlers:
        logger = configure_agent_logging(agent_name)

    return logger


# Environment-based configuration
def configure_from_environment():
    """Configure logging based on environment variables."""
    import os

    # Check for log level override
    log_level = os.environ.get("MNEMOSYNE_LOG_LEVEL", "INFO")

    # Check for log file
    log_file = os.environ.get("MNEMOSYNE_LOG_FILE")

    return log_level, log_file
