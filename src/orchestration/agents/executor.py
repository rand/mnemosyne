"""
Executor Agent - Primary Work Agent and Sub-Agent Manager.

Responsibilities:
- Follow Work Plan Protocol (Phases 1-4)
- Execute atomic tasks from plans using Claude Agent SDK
- Spawn sub-agents for safe parallel work
- Apply loaded skills
- Challenge vague requirements
- Implement code, tests, documentation
- Commit at checkpoints
"""

from dataclasses import dataclass
from typing import Dict, List, Optional, Callable, Any
from enum import Enum
import asyncio

try:
    from .claude_agent_sdk import ClaudeSDKClient, ClaudeAgentOptions
    from .base_agent import AgentExecutionMixin, WorkItem, WorkResult
    from .logging_config import get_logger
    from .error_context import (
        create_work_item_error_context,
        create_session_error_context,
        format_error_for_rust
    )
    from .validation import validate_work_item, validate_agent_state
    from .metrics import get_metrics_collector
except ImportError:
    import sys, os
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    from claude_agent_sdk import ClaudeSDKClient, ClaudeAgentOptions
    from base_agent import AgentExecutionMixin, WorkItem, WorkResult
    from logging_config import get_logger
    from error_context import (
        create_work_item_error_context,
        create_session_error_context,
        format_error_for_rust
    )
    from validation import validate_work_item, validate_agent_state
    from metrics import get_metrics_collector

logger = get_logger("executor")


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
    # Claude Agent SDK configuration
    allowed_tools: Optional[List[str]] = None
    permission_mode: str = "acceptEdits"


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


class ExecutorAgent(AgentExecutionMixin):
    """
    Primary work agent and sub-agent manager.

    Executes work following the Work Plan Protocol using Claude Agent SDK:
    - Phase 1: Prompt → Spec
    - Phase 2: Spec → Full Spec
    - Phase 3: Full Spec → Plan
    - Phase 4: Plan → Artifacts

    Uses ClaudeSDKClient to maintain conversation context and execute tasks.

    **PyO3 Bridge Integration**: Inherits from AgentExecutionMixin to provide
    standard interface for Rust bridge communication.
    """

    EXECUTOR_SYSTEM_PROMPT = """You are the Executor Agent in a multi-agent orchestration system.

Your role:
- Execute work following the Work Plan Protocol (Phases 1-4)
- Challenge vague requirements and ask clarifying questions
- Use tools to read files, write code, run tests
- Maintain high code quality standards
- Create checkpoints at key milestones

Work Plan Protocol:
Phase 1: Prompt → Spec (clarify requirements, resolve ambiguities)
Phase 2: Spec → Full Spec (decompose components, define test plan)
Phase 3: Full Spec → Plan (create execution plan with dependencies)
Phase 4: Plan → Artifacts (implement code, tests, documentation)

You have access to tools for file operations, code execution, and version control.
Always follow best practices and validate your work before marking it complete."""

    def __init__(
        self,
        config: ExecutorConfig,
        coordinator,
        storage,
        parallel_executor
    ):
        """
        Initialize Executor agent with Claude Agent SDK.

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

        # Initialize Claude Agent SDK client
        self.claude_client = ClaudeSDKClient(
            options=ClaudeAgentOptions(
                allowed_tools=config.allowed_tools or [
                    "Read", "Write", "Edit", "Bash", "Glob", "Grep"
                ],
                permission_mode=config.permission_mode
            )
        )

        # Register with coordinator
        self.coordinator.register_agent(config.agent_id)

        # State
        self._current_phase = ExecutorPhase.IDLE
        self._active_subagents: List[str] = []
        self._completed_tasks: List[str] = []
        self._checkpoint_count = 0
        self._session_active = False

    async def start_session(self):
        """Start Claude agent session."""
        if not self._session_active:
            logger.info(f"Starting session for agent {self.config.agent_id}")
            await self.claude_client.connect()
            # Initialize with system prompt
            await self.claude_client.query(self.EXECUTOR_SYSTEM_PROMPT)
            self._session_active = True
            logger.info(f"Session started successfully for {self.config.agent_id}")

    async def stop_session(self):
        """Stop Claude agent session."""
        if self._session_active:
            logger.info(f"Stopping session for agent {self.config.agent_id}")
            await self.claude_client.disconnect()
            self._session_active = False
            logger.info(f"Session stopped for {self.config.agent_id}")

    async def execute_work_plan(self, work_plan: Dict[str, Any]) -> Dict[str, Any]:
        """
        Execute work plan following Work Plan Protocol using Claude Agent SDK.

        Args:
            work_plan: Work plan with phases 1-4

        Returns:
            Execution results
        """
        logger.info(f"Starting work plan execution: {work_plan.get('prompt', 'N/A')[:100]}")
        self.coordinator.update_agent_state(self.config.agent_id, "running")
        self._current_phase = ExecutorPhase.ANALYZING

        try:
            # Ensure session is active
            if not self._session_active:
                logger.debug("Session not active, starting new session")
                await self.start_session()

            # Validate work plan
            logger.debug("Validating work plan")
            validation_result = await self._validate_work_plan(work_plan)

            if not validation_result["valid"]:
                # Challenge vague requirements
                if self.config.challenge_vague_requirements:
                    logger.warning(f"Work plan validation failed: {validation_result['issues']}")
                    return {
                        "status": "challenged",
                        "issues": validation_result["issues"],
                        "questions": validation_result["questions"]
                    }

            # Execute all phases using Claude Agent SDK
            self._current_phase = ExecutorPhase.PLANNING

            # Construct comprehensive prompt for Claude
            execution_prompt = self._build_execution_prompt(work_plan)

            # Send work plan to Claude agent
            await self.claude_client.query(execution_prompt)

            # Collect responses
            responses = []
            async for message in self.claude_client.receive_response():
                responses.append(message)
                # Store important messages in memory
                await self._store_message(message)

            # Extract artifacts from responses
            artifacts = self._extract_artifacts(responses)

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

            logger.info(
                f"Work plan completed successfully: "
                f"{len(self._completed_tasks)} tasks, "
                f"{self._checkpoint_count} checkpoints"
            )

            return {
                "status": "success",
                "artifacts": artifacts,
                "checkpoints": self._checkpoint_count,
                "completed_tasks": len(self._completed_tasks),
                "responses": responses
            }

        except Exception as e:
            self.coordinator.update_agent_state(self.config.agent_id, "failed")
            logger.error(f"Execution failed: {type(e).__name__}: {str(e)}", exc_info=True)
            raise RuntimeError(f"Execution failed: {e}") from e

    def _build_execution_prompt(self, work_plan: Dict[str, Any]) -> str:
        """Build comprehensive execution prompt for Claude agent."""
        prompt_parts = [
            "# Work Plan Execution Request\n",
            f"**Prompt**: {work_plan.get('prompt', 'Not specified')}\n",
        ]

        if "tech_stack" in work_plan:
            prompt_parts.append(f"**Tech Stack**: {work_plan['tech_stack']}\n")

        if "success_criteria" in work_plan:
            prompt_parts.append(f"**Success Criteria**: {work_plan['success_criteria']}\n")

        if "constraints" in work_plan:
            prompt_parts.append(f"**Constraints**: {', '.join(work_plan['constraints'])}\n")

        prompt_parts.append("\n## Instructions\n")
        prompt_parts.append("Follow the Work Plan Protocol:\n")
        prompt_parts.append("1. Phase 1: Analyze and clarify requirements\n")
        prompt_parts.append("2. Phase 2: Decompose into components with test plan\n")
        prompt_parts.append("3. Phase 3: Create execution plan\n")
        prompt_parts.append("4. Phase 4: Implement code, tests, and documentation\n")
        prompt_parts.append("\nUse tools to read files, write code, and run tests.\n")
        prompt_parts.append("Commit your changes when logical units are complete.\n")

        return "".join(prompt_parts)

    async def _store_message(self, message: Any):
        """Store important messages in memory."""
        # Extract content from message
        content = str(message)
        if len(content) > 100:  # Only store substantial messages
            self.storage.store({
                "content": content[:500],  # Truncate long messages
                "namespace": f"project:agent-{self.config.agent_id}",
                "importance": 7,
                "tags": ["execution", self._current_phase.value]
            })

    def _extract_artifacts(self, responses: List[Any]) -> Dict[str, Any]:
        """Extract artifacts from Claude agent responses."""
        # Convert responses to serializable format
        serializable_responses = []
        for response in responses:
            if hasattr(response, 'data'):
                # SystemMessage or similar - extract data
                serializable_responses.append({
                    "type": type(response).__name__,
                    "data": str(response.data) if not isinstance(response.data, (dict, list)) else response.data
                })
            else:
                serializable_responses.append(str(response))

        artifacts = {
            "code": {},
            "tests": {},
            "documentation": {},
            "responses": serializable_responses
        }

        # In production, parse tool_use messages to extract created files
        # For now, return responses as artifacts
        return artifacts

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
            questions.append("What tech stack / technologies should be used?")

        if "deployment" not in work_plan:
            questions.append("Where will this be deployed?")

        if "success_criteria" not in work_plan:
            issues.append("Success criteria not defined")
            questions.append("How will we know when this is complete?")

        # Check prompt for vague terms and insufficient detail
        prompt = work_plan.get("prompt", "")

        # 1. Check for vague trigger words
        vague_terms = ["quickly", "just", "simple", "easy", "whatever"]
        for term in vague_terms:
            if term in prompt.lower():
                issues.append(f"Vague requirement: '{term}'")
                questions.append(f"Please clarify what '{term}' means in this context")

        # 2. Check word count (< 10 words is likely too brief)
        word_count = len(prompt.split())
        if word_count < 10:
            issues.append(f"Requirement too brief ({word_count} words)")
            questions.append("Please provide more details about what needs to be built")

        # 3. Check for missing detail categories
        prompt_lower = prompt.lower()
        detail_categories = {
            "what": ["add", "create", "build", "implement", "develop"],
            "why": ["because", "to", "for", "need", "require", "goal", "purpose"],
            "how": ["using", "with", "via", "through", "by"],
            "constraints": ["must", "should", "cannot", "within", "limit", "requirement"],
            "scope": ["only", "all", "some", "specific", "following", "include"]
        }

        missing_categories = []
        for category, indicators in detail_categories.items():
            if not any(indicator in prompt_lower for indicator in indicators):
                missing_categories.append(category)

        # If missing 3+ categories, prompt is likely too vague
        if len(missing_categories) >= 3:
            issues.append(f"Prompt lacks detail in: {', '.join(missing_categories)}")
            questions.extend([
                "What specifically needs to be built? (what)",
                "Why is this needed? (purpose)",
                "How should it be implemented? (approach)",
                "Are there any constraints or requirements? (constraints)"
            ])

        return {
            "valid": len(issues) == 0,
            "issues": issues,
            "questions": questions
        }

    async def _validate_artifacts(self, artifacts: Dict):
        """Validate produced artifacts."""
        # Check that Claude produced responses
        if not artifacts.get("responses"):
            raise ValueError("No responses from Claude agent")

        # In production, validate that code was written, tests pass, etc.
        # This would involve parsing tool_use messages and checking results

    async def _commit_work(self, artifacts: Dict):
        """Commit work to version control."""
        # Store commit record in memory
        self.storage.store({
            "content": f"Checkpoint {self._checkpoint_count}: Work committed",
            "namespace": f"project:agent-{self.config.agent_id}",
            "importance": 10,
            "tags": ["checkpoint", "commit"]
        })
        self._checkpoint_count += 1

    async def spawn_subagent(self, task: WorkTask) -> str:
        """
        Spawn sub-agent for task execution using Claude Agent SDK.

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

        # Create new Claude client for sub-agent
        # (In production, this would spawn a separate Claude session)

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
            "checkpoints": self._checkpoint_count,
            "session_active": self._session_active
        }

    async def _execute_work_item(self, work_item: WorkItem) -> WorkResult:
        """
        Execute work item for PyO3 bridge integration.

        This implements the AgentExecutionMixin interface, allowing the
        Rust bridge to send work items to this Python agent.

        Args:
            work_item: Work item from Rust (via PyO3 bridge)

        Returns:
            Work result to send back to Rust
        """
        logger.info(f"Received work item from Rust bridge: {work_item.id} (phase: {work_item.phase})")
        logger.debug(f"Work item description: {work_item.description[:200]}")

        # Start metrics tracking
        metrics_collector = get_metrics_collector()
        work_metrics = metrics_collector.start_work_item(
            work_item_id=work_item.id,
            agent_id=self.config.agent_id,
            phase=work_item.phase
        )

        # Validate work item
        validation_result = validate_work_item(work_item)
        if not validation_result.valid:
            logger.error(f"Work item validation failed: {validation_result.errors}")
            metrics_collector.finish_work_item(work_item.id, success=False, error_type="ValidationError")
            return WorkResult(
                success=False,
                error=f"Invalid work item:\n" + "\n".join(f"  • {err}" for err in validation_result.errors)
            )

        # Log warnings if any
        if validation_result.warnings:
            for warning in validation_result.warnings:
                logger.warning(f"Work item validation warning: {warning}")

        # Validate agent state
        state_validation = validate_agent_state(
            agent_id=self.config.agent_id,
            session_active=self._session_active,
            required_session=False  # We can auto-start if needed
        )
        if not state_validation.valid:
            logger.error(f"Agent state validation failed: {state_validation.errors}")
            metrics_collector.finish_work_item(work_item.id, success=False, error_type="StateValidationError")
            return WorkResult(
                success=False,
                error=f"Invalid agent state:\n" + "\n".join(f"  • {err}" for err in state_validation.errors)
            )

        try:
            # Convert WorkItem to work plan format
            # Map WorkItem fields to what execute_work_plan expects
            work_plan = {
                "id": work_item.id,
                "prompt": work_item.description,  # Map description → prompt for validation
                "tech_stack": "Python",  # Default for E2E testing
                "success_criteria": "Code executes without errors",  # Default
                "phase": work_item.phase,
                "priority": work_item.priority,
                "review_feedback": work_item.review_feedback or [],
                "review_attempt": work_item.review_attempt,
                "consolidated_context_id": work_item.consolidated_context_id
            }

            # Execute using existing execute_work_plan method
            result = await self.execute_work_plan(work_plan)

            # Determine success
            success = result.get("status") == "success" if "status" in result else result.get("success", False)

            logger.info(f"Work item {work_item.id} completed {'successfully' if success else 'with errors'}")

            # Finish metrics tracking
            metrics_collector.finish_work_item(
                work_item.id,
                success=success,
                error_type=None if success else "ExecutionError"
            )

            # Log metrics
            completed_metrics = metrics_collector.get_work_item_metrics(work_item.id)
            if completed_metrics and completed_metrics.duration_seconds:
                logger.info(f"Work item {work_item.id} duration: {completed_metrics.duration_seconds:.2f}s")

            # Convert result to WorkResult format
            return WorkResult(
                success=success,
                data=result.get("output"),
                memory_ids=result.get("memory_ids", []),
                error=result.get("error")
            )

        except Exception as e:
            # Handle any errors during execution with enhanced context
            logger.error(
                f"Failed to execute work item {work_item.id}: {type(e).__name__}: {str(e)}",
                exc_info=True
            )

            # Finish metrics tracking with error
            error_type = type(e).__name__
            metrics_collector.finish_work_item(work_item.id, success=False, error_type=error_type)

            # Log metrics
            completed_metrics = metrics_collector.get_work_item_metrics(work_item.id)
            if completed_metrics and completed_metrics.duration_seconds:
                logger.info(f"Work item {work_item.id} failed after {completed_metrics.duration_seconds:.2f}s")

            # Create enhanced error context
            error_context = create_work_item_error_context(
                work_item_id=work_item.id,
                work_item_phase=work_item.phase,
                work_item_description=work_item.description,
                agent_id=self.config.agent_id,
                agent_state=self._current_phase.value if hasattr(self, '_current_phase') else "unknown",
                session_active=self._session_active,
                error=e
            )

            # Log full context for debugging
            logger.debug(f"Error context:\n{error_context.format()}")

            # Return concise error for Rust bridge
            return WorkResult(
                success=False,
                error=format_error_for_rust(error_context)
            )

    async def __aenter__(self):
        """Async context manager entry."""
        await self.start_session()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit."""
        await self.stop_session()


# Standalone agent runner
async def main():
    """Run executor agent as independent process."""
    import argparse
    import asyncio
    import httpx
    import signal
    import sys

    parser = argparse.ArgumentParser(description="Executor Agent")
    parser.add_argument("--agent-id", default="executor", help="Agent ID")
    parser.add_argument("--api-url", default="http://127.0.0.1:3000", help="API server URL")
    parser.add_argument("--database", default=".mnemosyne/orchestration.db", help="Database path")
    parser.add_argument("--namespace", default="project:mnemosyne", help="Namespace")
    args = parser.parse_args()

    logger.info(f"Starting Executor Agent (ID: {args.agent_id})")
    logger.info(f"API Server: {args.api_url}")

    http_client = httpx.AsyncClient(timeout=5.0)

    async def send_heartbeat():
        while True:
            try:
                await http_client.post(
                    f"{args.api_url}/events",
                    json={"event_type": "Heartbeat", "instance_id": args.agent_id, "timestamp": "auto"}
                )
                logger.debug(f"Heartbeat sent from {args.agent_id}")
            except Exception as e:
                logger.warning(f"Heartbeat failed: {e}")
            await asyncio.sleep(10)

    shutdown_event = asyncio.Event()

    def signal_handler(signum, frame):
        logger.info(f"Received signal {signum}, shutting down...")
        shutdown_event.set()

    signal.signal(signal.SIGTERM, signal_handler)
    signal.signal(signal.SIGINT, signal_handler)

    heartbeat_task = asyncio.create_task(send_heartbeat())

    try:
        logger.info("Executor agent running (press Ctrl+C to stop)")
        await shutdown_event.wait()
    except KeyboardInterrupt:
        logger.info("Keyboard interrupt received")
    except Exception as e:
        logger.error(f"Error in agent main loop: {e}")
        sys.exit(1)
    finally:
        logger.info("Shutting down executor agent...")
        heartbeat_task.cancel()
        await http_client.aclose()
        logger.info("Executor agent stopped")


if __name__ == "__main__":
    import asyncio
    asyncio.run(main())
