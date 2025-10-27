"""
Multi-Agent System - Four Primary Agents.

Implements the CLAUDE.md multi-agent architecture:
1. Orchestrator - Central coordinator and state manager
2. Optimizer - Context and resource optimization specialist
3. Reviewer - Quality assurance and validation specialist
4. Executor - Primary work agent and sub-agent manager
"""

from .orchestrator import OrchestratorAgent, OrchestratorConfig
from .optimizer import OptimizerAgent, OptimizerConfig
from .reviewer import ReviewerAgent, ReviewerConfig, QualityGate, ReviewResult
from .executor import ExecutorAgent, ExecutorConfig, WorkTask

__all__ = [
    "OrchestratorAgent",
    "OrchestratorConfig",
    "OptimizerAgent",
    "OptimizerConfig",
    "ReviewerAgent",
    "ReviewerConfig",
    "QualityGate",
    "ReviewResult",
    "ExecutorAgent",
    "ExecutorConfig",
    "WorkTask",
]
