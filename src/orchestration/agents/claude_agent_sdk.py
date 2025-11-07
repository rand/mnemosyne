"""
Minimal wrapper around Anthropic Python SDK for Claude agents.

This module provides a simplified interface for mnemosyne orchestration agents
to interact with the Claude API through the official Anthropic SDK.

Future: This wrapper will be replaced with the official claude-agent-sdk
when it becomes available.
"""

import os
from dataclasses import dataclass
from typing import List, Optional, Dict, Any

try:
    import anthropic
except ImportError:
    raise ImportError(
        "anthropic package not installed. "
        "Install with: uv pip install anthropic"
    )


@dataclass
class ClaudeAgentOptions:
    """Configuration options for Claude SDK client."""

    allowed_tools: List[str]
    permission_mode: str = "auto"  # auto, manual, none
    max_tokens: int = 4096
    model: str = "claude-sonnet-4-20250514"
    temperature: float = 1.0


class ClaudeSDKClient:
    """
    Wrapper around Anthropic SDK for agent interactions.

    Provides a simplified interface for orchestration agents to use
    Claude models with conversation context and tool support.

    Example:
        ```python
        options = ClaudeAgentOptions(
            allowed_tools=["Read", "Write", "Bash"],
            permission_mode="auto"
        )
        client = ClaudeSDKClient(options=options)

        # Future: conversation-based API
        # response = await client.send_message("Hello, Claude!")
        ```
    """

    def __init__(self, options: ClaudeAgentOptions):
        """
        Initialize Claude SDK client.

        Args:
            options: Configuration options for the client

        Raises:
            RuntimeError: If ANTHROPIC_API_KEY environment variable not set
        """
        self.options = options

        # Get API key from environment
        api_key = os.getenv("ANTHROPIC_API_KEY")
        if not api_key:
            raise RuntimeError(
                "ANTHROPIC_API_KEY environment variable not set. "
                "Get your API key from: https://console.anthropic.com/settings/keys"
            )

        # Initialize Anthropic client
        self.client = anthropic.Anthropic(api_key=api_key)

        # Conversation state
        self._messages = []
        self._system_prompt = None
        self._connected = False

    async def connect(self):
        """Initialize connection (no-op for HTTP API, but tracks state)."""
        self._connected = True

    async def disconnect(self):
        """Close connection (no-op for HTTP API, but tracks state)."""
        self._connected = False

    async def query(self, prompt: str):
        """
        Send a query to Claude and store in conversation context.

        Args:
            prompt: User message or system prompt to send
        """
        if not self._connected:
            raise RuntimeError("Client not connected. Call connect() first.")

        # If this looks like a system prompt, store it
        if len(self._messages) == 0 and (
            "You are" in prompt or
            "Your role" in prompt or
            "Responsibilities" in prompt
        ):
            self._system_prompt = prompt
        else:
            # Add as user message
            self._messages.append({
                "role": "user",
                "content": prompt
            })

    async def receive_response(self):
        """
        Get response from Claude (async generator).

        Yields response messages from Claude.
        """
        if not self._messages:
            return

        # Call Claude API
        response = self.client.messages.create(
            model=self.options.model,
            max_tokens=self.options.max_tokens,
            temperature=self.options.temperature,
            system=self._system_prompt if self._system_prompt else "",
            messages=self._messages
        )

        # Add assistant response to conversation
        assistant_message = {
            "role": "assistant",
            "content": response.content[0].text if response.content else ""
        }
        self._messages.append(assistant_message)

        # Yield response
        yield assistant_message

    async def send_message(
        self,
        message: str,
        system_prompt: Optional[str] = None,
        context: Optional[Dict[str, Any]] = None
    ) -> Dict[str, Any]:
        """
        Send a message to Claude and get response.

        Args:
            message: User message to send
            system_prompt: Optional system prompt for context
            context: Optional conversation context

        Returns:
            Response dictionary with:
                - content: Claude's response text
                - stop_reason: Why the response stopped
                - usage: Token usage information

        Note:
            This is a minimal implementation. Full conversation management
            will be added in future versions.
        """
        # TODO: Implement full conversation-based API
        # For now, this is a placeholder that will be implemented in Phase 5.7
        raise NotImplementedError(
            "send_message() will be implemented in Phase 5.7 E2E validation. "
            "Current agents use direct Anthropic SDK calls."
        )

    def __repr__(self) -> str:
        return f"ClaudeSDKClient(model={self.options.model}, tools={len(self.options.allowed_tools)})"
