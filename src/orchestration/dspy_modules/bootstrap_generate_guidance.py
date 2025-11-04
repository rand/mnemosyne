#!/usr/bin/env python3
"""BootstrapFewShot optimization for generate_guidance signature.

Tier 2 optimizer - better suited for datasets with <200 examples.
Research shows BootstrapFewShot often outperforms MIPROv2 on small datasets.

# Usage

```bash
# Test run
python bootstrap_generate_guidance.py --test-mode --output /tmp/generate_guidance_bootstrap_test.json

# Full optimization
python bootstrap_generate_guidance.py --max-demos 8 --output /tmp/generate_guidance_bootstrap.json
```

# Expected Improvement

MIPROv2 baseline: 50% → 50% (stagnant)
BootstrapFewShot target: 50% → 65-75% (+15-25% improvement)

Better for <200 examples - simpler, faster, often more effective.
"""

import dspy
from dspy.teleprompt import BootstrapFewShot
import os
import json
import argparse
import logging
from pathlib import Path
from typing import List
from datetime import datetime

from reviewer_module import GenerateImprovementGuidance

# Inline simple guidance metric
def guidance_metric(example, pred, trace=None) -> float:
    """Simple metric checking if guidance was generated."""
    if not hasattr(pred, 'guidance') or not pred.guidance:
        return 0.0
    return 1.0 if len(pred.guidance) > 50 else 0.5

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


# =============================================================================
# Training Data Loading
# =============================================================================

def load_generate_guidance_data(data_dir: Path) -> List[dspy.Example]:
    """Load training data for generate_guidance signature only."""
    json_path = data_dir / "generate_guidance.json"

    if not json_path.exists():
        raise FileNotFoundError(f"Training data not found: {json_path}")

    with open(json_path) as f:
        raw_data = json.load(f)

    examples = []
    for item in raw_data:
        inputs = item.get("inputs", {})
        outputs = item.get("outputs", {})

        if not all(field in inputs for field in ["review_findings"]):
            logger.warning(f"Skipping example missing required inputs")
            continue
        if not all(field in outputs for field in ["guidance"]):
            logger.warning(f"Skipping example missing required outputs")
            continue

        example = dspy.Example(**inputs, **outputs).with_inputs(*inputs.keys())
        examples.append(example)

    logger.info(f"Loaded {len(examples)} generate_guidance examples")
    return examples


# =============================================================================
# Simple Wrapper Module
# =============================================================================

class GenerateGuidanceModule(dspy.Module):
    """Minimal module wrapping only the generate_guidance signature."""

    def __init__(self):
        super().__init__()
        self.predictor = dspy.ChainOfThought(GenerateImprovementGuidance)

    def forward(self, review_findings: dict):
        """Generate actionable guidance from review findings."""
        return self.predictor(review_findings=review_findings)


# =============================================================================
# Optimization
# =============================================================================

def optimize_with_bootstrap(
    training_data: List[dspy.Example],
    max_bootstrapped_demos: int = 8,
    max_labeled_demos: int = 4,
    test_mode: bool = False
) -> GenerateGuidanceModule:
    """Optimize generate_guidance using BootstrapFewShot."""
    logger.info("Initializing GenerateGuidanceModule for optimization")
    module = GenerateGuidanceModule()

    if test_mode:
        max_bootstrapped_demos = min(max_bootstrapped_demos, 4)
        max_labeled_demos = min(max_labeled_demos, 2)
        logger.info(f"Test mode: reducing demos to {max_bootstrapped_demos}/{max_labeled_demos}")

    logger.info(f"Total training examples: {len(training_data)}")

    # Configure BootstrapFewShot
    logger.info("Configuring BootstrapFewShot teleprompter")
    teleprompter = BootstrapFewShot(
        metric=guidance_metric,
        max_bootstrapped_demos=max_bootstrapped_demos,
        max_labeled_demos=max_labeled_demos,
        max_rounds=1,
        max_errors=5,
    )

    logger.info(f"Starting BootstrapFewShot optimization")
    logger.info(f"  max_bootstrapped_demos: {max_bootstrapped_demos}")
    logger.info(f"  max_labeled_demos: {max_labeled_demos}")
    logger.info("This typically takes 5-15 minutes (faster than MIPROv2)")

    optimized = teleprompter.compile(
        module,
        trainset=training_data
    )

    logger.info("Optimization complete!")
    return optimized


# =============================================================================
# Evaluation
# =============================================================================

def evaluate_module(
    module: GenerateGuidanceModule,
    test_data: List[dspy.Example]
) -> float:
    """Evaluate module on test data."""
    logger.info(f"Evaluating on {len(test_data)} test examples")
    scores = []

    for example in test_data:
        try:
            pred = module(**{k: getattr(example, k) for k in ['review_findings']})
            score = guidance_metric(example, pred)
            scores.append(score)
            logger.debug(f"Example scored {score:.3f}")
        except Exception as e:
            logger.error(f"Evaluation error: {e}")
            scores.append(0.0)

    avg_score = sum(scores) / len(scores) if scores else 0.0
    logger.info(f"Average score: {avg_score:.3f}")
    return avg_score


# =============================================================================
# Main
# =============================================================================

def main():
    parser = argparse.ArgumentParser(
        description="Optimize generate_guidance signature with BootstrapFewShot (Tier 2)"
    )
    parser.add_argument(
        "--max-demos",
        type=int,
        default=8,
        help="Maximum bootstrapped demonstrations (default: 8)"
    )
    parser.add_argument(
        "--max-labeled",
        type=int,
        default=4,
        help="Maximum labeled demonstrations (default: 4)"
    )
    parser.add_argument(
        "--output",
        type=str,
        default="/tmp/generate_guidance_bootstrap.json",
        help="Output file for optimized module"
    )
    parser.add_argument(
        "--test-mode",
        action="store_true",
        help="Run in test mode (fewer demos)"
    )
    parser.add_argument(
        "--data-dir",
        type=str,
        default="training_data",
        help="Directory containing training data"
    )

    args = parser.parse_args()

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

    data_dir = Path(__file__).parent / args.data_dir
    all_data = load_generate_guidance_data(data_dir)

    # Split 20/80 (INVERTED - more validation data for prompt optimization)
    split_idx = int(len(all_data) * 0.2)
    train_data = all_data[:split_idx]
    test_data = all_data[split_idx:]

    logger.info(f"Training/test split: {len(train_data)}/{len(test_data)} (20/80 for prompt optimization)")

    logger.info("Evaluating baseline module")
    baseline_module = GenerateGuidanceModule()
    baseline_score = evaluate_module(baseline_module, test_data)

    optimized_module = optimize_with_bootstrap(
        train_data,
        args.max_demos,
        args.max_labeled,
        args.test_mode
    )

    logger.info("Evaluating optimized module")
    optimized_score = evaluate_module(optimized_module, test_data)

    improvement = optimized_score - baseline_score
    pct_improvement = (improvement / baseline_score * 100) if baseline_score > 0 else 0

    logger.info("=" * 60)
    logger.info("OPTIMIZATION RESULTS (BootstrapFewShot - Tier 2)")
    logger.info("=" * 60)
    logger.info(f"Baseline:  {baseline_score:.3f}")
    logger.info(f"Optimized: {optimized_score:.3f}")
    logger.info(f"Improvement: {improvement:+.3f} ({pct_improvement:+.1f}%)")

    output_path = Path(args.output)
    optimized_module.save(str(output_path))
    logger.info(f"\nOptimized module saved to: {output_path}")

    results = {
        "timestamp": datetime.now().isoformat(),
        "signature": "generate_guidance",
        "optimizer": "BootstrapFewShot",
        "tier": 2,
        "config": {
            "max_bootstrapped_demos": args.max_demos,
            "max_labeled_demos": args.max_labeled,
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
