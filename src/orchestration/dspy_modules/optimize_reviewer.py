#!/usr/bin/env python3
"""MIPROv2 optimization for ReviewerModule.

Uses training data to optimize prompts via DSPy MIPROv2 teleprompter.
Measures improvement over baseline and saves optimized module.

# Usage

```bash
# Optimize ReviewerModule with MIPROv2
python optimize_reviewer.py --trials 50 --output optimized_reviewer_v1.json

# Quick test run
python optimize_reviewer.py --trials 10 --test-mode
```

# Requirements

- Training data in training_data/ directory
- Baseline benchmark results for comparison
- ANTHROPIC_API_KEY configured
- ~1-2 hours for full optimization (50 trials)

# Outputs

- Optimized module saved to output file
- Performance comparison report
- Optimization statistics (trials, improvements)
"""

import dspy
from dspy.teleprompt import MIPROv2
import os
import json
import argparse
import logging
from pathlib import Path
from typing import Dict, List, Any
from datetime import datetime

from reviewer_module import ReviewerModule

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


# =============================================================================
# Training Data Loading
# =============================================================================

def load_training_data(data_dir: Path) -> Dict[str, List[dspy.Example]]:
    """Load training data for all ReviewerModule signatures.

    Args:
        data_dir: Directory containing training data JSON files

    Returns:
        Dictionary mapping signature names to lists of DSPy Examples
    """
    signatures = [
        "extract_requirements",
        "validate_intent",
        "validate_completeness",
        "validate_correctness",
        "generate_guidance"
    ]

    training_data = {}

    for sig in signatures:
        json_path = data_dir / f"{sig}.json"

        if not json_path.exists():
            logger.warning(f"Training data not found: {json_path}")
            continue

        with open(json_path) as f:
            raw_data = json.load(f)

        # Convert to DSPy Examples
        examples = []
        for item in raw_data:
            # Extract input and output field names
            inputs = item.get("inputs", {})
            outputs = item.get("outputs", {})

            # Create DSPy Example with all fields
            example = dspy.Example(**inputs, **outputs).with_inputs(*inputs.keys())
            examples.append(example)

        training_data[sig] = examples
        logger.info(f"Loaded {len(examples)} examples for {sig}")

    return training_data


# =============================================================================
# Evaluation Metrics
# =============================================================================

def requirement_extraction_metric(example, pred, trace=None) -> float:
    """Evaluate requirement extraction quality.

    Measures overlap between predicted and gold requirements using F1 score.
    """
    try:
        gold_reqs = set(example.requirements)
        pred_reqs = set(pred.requirements) if hasattr(pred, 'requirements') else set()

        if not pred_reqs:
            return 0.0

        intersection = len(gold_reqs & pred_reqs)
        precision = intersection / len(pred_reqs) if pred_reqs else 0
        recall = intersection / len(gold_reqs) if gold_reqs else 0

        if precision + recall == 0:
            return 0.0

        f1 = 2 * (precision * recall) / (precision + recall)
        return f1
    except Exception as e:
        logger.error(f"Metric error: {e}")
        return 0.0


def intent_validation_metric(example, pred, trace=None) -> float:
    """Evaluate intent validation accuracy.

    Binary match: 1.0 if intent_satisfied matches, 0.0 otherwise.
    """
    try:
        gold_satisfied = example.intent_satisfied
        pred_satisfied = pred.intent_satisfied if hasattr(pred, 'intent_satisfied') else None

        if pred_satisfied is None:
            return 0.0

        return 1.0 if gold_satisfied == pred_satisfied else 0.0
    except Exception as e:
        logger.error(f"Metric error: {e}")
        return 0.0


def completeness_metric(example, pred, trace=None) -> float:
    """Evaluate completeness validation accuracy.

    Combination of binary match and missing requirements overlap.
    """
    try:
        gold_complete = example.is_complete
        pred_complete = pred.is_complete if hasattr(pred, 'is_complete') else None

        if pred_complete is None:
            return 0.0

        # Binary accuracy
        binary_score = 1.0 if gold_complete == pred_complete else 0.0

        # Missing requirements overlap (if incomplete)
        if not gold_complete and hasattr(pred, 'missing_requirements') and hasattr(example, 'missing_requirements'):
            gold_missing = set(example.missing_requirements)
            pred_missing = set(pred.missing_requirements)

            if gold_missing and pred_missing:
                overlap = len(gold_missing & pred_missing) / len(gold_missing)
                return (binary_score + overlap) / 2

        return binary_score
    except Exception as e:
        logger.error(f"Metric error: {e}")
        return 0.0


def correctness_metric(example, pred, trace=None) -> float:
    """Evaluate correctness validation accuracy.

    Binary match: 1.0 if is_correct matches, 0.0 otherwise.
    """
    try:
        gold_correct = example.is_correct
        pred_correct = pred.is_correct if hasattr(pred, 'is_correct') else None

        return 1.0 if gold_correct == pred_correct else 0.0
    except Exception as e:
        logger.error(f"Metric error: {e}")
        return 0.0


def guidance_metric(example, pred, trace=None) -> float:
    """Evaluate improvement guidance quality.

    Measures overlap in guidance items and priority accuracy.
    """
    try:
        gold_guidance = example.guidance if hasattr(example, 'guidance') else []
        pred_guidance = pred.guidance if hasattr(pred, 'guidance') else []

        if not pred_guidance:
            return 0.0

        # Extract titles for comparison
        gold_titles = {item.get('title', '') for item in gold_guidance}
        pred_titles = {item.get('title', '') for item in pred_guidance}

        # F1 score on titles
        intersection = len(gold_titles & pred_titles)
        precision = intersection / len(pred_titles) if pred_titles else 0
        recall = intersection / len(gold_titles) if gold_titles else 0

        if precision + recall == 0:
            return 0.0

        f1 = 2 * (precision * recall) / (precision + recall)
        return f1
    except Exception as e:
        logger.error(f"Metric error: {e}")
        return 0.0


# Metric mapping
METRICS = {
    "extract_requirements": requirement_extraction_metric,
    "validate_intent": intent_validation_metric,
    "validate_completeness": completeness_metric,
    "validate_correctness": correctness_metric,
    "generate_guidance": guidance_metric,
}


# =============================================================================
# Optimization
# =============================================================================

def optimize_module(
    training_data: Dict[str, List[dspy.Example]],
    num_trials: int = 50,
    test_mode: bool = False
) -> ReviewerModule:
    """Optimize ReviewerModule using MIPROv2.

    Args:
        training_data: Training examples for each signature
        num_trials: Number of optimization trials
        test_mode: If True, use fewer trials for testing

    Returns:
        Optimized ReviewerModule instance
    """
    logger.info("Initializing ReviewerModule for optimization")
    module = ReviewerModule()

    if test_mode:
        num_trials = min(num_trials, 10)
        logger.info(f"Test mode: reducing trials to {num_trials}")

    # Combine all training data for module-level optimization
    all_examples = []
    for sig_name, examples in training_data.items():
        all_examples.extend(examples)

    logger.info(f"Total training examples: {len(all_examples)}")

    # Create composite metric that evaluates all operations
    def composite_metric(example, pred, trace=None) -> float:
        """Composite metric across all ReviewerModule operations."""
        # Determine which operation to evaluate based on example fields
        # Check for output fields that indicate the operation type
        if hasattr(example, 'requirements') and hasattr(example, 'user_intent'):
            return requirement_extraction_metric(example, pred, trace)
        elif hasattr(example, 'intent_satisfied'):
            return intent_validation_metric(example, pred, trace)
        elif hasattr(example, 'is_complete'):
            return completeness_metric(example, pred, trace)
        elif hasattr(example, 'is_correct'):
            return correctness_metric(example, pred, trace)
        elif hasattr(example, 'guidance'):
            return guidance_metric(example, pred, trace)
        else:
            logger.warning(f"Unknown example type (fields: {list(example.__dict__.keys())}), returning 0.0")
            return 0.0

    # Configure MIPROv2 teleprompter
    logger.info("Configuring MIPROv2 teleprompter")
    teleprompter = MIPROv2(
        metric=composite_metric,
        auto=None,                  # Disable auto mode to use manual settings
        num_candidates=10,          # Number of prompt candidates per trial
        init_temperature=1.0,       # Temperature for initial prompt generation
        verbose=True                # Log optimization progress
    )

    # Run optimization
    logger.info(f"Starting optimization with {num_trials} trials")
    logger.info("This may take 1-2 hours depending on trials and API latency")

    optimized = teleprompter.compile(
        module,
        trainset=all_examples,
        num_trials=num_trials
    )

    logger.info("Optimization complete!")
    return optimized


# =============================================================================
# Evaluation
# =============================================================================

def evaluate_module(module: ReviewerModule, test_data: Dict[str, List[dspy.Example]]) -> Dict[str, float]:
    """Evaluate module performance on test data.

    Args:
        module: ReviewerModule to evaluate
        test_data: Test examples for each signature

    Returns:
        Dictionary of metric scores per signature
    """
    logger.info("Evaluating module performance")
    scores = {}

    for sig_name, examples in test_data.items():
        if not examples:
            continue

        metric_fn = METRICS.get(sig_name)
        if not metric_fn:
            logger.warning(f"No metric for {sig_name}")
            continue

        sig_scores = []
        for example in examples:
            try:
                # Run module operation - use getattr with defaults for optional fields
                if sig_name == "extract_requirements":
                    if not (hasattr(example, 'user_intent') and hasattr(example, 'context')):
                        continue
                    pred = module.extract_requirements(
                        user_intent=example.user_intent,
                        context=example.context
                    )
                elif sig_name == "validate_intent":
                    # Check for required fields
                    if not all(hasattr(example, f) for f in ['user_intent', 'work_item', 'implementation', 'requirements']):
                        continue
                    pred = module.validate_intent_satisfaction(
                        user_intent=example.user_intent,
                        work_item=example.work_item,
                        implementation=example.implementation,
                        requirements=example.requirements
                    )
                elif sig_name == "validate_completeness":
                    # work_item is optional in training data
                    if not (hasattr(example, 'implementation') and hasattr(example, 'requirements')):
                        continue
                    pred = module.validate_implementation_completeness(
                        work_item=getattr(example, 'work_item', ''),
                        implementation=example.implementation,
                        requirements=example.requirements
                    )
                elif sig_name == "validate_correctness":
                    # work_item is optional
                    if not (hasattr(example, 'implementation')):
                        continue
                    pred = module.validate_implementation_correctness(
                        work_item=getattr(example, 'work_item', ''),
                        implementation=example.implementation,
                        test_results=getattr(example, 'test_results', '')
                    )
                elif sig_name == "generate_guidance":
                    if not all(hasattr(example, f) for f in ['user_intent', 'work_item', 'implementation']):
                        continue
                    pred = module.generate_improvement_guidance_for_failed_review(
                        user_intent=example.user_intent,
                        work_item=example.work_item,
                        implementation=example.implementation,
                        failed_gates=getattr(example, 'failed_gates', []),
                        all_issues=getattr(example, 'all_issues', [])
                    )
                else:
                    continue

                score = metric_fn(example, pred)
                sig_scores.append(score)
            except Exception as e:
                logger.error(f"Evaluation error for {sig_name}: {e}")
                sig_scores.append(0.0)

        avg_score = sum(sig_scores) / len(sig_scores) if sig_scores else 0.0
        scores[sig_name] = avg_score
        logger.info(f"{sig_name}: {avg_score:.3f}")

    return scores


# =============================================================================
# Main
# =============================================================================

def main():
    parser = argparse.ArgumentParser(
        description="Optimize ReviewerModule with MIPROv2"
    )
    parser.add_argument(
        "--trials",
        type=int,
        default=50,
        help="Number of optimization trials (default: 50)"
    )
    parser.add_argument(
        "--output",
        type=str,
        default="optimized_reviewer_v1.json",
        help="Output file for optimized module"
    )
    parser.add_argument(
        "--test-mode",
        action="store_true",
        help="Run in test mode (fewer trials)"
    )
    parser.add_argument(
        "--data-dir",
        type=str,
        default="training_data",
        help="Directory containing training data"
    )

    args = parser.parse_args()

    # Initialize DSPy with Anthropic Claude Haiku 4.5
    api_key = os.getenv("ANTHROPIC_API_KEY")
    if not api_key:
        logger.error("ANTHROPIC_API_KEY not set")
        return

    try:
        dspy.configure(lm=dspy.LM('anthropic/claude-haiku-4-5-20251001', api_key=api_key))
        logger.info("DSPy configured with Claude Haiku 4.5")
    except Exception as e:
        logger.error(f"Failed to configure DSPy: {e}")
        return

    # Load training data
    data_dir = Path(__file__).parent / args.data_dir
    training_data = load_training_data(data_dir)

    if not training_data:
        logger.error("No training data loaded")
        return

    # Split into train/test (80/20)
    train_data = {}
    test_data = {}

    for sig_name, examples in training_data.items():
        split_idx = int(len(examples) * 0.8)
        train_data[sig_name] = examples[:split_idx]
        test_data[sig_name] = examples[split_idx:]

    logger.info("Training/test split:")
    for sig_name in train_data:
        logger.info(f"  {sig_name}: {len(train_data[sig_name])}/{len(test_data[sig_name])}")

    # Evaluate baseline
    logger.info("Evaluating baseline module")
    baseline_module = ReviewerModule()
    baseline_scores = evaluate_module(baseline_module, test_data)

    # Optimize
    optimized_module = optimize_module(train_data, args.trials, args.test_mode)

    # Evaluate optimized
    logger.info("Evaluating optimized module")
    optimized_scores = evaluate_module(optimized_module, test_data)

    # Compare
    logger.info("=" * 60)
    logger.info("OPTIMIZATION RESULTS")
    logger.info("=" * 60)

    improvements = {}
    for sig_name in baseline_scores:
        baseline = baseline_scores[sig_name]
        optimized = optimized_scores.get(sig_name, 0.0)
        improvement = optimized - baseline
        improvements[sig_name] = improvement

        symbol = "↑" if improvement > 0 else "↓" if improvement < 0 else "="
        logger.info(f"{sig_name}:")
        logger.info(f"  Baseline:  {baseline:.3f}")
        logger.info(f"  Optimized: {optimized:.3f}")
        logger.info(f"  Change:    {improvement:+.3f} {symbol}")

    avg_improvement = sum(improvements.values()) / len(improvements) if improvements else 0.0
    logger.info(f"\nAverage improvement: {avg_improvement:+.3f}")

    # Save optimized module
    output_path = Path(args.output)
    optimized_module.save(str(output_path))
    logger.info(f"\nOptimized module saved to: {output_path}")

    # Save results summary
    results = {
        "timestamp": datetime.now().isoformat(),
        "config": {
            "trials": args.trials,
            "test_mode": args.test_mode,
        },
        "baseline_scores": baseline_scores,
        "optimized_scores": optimized_scores,
        "improvements": improvements,
        "average_improvement": avg_improvement,
    }

    results_path = output_path.with_suffix('.results.json')
    with open(results_path, 'w') as f:
        json.dump(results, f, indent=2)
    logger.info(f"Results summary saved to: {results_path}")


if __name__ == "__main__":
    main()
