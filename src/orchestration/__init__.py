"""
Multi-Agent Orchestration System for Mnemosyne.

Implements the 4-agent architecture from CLAUDE.md with:
- Orchestrator: Central coordinator
- Optimizer: Context and resource optimization
- Reviewer: Quality assurance and validation
- Executor: Primary work agent with sub-agent spawning

Features:
- Low-latency context monitoring (<10ms polling)
- Parallel execution with up to 4 concurrent sub-agents
- Direct Rust storage access via PyO3 (<1ms operations)
- Work Plan Protocol enforcement (Phases 1-4)
- Automatic context preservation at 75% threshold

Usage:
    from orchestration import create_engine

    engine = await create_engine()
    result = await engine.execute_work_plan({
        "prompt": "Implement feature X",
        "tech_stack": "Rust + Python",
        "success_criteria": "Tests passing, docs complete"
    })
"""

from .engine import (
    OrchestrationEngine,
    EngineConfig,
    create_engine
)
from .context_monitor import (
    LowLatencyContextMonitor,
    ContextMetrics,
    ContextState
)
from .parallel_executor import (
    ParallelExecutor,
    ExecutionPlan,
    SubTask,
    TaskStatus
)
from .agents import (
    OrchestratorAgent,
    OrchestratorConfig,
    OptimizerAgent,
    OptimizerConfig,
    ReviewerAgent,
    ReviewerConfig,
    QualityGate,
    ReviewResult,
    ExecutorAgent,
    ExecutorConfig,
    WorkTask
)

__version__ = "0.1.0"

__all__ = [
    # Engine
    "OrchestrationEngine",
    "EngineConfig",
    "create_engine",
    # Context monitoring
    "LowLatencyContextMonitor",
    "ContextMetrics",
    "ContextState",
    # Parallel execution
    "ParallelExecutor",
    "ExecutionPlan",
    "SubTask",
    "TaskStatus",
    # Agents
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
