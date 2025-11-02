"""DSPy modules for Mnemosyne agents and semantic analysis.

This package contains DSPy modules that implement systematic prompt optimization
for agent intelligence and Tier 3 semantic highlighting.

# Agent Modules

- **OrchestratorModule**: Work queue management, dependency tracking
- **OptimizerModule**: Context optimization, skill discovery
- **ReviewerModule**: Quality gates, requirement validation
- **ExecutorModule**: Task execution, artifact generation

# Semantic Modules

- **SemanticModule**: Tier 3 analytical highlighting
  - Discourse analysis
  - Contradiction detection
  - Pragmatics extraction

# Usage

```python
from mnemosyne.orchestration.dspy_modules.reviewer_module import ReviewerModule

reviewer = ReviewerModule()
result = reviewer(
    user_intent="Implement authentication",
    work_item="auth.py changes",
    implementation="Added JWT support"
)
```
"""

__all__ = [
    # Agent modules (will be added as we implement them)
    # "OrchestratorModule",
    # "OptimizerModule",
    # "ReviewerModule",
    # "ExecutorModule",
    # Semantic module
    # "SemanticModule",
]
