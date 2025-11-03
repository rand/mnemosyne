"""DSPy service for Mnemosyne agent intelligence.

This module provides the Python-side DSPy infrastructure for systematic prompt
optimization and LLM interactions. It manages:

- Agent DSPy modules (Orchestrator, Optimizer, Reviewer, Executor)
- Semantic DSPy modules (Discourse, Contradiction, Pragmatics)
- LLM provider configuration (Anthropic, OpenAI)
- Module registry and lifecycle

# Architecture

```
Rust (dspy_bridge.rs) → DSpyService → Agent Modules → DSPy → LLM
```

# Usage from Rust

```python
# Called via PyO3 from Rust
service = DSpyService()
reviewer_module = service.get_agent_module("reviewer")
result = reviewer_module(user_intent="Implement auth", ...)
```
"""

import os
import sys
import logging
from typing import Dict, List, Any, Optional
from pathlib import Path

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


class DSpyService:
    """Central service for DSPy module management.

    Manages agent and semantic DSPy modules, providing:
    - Module registry and discovery
    - LLM provider configuration
    - Module compilation and optimization
    - Hot reloading for development

    Attributes:
        agent_modules: Dictionary of agent name -> DSPy module
        semantic_module: Tier 3 semantic analysis module
        lm: Configured language model (Anthropic/OpenAI)
    """

    def __init__(self):
        """Initialize DSPy service.

        Imports DSPy, configures LLM provider, and loads all modules.
        """
        logger.info("Initializing DSpyService")

        # Import DSPy (lazy import to avoid startup overhead)
        try:
            import dspy
            self.dspy = dspy
            logger.info("DSPy imported successfully")
        except ImportError as e:
            logger.error(f"Failed to import DSPy: {e}")
            logger.error("Install with: pip install dspy-ai")
            raise

        # Configure LLM provider
        self._configure_lm()

        # Initialize module registries
        self.agent_modules: Dict[str, Any] = {}
        self.semantic_module: Optional[Any] = None
        self.memory_evolution_module: Optional[Any] = None

        # Load modules
        self._load_agent_modules()
        logger.info("DSpyService initialized successfully")

    def _configure_lm(self):
        """Configure language model from environment variables.

        Supports:
        - Anthropic (ANTHROPIC_API_KEY)
        - OpenAI (OPENAI_API_KEY)

        Defaults to Anthropic Claude 3.5 Haiku if available.
        """
        anthropic_key = os.getenv("ANTHROPIC_API_KEY")
        openai_key = os.getenv("OPENAI_API_KEY")

        if anthropic_key:
            logger.info("Configuring Anthropic Claude as LLM provider")
            self.lm = self.dspy.Claude(
                model="claude-haiku-4-5-20251001",
                api_key=anthropic_key,
                max_tokens=4096,
            )
            self.dspy.settings.configure(lm=self.lm)
        elif openai_key:
            logger.info("Configuring OpenAI GPT-4 as LLM provider")
            self.lm = self.dspy.OpenAI(
                model="gpt-4-turbo-preview",
                api_key=openai_key,
                max_tokens=4096,
            )
            self.dspy.settings.configure(lm=self.lm)
        else:
            logger.warning("No API keys found, DSPy modules will fail at runtime")
            logger.warning("Set ANTHROPIC_API_KEY or OPENAI_API_KEY environment variable")
            self.lm = None

    def _load_agent_modules(self):
        """Load all agent DSPy modules from dspy_modules package.

        Attempts to import:
        - orchestrator_module
        - optimizer_module
        - reviewer_module
        - executor_module
        - memory_evolution_module
        - semantic_module

        Missing modules are logged but don't fail initialization.
        """
        module_names = [
            "orchestrator",
            "optimizer",
            "reviewer",
            "executor",
            "memory_evolution",
            "semantic",
        ]

        for name in module_names:
            try:
                # Import module
                module_path = f"mnemosyne.orchestration.dspy_modules.{name}_module"
                logger.info(f"Attempting to import {module_path}")

                # Try to import (will fail if module doesn't exist yet)
                mod = __import__(module_path, fromlist=[f"{name.capitalize()}Module"])
                module_class = getattr(mod, f"{name.capitalize()}Module")

                # Instantiate module
                self.agent_modules[name] = module_class()
                logger.info(f"Loaded {name} module successfully")

            except ModuleNotFoundError:
                logger.warning(f"Module {name}_module.py not found yet, skipping")
            except Exception as e:
                logger.error(f"Failed to load {name} module: {e}")

    def _load_semantic_module(self):
        """Load Tier 3 semantic analysis module.

        Lazy loading - only imported when first accessed.
        """
        if self.semantic_module is not None:
            return

        try:
            from mnemosyne.orchestration.dspy_modules.semantic_module import SemanticModule
            self.semantic_module = SemanticModule()
            logger.info("Loaded semantic module successfully")
        except ModuleNotFoundError:
            logger.warning("semantic_module.py not found yet")
        except Exception as e:
            logger.error(f"Failed to load semantic module: {e}")

    def _load_memory_evolution_module(self):
        """Load memory evolution module for consolidation/archival/recalibration.

        Lazy loading - only imported when first accessed.
        """
        if self.memory_evolution_module is not None:
            return

        try:
            from mnemosyne.orchestration.dspy_modules.memory_evolution_module import (
                MemoryEvolutionModule,
            )

            self.memory_evolution_module = MemoryEvolutionModule()
            logger.info("Loaded memory evolution module successfully")
        except ModuleNotFoundError:
            logger.warning("memory_evolution_module.py not found yet")
        except Exception as e:
            logger.error(f"Failed to load memory evolution module: {e}")

    def get_agent_module(self, agent_name: str) -> Any:
        """Get DSPy module for specified agent.

        Args:
            agent_name: Name of agent ("orchestrator", "optimizer", "reviewer", "executor")

        Returns:
            DSPy module instance for the agent

        Raises:
            KeyError: If agent module not found
        """
        if agent_name not in self.agent_modules:
            available = ", ".join(self.agent_modules.keys())
            raise KeyError(
                f"Agent module '{agent_name}' not found. "
                f"Available modules: {available}"
            )

        return self.agent_modules[agent_name]

    def get_semantic_module(self) -> Any:
        """Get Tier 3 semantic analysis module.

        Lazy loads module on first access.

        Returns:
            SemanticModule instance

        Raises:
            RuntimeError: If semantic module failed to load
        """
        if self.semantic_module is None:
            self._load_semantic_module()

        if self.semantic_module is None:
            raise RuntimeError("Semantic module failed to load")

        return self.semantic_module

    def get_memory_evolution_module(self) -> Any:
        """Get memory evolution module.

        Lazy loads module on first access.

        Returns:
            MemoryEvolutionModule instance

        Raises:
            RuntimeError: If memory evolution module failed to load
        """
        if self.memory_evolution_module is None:
            self._load_memory_evolution_module()

        if self.memory_evolution_module is None:
            raise RuntimeError("Memory evolution module failed to load")

        return self.memory_evolution_module

    def list_modules(self) -> List[str]:
        """List all loaded agent module names.

        Returns:
            List of agent names with loaded modules
        """
        return list(self.agent_modules.keys())

    def reload_modules(self):
        """Reload all DSPy modules (for development).

        Forces reimport of all Python modules to pick up code changes
        without restarting the Rust process.
        """
        logger.info("Reloading all DSPy modules")

        # Clear existing modules
        self.agent_modules.clear()
        self.semantic_module = None
        self.memory_evolution_module = None

        # Reload agent modules
        self._load_agent_modules()

        logger.info("All modules reloaded")


# Singleton instance (created when imported from Rust)
_service_instance: Optional[DSpyService] = None


def get_service() -> DSpyService:
    """Get or create singleton DSpyService instance.

    Returns:
        Global DSpyService instance
    """
    global _service_instance
    if _service_instance is None:
        _service_instance = DSpyService()
    return _service_instance
