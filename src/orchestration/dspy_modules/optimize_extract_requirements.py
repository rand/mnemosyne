#!/usr/bin/env python3
"""MIPROv2 optimization for extract_requirements signature.

Focused single-signature optimization to avoid dispatch ambiguity.
Optimizes only the requirement extraction predictor.

# Usage

```bash
# Test run (5 trials)
python optimize_extract_requirements.py --test-mode --output /tmp/extract_reqs_test.json

# Full optimization (20-30 trials)
python optimize_extract_requirements.py --trials 25 --output extract_reqs_v1.json
```

# Expected Improvement

Baseline: ~0.71 F1
Target: 0.75-0.80 F1 (5-15% improvement)
"""

import dspy
from dspy.teleprompt import MIPROv2
import os
import json
import argparse
import logging
from pathlib import Path
from typing import List
from datetime import datetime

from reviewer_module import ReviewerModule, ExtractRequirements
from semantic_metrics_fast import semantic_requirement_f1

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


# =============================================================================
# Training Data Loading
# =============================================================================

def load_extract_requirements_data(data_dir: Path) -> List[dspy.Example]:
    """Load training data for extract_requirements signature only.

    Args:
        data_dir: Directory containing training_data/

    Returns:
        List of DSPy Examples with inputs (user_intent, context) and
        outputs (requirements, priorities)
    """
    json_path = data_dir / "extract_requirements.json"

    if not json_path.exists():
        raise FileNotFoundError(f"Training data not found: {json_path}")

    with open(json_path) as f:
        raw_data = json.load(f)

    examples = []
    for item in raw_data:
        inputs = item.get("inputs", {})
        outputs = item.get("outputs", {})

        # Validate required fields
        if not ("user_intent" in inputs and "context" in inputs):
            logger.warning(f"Skipping example missing required inputs: {inputs.keys()}")
            continue
        if not ("requirements" in outputs and "priorities" in outputs):
            logger.warning(f"Skipping example missing required outputs: {outputs.keys()}")
            continue

        example = dspy.Example(**inputs, **outputs).with_inputs(*inputs.keys())
        examples.append(example)

    logger.info(f"Loaded {len(examples)} extract_requirements examples")
    return examples


# =============================================================================
# Simple Wrapper Module
# =============================================================================

class ExtractRequirementsModule(dspy.Module):
    """Minimal module wrapping only the extract_requirements signature.

    This avoids the complexity of optimizing the full ReviewerModule.
    """

    def __init__(self):
        super().__init__()
        self.extract_reqs = dspy.ChainOfThought(ExtractRequirements)

    def forward(self, user_intent: str, context: str):
        """Extract requirements from user intent and context."""
        return self.extract_reqs(user_intent=user_intent, context=context)


# =============================================================================
# Optimization
# =============================================================================

def optimize_extract_requirements(
    training_data: List[dspy.Example],
    num_trials: int = 25,
    test_mode: bool = False
) -> ExtractRequirementsModule:
    """Optimize extract_requirements using MIPROv2.

    Args:
        training_data: Training examples
        num_trials: Number of optimization trials
        test_mode: If True, use fewer trials for testing

    Returns:
        Optimized module
    """
    logger.info("Initializing ExtractRequirementsModule for optimization")
    module = ExtractRequirementsModule()

    if test_mode:
        num_trials = min(num_trials, 5)
        logger.info(f"Test mode: reducing trials to {num_trials}")

    logger.info(f"Total training examples: {len(training_data)}")

    # Configure MIPROv2
    logger.info("Configuring MIPROv2 teleprompter")
    teleprompter = MIPROv2(
        metric=semantic_requirement_f1,  # FOCUSED METRIC - no dispatch ambiguity
        auto=None,
        num_candidates=10,
        init_temperature=1.0,
        verbose=True,
        num_threads=2  # Limit parallelism to avoid rate limits
    )

    # Run optimization
    logger.info(f"Starting optimization with {num_trials} trials")
    logger.info("This may take 15-30 minutes depending on trials")

    # Use smaller minibatch for small training set (16 examples)
    optimized = teleprompter.compile(
        module,
        trainset=training_data,
        num_trials=num_trials,
        minibatch=True,
        minibatch_size=4,  # Small batches for small training set
        minibatch_full_eval_steps=2  # Evaluate fully every 2 steps
    )

    logger.info("Optimization complete!")
    return optimized


# =============================================================================
# Evaluation
# =============================================================================

def evaluate_module(
    module: ExtractRequirementsModule,
    test_data: List[dspy.Example]
) -> float:
    """Evaluate module on test data.

    Args:
        module: Module to evaluate
        test_data: Test examples

    Returns:
        Average F1 score
    """
    logger.info(f"Evaluating on {len(test_data)} test examples")
    scores = []

    for example in test_data:
        try:
            pred = module(
                user_intent=example.user_intent,
                context=example.context
            )
            score = semantic_requirement_f1(example, pred)
            scores.append(score)
            logger.debug(f"Example scored {score:.3f}")
        except Exception as e:
            logger.error(f"Evaluation error: {e}")
            scores.append(0.0)

    avg_score = sum(scores) / len(scores) if scores else 0.0
    logger.info(f"Average F1 score: {avg_score:.3f}")
    return avg_score


# =============================================================================
# Main
# =============================================================================

def main():
    parser = argparse.ArgumentParser(
        description="Optimize extract_requirements signature with MIPROv2"
    )
    parser.add_argument(
        "--trials",
        type=int,
        default=25,
        help="Number of optimization trials (default: 25)"
    )
    parser.add_argument(
        "--output",
        type=str,
        default="extract_reqs_v1.json",
        help="Output file for optimized module"
    )
    parser.add_argument(
        "--test-mode",
        action="store_true",
        help="Run in test mode (5 trials)"
    )
    parser.add_argument(
        "--data-dir",
        type=str,
        default="training_data",
        help="Directory containing training data"
    )

    args = parser.parse_args()

    # Initialize DSPy with Claude Haiku 4.5
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
    all_data = load_extract_requirements_data(data_dir)

    # Split 80/20
    split_idx = int(len(all_data) * 0.8)
    train_data = all_data[:split_idx]
    test_data = all_data[split_idx:]

    logger.info(f"Training/test split: {len(train_data)}/{len(test_data)}")

    # Evaluate baseline
    logger.info("Evaluating baseline module")
    baseline_module = ExtractRequirementsModule()
    baseline_score = evaluate_module(baseline_module, test_data)

    # Optimize
    optimized_module = optimize_extract_requirements(train_data, args.trials, args.test_mode)

    # Evaluate optimized
    logger.info("Evaluating optimized module")
    optimized_score = evaluate_module(optimized_module, test_data)

    # Compare
    improvement = optimized_score - baseline_score
    pct_improvement = (improvement / baseline_score * 100) if baseline_score > 0 else 0

    logger.info("=" * 60)
    logger.info("OPTIMIZATION RESULTS")
    logger.info("=" * 60)
    logger.info(f"Baseline F1:  {baseline_score:.3f}")
    logger.info(f"Optimized F1: {optimized_score:.3f}")
    logger.info(f"Improvement:  {improvement:+.3f} ({pct_improvement:+.1f}%)")

    # Save optimized module
    output_path = Path(args.output)
    optimized_module.save(str(output_path))
    logger.info(f"\nOptimized module saved to: {output_path}")

    # Save results summary
    results = {
        "timestamp": datetime.now().isoformat(),
        "signature": "extract_requirements",
        "config": {
            "trials": args.trials,
            "test_mode": args.test_mode,
        },
        "baseline_score": baseline_score,
        "optimized_score": optimized_score,
        "improvement": improvement,
        "pct_improvement": pct_improvement,
    }

    results_path = output_path.with_suffix('.results.json')
    with open(results_path, 'w') as f:
        json.dump(results, f, indent=2)
    logger.info(f"Results summary saved to: {results_path}")


if __name__ == "__main__":
    main()
