#!/usr/bin/env python3
"""
Quality Gates for DSPy Training Data

Enforces quality thresholds and difficulty stratification for training data
before it enters versioned datasets. Ensures balanced, high-quality datasets
for optimal MIPROv2 optimization results.

Quality Gates:
- Minimum quality score threshold (configurable per signature)
- Difficulty distribution targets (30% easy, 50% medium, 20% hard)
- Completeness distribution targets (60% incomplete, 40% complete for validation signatures)
- Category diversity requirements
- Source distribution balance

Integration:
- Works with DataValidator for quality scoring
- Works with DatasetManager for versioned storage
- Provides batch processing and reporting
"""

import json
import sys
from dataclasses import dataclass, asdict
from pathlib import Path
from typing import List, Dict, Any, Optional, Tuple
from collections import Counter


@dataclass
class QualityGateConfig:
    """Configuration for quality gates"""
    signature_name: str
    min_quality_score: float = 70.0  # Minimum quality score (0-100)

    # Difficulty distribution targets (percentages)
    difficulty_targets: Dict[str, float] = None
    difficulty_tolerance: float = 0.10  # ±10% tolerance

    # Completeness distribution (for validation signatures only)
    completeness_targets: Dict[str, float] = None
    completeness_tolerance: float = 0.10

    # Category diversity (minimum unique categories)
    min_categories: int = 3

    # Source diversity (no single source > 80%)
    max_source_percentage: float = 0.80

    def __post_init__(self):
        if self.difficulty_targets is None:
            self.difficulty_targets = {'easy': 0.30, 'medium': 0.50, 'hard': 0.20}

        if self.completeness_targets is None:
            # Default for validation signatures
            self.completeness_targets = {'complete': 0.40, 'incomplete': 0.60}


@dataclass
class QualityGateResult:
    """Result of quality gate evaluation"""
    passed: bool
    total_examples: int
    accepted_examples: int
    rejected_examples: int
    rejection_reasons: Dict[str, int]  # reason -> count
    difficulty_distribution: Dict[str, float]
    completeness_distribution: Dict[str, float]
    category_diversity: int
    source_distribution: Dict[str, float]
    avg_quality_score: float
    warnings: List[str]


class QualityGate:
    """
    Quality gate for DSPy training data.

    Validates incoming examples against quality thresholds and distribution targets,
    ensuring balanced, high-quality datasets for optimization.
    """

    # Default configs for each signature
    DEFAULT_CONFIGS = {
        'extract_requirements': QualityGateConfig(
            signature_name='extract_requirements',
            min_quality_score=70.0,
            completeness_targets=None  # Not applicable
        ),
        'validate_intent': QualityGateConfig(
            signature_name='validate_intent',
            min_quality_score=75.0,
            completeness_targets=None  # Binary classification
        ),
        'validate_completeness': QualityGateConfig(
            signature_name='validate_completeness',
            min_quality_score=75.0,
            completeness_targets={'complete': 0.40, 'incomplete': 0.60}
        ),
        'validate_correctness': QualityGateConfig(
            signature_name='validate_correctness',
            min_quality_score=75.0,
            completeness_targets={'correct': 0.50, 'incorrect': 0.50}
        ),
        'generate_guidance': QualityGateConfig(
            signature_name='generate_guidance',
            min_quality_score=80.0,  # Higher bar for generation tasks
            completeness_targets=None  # Not applicable
        )
    }

    def __init__(self, config: Optional[QualityGateConfig] = None):
        """
        Initialize quality gate.

        Args:
            config: Quality gate configuration (uses defaults if not provided)
        """
        self.config = config

    def _get_config(self, signature_name: str) -> QualityGateConfig:
        """Get config for signature (use provided config or default)"""
        if self.config and self.config.signature_name == signature_name:
            return self.config
        return self.DEFAULT_CONFIGS.get(signature_name, QualityGateConfig(signature_name=signature_name))

    def evaluate_batch(
        self,
        examples: List[Dict[str, Any]],
        signature_name: str,
        strict: bool = False
    ) -> Tuple[List[Dict[str, Any]], QualityGateResult]:
        """
        Evaluate a batch of examples against quality gates.

        Args:
            examples: List of training examples
            signature_name: Name of DSPy signature
            strict: If True, fail entire batch on distribution violations (default: accept with warnings)

        Returns:
            Tuple of (accepted_examples, quality_gate_result)
        """
        config = self._get_config(signature_name)

        accepted = []
        rejected = []
        rejection_reasons = Counter()

        # Filter individual examples by quality score
        for ex in examples:
            quality_score = ex.get('metadata', {}).get('quality_score', 0.0)

            if quality_score < config.min_quality_score:
                rejected.append(ex)
                rejection_reasons['low_quality_score'] += 1
            else:
                accepted.append(ex)

        # Calculate distributions
        difficulty_dist = self._calculate_difficulty_distribution(accepted)
        completeness_dist = self._calculate_completeness_distribution(accepted, signature_name)
        category_diversity = self._calculate_category_diversity(accepted)
        source_dist = self._calculate_source_distribution(accepted)
        avg_quality = self._calculate_avg_quality(accepted)

        # Check distribution targets
        warnings = []
        passed = True

        # Check difficulty distribution
        diff_violations = self._check_difficulty_distribution(difficulty_dist, config)
        if diff_violations:
            warnings.extend(diff_violations)
            if strict:
                passed = False

        # Check completeness distribution (if applicable)
        if config.completeness_targets:
            comp_violations = self._check_completeness_distribution(completeness_dist, config)
            if comp_violations:
                warnings.extend(comp_violations)
                if strict:
                    passed = False

        # Check category diversity
        if category_diversity < config.min_categories:
            warnings.append(f"Low category diversity: {category_diversity} < {config.min_categories}")
            if strict:
                passed = False

        # Check source distribution
        max_source_pct = max(source_dist.values()) if source_dist else 0.0
        if max_source_pct > config.max_source_percentage:
            warnings.append(f"Single source dominance: {max_source_pct:.1%} > {config.max_source_percentage:.1%}")
            if strict:
                passed = False

        result = QualityGateResult(
            passed=passed,
            total_examples=len(examples),
            accepted_examples=len(accepted),
            rejected_examples=len(rejected),
            rejection_reasons=dict(rejection_reasons),
            difficulty_distribution=difficulty_dist,
            completeness_distribution=completeness_dist,
            category_diversity=category_diversity,
            source_distribution=source_dist,
            avg_quality_score=avg_quality,
            warnings=warnings
        )

        return accepted, result

    def _calculate_difficulty_distribution(self, examples: List[Dict[str, Any]]) -> Dict[str, float]:
        """Calculate difficulty distribution as percentages"""
        if not examples:
            return {'easy': 0.0, 'medium': 0.0, 'hard': 0.0}

        counts = Counter(ex.get('metadata', {}).get('difficulty', 'unknown') for ex in examples)
        total = len(examples)

        return {
            'easy': counts['easy'] / total,
            'medium': counts['medium'] / total,
            'hard': counts['hard'] / total
        }

    def _calculate_completeness_distribution(
        self,
        examples: List[Dict[str, Any]],
        signature_name: str
    ) -> Dict[str, float]:
        """Calculate completeness distribution as percentages"""
        if not examples:
            return {}

        if signature_name in ['validate_completeness']:
            # is_complete field
            counts = Counter(ex.get('outputs', {}).get('is_complete', None) for ex in examples)
            total = len(examples)
            return {
                'complete': counts.get(True, 0) / total,
                'incomplete': counts.get(False, 0) / total
            }
        elif signature_name in ['validate_correctness']:
            # is_correct field
            counts = Counter(ex.get('outputs', {}).get('is_correct', None) for ex in examples)
            total = len(examples)
            return {
                'correct': counts.get(True, 0) / total,
                'incorrect': counts.get(False, 0) / total
            }
        else:
            return {}

    def _calculate_category_diversity(self, examples: List[Dict[str, Any]]) -> int:
        """Count unique categories"""
        categories = set(ex.get('metadata', {}).get('category', 'unknown') for ex in examples)
        return len(categories)

    def _calculate_source_distribution(self, examples: List[Dict[str, Any]]) -> Dict[str, float]:
        """Calculate source distribution as percentages"""
        if not examples:
            return {}

        counts = Counter(ex.get('metadata', {}).get('source', 'unknown') for ex in examples)
        total = len(examples)

        return {source: count / total for source, count in counts.items()}

    def _calculate_avg_quality(self, examples: List[Dict[str, Any]]) -> float:
        """Calculate average quality score"""
        if not examples:
            return 0.0

        scores = [ex.get('metadata', {}).get('quality_score', 0.0) for ex in examples]
        return sum(scores) / len(scores)

    def _check_difficulty_distribution(
        self,
        actual: Dict[str, float],
        config: QualityGateConfig
    ) -> List[str]:
        """Check if difficulty distribution meets targets"""
        violations = []

        for difficulty, target in config.difficulty_targets.items():
            actual_pct = actual.get(difficulty, 0.0)
            lower = target - config.difficulty_tolerance
            upper = target + config.difficulty_tolerance

            if actual_pct < lower:
                violations.append(
                    f"Difficulty '{difficulty}': {actual_pct:.1%} below target {target:.1%} (min {lower:.1%})"
                )
            elif actual_pct > upper:
                violations.append(
                    f"Difficulty '{difficulty}': {actual_pct:.1%} above target {target:.1%} (max {upper:.1%})"
                )

        return violations

    def _check_completeness_distribution(
        self,
        actual: Dict[str, float],
        config: QualityGateConfig
    ) -> List[str]:
        """Check if completeness distribution meets targets"""
        if not config.completeness_targets:
            return []

        violations = []

        for status, target in config.completeness_targets.items():
            actual_pct = actual.get(status, 0.0)
            lower = target - config.completeness_tolerance
            upper = target + config.completeness_tolerance

            if actual_pct < lower:
                violations.append(
                    f"Completeness '{status}': {actual_pct:.1%} below target {target:.1%} (min {lower:.1%})"
                )
            elif actual_pct > upper:
                violations.append(
                    f"Completeness '{status}': {actual_pct:.1%} above target {target:.1%} (max {upper:.1%})"
                )

        return violations

    def print_report(self, result: QualityGateResult):
        """Print human-readable quality gate report"""
        print(f"\n{'='*60}")
        print(f"Quality Gate Report")
        print(f"{'='*60}")
        print(f"Status: {'✓ PASSED' if result.passed else '✗ FAILED'}")
        print(f"\nExamples:")
        print(f"  Total:    {result.total_examples}")
        print(f"  Accepted: {result.accepted_examples}")
        print(f"  Rejected: {result.rejected_examples}")

        if result.rejection_reasons:
            print(f"\nRejection Reasons:")
            for reason, count in result.rejection_reasons.items():
                print(f"  {reason}: {count}")

        print(f"\nDifficulty Distribution:")
        for difficulty, pct in result.difficulty_distribution.items():
            print(f"  {difficulty}: {pct:.1%}")

        if result.completeness_distribution:
            print(f"\nCompleteness Distribution:")
            for status, pct in result.completeness_distribution.items():
                print(f"  {status}: {pct:.1%}")

        print(f"\nQuality Metrics:")
        print(f"  Avg Quality Score: {result.avg_quality_score:.1f}")
        print(f"  Category Diversity: {result.category_diversity}")

        print(f"\nSource Distribution:")
        for source, pct in result.source_distribution.items():
            print(f"  {source}: {pct:.1%}")

        if result.warnings:
            print(f"\n⚠ Warnings:")
            for warning in result.warnings:
                print(f"  - {warning}")

        print(f"{'='*60}\n")


def main():
    """CLI for quality gate evaluation"""
    import argparse

    parser = argparse.ArgumentParser(description="DSPy Quality Gate Evaluation")
    parser.add_argument('--input', required=True, help='Input JSON file with examples')
    parser.add_argument('--signature', required=True, help='Signature name')
    parser.add_argument('--output', help='Output file for accepted examples')
    parser.add_argument('--strict', action='store_true', help='Strict mode (fail on distribution violations)')
    parser.add_argument('--min-quality', type=float, help='Override minimum quality score')

    args = parser.parse_args()

    # Load examples
    with open(args.input, 'r') as f:
        examples = json.load(f)

    # Create quality gate with optional config override
    config = None
    if args.min_quality:
        config = QualityGateConfig(
            signature_name=args.signature,
            min_quality_score=args.min_quality
        )

    gate = QualityGate(config=config)

    # Evaluate batch
    accepted, result = gate.evaluate_batch(examples, args.signature, strict=args.strict)

    # Print report
    gate.print_report(result)

    # Write accepted examples if output specified
    if args.output:
        with open(args.output, 'w') as f:
            json.dump(accepted, f, indent=2)
        print(f"Wrote {len(accepted)} accepted examples to {args.output}")

    # Exit with appropriate code
    sys.exit(0 if result.passed else 1)


if __name__ == '__main__':
    main()
