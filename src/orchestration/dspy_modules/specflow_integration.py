#!/usr/bin/env python3
"""Integration between DSPy ReviewerModule and SpecFlow slash commands.

This module provides validation and analysis capabilities for feature specs
using the ReviewerModule's requirement extraction and validation signatures.

# Use Cases

1. **Spec Validation**: After creating a spec with /feature-specify, validate
   that all requirements are captured and clear.

2. **Ambiguity Detection**: Automatically detect vague terms, missing metrics,
   and incomplete acceptance criteria using LLM intelligence.

3. **Completeness Check**: Verify that scenarios have sufficient acceptance
   criteria and that requirements are testable.

4. **Quality Guidance**: Generate actionable suggestions for improving specs.

# Usage from Slash Commands

## From /feature-specify

```python
from specflow_integration import validate_feature_spec

# After creating spec
result = validate_feature_spec(
    spec_path=".mnemosyne/artifacts/specs/jwt-auth.md"
)

if not result["is_valid"]:
    print("⚠️  Spec validation found issues:")
    for issue in result["issues"]:
        print(f"  - {issue}")
    print("\nSuggestions:")
    for suggestion in result["suggestions"]:
        print(f"  - {suggestion}")
```

## From /feature-clarify

```python
from specflow_integration import detect_ambiguities

# Auto-detect ambiguities
ambiguities = detect_ambiguities(
    spec_path=".mnemosyne/artifacts/specs/jwt-auth.md"
)

for amb in ambiguities:
    print(f"Question: {amb['question']}")
    print(f"Impact: {amb['impact']}")
    print(f"Location: {amb['location']}")
```

# Architecture

- **Input**: Feature spec markdown files (with YAML frontmatter)
- **Processing**: Parse spec → Extract requirements → Validate completeness
- **Output**: Structured validation results with issues and suggestions
"""

import re
import logging
from pathlib import Path
from typing import Dict, List, Any, Optional
from dataclasses import dataclass

try:
    import dspy
    from reviewer_module import ReviewerModule
    DSPY_AVAILABLE = True
except ImportError:
    DSPY_AVAILABLE = False
    logging.warning("DSPy not available. Spec validation will use pattern matching only.")

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


@dataclass
class SpecValidationResult:
    """Result of feature spec validation."""
    is_valid: bool
    issues: List[str]
    suggestions: List[str]
    requirements: List[str]
    ambiguities: List[Dict[str, str]]
    completeness_score: float  # 0.0-1.0


@dataclass
class Ambiguity:
    """Detected ambiguity in feature spec."""
    location: str  # Section where found
    term: str  # Ambiguous term or phrase
    question: str  # Clarifying question
    impact: str  # Why this matters
    confidence: float  # 0.0-1.0


# =============================================================================
# Spec Parsing
# =============================================================================

def parse_feature_spec(spec_path: Path) -> Dict[str, Any]:
    """Parse feature spec markdown file.

    Args:
        spec_path: Path to feature spec markdown file

    Returns:
        Dictionary with:
        - frontmatter: YAML frontmatter as dict
        - overview: Overview section text
        - scenarios: List of user scenarios
        - requirements: Requirements section text
        - success_criteria: Success criteria section text
        - full_text: Complete spec text
    """
    if not spec_path.exists():
        raise FileNotFoundError(f"Spec not found: {spec_path}")

    content = spec_path.read_text()

    # Extract YAML frontmatter
    frontmatter = {}
    if content.startswith("---"):
        parts = content.split("---", 2)
        if len(parts) >= 3:
            import yaml
            try:
                frontmatter = yaml.safe_load(parts[1])
            except Exception as e:
                logger.warning(f"Failed to parse frontmatter: {e}")

    # Extract sections
    sections = {
        "frontmatter": frontmatter,
        "full_text": content,
    }

    # Extract overview
    overview_match = re.search(r"## Overview\s+(.*?)(?=\n## |\Z)", content, re.DOTALL)
    if overview_match:
        sections["overview"] = overview_match.group(1).strip()

    # Extract scenarios
    scenarios = []
    scenario_pattern = r"### (P[0-3]):\s+(.*?)\s+\*\*As a\*\*\s+(.*?)\s+\*\*I want\*\*\s+(.*?)\s+\*\*So that\*\*\s+(.*?)\s+\*\*Acceptance Criteria\*\*:\s+(.*?)(?=\n### |\n## |\Z)"
    for match in re.finditer(scenario_pattern, content, re.DOTALL):
        scenarios.append({
            "priority": match.group(1),
            "name": match.group(2).strip(),
            "actor": match.group(3).strip(),
            "goal": match.group(4).strip(),
            "benefit": match.group(5).strip(),
            "acceptance_criteria": [
                line.strip()[6:].strip()  # Remove "- [ ]"
                for line in match.group(6).strip().split("\n")
                if line.strip().startswith("- [ ]")
            ],
        })
    sections["scenarios"] = scenarios

    # Extract requirements
    req_match = re.search(r"## Requirements\s+(.*?)(?=\n## |\Z)", content, re.DOTALL)
    if req_match:
        sections["requirements"] = req_match.group(1).strip()

    # Extract success criteria
    success_match = re.search(r"## Success Criteria\s+(.*?)(?=\n## |\Z)", content, re.DOTALL)
    if success_match:
        sections["success_criteria"] = success_match.group(1).strip()

    return sections


# =============================================================================
# Pattern-Based Validation (Fallback)
# =============================================================================

VAGUE_TERMS = [
    "fast", "slow", "quick", "responsive",
    "easy", "simple", "intuitive",
    "secure", "safe",
    "scalable", "performant",
    "reliable", "stable",
    "user-friendly", "clean",
    "efficient", "optimized",
]


def detect_vague_terms(text: str) -> List[str]:
    """Detect vague terms without quantitative metrics.

    Args:
        text: Text to analyze

    Returns:
        List of detected vague terms
    """
    found_terms = []
    for term in VAGUE_TERMS:
        # Look for term not followed by numbers/metrics
        pattern = rf"\b{term}\b(?!\s*[:(<\[]?\s*\d)"
        if re.search(pattern, text, re.IGNORECASE):
            found_terms.append(term)
    return list(set(found_terms))


def check_scenario_completeness(scenarios: List[Dict[str, Any]]) -> List[str]:
    """Check if scenarios have sufficient acceptance criteria.

    Args:
        scenarios: List of parsed scenarios

    Returns:
        List of issues found
    """
    issues = []

    for scenario in scenarios:
        priority = scenario.get("priority", "")
        name = scenario.get("name", "Unknown")
        criteria = scenario.get("acceptance_criteria", [])

        # P0/P1 scenarios should have at least 3 criteria
        if priority in ["P0", "P1"] and len(criteria) < 3:
            issues.append(
                f"Scenario '{name}' ({priority}) has only {len(criteria)} "
                f"acceptance criteria. Recommended: 3-7 for critical scenarios."
            )

        # Check for vague criteria
        for criterion in criteria:
            vague = detect_vague_terms(criterion)
            if vague:
                issues.append(
                    f"Scenario '{name}': Criterion '{criterion[:50]}...' "
                    f"contains vague terms: {', '.join(vague)}"
                )

    return issues


def pattern_based_validation(spec: Dict[str, Any]) -> SpecValidationResult:
    """Validate spec using pattern matching (fallback when DSPy unavailable).

    Args:
        spec: Parsed feature spec

    Returns:
        SpecValidationResult
    """
    issues = []
    suggestions = []
    ambiguities = []

    # Check for vague terms in overview
    overview = spec.get("overview", "")
    vague_overview = detect_vague_terms(overview)
    if vague_overview:
        issues.append(
            f"Overview contains vague terms without metrics: {', '.join(vague_overview)}"
        )
        suggestions.append(
            "Add quantitative metrics for terms like 'fast', 'secure', 'scalable'"
        )

    # Check scenario completeness
    scenarios = spec.get("scenarios", [])
    scenario_issues = check_scenario_completeness(scenarios)
    issues.extend(scenario_issues)

    # Check for P0 scenarios
    p0_count = sum(1 for s in scenarios if s.get("priority") == "P0")
    if p0_count == 0:
        issues.append("No P0 (critical) scenarios defined")
        suggestions.append("Add at least one P0 scenario for MVP requirements")

    # Check for success criteria
    success_criteria = spec.get("success_criteria", "")
    if not success_criteria or len(success_criteria) < 50:
        issues.append("Success criteria section is missing or too brief")
        suggestions.append(
            "Define measurable success criteria (metrics, targets, thresholds)"
        )

    # Compute completeness score
    score = 1.0
    if vague_overview:
        score -= 0.2
    if scenario_issues:
        score -= 0.1 * min(len(scenario_issues), 3)
    if p0_count == 0:
        score -= 0.3
    if not success_criteria:
        score -= 0.2

    score = max(0.0, score)

    return SpecValidationResult(
        is_valid=(len(issues) == 0),
        issues=issues,
        suggestions=suggestions,
        requirements=[],  # Pattern matching doesn't extract requirements
        ambiguities=ambiguities,
        completeness_score=score,
    )


# =============================================================================
# DSPy-Based Validation (Advanced)
# =============================================================================

def dspy_based_validation(spec: Dict[str, Any], reviewer: ReviewerModule) -> SpecValidationResult:
    """Validate spec using ReviewerModule intelligence.

    Args:
        spec: Parsed feature spec
        reviewer: ReviewerModule instance

    Returns:
        SpecValidationResult with LLM-powered analysis
    """
    issues = []
    suggestions = []
    ambiguities = []

    # Extract user intent from spec
    feature_name = spec["frontmatter"].get("name", "Unknown Feature")
    overview = spec.get("overview", "")
    scenarios = spec.get("scenarios", [])

    # Construct user intent summary
    user_intent = f"{feature_name}. {overview}"
    if scenarios:
        user_intent += f" Key scenarios: {', '.join(s['name'] for s in scenarios[:3])}"

    # Extract requirements using ReviewerModule
    try:
        result = reviewer.extract_requirements(
            user_intent=user_intent,
            context=f"Feature spec with {len(scenarios)} scenarios"
        )

        extracted_requirements = result.requirements
        priorities = result.priorities if hasattr(result, 'priorities') else []

        logger.info(f"Extracted {len(extracted_requirements)} requirements from spec")

    except Exception as e:
        logger.error(f"Failed to extract requirements: {e}")
        extracted_requirements = []
        priorities = []

    # Check if scenarios match extracted requirements
    spec_requirements = spec.get("requirements", "")
    if extracted_requirements and spec_requirements:
        # Compare extracted vs documented
        for req in extracted_requirements[:5]:  # Check top 5
            if req.lower() not in spec_requirements.lower():
                issues.append(
                    f"Extracted requirement not explicitly stated: '{req}'"
                )

    # Check for ambiguities in scenarios
    for scenario in scenarios:
        goal = scenario.get("goal", "")
        benefit = scenario.get("benefit", "")

        # Check for vague terms
        vague_goal = detect_vague_terms(goal)
        vague_benefit = detect_vague_terms(benefit)

        if vague_goal or vague_benefit:
            ambiguities.append({
                "location": f"Scenario: {scenario['name']}",
                "term": ", ".join(vague_goal + vague_benefit),
                "question": f"How do we quantify '{', '.join(vague_goal + vague_benefit)}'?",
                "impact": "Acceptance criteria may be subjective without metrics",
            })

    # Add pattern-based checks as well
    pattern_result = pattern_based_validation(spec)
    issues.extend(pattern_result.issues)
    suggestions.extend(pattern_result.suggestions)

    # Compute completeness score
    score = 1.0
    if len(issues) > 0:
        score -= 0.1 * min(len(issues), 5)
    if len(ambiguities) > 0:
        score -= 0.05 * min(len(ambiguities), 4)

    score = max(0.0, score)

    return SpecValidationResult(
        is_valid=(len(issues) == 0 and len(ambiguities) == 0),
        issues=issues,
        suggestions=suggestions,
        requirements=extracted_requirements,
        ambiguities=ambiguities,
        completeness_score=score,
    )


# =============================================================================
# Public API
# =============================================================================

def validate_feature_spec(spec_path: str | Path) -> Dict[str, Any]:
    """Validate a feature specification.

    Args:
        spec_path: Path to feature spec markdown file

    Returns:
        Dictionary with validation results:
        - is_valid: bool
        - issues: List[str]
        - suggestions: List[str]
        - requirements: List[str] (extracted by LLM)
        - ambiguities: List[Dict]
        - completeness_score: float (0.0-1.0)
    """
    spec_path = Path(spec_path)

    # Parse spec
    try:
        spec = parse_feature_spec(spec_path)
    except Exception as e:
        logger.error(f"Failed to parse spec: {e}")
        return {
            "is_valid": False,
            "issues": [f"Failed to parse spec: {e}"],
            "suggestions": ["Verify spec file format and YAML frontmatter"],
            "requirements": [],
            "ambiguities": [],
            "completeness_score": 0.0,
        }

    # Validate using DSPy if available, otherwise use patterns
    if DSPY_AVAILABLE:
        try:
            reviewer = ReviewerModule()
            result = dspy_based_validation(spec, reviewer)
        except Exception as e:
            logger.warning(f"DSPy validation failed, falling back to patterns: {e}")
            result = pattern_based_validation(spec)
    else:
        result = pattern_based_validation(spec)

    return {
        "is_valid": result.is_valid,
        "issues": result.issues,
        "suggestions": result.suggestions,
        "requirements": result.requirements,
        "ambiguities": result.ambiguities,
        "completeness_score": result.completeness_score,
    }


def detect_ambiguities(spec_path: str | Path) -> List[Dict[str, str]]:
    """Detect ambiguities in feature spec for /feature-clarify.

    Args:
        spec_path: Path to feature spec markdown file

    Returns:
        List of ambiguities with:
        - location: Where ambiguity was found
        - term: Ambiguous term or phrase
        - question: Suggested clarifying question
        - impact: Why this matters
    """
    result = validate_feature_spec(spec_path)
    return result.get("ambiguities", [])


def suggest_improvements(spec_path: str | Path) -> List[str]:
    """Generate improvement suggestions for feature spec.

    Args:
        spec_path: Path to feature spec markdown file

    Returns:
        List of actionable suggestions
    """
    result = validate_feature_spec(spec_path)
    return result.get("suggestions", [])


# =============================================================================
# CLI Entry Point
# =============================================================================

def main():
    """CLI entry point for spec validation."""
    import argparse
    import json

    parser = argparse.ArgumentParser(description="Validate feature specifications")
    parser.add_argument("spec_path", help="Path to feature spec markdown file")
    parser.add_argument("--json", action="store_true", help="Output JSON format")
    parser.add_argument(
        "--ambiguities-only",
        action="store_true",
        help="Only detect ambiguities",
    )

    args = parser.parse_args()

    if args.ambiguities_only:
        ambiguities = detect_ambiguities(args.spec_path)
        if args.json:
            print(json.dumps(ambiguities, indent=2))
        else:
            if ambiguities:
                print("Ambiguities detected:")
                for amb in ambiguities:
                    print(f"\n  Location: {amb['location']}")
                    print(f"  Term: {amb['term']}")
                    print(f"  Question: {amb['question']}")
                    print(f"  Impact: {amb['impact']}")
            else:
                print("No ambiguities detected.")
    else:
        result = validate_feature_spec(args.spec_path)
        if args.json:
            print(json.dumps(result, indent=2))
        else:
            print(f"Validation Result: {'✓ VALID' if result['is_valid'] else '✗ ISSUES FOUND'}")
            print(f"Completeness Score: {result['completeness_score']:.1%}")

            if result["issues"]:
                print("\nIssues:")
                for issue in result["issues"]:
                    print(f"  ✗ {issue}")

            if result["suggestions"]:
                print("\nSuggestions:")
                for suggestion in result["suggestions"]:
                    print(f"  → {suggestion}")

            if result["ambiguities"]:
                print(f"\nAmbiguities: {len(result['ambiguities'])}")

            if result["requirements"]:
                print(f"\nExtracted Requirements: {len(result['requirements'])}")


if __name__ == "__main__":
    main()
