#!/usr/bin/env python3
"""Aggregate optimized per-signature modules into unified ReviewerModule.

Combines the best-performing version of each signature into a single
ReviewerModule that can be saved and deployed.

Since DSPy modules serialize their predictors' instructions and demos,
we extract these from each optimized per-signature module and apply them
to a new ReviewerModule.
"""

import dspy
import json
import argparse
import logging
from pathlib import Path
from datetime import datetime

from reviewer_module import ReviewerModule
from optimize_extract_requirements import ExtractRequirementsModule
from optimize_validate_intent import ValidateIntentModule
from optimize_validate_completeness import ValidateCompletenessModule
from optimize_validate_correctness import ValidateCorrectnessModule
from optimize_generate_guidance import GenerateGuidanceModule

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


def load_optimized_modules(modules_dir: Path):
    """Load all optimized per-signature modules.

    Args:
        modules_dir: Directory containing optimized module JSON files

    Returns:
        Dict mapping signature names to loaded modules
    """
    modules = {}

    module_files = {
        'extract_requirements': ('extract_requirements_v1.json', ExtractRequirementsModule),
        'validate_intent': ('validate_intent_v1.json', ValidateIntentModule),
        'validate_completeness': ('validate_completeness_v1.json', ValidateCompletenessModule),
        'validate_correctness': ('validate_correctness_v1.json', ValidateCorrectnessModule),
        'generate_guidance': ('generate_guidance_v1.json', GenerateGuidanceModule),
    }

    for name, (filename, module_class) in module_files.items():
        filepath = modules_dir / filename
        if filepath.exists():
            logger.info(f"Loading {name} from {filepath}")
            module = module_class()
            module.load(str(filepath))
            modules[name] = module
        else:
            logger.warning(f"Missing optimized module: {filepath}")
            modules[name] = None

    return modules


def create_aggregated_reviewer(optimized_modules):
    """Create ReviewerModule with optimized predictors.

    Args:
        optimized_modules: Dict of loaded optimized modules

    Returns:
        ReviewerModule with optimized predictors applied
    """
    logger.info("Creating aggregated ReviewerModule")
    reviewer = ReviewerModule()

    # Extract and apply optimized predictors
    # Each per-signature module has a single predictor that we want to transfer

    if optimized_modules['extract_requirements']:
        logger.info("Applying optimized extract_requirements predictor")
        reviewer.extract_reqs = optimized_modules['extract_requirements'].extract_reqs

    if optimized_modules['validate_intent']:
        logger.info("Applying optimized validate_intent predictor")
        reviewer.validate_intent = optimized_modules['validate_intent'].predictor

    if optimized_modules['validate_completeness']:
        logger.info("Applying optimized validate_completeness predictor")
        reviewer.validate_completeness = optimized_modules['validate_completeness'].predictor

    if optimized_modules['validate_correctness']:
        logger.info("Applying optimized validate_correctness predictor")
        reviewer.validate_correctness = optimized_modules['validate_correctness'].predictor

    if optimized_modules['generate_guidance']:
        logger.info("Applying optimized generate_guidance predictor")
        reviewer.generate_guidance = optimized_modules['generate_guidance'].predictor

    return reviewer


def save_summary(output_path: Path, optimized_modules):
    """Save aggregation summary with improvement metrics.

    Args:
        output_path: Path to save summary JSON
        optimized_modules: Dict of loaded modules
    """
    summary = {
        "timestamp": datetime.now().isoformat(),
        "aggregation_method": "per-signature",
        "modules_aggregated": list(optimized_modules.keys()),
        "improvements": {}
    }

    # Load results from each optimization
    for name in optimized_modules.keys():
        results_file = output_path.parent / f"{name}_v1.results.json"
        if results_file.exists():
            with open(results_file) as f:
                results = json.load(f)
                summary["improvements"][name] = {
                    "baseline": results.get("baseline_score"),
                    "optimized": results.get("optimized_score"),
                    "improvement_pct": results.get("pct_improvement")
                }

    summary_path = output_path.with_suffix('.summary.json')
    with open(summary_path, 'w') as f:
        json.dump(summary, f, indent=2)

    logger.info(f"Summary saved to: {summary_path}")
    return summary


def main():
    parser = argparse.ArgumentParser(
        description="Aggregate optimized per-signature modules"
    )
    parser.add_argument(
        "--modules-dir",
        type=str,
        default="/tmp",
        help="Directory containing optimized module JSON files"
    )
    parser.add_argument(
        "--output",
        type=str,
        default="reviewer_optimized_v1.json",
        help="Output file for aggregated ReviewerModule"
    )

    args = parser.parse_args()

    modules_dir = Path(args.modules_dir)
    output_path = Path(__file__).parent / args.output

    # Load all optimized modules
    optimized_modules = load_optimized_modules(modules_dir)

    # Check if we have all modules
    missing = [name for name, module in optimized_modules.items() if module is None]
    if missing:
        logger.error(f"Missing optimized modules: {missing}")
        logger.error("Run per-signature optimizations first")
        return 1

    # Create aggregated reviewer
    reviewer = create_aggregated_reviewer(optimized_modules)

    # Save aggregated module
    reviewer.save(str(output_path))
    logger.info(f"Aggregated ReviewerModule saved to: {output_path}")

    # Save summary
    summary = save_summary(output_path, optimized_modules)

    # Display results
    logger.info("=" * 60)
    logger.info("AGGREGATION COMPLETE")
    logger.info("=" * 60)
    for name, metrics in summary["improvements"].items():
        baseline = metrics["baseline"]
        optimized = metrics["optimized"]
        pct = metrics["improvement_pct"]
        logger.info(f"{name:25s}: {baseline:.3f} → {optimized:.3f} ({pct:+.1f}%)")

    return 0


if __name__ == "__main__":
    exit(main())
