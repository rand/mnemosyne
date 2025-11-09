"""
Enhanced Orchestration Engine for Multi-Agent System.

Integrates all components:
- 4-agent architecture (Orchestrator, Optimizer, Reviewer, Executor)
- Low-latency context monitoring (10ms polling)
- Parallel execution with sub-agents (max 4 concurrent)
- Work Plan Protocol enforcement (Phases 1-4)
- PyO3 bindings for <1ms storage operations
- Real-time monitoring dashboard (optional)
"""

import asyncio
from typing import Dict, Any, Optional
from dataclasses import dataclass

# Import PyO3 bindings (will be available after maturin build)
try:
    import mnemosyne_core
    BINDINGS_AVAILABLE = True
except ImportError:
    BINDINGS_AVAILABLE = False
    print("Warning: PyO3 bindings not available. Build with: maturin develop")

from .context_monitor import LowLatencyContextMonitor, ContextState
from .parallel_executor import ParallelExecutor, ExecutionPlan, SubTask
from .agents import (
    OrchestratorAgent, OrchestratorConfig,
    OptimizerAgent, OptimizerConfig,
    ReviewerAgent, ReviewerConfig, QualityGate,
    ExecutorAgent, ExecutorConfig
)


@dataclass
class EngineConfig:
    """Configuration for orchestration engine."""
    # Storage
    db_path: Optional[str] = None

    # Context monitoring
    polling_interval: float = 0.01  # 10ms
    preservation_threshold: float = 0.75
    critical_threshold: float = 0.90

    # Parallel execution
    max_concurrent: int = 4
    spawn_timeout: float = 30.0

    # Dashboard
    enable_dashboard: bool = False

    # Agent configs
    orchestrator_config: Optional[OrchestratorConfig] = None
    optimizer_config: Optional[OptimizerConfig] = None
    reviewer_config: Optional[ReviewerConfig] = None
    executor_config: Optional[ExecutorConfig] = None


class OrchestrationEngine:
    """
    Enhanced orchestration engine for multi-agent system.

    Coordinates:
    - 4 primary agents (Orchestrator, Optimizer, Reviewer, Executor)
    - Low-latency context monitoring
    - Parallel sub-agent execution
    - Work Plan Protocol enforcement
    """

    def __init__(self, config: EngineConfig):
        """
        Initialize orchestration engine.

        Args:
            config: Engine configuration
        """
        self.config = config

        # Check PyO3 bindings
        if not BINDINGS_AVAILABLE:
            raise RuntimeError(
                "PyO3 bindings not available. "
                "Build and install with: maturin develop --features python"
            )

        # Initialize PyO3 components
        self.coordinator = mnemosyne_core.PyCoordinator()
        self.storage = mnemosyne_core.PyStorage(config.db_path)

        # Initialize context monitor
        self.context_monitor = LowLatencyContextMonitor(
            coordinator=self.coordinator,
            polling_interval=config.polling_interval,
            preservation_threshold=config.preservation_threshold,
            critical_threshold=config.critical_threshold
        )

        # Initialize parallel executor
        self.parallel_executor = ParallelExecutor(
            coordinator=self.coordinator,
            storage=self.storage,
            max_concurrent=config.max_concurrent,
            spawn_timeout=config.spawn_timeout
        )

        # Initialize agents
        self.orchestrator = OrchestratorAgent(
            config=config.orchestrator_config or OrchestratorConfig(),
            coordinator=self.coordinator,
            storage=self.storage,
            context_monitor=self.context_monitor
        )

        self.optimizer = OptimizerAgent(
            config=config.optimizer_config or OptimizerConfig(),
            coordinator=self.coordinator,
            storage=self.storage
        )

        self.reviewer = ReviewerAgent(
            config=config.reviewer_config or ReviewerConfig(),
            coordinator=self.coordinator,
            storage=self.storage
        )

        self.executor = ExecutorAgent(
            config=config.executor_config or ExecutorConfig(),
            coordinator=self.coordinator,
            storage=self.storage,
            parallel_executor=self.parallel_executor
        )

        # Dashboard
        self._dashboard = None
        self._dashboard_task = None

        # State
        self._monitoring_active = False

    async def start(self):
        """Start orchestration engine."""
        # Start context monitoring
        await self.context_monitor.start()
        self._monitoring_active = True

        # Start dashboard if enabled
        if self.config.enable_dashboard:
            from .dashboard import run_dashboard
            self._dashboard_task = asyncio.create_task(run_dashboard(self))
            print(f"Dashboard: enabled")

        print(f"Orchestration engine started")
        print(f"- Context monitoring: {self.config.polling_interval*1000:.1f}ms interval")
        print(f"- Max concurrent agents: {self.config.max_concurrent}")
        print(f"- Preservation threshold: {self.config.preservation_threshold:.0%}")

    async def stop(self):
        """Stop orchestration engine."""
        # Stop dashboard if running
        if self._dashboard_task:
            self._dashboard_task.cancel()
            try:
                await self._dashboard_task
            except asyncio.CancelledError:
                pass

        # Stop context monitoring
        if self._monitoring_active:
            await self.context_monitor.stop()
            self._monitoring_active = False

        print("Orchestration engine stopped")

    async def execute_work_plan(self, work_plan: Dict[str, Any]) -> Dict[str, Any]:
        """
        Execute work plan using multi-agent orchestration.

        Work Plan Protocol (CLAUDE.md):
        1. Phase 1: Prompt → Spec (Optimizer discovers skills, Executor clarifies)
        2. Phase 2: Spec → Full Spec (Executor decomposes, Reviewer validates)
        3. Phase 3: Full Spec → Plan (Orchestrator schedules, Executor plans)
        4. Phase 4: Plan → Artifacts (Executor implements, Reviewer validates)

        Args:
            work_plan: Work plan dictionary with prompt and requirements

        Returns:
            Execution results
        """
        if not self._monitoring_active:
            await self.start()

        try:
            print("\n=== Starting Multi-Agent Orchestration ===")

            # Step 1: Optimizer discovers relevant skills
            print("\n[Optimizer] Discovering relevant skills...")
            optimized_context = await self.optimizer.optimize_context(
                task_description=work_plan.get("prompt", ""),
                current_context={
                    "available_tokens": self.context_monitor.get_available_budget()
                }
            )
            print(f"[Optimizer] Loaded {len(optimized_context['skills'])} skills")

            # Step 2: Executor validates and executes work plan
            print("\n[Executor] Executing work plan...")
            execution_result = await self.executor.execute_work_plan(work_plan)

            # If executor challenged requirements, return immediately
            if execution_result["status"] == "challenged":
                print(f"\n[Executor] Requirements challenged: {len(execution_result.get('issues', []))} issues")
                return execution_result

            # Step 3: Reviewer validates results
            if execution_result["status"] == "success":
                print("\n[Reviewer] Validating artifacts...")
                # Extract first artifact from list (executor returns list of artifacts)
                artifacts = execution_result["artifacts"]
                artifact_to_review = artifacts[0] if artifacts else {}
                review_result = await self.reviewer.review(artifact_to_review)

                if not review_result.passed:
                    print(f"[Reviewer] Validation failed: {len(review_result.issues)} issues")
                    return {
                        "status": "validation_failed",
                        "execution": execution_result,
                        "review": {
                            "passed": False,
                            "issues": review_result.issues,
                            "recommendations": review_result.recommendations
                        }
                    }

                print(f"[Reviewer] Validation passed (confidence: {review_result.confidence:.0%})")

            # Step 4: Orchestrator coordinates completion
            print("\n[Orchestrator] Coordinating completion...")
            orchestration_result = await self.orchestrator.coordinate_workflow(work_plan)

            # Gather statistics
            stats = {
                "context_monitor": self.context_monitor.get_statistics(),
                "parallel_executor": self.parallel_executor.get_status(),
                "orchestrator": self.orchestrator.get_status(),
                "optimizer": self.optimizer.get_status(),
                "reviewer": self.reviewer.get_statistics(),
                "executor": self.executor.get_status()
            }

            print("\n=== Orchestration Complete ===")
            print(f"- Context utilization: {self.context_monitor.get_current_metrics().utilization:.0%}")
            if execution_result.get("status") == "success":
                print(f"- Tasks completed: {execution_result.get('completed_tasks', 0)}")
                print(f"- Checkpoints: {execution_result.get('checkpoints', 0)}")

            return {
                "status": "success",
                "execution": execution_result,
                "orchestration": orchestration_result,
                "statistics": stats
            }

        except Exception as e:
            print(f"\n[ERROR] Orchestration failed: {e}")
            return {
                "status": "error",
                "error": str(e)
            }

    async def execute_parallel_tasks(self, plan: ExecutionPlan) -> Dict[str, Any]:
        """
        Execute tasks in parallel with dependency management.

        Args:
            plan: Execution plan with tasks and dependencies

        Returns:
            Execution results
        """
        print(f"\n[ParallelExecutor] Executing {len(plan.tasks)} tasks...")
        result = await self.parallel_executor.execute(plan)
        print(f"[ParallelExecutor] Completed: {result['completed']}/{len(plan.tasks)}")

        return result

    def get_status(self) -> Dict[str, Any]:
        """Get current engine status."""
        metrics = self.context_monitor.get_current_metrics()

        return {
            "monitoring_active": self._monitoring_active,
            "context": {
                "utilization": metrics.utilization if metrics else 0.0,
                "state": metrics.state.value if metrics else "unknown",
                "agent_count": metrics.agent_count if metrics else 0
            },
            "agents": {
                "orchestrator": self.orchestrator.get_status(),
                "optimizer": self.optimizer.get_status(),
                "reviewer": self.reviewer.get_status(),
                "executor": self.executor.get_status()
            },
            "parallel_executor": self.parallel_executor.get_status()
        }


# Convenience function for quick start
async def create_engine(db_path: Optional[str] = None) -> OrchestrationEngine:
    """
    Create and start orchestration engine with default configuration.

    Args:
        db_path: Optional path to SQLite database

    Returns:
        Started OrchestrationEngine instance
    """
    config = EngineConfig(db_path=db_path)
    engine = OrchestrationEngine(config)
    await engine.start()
    return engine
