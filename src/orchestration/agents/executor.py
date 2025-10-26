"""
Executor Agent - Primary Work Agent and Sub-Agent Manager.

Responsibilities:
- Follow Work Plan Protocol (Phases 1-4)
- Execute atomic tasks from plans
- Spawn sub-agents for safe parallel work
- Apply loaded skills
- Challenge vague requirements
- Implement code, tests, documentation
- Commit at checkpoints
"""

from dataclasses import dataclass
from typing import Dict, List, Optional, Callable
from enum import Enum


class ExecutorPhase(Enum):
    """Executor workflow phases."""
    IDLE = "idle"
    ANALYZING = "analyzing"
    PLANNING = "planning"
    EXECUTING = "executing"
    VALIDATING = "validating"
    COMMITTING = "committing"
    COMPLETED = "completed"


@dataclass
class ExecutorConfig:
    """Configuration for Executor agent."""
    agent_id: str = "executor"
    max_subagents: int = 4
    challenge_vague_requirements: bool = True
    auto_commit_checkpoints: bool = True
    validation_required: bool = True


@dataclass
class WorkTask:
    """Work task for execution."""
    id: str
    description: str
    phase: int  # Work Plan Protocol phase (1-4)
    requirements: List[str]
    deliverables: List[str]
    constraints: List[str]
    executor_func: Optional[Callable] = None


class ExecutorAgent:
    """
    Primary work agent and sub-agent manager.

    Executes work following the Work Plan Protocol:
    - Phase 1: Prompt → Spec
    - Phase 2: Spec → Full Spec
    - Phase 3: Full Spec → Plan
    - Phase 4: Plan → Artifacts
    """

    def __init__(
        self,
        config: ExecutorConfig,
        coordinator,
        storage,
        parallel_executor
    ):
        """
        Initialize Executor agent.

        Args:
            config: Executor configuration
            coordinator: PyCoordinator for shared state
            storage: PyStorage for memory operations
            parallel_executor: ParallelExecutor for sub-agent management
        """
        self.config = config
        self.coordinator = coordinator
        self.storage = storage
        self.parallel_executor = parallel_executor

        # Register with coordinator
        self.coordinator.register_agent(config.agent_id)

        # State
        self._current_phase = ExecutorPhase.IDLE
        self._active_subagents: List[str] = []
        self._completed_tasks: List[str] = []
        self._checkpoint_count = 0

    async def execute_work_plan(self, work_plan: Dict[str, Any]) -> Dict[str, Any]:
        """
        Execute work plan following Work Plan Protocol.

        Args:
            work_plan: Work plan with phases 1-4

        Returns:
            Execution results
        """
        self.coordinator.update_agent_state(self.config.agent_id, "running")
        self._current_phase = ExecutorPhase.ANALYZING

        try:
            # Validate work plan
            validation_result = await self._validate_work_plan(work_plan)
            if not validation_result["valid"]:
                # Challenge vague requirements
                if self.config.challenge_vague_requirements:
                    return {
                        "status": "challenged",
                        "issues": validation_result["issues"],
                        "questions": validation_result["questions"]
                    }

            # Phase 1: Prompt → Spec
            self._current_phase = ExecutorPhase.PLANNING
            spec = await self._execute_phase_1(work_plan.get("prompt"))

            # Phase 2: Spec → Full Spec
            full_spec = await self._execute_phase_2(spec)

            # Phase 3: Full Spec → Plan
            execution_plan = await self._execute_phase_3(full_spec)

            # Phase 4: Plan → Artifacts
            self._current_phase = ExecutorPhase.EXECUTING
            artifacts = await self._execute_phase_4(execution_plan)

            # Validation
            if self.config.validation_required:
                self._current_phase = ExecutorPhase.VALIDATING
                await self._validate_artifacts(artifacts)

            # Commit
            if self.config.auto_commit_checkpoints:
                self._current_phase = ExecutorPhase.COMMITTING
                await self._commit_work(artifacts)

            self._current_phase = ExecutorPhase.COMPLETED
            self.coordinator.update_agent_state(self.config.agent_id, "complete")

            return {
                "status": "success",
                "artifacts": artifacts,
                "checkpoints": self._checkpoint_count,
                "completed_tasks": len(self._completed_tasks)
            }

        except Exception as e:
            self.coordinator.update_agent_state(self.config.agent_id, "failed")
            raise RuntimeError(f"Execution failed: {e}") from e

    async def _validate_work_plan(self, work_plan: Dict) -> Dict[str, Any]:
        """Validate work plan for completeness and clarity."""
        issues = []
        questions = []

        # Check for required fields
        if "prompt" not in work_plan:
            issues.append("Missing prompt/requirements")
            questions.append("What is the goal of this work?")

        if "tech_stack" not in work_plan:
            issues.append("Tech stack not specified")
            questions.append("What technologies should be used?")

        if "deployment" not in work_plan:
            questions.append("Where will this be deployed?")

        if "success_criteria" not in work_plan:
            issues.append("Success criteria not defined")
            questions.append("How will we know when this is complete?")

        # Check for vague terms
        prompt = work_plan.get("prompt", "")
        vague_terms = ["quickly", "just", "simple", "easy", "whatever"]
        for term in vague_terms:
            if term in prompt.lower():
                issues.append(f"Vague requirement: '{term}'")
                questions.append(f"Please clarify what '{term}' means in this context")

        return {
            "valid": len(issues) == 0,
            "issues": issues,
            "questions": questions
        }

    async def _execute_phase_1(self, prompt: str) -> Dict[str, Any]:
        """
        Phase 1: Prompt → Spec.

        Transform request into clear specification.
        """
        # Store prompt in memory
        await self.storage.store({
            "content": f"Phase 1: Initial prompt - {prompt}",
            "namespace": "session:executor",
            "importance": 8,
            "tags": ["phase-1", "spec"]
        })

        # Create specification
        spec = {
            "intent": prompt,
            "ambiguities_resolved": [],
            "tech_stack_confirmed": False,
            "phase": 1
        }

        return spec

    async def _execute_phase_2(self, spec: Dict) -> Dict[str, Any]:
        """
        Phase 2: Spec → Full Spec.

        Decompose into components with dependencies and test plan.
        """
        # Store spec in memory
        await self.storage.store({
            "content": f"Phase 2: Full specification with components and test plan",
            "namespace": "session:executor",
            "importance": 9,
            "tags": ["phase-2", "full-spec"]
        })

        full_spec = {
            **spec,
            "components": [],
            "dependencies": [],
            "typed_holes": [],
            "test_plan": {},
            "phase": 2
        }

        return full_spec

    async def _execute_phase_3(self, full_spec: Dict) -> Dict[str, Any]:
        """
        Phase 3: Full Spec → Plan.

        Create execution plan with parallelization.
        """
        # Store plan in memory
        await self.storage.store({
            "content": f"Phase 3: Execution plan with parallel tasks",
            "namespace": "session:executor",
            "importance": 9,
            "tags": ["phase-3", "plan"]
        })

        execution_plan = {
            **full_spec,
            "tasks": [],
            "critical_path": [],
            "parallel_streams": [],
            "checkpoints": [],
            "phase": 3
        }

        return execution_plan

    async def _execute_phase_4(self, execution_plan: Dict) -> Dict[str, Any]:
        """
        Phase 4: Plan → Artifacts.

        Execute plan, create code/tests/docs.
        """
        # Use parallel executor for task execution
        # (Simplified for now - production would integrate with ParallelExecutor)

        artifacts = {
            "code": {},
            "tests": {},
            "documentation": {},
            "phase": 4
        }

        # Checkpoint after completion
        if self.config.auto_commit_checkpoints:
            self._checkpoint_count += 1

        return artifacts

    async def _validate_artifacts(self, artifacts: Dict):
        """Validate produced artifacts."""
        # Check for required artifacts
        if not artifacts.get("code"):
            raise ValueError("No code artifacts produced")

        if not artifacts.get("tests"):
            raise ValueError("No tests produced")

        if not artifacts.get("documentation"):
            raise ValueError("No documentation produced")

    async def _commit_work(self, artifacts: Dict):
        """Commit work to version control."""
        # Store commit record in memory
        await self.storage.store({
            "content": f"Checkpoint {self._checkpoint_count}: Work committed",
            "namespace": "session:executor",
            "importance": 10,
            "tags": ["checkpoint", "commit"]
        })

    async def spawn_subagent(self, task: WorkTask) -> str:
        """
        Spawn sub-agent for task execution.

        Safety checks:
        - Task truly independent
        - Context budget allows
        - No circular dependencies
        - Clear success criteria
        - Handoff protocol established
        - Rollback strategy exists

        Returns:
            Sub-agent ID
        """
        # Check safety criteria
        if len(self._active_subagents) >= self.config.max_subagents:
            raise RuntimeError(
                f"Max subagents ({self.config.max_subagents}) already active"
            )

        # Check context budget
        utilization = self.coordinator.get_context_utilization()
        if utilization > 0.75:
            raise RuntimeError("Insufficient context budget for sub-agent")

        # Generate sub-agent ID
        subagent_id = f"{self.config.agent_id}_sub_{len(self._active_subagents)}"

        # Register sub-agent
        self.coordinator.register_agent(subagent_id)
        self.coordinator.update_agent_state(subagent_id, "running")

        self._active_subagents.append(subagent_id)

        return subagent_id

    async def terminate_subagent(self, subagent_id: str):
        """Terminate a sub-agent."""
        if subagent_id in self._active_subagents:
            self.coordinator.update_agent_state(subagent_id, "complete")
            self._active_subagents.remove(subagent_id)

    def get_active_subagents(self) -> List[str]:
        """Get list of active sub-agent IDs."""
        return list(self._active_subagents)

    def get_status(self) -> Dict[str, Any]:
        """Get executor status."""
        return {
            "phase": self._current_phase.value,
            "active_subagents": len(self._active_subagents),
            "completed_tasks": len(self._completed_tasks),
            "checkpoints": self._checkpoint_count
        }
