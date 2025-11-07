"""
Input validation for Python agents.

Validates WorkItems and agent state before processing to prevent
invalid states and provide early error detection.
"""

from typing import Optional, List, Tuple
from dataclasses import dataclass

from .base_agent import WorkItem


@dataclass
class ValidationResult:
    """Result of validation check."""
    valid: bool
    errors: List[str]
    warnings: List[str]


def validate_work_item(work_item: WorkItem) -> ValidationResult:
    """
    Validate WorkItem fields for correctness and completeness.

    Checks:
    - Required fields present and non-empty
    - Field constraints (phase valid, priority in range)
    - Data consistency (description not too long, etc.)

    Args:
        work_item: WorkItem to validate

    Returns:
        ValidationResult with errors and warnings
    """
    errors = []
    warnings = []

    # Required field validation
    if not work_item.id:
        errors.append("WorkItem.id is required")
    elif len(work_item.id) > 256:
        errors.append(f"WorkItem.id too long ({len(work_item.id)} chars, max 256)")

    if not work_item.description:
        errors.append("WorkItem.description is required")
    elif len(work_item.description) < 10:
        warnings.append(f"WorkItem.description very short ({len(work_item.description)} chars)")
    elif len(work_item.description) > 50000:
        errors.append(f"WorkItem.description too long ({len(work_item.description)} chars, max 50000)")

    if not work_item.phase:
        errors.append("WorkItem.phase is required")
    else:
        # Validate phase is a known value
        valid_phases = {
            "planning", "implementation", "review", "testing",
            "documentation", "deployment", "optimization", "analysis"
        }
        if work_item.phase.lower() not in valid_phases:
            warnings.append(f"WorkItem.phase '{work_item.phase}' is non-standard (valid: {', '.join(sorted(valid_phases))})")

    # Priority validation
    if work_item.priority < 0:
        errors.append(f"WorkItem.priority must be >= 0 (got {work_item.priority})")
    elif work_item.priority > 10:
        warnings.append(f"WorkItem.priority very high ({work_item.priority}, typical range 0-5)")

    # Review attempt validation
    if work_item.review_attempt < 0:
        errors.append(f"WorkItem.review_attempt must be >= 0 (got {work_item.review_attempt})")
    elif work_item.review_attempt > 5:
        warnings.append(f"WorkItem.review_attempt high ({work_item.review_attempt}, may indicate repeated failures)")

    # Review feedback consistency
    if work_item.review_attempt > 0 and not work_item.review_feedback:
        warnings.append(f"WorkItem.review_attempt is {work_item.review_attempt} but review_feedback is empty")

    return ValidationResult(
        valid=len(errors) == 0,
        errors=errors,
        warnings=warnings
    )


def validate_agent_state(
    agent_id: str,
    session_active: bool,
    required_session: bool = True
) -> ValidationResult:
    """
    Validate agent state before processing work.

    Args:
        agent_id: Agent identifier
        session_active: Whether agent session is active
        required_session: Whether session must be active

    Returns:
        ValidationResult with errors and warnings
    """
    errors = []
    warnings = []

    if not agent_id:
        errors.append("Agent ID is required")

    if required_session and not session_active:
        errors.append("Agent session not active (call start_session() first)")
    elif not session_active:
        warnings.append("Agent session not active (will attempt auto-start)")

    return ValidationResult(
        valid=len(errors) == 0,
        errors=errors,
        warnings=warnings
    )


def validate_work_plan(work_plan: dict) -> Tuple[bool, List[str], List[str]]:
    """
    Validate work plan structure and completeness.

    Args:
        work_plan: Work plan dictionary

    Returns:
        Tuple of (valid, errors, warnings)
    """
    errors = []
    warnings = []

    # Check required fields
    if "description" not in work_plan and "prompt" not in work_plan:
        errors.append("Work plan missing 'description' or 'prompt' field")

    if "id" not in work_plan:
        warnings.append("Work plan missing 'id' field (will use default)")

    # Check for vague requirements
    description = work_plan.get("description") or work_plan.get("prompt", "")
    if description:
        word_count = len(description.split())
        if word_count < 5:
            errors.append(f"Description too brief ({word_count} words, minimum 5)")

        # Check for vague terms
        vague_terms = ["quickly", "just", "simple", "easy", "whatever", "somehow"]
        found_vague = [term for term in vague_terms if term in description.lower()]
        if found_vague:
            warnings.append(f"Description contains vague terms: {', '.join(found_vague)}")

    return (len(errors) == 0, errors, warnings)


def validate_review_artifact(artifact: dict) -> ValidationResult:
    """
    Validate artifact for review.

    Args:
        artifact: Artifact dictionary to review

    Returns:
        ValidationResult with errors and warnings
    """
    errors = []
    warnings = []

    if not artifact.get("id"):
        errors.append("Artifact missing 'id' field")

    if not artifact.get("description"):
        errors.append("Artifact missing 'description' field")

    if "phase" not in artifact:
        warnings.append("Artifact missing 'phase' field (may affect review)")

    return ValidationResult(
        valid=len(errors) == 0,
        errors=errors,
        warnings=warnings
    )


def validate_optimization_request(
    task_description: str,
    current_context: dict
) -> ValidationResult:
    """
    Validate optimization request.

    Args:
        task_description: Task to optimize for
        current_context: Current context state

    Returns:
        ValidationResult with errors and warnings
    """
    errors = []
    warnings = []

    if not task_description:
        errors.append("Task description is required for optimization")
    elif len(task_description) < 10:
        warnings.append(f"Task description very short ({len(task_description)} chars)")

    if not current_context:
        warnings.append("Current context is empty")
    elif "available_tokens" not in current_context:
        warnings.append("Context missing 'available_tokens' (will use default)")

    return ValidationResult(
        valid=len(errors) == 0,
        errors=errors,
        warnings=warnings
    )
