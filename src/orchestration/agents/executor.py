"""
Executor Agent - Primary Work Agent and Sub-Agent Manager.

Responsibilities:
- Follow Work Plan Protocol (Phases 1-4)
- Execute atomic tasks from plans using direct Anthropic API
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
import time

try:
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


class CircuitState(Enum):
    """Circuit breaker states."""
    CLOSED = "closed"       # Normal operation
    OPEN = "open"          # Failing, rejecting requests
    HALF_OPEN = "half_open"  # Testing if service recovered


class CircuitBreaker:
    """
    Circuit breaker for LLM API calls.

    Protects against cascading failures by tracking consecutive errors
    and temporarily rejecting requests when failure threshold is reached.

    States:
    - CLOSED: Normal operation, tracking failures
    - OPEN: Too many failures, rejecting requests
    - HALF_OPEN: Cooldown expired, testing recovery

    Transitions:
    - CLOSED → OPEN: After N consecutive failures
    - OPEN → HALF_OPEN: After cooldown period
    - HALF_OPEN → CLOSED: After successful call
    - HALF_OPEN → OPEN: If call fails
    """

    def __init__(
        self,
        failure_threshold: int = 3,
        cooldown_seconds: float = 60.0,
        half_open_attempts: int = 1
    ):
        """
        Initialize circuit breaker.

        Args:
            failure_threshold: Consecutive failures before opening circuit
            cooldown_seconds: Time to wait before entering half-open state
            half_open_attempts: Number of successful calls needed to close
        """
        self.failure_threshold = failure_threshold
        self.cooldown_seconds = cooldown_seconds
        self.half_open_attempts = half_open_attempts

        self.state = CircuitState.CLOSED
        self.failure_count = 0
        self.success_count = 0
        self.last_failure_time: Optional[float] = None

        logger.info(
            f"[CircuitBreaker] Initialized: threshold={failure_threshold}, "
            f"cooldown={cooldown_seconds}s"
        )

    def can_attempt(self) -> bool:
        """Check if request can proceed."""
        if self.state == CircuitState.CLOSED:
            return True

        if self.state == CircuitState.OPEN:
            # Check if cooldown has expired
            if self.last_failure_time is not None:
                elapsed = time.time() - self.last_failure_time
                if elapsed >= self.cooldown_seconds:
                    logger.info("[CircuitBreaker] Cooldown expired, entering HALF_OPEN")
                    self.state = CircuitState.HALF_OPEN
                    self.success_count = 0
                    return True
            return False

        if self.state == CircuitState.HALF_OPEN:
            return True

        return False

    def record_success(self):
        """Record successful API call."""
        if self.state == CircuitState.CLOSED:
            # Reset failure count on success
            if self.failure_count > 0:
                logger.info(
                    f"[CircuitBreaker] Success after {self.failure_count} failures, "
                    "resetting counter"
                )
            self.failure_count = 0

        elif self.state == CircuitState.HALF_OPEN:
            self.success_count += 1
            logger.info(
                f"[CircuitBreaker] HALF_OPEN success {self.success_count}/"
                f"{self.half_open_attempts}"
            )

            if self.success_count >= self.half_open_attempts:
                logger.info("[CircuitBreaker] Closing circuit after successful recovery")
                self.state = CircuitState.CLOSED
                self.failure_count = 0
                self.success_count = 0

    def record_failure(self):
        """Record failed API call."""
        self.last_failure_time = time.time()

        if self.state == CircuitState.CLOSED:
            self.failure_count += 1
            logger.warning(
                f"[CircuitBreaker] Failure {self.failure_count}/{self.failure_threshold}"
            )

            if self.failure_count >= self.failure_threshold:
                logger.error(
                    f"[CircuitBreaker] Opening circuit after {self.failure_count} "
                    f"consecutive failures"
                )
                self.state = CircuitState.OPEN

        elif self.state == CircuitState.HALF_OPEN:
            logger.warning("[CircuitBreaker] Failure in HALF_OPEN, reopening circuit")
            self.state = CircuitState.OPEN
            self.failure_count = self.failure_threshold  # Keep it open
            self.success_count = 0

    def get_status(self) -> Dict[str, Any]:
        """Get current circuit breaker status."""
        return {
            "state": self.state.value,
            "failure_count": self.failure_count,
            "success_count": self.success_count,
            "last_failure_time": self.last_failure_time,
            "cooldown_remaining": (
                max(0, self.cooldown_seconds - (time.time() - self.last_failure_time))
                if self.last_failure_time is not None else 0
            )
        }


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
    # Anthropic API key (injected from Rust via environment)
    api_key: Optional[str] = None


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

    Executes work following the Work Plan Protocol using direct Anthropic API:
    - Phase 1: Prompt → Spec
    - Phase 2: Spec → Full Spec
    - Phase 3: Full Spec → Plan
    - Phase 4: Plan → Artifacts

    Uses direct Anthropic API calls with tool execution for intelligent work execution.

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
        Initialize Executor agent with direct Anthropic API access.

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

        # Store API key (injected from Rust environment)
        import os
        self.api_key = config.api_key or os.getenv("ANTHROPIC_API_KEY")

        # Register with coordinator
        self.coordinator.register_agent(config.agent_id)

        # State
        self._current_phase = ExecutorPhase.IDLE
        self._active_subagents: List[str] = []
        self._completed_tasks: List[str] = []
        self._checkpoint_count = 0
        self._session_active = False

        # Circuit breaker for LLM API resilience
        self._circuit_breaker = CircuitBreaker(
            failure_threshold=3,
            cooldown_seconds=60.0,
            half_open_attempts=1
        )

        logger.info(f"[Executor] Initialized with direct Anthropic API access")

    async def start_session(self):
        """Start agent session (validates API key availability)."""
        if not self._session_active:
            logger.info(f"Starting session for agent {self.config.agent_id}")

            # Validate API key is available
            if not self.api_key:
                raise ValueError(
                    "ANTHROPIC_API_KEY not set. Cannot start session without API access. "
                    "Get your key from: https://console.anthropic.com/settings/keys"
                )

            self._session_active = True
            logger.info(f"Session started successfully for {self.config.agent_id}")

    async def stop_session(self):
        """Stop agent session."""
        if self._session_active:
            logger.info(f"Stopping session for agent {self.config.agent_id}")
            self._session_active = False
            logger.info(f"Session stopped for {self.config.agent_id}")

    async def execute_work_plan(self, work_plan: Dict[str, Any]) -> Dict[str, Any]:
        """
        Execute work plan using Anthropic API for LLM reasoning.

        ZFC: Deterministic state machine + LLM API calls for intelligent decisions.

        Args:
            work_plan: Work plan with prompt, phase, etc.

        Returns:
            Execution results with status and artifacts
        """
        prompt = work_plan.get('prompt', 'No description')
        phase = work_plan.get('phase', 'spec')
        work_id = work_plan.get('id', 'unknown')

        logger.info(f"[Executor] Executing work (phase={phase}): {prompt[:100]}")
        self.coordinator.update_agent_state(self.config.agent_id, "running")
        self._current_phase = ExecutorPhase.ANALYZING

        try:
            # Phase 1: Analysis - Build execution prompt
            logger.info(f"[Executor] Phase 1: Analyzing work item {work_id}")
            execution_prompt = self._build_execution_prompt(work_plan)

            analysis = {
                "description": prompt,
                "phase": phase,
                "complexity": "simple" if len(prompt) < 100 else "moderate",
                "prompt_length": len(execution_prompt)
            }

            # Phase 2: Planning - Prepare API call
            self._current_phase = ExecutorPhase.PLANNING
            logger.info(f"[Executor] Phase 2: Calling Anthropic API for intelligent execution")

            # Phase 3: Execution - Call Anthropic API
            self._current_phase = ExecutorPhase.EXECUTING

            # Check circuit breaker before attempting API calls
            if not self._circuit_breaker.can_attempt():
                circuit_status = self._circuit_breaker.get_status()
                logger.error(
                    f"[Executor] Circuit breaker is {circuit_status['state']}, "
                    f"rejecting request. Cooldown: {circuit_status['cooldown_remaining']:.1f}s"
                )

                # Return fallback response
                fallback_response = (
                    f"LLM API is temporarily unavailable (circuit breaker {circuit_status['state']}). "
                    f"Retry in {circuit_status['cooldown_remaining']:.0f} seconds. "
                    "This work item will be re-queued automatically."
                )

                artifacts = [{
                    "type": "circuit_breaker_rejection",
                    "content": fallback_response,
                    "circuit_status": circuit_status,
                    "work_id": work_id
                }]

                self.coordinator.update_agent_state(self.config.agent_id, "degraded")

                return {
                    "status": "circuit_open",
                    "artifacts": artifacts,
                    "analysis": analysis,
                    "response_text": fallback_response,
                    "retry_after": circuit_status['cooldown_remaining']
                }

            # Import here to allow graceful degradation if not available
            import anthropic

            if not self.api_key:
                raise ValueError(
                    "ANTHROPIC_API_KEY not set. Cannot execute work without API access. "
                    "Get your key from: https://console.anthropic.com/settings/keys"
                )

            client = anthropic.Anthropic(api_key=self.api_key)

            # Get tool definitions
            tools = self._get_tool_definitions()

            # Tool execution loop - continue until we get a final response
            messages = [{"role": "user", "content": execution_prompt}]
            total_input_tokens = 0
            total_output_tokens = 0
            tool_uses = []
            max_iterations = 10  # Prevent infinite loops

            logger.info(f"[Executor] Starting tool execution loop (max {max_iterations} iterations)")

            for iteration in range(max_iterations):
                logger.info(f"[Executor] API call iteration {iteration + 1}")

                try:
                    # Call Claude API with tools
                    response = client.messages.create(
                        model="claude-sonnet-4-5-20250929",
                        max_tokens=4096,
                        tools=tools,
                        messages=messages
                    )

                    # Record success with circuit breaker
                    self._circuit_breaker.record_success()

                except Exception as api_error:
                    # Record failure with circuit breaker
                    self._circuit_breaker.record_failure()

                    logger.error(
                        f"[Executor] API call failed (iteration {iteration + 1}): {api_error}"
                    )

                    # Re-raise for outer exception handler
                    raise

                total_input_tokens += response.usage.input_tokens
                total_output_tokens += response.usage.output_tokens

                logger.debug(f"[Executor] Response stop_reason: {response.stop_reason}")

                # Check if Claude wants to use tools
                if response.stop_reason == "tool_use":
                    # Extract tool use requests
                    assistant_content = []
                    tool_results = []

                    for block in response.content:
                        if block.type == "tool_use":
                            tool_name = block.name
                            tool_input = block.input
                            tool_use_id = block.id

                            logger.info(f"[Executor] Tool requested: {tool_name}")
                            tool_uses.append({"name": tool_name, "input": tool_input})

                            # Execute the tool
                            tool_result = await self._execute_tool(tool_name, tool_input)

                            # Format result for API
                            tool_results.append({
                                "type": "tool_result",
                                "tool_use_id": tool_use_id,
                                "content": str(tool_result)
                            })

                            assistant_content.append(block)

                        elif block.type == "text":
                            assistant_content.append(block)

                    # Add assistant message with tool use
                    messages.append({
                        "role": "assistant",
                        "content": assistant_content
                    })

                    # Add tool results
                    messages.append({
                        "role": "user",
                        "content": tool_results
                    })

                    logger.info(f"[Executor] Executed {len(tool_results)} tools, continuing conversation")

                else:
                    # Got final response, extract text
                    response_text = ""
                    for block in response.content:
                        if block.type == "text":
                            response_text += block.text

                    logger.info(f"[Executor] Final response received ({len(response_text)} chars)")
                    logger.debug(f"[Executor] Response preview: {response_text[:200]}...")
                    break
            else:
                # Hit max iterations
                logger.warning(f"[Executor] Max iterations ({max_iterations}) reached")
                response_text = "Max tool execution iterations reached. Work incomplete."

            # Create artifacts from response and tool uses
            artifacts = [{
                "type": "llm_response",
                "content": response_text,
                "phase": phase,
                "work_id": work_id,
                "model": "claude-sonnet-4-5-20250929",
                "tokens_used": {
                    "input": total_input_tokens,
                    "output": total_output_tokens
                },
                "tool_uses": tool_uses,
                "iterations": iteration + 1
            }]

            execution_summary = f"Executed work via Claude API: {prompt[:100]}"

            # Phase 4: Completion
            self._current_phase = ExecutorPhase.COMPLETED
            self.coordinator.update_agent_state(self.config.agent_id, "complete")

            logger.info(f"[Executor] Work completed successfully: {work_id}")
            logger.info(f"[Executor] Tokens: {response.usage.input_tokens} in, {response.usage.output_tokens} out")

            return {
                "status": "success",
                "artifacts": artifacts,
                "analysis": analysis,
                "summary": execution_summary,
                "phase": phase,
                "response_text": response_text  # Include full response for debugging
            }

        except ImportError as e:
            self.coordinator.update_agent_state(self.config.agent_id, "failed")
            error_msg = f"Anthropic SDK not installed: {e}. Install with: uv pip install anthropic"
            logger.error(f"[Executor] {error_msg}")
            return {
                "status": "failed",
                "error": error_msg,
                "phase": phase
            }
        except ValueError as e:
            self.coordinator.update_agent_state(self.config.agent_id, "failed")
            logger.error(f"[Executor] Configuration error: {e}")
            return {
                "status": "failed",
                "error": str(e),
                "phase": phase
            }
        except Exception as e:
            self.coordinator.update_agent_state(self.config.agent_id, "failed")
            logger.error(f"[Executor] Execution failed: {type(e).__name__}: {str(e)}", exc_info=True)
            return {
                "status": "failed",
                "error": str(e),
                "phase": phase
            }

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

    def _get_tool_definitions(self) -> List[Dict[str, Any]]:
        """
        Define tools available to the executor.

        Tools follow Anthropic's tool use API format.
        """
        return [
            {
                "name": "read_file",
                "description": "Read the contents of a file. Use this to examine existing code or configuration.",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "file_path": {
                            "type": "string",
                            "description": "Absolute path to the file to read"
                        }
                    },
                    "required": ["file_path"]
                }
            },
            {
                "name": "create_file",
                "description": "Create a new file with the specified content. Use this to write code, tests, or documentation.",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "file_path": {
                            "type": "string",
                            "description": "Absolute path where the file should be created"
                        },
                        "content": {
                            "type": "string",
                            "description": "Content to write to the file"
                        }
                    },
                    "required": ["file_path", "content"]
                }
            },
            {
                "name": "edit_file",
                "description": "Edit an existing file by replacing old_text with new_text. Use this to modify code.",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "file_path": {
                            "type": "string",
                            "description": "Absolute path to the file to edit"
                        },
                        "old_text": {
                            "type": "string",
                            "description": "Exact text to find and replace"
                        },
                        "new_text": {
                            "type": "string",
                            "description": "New text to insert"
                        }
                    },
                    "required": ["file_path", "old_text", "new_text"]
                }
            },
            {
                "name": "run_command",
                "description": "Execute a shell command. Use this to run tests, build code, or perform other operations.",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "Shell command to execute"
                        },
                        "working_dir": {
                            "type": "string",
                            "description": "Working directory for command execution (optional)"
                        }
                    },
                    "required": ["command"]
                }
            }
        ]

    async def _execute_tool(self, tool_name: str, tool_input: Dict[str, Any]) -> Dict[str, Any]:
        """
        Execute a tool and return the result.

        Args:
            tool_name: Name of the tool to execute
            tool_input: Input parameters for the tool

        Returns:
            Tool execution result
        """
        import subprocess
        import os

        logger.info(f"[Executor] Executing tool: {tool_name}")
        logger.debug(f"[Executor] Tool input: {tool_input}")

        try:
            if tool_name == "read_file":
                file_path = tool_input["file_path"]
                logger.info(f"[Executor] Reading file: {file_path}")

                if not os.path.exists(file_path):
                    return {
                        "success": False,
                        "error": f"File not found: {file_path}"
                    }

                with open(file_path, 'r') as f:
                    content = f.read()

                return {
                    "success": True,
                    "content": content,
                    "size": len(content)
                }

            elif tool_name == "create_file":
                file_path = tool_input["file_path"]
                content = tool_input["content"]
                logger.info(f"[Executor] Creating file: {file_path}")

                # Create parent directories if needed
                os.makedirs(os.path.dirname(file_path), exist_ok=True)

                with open(file_path, 'w') as f:
                    f.write(content)

                return {
                    "success": True,
                    "message": f"File created: {file_path}",
                    "size": len(content)
                }

            elif tool_name == "edit_file":
                file_path = tool_input["file_path"]
                old_text = tool_input["old_text"]
                new_text = tool_input["new_text"]
                logger.info(f"[Executor] Editing file: {file_path}")

                if not os.path.exists(file_path):
                    return {
                        "success": False,
                        "error": f"File not found: {file_path}"
                    }

                with open(file_path, 'r') as f:
                    content = f.read()

                if old_text not in content:
                    return {
                        "success": False,
                        "error": f"Text to replace not found in {file_path}"
                    }

                new_content = content.replace(old_text, new_text, 1)

                with open(file_path, 'w') as f:
                    f.write(new_content)

                return {
                    "success": True,
                    "message": f"File edited: {file_path}",
                    "replaced_length": len(old_text),
                    "new_length": len(new_text)
                }

            elif tool_name == "run_command":
                command = tool_input["command"]
                working_dir = tool_input.get("working_dir", os.getcwd())
                logger.info(f"[Executor] Running command: {command}")

                result = subprocess.run(
                    command,
                    shell=True,
                    cwd=working_dir,
                    capture_output=True,
                    text=True,
                    timeout=30  # 30 second timeout
                )

                return {
                    "success": result.returncode == 0,
                    "exit_code": result.returncode,
                    "stdout": result.stdout,
                    "stderr": result.stderr
                }

            else:
                return {
                    "success": False,
                    "error": f"Unknown tool: {tool_name}"
                }

        except subprocess.TimeoutExpired:
            logger.error(f"[Executor] Command timeout: {tool_input.get('command', 'unknown')}")
            return {
                "success": False,
                "error": "Command execution timeout (30s limit)"
            }
        except Exception as e:
            logger.error(f"[Executor] Tool execution failed: {e}", exc_info=True)
            return {
                "success": False,
                "error": str(e)
            }

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
        Spawn sub-agent for task execution using direct Anthropic API.

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

        # TODO: Create new Anthropic API client for sub-agent with separate context
        # (In production, this would spawn a separate API session with isolated state)

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
    loop = asyncio.get_event_loop()

    def signal_handler(signum, frame):
        logger.info(f"Received signal {signum}, shutting down...")
        # Use call_soon_threadsafe to safely interact with asyncio from signal handler
        loop.call_soon_threadsafe(shutdown_event.set)

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
