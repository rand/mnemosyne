#!/usr/bin/env python3
"""Tier 3 semantic evaluation metrics: LLM-as-judge for correctness and guidance.

Addresses Tier 2 stagnation (validate_correctness: 0%, generate_guidance: 0%)
by replacing binary/length-based metrics with nuanced semantic evaluation.

Based on successful RequirementSetEvaluator pattern from semantic_metrics_fast.py.
"""

import dspy
from typing import List, Dict, Any
import logging
import json
import re

logger = logging.getLogger(__name__)


# =============================================================================
# Correctness Evaluation (Multi-Dimensional)
# =============================================================================

class CorrectnessEvaluator(dspy.Signature):
    """Evaluate correctness validation quality holistically.

    Replaces binary boolean metric with multi-dimensional assessment:
    - Issue Detection: Are real issues identified?
    - False Positives: Are non-issues incorrectly flagged?
    - Explanation Quality: Is the reasoning clear and accurate?
    - Severity Assessment: Are critical issues prioritized?

    Scoring guidelines (0.0-1.0):
    - 1.0: All real issues found, no false positives, excellent explanation
    - 0.8-0.9: Most issues found, minor false positives, good explanation
    - 0.6-0.7: Core issues found, some false positives/misses, adequate explanation
    - 0.4-0.5: Partial detection, significant gaps or false positives
    - 0.0-0.3: Poor detection, mostly incorrect or missing issues

    Output a quality score from 0.0 to 1.0.
    """

    implementation = dspy.InputField(
        desc="Implementation being validated (string)"
    )
    code_sample = dspy.InputField(
        desc="Code sample from implementation (string)"
    )
    gold_is_correct = dspy.InputField(
        desc="Gold standard: is implementation correct? (boolean)"
    )
    gold_issues = dspy.InputField(
        desc="Gold standard: list of real issues (JSON array of strings)"
    )
    predicted_is_correct = dspy.InputField(
        desc="Model prediction: is implementation correct? (boolean)"
    )
    predicted_issues = dspy.InputField(
        desc="Model prediction: list of identified issues (JSON array of strings)"
    )

    reasoning = dspy.OutputField(
        prefix="Reasoning: Let's analyze correctness validation quality step by step:",
        desc="Detailed analysis of issue detection, false positives, and explanation quality"
    )
    issue_coverage = dspy.OutputField(
        desc="Which gold issues were correctly identified and which were missed?"
    )
    false_positive_analysis = dspy.OutputField(
        desc="Are any predicted issues incorrect or not real issues?"
    )
    quality_score = dspy.OutputField(
        desc="Overall correctness validation quality score from 0.0 to 1.0"
    )


class FastCorrectnessQualityEvaluator(dspy.Module):
    """Fast DSPy module for correctness validation quality evaluation."""

    def __init__(self):
        super().__init__()
        self.evaluator = dspy.ChainOfThought(CorrectnessEvaluator)

    def forward(
        self,
        implementation: str,
        code_sample: str,
        gold_is_correct: bool,
        gold_issues: List[str],
        pred_is_correct: bool,
        pred_issues: List[str]
    ) -> float:
        """Evaluate correctness validation quality.

        Args:
            implementation: Implementation description
            code_sample: Code being validated
            gold_is_correct: Gold standard correctness boolean
            gold_issues: Reference issues from training data
            pred_is_correct: Predicted correctness boolean
            pred_issues: Issues identified by model

        Returns:
            Quality score from 0.0 to 1.0
        """
        # Format as JSON for clear structure
        gold_issues_json = json.dumps(gold_issues)
        pred_issues_json = json.dumps(pred_issues)

        result = self.evaluator(
            implementation=implementation,
            code_sample=code_sample,
            gold_is_correct=str(gold_is_correct),
            gold_issues=gold_issues_json,
            predicted_is_correct=str(pred_is_correct),
            predicted_issues=pred_issues_json
        )

        try:
            # Parse score from string output
            score_str = str(result.quality_score).strip()
            # Extract numeric portion (handle cases like "0.82\n]]" or "0.82]]")
            match = re.search(r'(\d+\.?\d*)', score_str)
            if match:
                score = float(match.group(1))
            else:
                score = float(score_str)

            # Clamp to valid range
            score = max(0.0, min(1.0, score))

            logger.debug(
                f"Correctness quality: {score:.3f} "
                f"(gold: {len(gold_issues)} issues, pred: {len(pred_issues)} issues)"
            )

            return score
        except (ValueError, AttributeError) as e:
            logger.warning(f"Failed to parse quality score from '{result.quality_score}': {e}")
            # Fallback: analyze coverage and false positives
            coverage = str(result.issue_coverage).lower()
            fp_analysis = str(result.false_positive_analysis).lower()

            if "all" in coverage and "no false" in fp_analysis:
                return 0.9
            elif "most" in coverage and ("minor" in fp_analysis or "few" in fp_analysis):
                return 0.7
            elif "core" in coverage or "some" in coverage:
                return 0.5
            else:
                return 0.2


def correctness_quality_metric(
    example: dspy.Example,
    pred: dspy.Prediction,
    trace=None
) -> float:
    """Compute semantic quality score for correctness validation (Tier 3).

    Replaces binary boolean metric with multi-dimensional LLM-as-judge evaluation.

    Args:
        example: Training example with gold correctness and issues
        pred: Model prediction with predicted correctness and issues
        trace: Optional DSPy trace (unused)

    Returns:
        Quality score from 0.0 to 1.0
    """
    try:
        # Extract gold standard
        implementation = str(example.implementation) if hasattr(example, 'implementation') else ""
        code_sample = str(example.code_sample) if hasattr(example, 'code_sample') else ""
        gold_is_correct = bool(example.is_correct) if hasattr(example, 'is_correct') else True
        gold_issues = list(example.issues) if hasattr(example, 'issues') else []

        # Extract prediction
        pred_is_correct = bool(pred.is_correct) if hasattr(pred, 'is_correct') else True
        pred_issues = list(pred.issues) if hasattr(pred, 'issues') else []

        # Holistic evaluation in single API call
        evaluator = FastCorrectnessQualityEvaluator()
        score = evaluator(
            implementation=implementation,
            code_sample=code_sample,
            gold_is_correct=gold_is_correct,
            gold_issues=gold_issues,
            pred_is_correct=pred_is_correct,
            pred_issues=pred_issues
        )

        return score

    except Exception as e:
        logger.error(f"Error computing correctness quality score: {e}")
        return 0.0


# =============================================================================
# Guidance Quality Evaluation (Actionability + Clarity)
# =============================================================================

class GuidanceEvaluator(dspy.Signature):
    """Evaluate improvement guidance quality holistically.

    Replaces length-based metric (>50 chars = 1.0) with semantic quality assessment:
    - Actionability: Are steps specific and implementable?
    - Clarity: Is guidance easy to understand?
    - Relevance: Does guidance address actual issues?
    - Prioritization: Are critical fixes highlighted?
    - Examples: Are code examples helpful and correct?

    Scoring guidelines (0.0-1.0):
    - 1.0: Specific, actionable, prioritized, excellent examples, perfect clarity
    - 0.8-0.9: Clear guidance, good prioritization, helpful examples
    - 0.6-0.7: Actionable but somewhat generic, basic prioritization
    - 0.4-0.5: Vague guidance, poor prioritization, limited examples
    - 0.0-0.3: Generic or unhelpful advice, no actionable steps

    Output a quality score from 0.0 to 1.0.
    """

    review_findings = dspy.InputField(
        desc="Issues that need guidance (JSON object with missing_requirements and correctness_issues)"
    )
    gold_guidance = dspy.InputField(
        desc="Gold standard guidance from training data (JSON array of guidance objects)"
    )
    predicted_guidance = dspy.InputField(
        desc="Generated guidance from model (string or array)"
    )

    reasoning = dspy.OutputField(
        prefix="Reasoning: Let's analyze guidance quality step by step:",
        desc="Detailed analysis of actionability, clarity, relevance, prioritization, and examples"
    )
    actionability_assessment = dspy.OutputField(
        desc="Are the recommended steps specific, concrete, and implementable?"
    )
    clarity_assessment = dspy.OutputField(
        desc="Is the guidance clear, well-organized, and easy to understand?"
    )
    relevance_assessment = dspy.OutputField(
        desc="Does the guidance directly address the review findings?"
    )
    quality_score = dspy.OutputField(
        desc="Overall guidance quality score from 0.0 to 1.0"
    )


class FastGuidanceQualityEvaluator(dspy.Module):
    """Fast DSPy module for guidance quality evaluation."""

    def __init__(self):
        super().__init__()
        self.evaluator = dspy.ChainOfThought(GuidanceEvaluator)

    def forward(
        self,
        review_findings: Dict[str, List[str]],
        gold_guidance: Any,
        pred_guidance: Any
    ) -> float:
        """Evaluate guidance quality.

        Args:
            review_findings: Issues needing guidance (dict with missing_requirements, correctness_issues)
            gold_guidance: Reference guidance from training
            pred_guidance: Generated guidance

        Returns:
            Quality score from 0.0 to 1.0
        """
        # Format as JSON for clear structure
        findings_json = json.dumps(review_findings)

        # Handle various guidance formats
        if isinstance(gold_guidance, str):
            gold_json = gold_guidance
        elif isinstance(gold_guidance, list):
            gold_json = json.dumps(gold_guidance)
        else:
            gold_json = json.dumps([])

        if isinstance(pred_guidance, str):
            pred_json = pred_guidance
        elif isinstance(pred_guidance, list):
            pred_json = json.dumps(pred_guidance)
        else:
            pred_json = json.dumps([])

        result = self.evaluator(
            review_findings=findings_json,
            gold_guidance=gold_json,
            predicted_guidance=pred_json
        )

        try:
            # Parse score from string output
            score_str = str(result.quality_score).strip()
            # Extract numeric portion
            match = re.search(r'(\d+\.?\d*)', score_str)
            if match:
                score = float(match.group(1))
            else:
                score = float(score_str)

            # Clamp to valid range
            score = max(0.0, min(1.0, score))

            logger.debug(f"Guidance quality: {score:.3f}")

            return score
        except (ValueError, AttributeError) as e:
            logger.warning(f"Failed to parse quality score from '{result.quality_score}': {e}")
            # Fallback: analyze assessments
            actionability = str(result.actionability_assessment).lower()
            clarity = str(result.clarity_assessment).lower()
            relevance = str(result.relevance_assessment).lower()

            # Count positive signals
            positive_signals = 0
            if "specific" in actionability or "concrete" in actionability:
                positive_signals += 1
            if "clear" in clarity or "well" in clarity:
                positive_signals += 1
            if "directly" in relevance or "addresses" in relevance:
                positive_signals += 1

            if positive_signals == 3:
                return 0.85
            elif positive_signals == 2:
                return 0.65
            elif positive_signals == 1:
                return 0.45
            else:
                return 0.25


def guidance_quality_metric(
    example: dspy.Example,
    pred: dspy.Prediction,
    trace=None
) -> float:
    """Compute semantic quality score for guidance generation (Tier 3).

    Replaces length-based metric with multi-dimensional LLM-as-judge evaluation.

    Args:
        example: Training example with gold guidance
        pred: Model prediction with generated guidance
        trace: Optional DSPy trace (unused)

    Returns:
        Quality score from 0.0 to 1.0
    """
    try:
        # Extract review findings
        review_findings = {}
        if hasattr(example, 'review_findings'):
            review_findings = example.review_findings
        else:
            # Construct from separate fields if needed
            review_findings = {
                'missing_requirements': getattr(example, 'missing_requirements', []),
                'correctness_issues': getattr(example, 'correctness_issues', [])
            }

        # Extract gold guidance
        gold_guidance = getattr(example, 'guidance', [])

        # Extract predicted guidance
        pred_guidance = getattr(pred, 'guidance', "")

        # Check if guidance was generated
        if not pred_guidance or (isinstance(pred_guidance, str) and len(pred_guidance) < 10):
            return 0.0

        # Holistic evaluation in single API call
        evaluator = FastGuidanceQualityEvaluator()
        score = evaluator(
            review_findings=review_findings,
            gold_guidance=gold_guidance,
            pred_guidance=pred_guidance
        )

        return score

    except Exception as e:
        logger.error(f"Error computing guidance quality score: {e}")
        return 0.0
