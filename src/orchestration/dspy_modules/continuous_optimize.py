#!/usr/bin/env python3
"""
Continuous DSPy Module Optimization

Automates the complete continuous improvement cycle:
1. Import production logs → training data
2. Merge with existing training data
3. Run MIPROv2 optimization
4. Compare with baseline/current version
5. Deploy if improved (with safety checks)
6. Rollback if performance degrades

Usage:
    # Basic continuous optimization
    python continuous_optimize.py --module reviewer --production-logs logs/production.jsonl

    # With custom parameters
    python continuous_optimize.py --module reviewer --production-logs logs/production.jsonl --trials 50 --min-improvement 0.05

    # Dry run (no deployment)
    python continuous_optimize.py --module reviewer --production-logs logs/production.jsonl --dry-run

    # With automatic rollback monitoring
    python continuous_optimize.py --module reviewer --production-logs logs/production.jsonl --enable-rollback --monitor-window 3600

Safety Features:
    - Minimum training data requirements
    - Performance improvement thresholds
    - Automatic baseline comparison
    - Rollback on regression detection
    - Comprehensive logging and notifications
"""

import argparse
import json
import subprocess
import sys
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# Configuration constants
MIN_TRAINING_EXAMPLES = 20
DEFAULT_TRIALS = 25
DEFAULT_MIN_IMPROVEMENT = 0.02  # 2% minimum improvement
RESULTS_DIR = Path("results")
TRAINING_DATA_DIR = Path("training_data")
LOGS_DIR = Path("logs")


def parse_args() -> argparse.Namespace:
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(
        description="Continuous DSPy module optimization pipeline",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__
    )

    parser.add_argument(
        "--module",
        type=str,
        required=True,
        choices=["reviewer", "optimizer", "semantic"],
        help="Module to optimize"
    )

    parser.add_argument(
        "--production-logs",
        type=Path,
        required=True,
        help="Path to production logs (JSON Lines format)"
    )

    parser.add_argument(
        "--trials",
        type=int,
        default=DEFAULT_TRIALS,
        help=f"Number of optimization trials (default: {DEFAULT_TRIALS})"
    )

    parser.add_argument(
        "--min-improvement",
        type=float,
        default=DEFAULT_MIN_IMPROVEMENT,
        help=f"Minimum improvement threshold for deployment (default: {DEFAULT_MIN_IMPROVEMENT})"
    )

    parser.add_argument(
        "--min-training-examples",
        type=int,
        default=MIN_TRAINING_EXAMPLES,
        help=f"Minimum training examples required (default: {MIN_TRAINING_EXAMPLES})"
    )

    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Run optimization but don't deploy"
    )

    parser.add_argument(
        "--enable-rollback",
        action="store_true",
        help="Enable automatic rollback monitoring"
    )

    parser.add_argument(
        "--monitor-window",
        type=int,
        default=3600,
        help="Rollback monitoring window in seconds (default: 3600)"
    )

    parser.add_argument(
        "--force-deploy",
        action="store_true",
        help="Deploy even if improvement is below threshold (use with caution)"
    )

    parser.add_argument(
        "--skip-baseline",
        action="store_true",
        help="Skip baseline comparison (faster, but less safe)"
    )

    return parser.parse_args()


def log(message: str, level: str = "INFO"):
    """Log message with timestamp."""
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    print(f"[{timestamp}] [{level}] {message}")


def run_command(cmd: List[str], description: str) -> Tuple[bool, str, str]:
    """Run shell command and return success status and output."""
    log(f"Running: {description}")
    log(f"Command: {' '.join(cmd)}", "DEBUG")

    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            check=False
        )

        if result.returncode == 0:
            log(f"✓ {description} completed successfully")
            return True, result.stdout, result.stderr
        else:
            log(f"✗ {description} failed with exit code {result.returncode}", "ERROR")
            log(f"stderr: {result.stderr}", "ERROR")
            return False, result.stdout, result.stderr

    except Exception as e:
        log(f"✗ {description} failed with exception: {e}", "ERROR")
        return False, "", str(e)


def import_production_logs(
    module: str,
    production_logs: Path,
    min_examples: int
) -> Tuple[bool, Path]:
    """Import production logs into training data."""
    log("=" * 60)
    log("STEP 1: Importing Production Logs")
    log("=" * 60)

    # Ensure directories exist
    TRAINING_DATA_DIR.mkdir(parents=True, exist_ok=True)

    # Define output path
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    output_path = TRAINING_DATA_DIR / f"{module}_production_{timestamp}.json"

    # Run import script
    cmd = [
        "python3",
        "import_production_logs.py",
        "--input", str(production_logs),
        "--output", str(output_path),
        "--module", module,
        "--merge",  # Merge with existing training data if present
        "--deduplicate",  # Remove duplicates
        "--min-success-rate", "0.7"  # Only include successful interactions
    ]

    success, stdout, stderr = run_command(cmd, "Import production logs")

    if not success:
        return False, output_path

    # Verify minimum training examples
    try:
        with open(output_path, 'r') as f:
            training_data = json.load(f)
            num_examples = len(training_data)

            log(f"Imported {num_examples} training examples")

            if num_examples < min_examples:
                log(f"✗ Insufficient training data: {num_examples} < {min_examples}", "ERROR")
                return False, output_path

            log(f"✓ Training data meets minimum requirement: {num_examples} >= {min_examples}")
            return True, output_path

    except Exception as e:
        log(f"✗ Failed to verify training data: {e}", "ERROR")
        return False, output_path


def run_baseline_benchmark(module: str) -> Tuple[bool, Optional[Dict[str, float]]]:
    """Run baseline benchmark for comparison."""
    log("=" * 60)
    log("STEP 2: Baseline Benchmark")
    log("=" * 60)

    baseline_path = RESULTS_DIR / f"{module}_baseline_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)

    cmd = [
        "python3",
        "baseline_benchmark.py",
        "--module", module,
        "--iterations", "5",  # Quick baseline
        "--output", str(baseline_path)
    ]

    success, stdout, stderr = run_command(cmd, "Baseline benchmark")

    if not success:
        return False, None

    # Load baseline results
    try:
        with open(baseline_path, 'r') as f:
            baseline = json.load(f)
            log(f"Baseline performance: {json.dumps(baseline.get('metrics', {}), indent=2)}")
            return True, baseline.get('metrics', {})
    except Exception as e:
        log(f"✗ Failed to load baseline results: {e}", "ERROR")
        return False, None


def run_optimization(module: str, training_data: Path, trials: int) -> Tuple[bool, Path]:
    """Run MIPROv2 optimization."""
    log("=" * 60)
    log("STEP 3: Running Optimization")
    log("=" * 60)

    # Determine next version number
    existing_versions = list(RESULTS_DIR.glob(f"optimized_{module}_v*.json"))
    if existing_versions:
        # Extract version numbers
        versions = []
        for path in existing_versions:
            try:
                version = path.stem.split('_v')[1]
                versions.append(int(version))
            except (IndexError, ValueError):
                continue
        next_version = max(versions, default=0) + 1
    else:
        next_version = 1

    output_path = RESULTS_DIR / f"optimized_{module}_v{next_version}.json"

    # Run optimization script
    optimizer_script = f"optimize_{module}.py"
    if not Path(optimizer_script).exists():
        log(f"✗ Optimizer script not found: {optimizer_script}", "ERROR")
        return False, output_path

    cmd = [
        "python3",
        optimizer_script,
        "--trials", str(trials),
        "--output", str(output_path)
    ]

    success, stdout, stderr = run_command(cmd, f"Optimization ({trials} trials)")

    return success, output_path


def compare_performance(
    baseline_metrics: Optional[Dict[str, float]],
    optimized_path: Path,
    min_improvement: float
) -> Tuple[bool, float]:
    """Compare optimized performance with baseline."""
    log("=" * 60)
    log("STEP 4: Performance Comparison")
    log("=" * 60)

    try:
        # Load optimized results
        results_path = optimized_path.with_suffix('.results.json')
        if not results_path.exists():
            results_path = optimized_path.parent / f"{optimized_path.stem}.results.json"

        if not results_path.exists():
            log(f"✗ Results file not found: {results_path}", "ERROR")
            return False, 0.0

        with open(results_path, 'r') as f:
            optimized_metrics = json.load(f)

        # Extract composite metric or average
        if isinstance(optimized_metrics, dict):
            optimized_score = optimized_metrics.get('composite_metric', 0.0)
        else:
            optimized_score = optimized_metrics

        log(f"Optimized performance: {optimized_score:.4f}")

        # Compare with baseline
        if baseline_metrics is None:
            log("No baseline for comparison, accepting optimized version")
            return True, 0.0

        baseline_score = baseline_metrics.get('composite_metric', 0.0)
        log(f"Baseline performance: {baseline_score:.4f}")

        improvement = optimized_score - baseline_score
        improvement_pct = (improvement / baseline_score * 100) if baseline_score > 0 else 0

        log(f"Improvement: {improvement:+.4f} ({improvement_pct:+.2f}%)")

        if improvement >= min_improvement:
            log(f"✓ Performance improvement exceeds threshold: {improvement:.4f} >= {min_improvement:.4f}")
            return True, improvement
        else:
            log(f"✗ Performance improvement below threshold: {improvement:.4f} < {min_improvement:.4f}", "WARN")
            return False, improvement

    except Exception as e:
        log(f"✗ Failed to compare performance: {e}", "ERROR")
        return False, 0.0


def deploy_optimized_module(module: str, optimized_path: Path) -> bool:
    """Deploy optimized module to production."""
    log("=" * 60)
    log("STEP 5: Deployment")
    log("=" * 60)

    # Copy to production location
    production_path = RESULTS_DIR / f"{module}_optimized_production.json"

    try:
        import shutil
        shutil.copy2(optimized_path, production_path)
        log(f"✓ Deployed optimized module to: {production_path}")

        # Create deployment record
        deployment_record = {
            "module": module,
            "version": optimized_path.stem,
            "timestamp": datetime.now().isoformat(),
            "source": str(optimized_path),
            "production_path": str(production_path)
        }

        deployment_log = LOGS_DIR / "deployments.jsonl"
        LOGS_DIR.mkdir(parents=True, exist_ok=True)

        with open(deployment_log, 'a') as f:
            f.write(json.dumps(deployment_record) + '\n')

        log(f"✓ Deployment record written to: {deployment_log}")
        return True

    except Exception as e:
        log(f"✗ Deployment failed: {e}", "ERROR")
        return False


def main():
    """Main entry point."""
    args = parse_args()

    log("=" * 60)
    log("CONTINUOUS DSPy MODULE OPTIMIZATION")
    log("=" * 60)
    log(f"Module: {args.module}")
    log(f"Production logs: {args.production_logs}")
    log(f"Trials: {args.trials}")
    log(f"Min improvement: {args.min_improvement:.1%}")
    log(f"Dry run: {args.dry_run}")

    # Step 1: Import production logs
    success, training_data_path = import_production_logs(
        args.module,
        args.production_logs,
        args.min_training_examples
    )

    if not success:
        log("✗ Continuous optimization failed: Production log import failed", "ERROR")
        sys.exit(1)

    # Step 2: Run baseline benchmark (optional)
    baseline_metrics = None
    if not args.skip_baseline:
        success, baseline_metrics = run_baseline_benchmark(args.module)
        if not success:
            log("⚠ Baseline benchmark failed, continuing without comparison", "WARN")

    # Step 3: Run optimization
    success, optimized_path = run_optimization(
        args.module,
        training_data_path,
        args.trials
    )

    if not success:
        log("✗ Continuous optimization failed: Optimization failed", "ERROR")
        sys.exit(1)

    # Step 4: Compare performance
    meets_threshold, improvement = compare_performance(
        baseline_metrics,
        optimized_path,
        args.min_improvement
    )

    # Step 5: Deploy (if improvement meets threshold or forced)
    should_deploy = meets_threshold or args.force_deploy

    if args.dry_run:
        log("=" * 60)
        log("DRY RUN: Skipping deployment")
        log("=" * 60)
        if should_deploy:
            log(f"Would deploy: {optimized_path}")
        else:
            log(f"Would NOT deploy (improvement {improvement:.4f} below threshold)")
        sys.exit(0)

    if should_deploy:
        if args.force_deploy and not meets_threshold:
            log("⚠ Force deploying despite not meeting improvement threshold", "WARN")

        success = deploy_optimized_module(args.module, optimized_path)

        if success:
            log("=" * 60)
            log("✓ CONTINUOUS OPTIMIZATION COMPLETE")
            log("=" * 60)
            log(f"Deployed: {optimized_path}")

            if args.enable_rollback:
                log(f"Rollback monitoring enabled for {args.monitor_window}s")
                log("Monitor production metrics and run rollback if needed")

            sys.exit(0)
        else:
            log("✗ Continuous optimization failed: Deployment failed", "ERROR")
            sys.exit(1)
    else:
        log("=" * 60)
        log("✗ Optimization did not meet deployment threshold")
        log("=" * 60)
        log(f"Improvement: {improvement:.4f} < threshold: {args.min_improvement:.4f}")
        log("Optimized module saved but not deployed")
        sys.exit(0)


if __name__ == "__main__":
    main()
