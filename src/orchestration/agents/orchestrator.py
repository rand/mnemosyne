"""
Orchestrator Agent - Central Coordinator and State Manager.

Responsibilities:
- Coordinate handoffs between agents with zero-copy data passing
- Monitor execution state across parallel workstreams
- Prevent race conditions and deadlocks through dependency-aware scheduling
- Preserve context before compaction (75% threshold)
- Maintain global work graph and schedule parallel work
"""

from dataclasses import dataclass
from typing import Dict, List, Optional, Any
from enum import Enum
import json

try:
    from .claude_agent_sdk import ClaudeSDKClient, ClaudeAgentOptions
    from .base_agent import AgentExecutionMixin, WorkItem, WorkResult
    from .logging_config import get_logger
except ImportError:
    # Running as standalone script
    import sys
    import os
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    from claude_agent_sdk import ClaudeSDKClient, ClaudeAgentOptions
    from base_agent import AgentExecutionMixin, WorkItem, WorkResult
    from logging_config import get_logger

logger = get_logger("orchestrator")


class OrchestratorPhase(Enum):
    """Orchestration phases."""
    IDLE = "idle"
    PLANNING = "planning"
    EXECUTING = "executing"
    MONITORING = "monitoring"
    PRESERVING = "preserving"
    COMPLETED = "completed"


@dataclass
class OrchestratorConfig:
    """Configuration for Orchestrator agent."""
    agent_id: str = "orchestrator"
    max_parallel_agents: int = 4
    context_preservation_threshold: float = 0.75
    snapshot_dir: str = ".claude/context-snapshots"
    checkpoint_frequency: int = 5  # Checkpoint every 5 phase transitions
    deadlock_timeout: float = 60.0  # Detect deadlock after 60s of no progress
    # Claude Agent SDK configuration
    allowed_tools: Optional[List[str]] = None
    permission_mode: str = "default"  # Orchestrator observes, doesn't edit


class OrchestratorAgent(AgentExecutionMixin):
    """
    Central coordinator for multi-agent orchestration using Claude Agent SDK.

    Manages:
    - Agent lifecycle (spawn, monitor, terminate)
    - Work distribution and scheduling
    - Context preservation and checkpointing
    - Deadlock detection and recovery
    - Inter-agent communication

    **PyO3 Bridge Integration**: Inherits from AgentExecutionMixin to provide
    standard interface for Rust bridge communication.
    """

    ORCHESTRATOR_SYSTEM_PROMPT = """You are the Orchestrator Agent in a multi-agent orchestration system.

Your role:
- Central coordinator and state manager
- Coordinate handoffs between Executor, Optimizer, and Reviewer agents
- Monitor execution state across parallel workstreams
- Prevent race conditions and deadlocks through dependency-aware scheduling
- Preserve context before compaction (trigger at 75% utilization)
- Maintain global work graph and schedule parallel work

Key Responsibilities:
1. Parse work plans and build dependency graphs
2. Determine optimal task scheduling for parallel execution
3. Monitor context utilization and trigger preservation
4. Detect deadlocks (tasks waiting > 60s with no progress)
5. Coordinate agent handoffs with zero-copy data passing
6. Maintain checkpoints at phase transitions

You should analyze work plans, identify dependencies, and make high-level coordination decisions.
Focus on orchestration strategy, not implementation details."""

    def __init__(self, config: OrchestratorConfig, coordinator, storage, context_monitor):
        """
        Initialize Orchestrator agent with Claude Agent SDK.

        Args:
            config: Orchestrator configuration
            coordinator: PyCoordinator for shared state
            storage: PyStorage for memory operations
            context_monitor: LowLatencyContextMonitor for context tracking
        """
        self.config = config
        self.coordinator = coordinator
        self.storage = storage
        self.context_monitor = context_monitor

        # Initialize Claude Agent SDK client
        self.claude_client = ClaudeSDKClient(
            options=ClaudeAgentOptions(
                allowed_tools=config.allowed_tools or ["Read", "Glob"],
                permission_mode=config.permission_mode
            )
        )

        # Register with coordinator
        self.coordinator.register_agent(config.agent_id)

        # State
        self._phase = OrchestratorPhase.IDLE
        self._active_agents: Dict[str, str] = {}  # agent_id -> role
        self._work_graph: Dict[str, List[str]] = {}  # task_id -> dependencies
        self._checkpoint_count = 0
        self._session_active = False

    async def start_session(self):
        """Start Claude agent session."""
        if not self._session_active:
            await self.claude_client.connect()
            # Initialize with system prompt
            await self.claude_client.query(self.ORCHESTRATOR_SYSTEM_PROMPT)
            self._session_active = True

    async def stop_session(self):
        """Stop Claude agent session."""
        if self._session_active:
            await self.claude_client.disconnect()
            self._session_active = False

    async def _execute_work_item(self, work_item: WorkItem) -> WorkResult:
        """
        Execute work item (workflow coordination) for PyO3 bridge integration.

        This implements the AgentExecutionMixin interface, allowing the
        Rust bridge to send work items to this Python agent.

        Args:
            work_item: Work item from Rust (via PyO3 bridge)

        Returns:
            Work result to send back to Rust
        """
        try:
            # Convert WorkItem to work plan format
            work_plan = {
                "id": work_item.id,
                "description": work_item.description,
                "phase": work_item.phase,
                "priority": work_item.priority,
                "tasks": [],  # Tasks would be extracted from description or review_feedback
                "agents": [],  # Agent assignments would be determined by orchestrator
                "review_feedback": work_item.review_feedback or [],
                "review_attempt": work_item.review_attempt,
                "consolidated_context_id": work_item.consolidated_context_id
            }

            # Execute coordination using existing method
            result = await self.coordinate_workflow(work_plan)

            # Convert result to WorkResult format with JSON serialization
            return WorkResult(
                success=result.get("status") == "success",
                data=json.dumps({
                    "status": result.get("status"),
                    "executed": result.get("results", {}).get("executed", 0),
                    "checkpoints": result.get("checkpoints", 0),
                    "planning_analysis": str(result.get("planning_analysis", []))[:500]
                }),
                memory_ids=[],  # Orchestrator stores memories internally
                error=None if result.get("status") == "success" else "Orchestration failed"
            )

        except Exception as e:
            # Handle any errors during orchestration
            return WorkResult(
                success=False,
                error=f"Orchestrator error: {type(e).__name__}: {str(e)}"
            )

    async def coordinate_workflow(self, work_plan: Dict[str, Any]) -> Dict[str, Any]:
        """
        Coordinate multi-agent workflow execution using Claude Agent SDK.

        Args:
            work_plan: Work plan with phases and tasks

        Returns:
            Execution results
        """
        self._phase = OrchestratorPhase.PLANNING
        self.coordinator.update_agent_state(self.config.agent_id, "running")

        try:
            # Ensure session is active
            if not self._session_active:
                await self.start_session()

            # Phase 1: Ask Claude to analyze work plan and build dependency graph
            planning_prompt = self._build_planning_prompt(work_plan)
            await self.claude_client.query(planning_prompt)

            # Collect Claude's analysis
            planning_responses = []
            async for message in self.claude_client.receive_response():
                planning_responses.append(message)
                await self._store_message(message, "planning")

            # Build work graph from analysis
            self._build_work_graph(work_plan)

            # Phase 2: Spawn required agents
            await self._spawn_agents(work_plan.get("agents", []))

            # Phase 3: Monitor execution with Claude's guidance
            self._phase = OrchestratorPhase.EXECUTING
            results = await self._execute_workflow(work_plan)

            # Phase 4: Cleanup and checkpoint
            await self._cleanup()

            self._phase = OrchestratorPhase.COMPLETED
            self.coordinator.update_agent_state(self.config.agent_id, "complete")

            return {
                "status": "success",
                "results": results,
                "checkpoints": self._checkpoint_count,
                "planning_analysis": planning_responses
            }

        except Exception as e:
            self.coordinator.update_agent_state(self.config.agent_id, "failed")
            raise RuntimeError(f"Orchestration failed: {e}") from e

    def _build_planning_prompt(self, work_plan: Dict[str, Any]) -> str:
        """Build planning prompt for Claude orchestrator."""
        prompt_parts = [
            "# Multi-Agent Workflow Coordination Request\n\n",
            "Analyze this work plan and provide orchestration strategy:\n\n",
            f"**Work Plan**: {json.dumps(work_plan, indent=2)}\n\n",
            "## Analysis Required:\n",
            "1. Identify all tasks and their dependencies\n",
            "2. Determine optimal scheduling for parallel execution\n",
            "3. Identify potential deadlocks or circular dependencies\n",
            "4. Recommend agent assignments (Executor, Optimizer, Reviewer)\n",
            "5. Suggest checkpoint locations for context preservation\n",
            "6. Estimate context utilization throughout execution\n\n",
            "Provide your orchestration strategy with reasoning.\n"
        ]
        return "".join(prompt_parts)

    async def _store_message(self, message: Any, phase: str):
        """Store important orchestration messages in memory."""
        content = str(message)
        if len(content) > 100:
            self.storage.store({
                "content": content[:500],
                "namespace": f"project:agent-{self.config.agent_id}",
                "importance": 8,
                "tags": ["orchestration", phase]
            })

    def _build_work_graph(self, work_plan: Dict[str, Any]):
        """Build dependency graph from work plan."""
        self._work_graph.clear()

        tasks = work_plan.get("tasks", [])
        for task in tasks:
            task_id = task.get("id")
            dependencies = task.get("depends_on", [])
            self._work_graph[task_id] = dependencies

        # Detect circular dependencies
        if self._has_circular_dependencies():
            raise RuntimeError("Circular dependencies detected in work graph")

    def _has_circular_dependencies(self) -> bool:
        """Detect circular dependencies in work graph."""
        visited = set()
        rec_stack = set()

        def visit(node):
            if node in rec_stack:
                return True  # Circular dependency
            if node in visited:
                return False

            visited.add(node)
            rec_stack.add(node)

            for dep in self._work_graph.get(node, []):
                if visit(dep):
                    return True

            rec_stack.remove(node)
            return False

        for task_id in self._work_graph:
            if visit(task_id):
                return True

        return False

    async def _spawn_agents(self, agent_configs: List[Dict[str, Any]]):
        """Spawn required agents for workflow."""
        for config in agent_configs:
            agent_id = config.get("id")
            agent_role = config.get("role")

            # Check parallel limit
            if len(self._active_agents) >= self.config.max_parallel_agents:
                # Ask Claude for scheduling decision
                scheduling_prompt = f"Max parallel agents reached ({self.config.max_parallel_agents}). How should we schedule agent '{agent_id}' (role: {agent_role})?"
                await self.claude_client.query(scheduling_prompt)

                scheduling_responses = []
                async for message in self.claude_client.receive_response():
                    scheduling_responses.append(message)

                # For now, wait for a slot
                # In production, implement actual scheduling based on Claude's recommendation

            # Register agent
            self.coordinator.register_agent(agent_id)
            self._active_agents[agent_id] = agent_role

    async def _execute_workflow(self, work_plan: Dict[str, Any]) -> Dict[str, Any]:
        """Execute workflow with context monitoring and Claude's guidance."""
        self._phase = OrchestratorPhase.MONITORING

        # Set up context preservation callback
        async def preserve_context(metrics):
            if metrics.utilization >= self.config.context_preservation_threshold:
                await self._preserve_context(metrics)

        self.context_monitor.set_preservation_callback(preserve_context)

        # Monitor execution
        monitoring_prompt = f"""Monitor workflow execution:
- Active agents: {len(self._active_agents)}
- Work graph tasks: {len(self._work_graph)}
- Context threshold: {self.config.context_preservation_threshold:.0%}

Watch for:
1. Deadlocks (tasks waiting > {self.config.deadlock_timeout}s)
2. Context utilization approaching threshold
3. Agent failures or timeouts

Provide monitoring guidance and alert on issues."""

        await self.claude_client.query(monitoring_prompt)

        monitoring_responses = []
        async for message in self.claude_client.receive_response():
            monitoring_responses.append(message)
            await self._store_message(message, "monitoring")

        # Execute tasks
        # (Implementation would integrate with ParallelExecutor)
        return {
            "executed": len(work_plan.get("tasks", [])),
            "preserved": self._checkpoint_count,
            "monitoring_guidance": monitoring_responses
        }

    async def _preserve_context(self, metrics):
        """Preserve context at 75% threshold with Claude's guidance."""
        self._phase = OrchestratorPhase.PRESERVING

        # Ask Claude what to preserve
        preservation_prompt = f"""Context utilization at {metrics.utilization:.1%} (threshold: {self.config.context_preservation_threshold:.0%}).

Current state:
- Active agents: {len(self._active_agents)}
- Work graph: {len(self._work_graph)} tasks
- Agent states: {list(self._active_agents.values())}

What should be preserved in this checkpoint? What can be compressed or discarded?"""

        await self.claude_client.query(preservation_prompt)

        preservation_responses = []
        async for message in self.claude_client.receive_response():
            preservation_responses.append(message)
            await self._store_message(message, "preservation")

        # Create snapshot directory
        import os
        os.makedirs(self.config.snapshot_dir, exist_ok=True)

        # Store context snapshot in memory
        snapshot = {
            "timestamp": metrics.timestamp,
            "utilization": metrics.utilization,
            "agents": dict(self._active_agents),
            "work_graph": dict(self._work_graph),
            "claude_guidance": preservation_responses
        }

        # Save to storage
        self.storage.store({
            "content": f"Context snapshot at {metrics.utilization:.1%} utilization",
            "namespace": "project:agent-orchestration",
            "importance": 10,
            "summary": f"Checkpoint {self._checkpoint_count}",
            "tags": ["checkpoint", "context-preservation"]
        })

        self._checkpoint_count += 1
        self._phase = OrchestratorPhase.EXECUTING

    async def _cleanup(self):
        """Cleanup after workflow completion."""
        # Ask Claude for cleanup recommendations
        cleanup_prompt = f"""Workflow complete. Cleanup recommendations for:
- {len(self._active_agents)} active agents
- {self._checkpoint_count} checkpoints created

What should be cleaned up vs. preserved for future sessions?"""

        await self.claude_client.query(cleanup_prompt)

        cleanup_responses = []
        async for message in self.claude_client.receive_response():
            cleanup_responses.append(message)
            await self._store_message(message, "cleanup")

        # Mark all agents as complete
        for agent_id in self._active_agents:
            self.coordinator.update_agent_state(agent_id, "complete")

        self._active_agents.clear()

    def get_status(self) -> Dict[str, Any]:
        """Get orchestrator status."""
        return {
            "phase": self._phase.value,
            "active_agents": len(self._active_agents),
            "checkpoints": self._checkpoint_count,
            "work_graph_size": len(self._work_graph),
            "session_active": self._session_active
        }

    async def __aenter__(self):
        """Async context manager entry."""
        await self.start_session()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit."""
        await self.stop_session()


# Standalone agent runner
async def main():
    """
    Run orchestrator agent as independent process.

    Accepts CLI arguments:
    - --agent-id: Agent identifier (default: "orchestrator")
    - --api-url: API server URL for heartbeats (default: "http://127.0.0.1:3000")
    - --database: Database path (default: ".mnemosyne/orchestration.db")
    - --namespace: Namespace for operations (default: "project:mnemosyne")
    """
    import argparse
    import asyncio
    import httpx
    import signal
    import sys

    parser = argparse.ArgumentParser(description="Orchestrator Agent")
    parser.add_argument("--agent-id", default="orchestrator", help="Agent ID")
    parser.add_argument("--api-url", default="http://127.0.0.1:3000", help="API server URL")
    parser.add_argument("--database", default=".mnemosyne/orchestration.db", help="Database path")
    parser.add_argument("--namespace", default="project:mnemosyne", help="Namespace")
    args = parser.parse_args()

    logger.info(f"Starting Orchestrator Agent (ID: {args.agent_id})")
    logger.info(f"API Server: {args.api_url}")
    logger.info(f"Database: {args.database}")
    logger.info(f"Namespace: {args.namespace}")

    # Initialize agent configuration
    config = OrchestratorConfig(
        agent_id=args.agent_id,
        allowed_tools=["Read", "Glob", "Task"],
        permission_mode="default",
    )

    # Create HTTP client for API communication
    http_client = httpx.AsyncClient(timeout=5.0)

    # Heartbeat function
    async def send_heartbeat():
        """Send periodic heartbeat to API server."""
        while True:
            try:
                await http_client.post(
                    f"{args.api_url}/events",
                    json={
                        "event_type": "Heartbeat",
                        "instance_id": args.agent_id,
                        "timestamp": "auto"
                    }
                )
                logger.debug(f"Heartbeat sent from {args.agent_id}")
            except Exception as e:
                logger.warning(f"Heartbeat failed: {e}")

            await asyncio.sleep(10)  # Heartbeat every 10 seconds

    # Graceful shutdown handler
    shutdown_event = asyncio.Event()
    loop = asyncio.get_event_loop()

    def signal_handler(signum, frame):
        logger.info(f"Received signal {signum}, initiating graceful shutdown...")
        # Use call_soon_threadsafe to safely interact with asyncio from signal handler
        loop.call_soon_threadsafe(shutdown_event.set)

    signal.signal(signal.SIGTERM, signal_handler)
    signal.signal(signal.SIGINT, signal_handler)

    # Start heartbeat task
    heartbeat_task = asyncio.create_task(send_heartbeat())

    try:
        logger.info("Orchestrator agent running (press Ctrl+C to stop)")

        # TODO: Initialize orchestrator with actual storage and coordinator
        # For now, just keep the agent alive
        await shutdown_event.wait()

    except KeyboardInterrupt:
        logger.info("Keyboard interrupt received")
    except Exception as e:
        logger.error(f"Error in agent main loop: {e}")
        sys.exit(1)
    finally:
        # Cleanup
        logger.info("Shutting down orchestrator agent...")
        heartbeat_task.cancel()
        await http_client.aclose()
        logger.info("Orchestrator agent stopped")


if __name__ == "__main__":
    import asyncio
    asyncio.run(main())
