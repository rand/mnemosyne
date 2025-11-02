"""OptimizerModule - DSPy signatures for Optimizer agent operations.

This module provides DSPy-based implementations for:
- Context consolidation (progressive summarization based on review attempts)
- Skills discovery (intelligent skill matching for tasks)
- Context budget optimization (resource allocation)

All operations use ChainOfThought for transparency and optimizability.
"""

import dspy
from typing import List, Dict, Any


# =============================================================================
# Signatures
# =============================================================================


class ConsolidateContext(dspy.Signature):
    """Consolidate work item context based on review feedback.

    Applies progressive consolidation strategy:
    - detailed: Preserve all feedback and context (attempt 1)
    - summary: Key issues and patterns (attempts 2-3)
    - compressed: Critical blockers only (attempt 4+)
    """

    original_intent = dspy.InputField(desc="User's original work intent")
    execution_summaries = dspy.InputField(
        desc="List of execution memory summaries from previous attempts"
    )
    review_feedback = dspy.InputField(desc="List of quality gate issues from review")
    suggested_tests = dspy.InputField(
        desc="List of test improvements suggested by reviewer"
    )
    review_attempt = dspy.InputField(
        desc="Review attempt number (1=first review, 2=second, etc.)"
    )
    consolidation_mode = dspy.InputField(
        desc="Consolidation mode: detailed|summary|compressed"
    )

    consolidated_content = dspy.OutputField(
        desc="Consolidated context content formatted as markdown with sections"
    )
    key_issues = dspy.OutputField(
        desc="List of most critical issues to address, prioritized by impact"
    )
    strategic_guidance = dspy.OutputField(
        desc="Strategic recommendations for resolving issues systematically"
    )
    estimated_tokens = dspy.OutputField(
        desc="Estimated token count for consolidated content"
    )


class DiscoverSkills(dspy.Signature):
    """Discover relevant skills for a task description.

    Analyzes task semantically to find best skill matches beyond simple
    keyword matching. Considers current context budget to recommend
    optimal skill count.
    """

    task_description = dspy.InputField(desc="Description of task to perform")
    available_skills = dspy.InputField(
        desc="List of skill metadata: name, description, keywords, domains"
    )
    max_skills = dspy.InputField(desc="Maximum number of skills to return")
    current_context_usage = dspy.InputField(
        desc="Current context usage percentage (0.0-1.0)"
    )

    selected_skills = dspy.OutputField(
        desc="List of skill names to load, ordered by relevance"
    )
    relevance_scores = dspy.OutputField(
        desc="Relevance score (0.0-1.0) for each selected skill"
    )
    reasoning = dspy.OutputField(
        desc="Explanation of why these skills were selected and how they complement each other"
    )


class OptimizeContextBudget(dspy.Signature):
    """Optimize context allocation across different resource types.

    Analyzes current context usage and makes intelligent decisions about
    what to unload to reach target percentage while preserving critical
    resources for current work item.
    """

    current_usage = dspy.InputField(
        desc="Current context usage: {critical_pct, skills_pct, project_pct, general_pct, total_pct}"
    )
    loaded_resources = dspy.InputField(
        desc="Currently loaded: {skill_names: List[str], memory_ids: List[str], memory_summaries: List[str]}"
    )
    target_pct = dspy.InputField(
        desc="Target context usage percentage to achieve (0.0-1.0)"
    )
    work_priority = dspy.InputField(
        desc="Current work item priority (0-10, higher = more critical)"
    )

    unload_skills = dspy.OutputField(
        desc="Skill names to unload, ordered by removal priority (least critical first)"
    )
    unload_memory_ids = dspy.OutputField(
        desc="Memory IDs to unload, ordered by removal priority"
    )
    optimization_rationale = dspy.OutputField(
        desc="Explanation of optimization decisions and what to preserve vs unload"
    )


# =============================================================================
# Module
# =============================================================================


class OptimizerModule(dspy.Module):
    """DSPy module for Optimizer agent operations.

    Provides three core capabilities:
    1. consolidate_context: Progressive context consolidation
    2. discover_skills_for_task: Intelligent skill discovery
    3. optimize_context_allocation: Context budget optimization

    All methods use ChainOfThought for reasoning transparency.
    """

    def __init__(self):
        """Initialize OptimizerModule with ChainOfThought for all signatures."""
        super().__init__()
        self.consolidate = dspy.ChainOfThought(ConsolidateContext)
        self.discover_skills = dspy.ChainOfThought(DiscoverSkills)
        self.optimize_budget = dspy.ChainOfThought(OptimizeContextBudget)

    def consolidate_context(
        self,
        original_intent: str,
        execution_summaries: List[str],
        review_feedback: List[str],
        suggested_tests: List[str],
        review_attempt: int,
        consolidation_mode: str,
    ) -> dspy.Prediction:
        """Consolidate work item context based on review feedback.

        Args:
            original_intent: User's original work request
            execution_summaries: List of execution memory summaries
            review_feedback: List of review issues
            suggested_tests: List of suggested test improvements
            review_attempt: Review attempt number (1-N)
            consolidation_mode: "detailed"|"summary"|"compressed"

        Returns:
            Prediction with:
            - consolidated_content: Markdown-formatted consolidated context
            - key_issues: List of critical issues to address
            - strategic_guidance: Recommendations for systematic fixes
            - estimated_tokens: Token count estimate
        """
        result = self.consolidate(
            original_intent=original_intent,
            execution_summaries=execution_summaries,
            review_feedback=review_feedback,
            suggested_tests=suggested_tests,
            review_attempt=str(review_attempt),
            consolidation_mode=consolidation_mode,
        )
        return result

    def discover_skills_for_task(
        self,
        task_description: str,
        available_skills: List[Dict[str, Any]],
        max_skills: int,
        current_context_usage: float,
    ) -> dspy.Prediction:
        """Discover relevant skills for a task.

        Args:
            task_description: Description of task to perform
            available_skills: List of skill metadata dicts
            max_skills: Maximum number of skills to return
            current_context_usage: Current context usage (0.0-1.0)

        Returns:
            Prediction with:
            - selected_skills: List of skill names
            - relevance_scores: Scores for each skill
            - reasoning: Explanation of selections
        """
        # Format skills for prompt
        skills_formatted = str(available_skills)

        result = self.discover_skills(
            task_description=task_description,
            available_skills=skills_formatted,
            max_skills=str(max_skills),
            current_context_usage=str(current_context_usage),
        )
        return result

    def optimize_context_allocation(
        self,
        current_usage: Dict[str, float],
        loaded_resources: Dict[str, List[str]],
        target_pct: float,
        work_priority: int,
    ) -> dspy.Prediction:
        """Optimize context allocation to reach target percentage.

        Args:
            current_usage: Dict with usage percentages by category
            loaded_resources: Dict with skill_names and memory_ids/summaries
            target_pct: Target context usage (0.0-1.0)
            work_priority: Work item priority (0-10)

        Returns:
            Prediction with:
            - unload_skills: Skills to unload
            - unload_memory_ids: Memory IDs to unload
            - optimization_rationale: Explanation of decisions
        """
        # Format inputs for prompt
        usage_formatted = str(current_usage)
        resources_formatted = str(loaded_resources)

        result = self.optimize_budget(
            current_usage=usage_formatted,
            loaded_resources=resources_formatted,
            target_pct=str(target_pct),
            work_priority=str(work_priority),
        )
        return result


# =============================================================================
# Standalone testing (if run directly)
# =============================================================================

if __name__ == "__main__":
    # This would require actual DSPy setup with LM
    print("OptimizerModule loaded successfully")
    print("Signatures:")
    print("  - ConsolidateContext")
    print("  - DiscoverSkills")
    print("  - OptimizeContextBudget")
