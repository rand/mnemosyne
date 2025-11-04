#!/usr/bin/env python3
"""
Monthly Optimization Orchestrator for DSPy Modules

Coordinates the end-to-end optimization pipeline:
1. Data collection from multiple sources (git, synthetic, telemetry)
2. Quality validation and scoring
3. Quality gate filtering
4. Dataset versioning
5. MIPROv2 optimization execution
6. Result evaluation and comparison with production
7. Deployment decision and module updates

Designed for monthly automated execution via cron.
"""

import json
import os
import sys
import subprocess
from dataclasses import dataclass, asdict
from datetime import datetime
from pathlib import Path
from typing import List, Dict, Any, Optional, Tuple
import logging

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


@dataclass
class OptimizationConfig:
    """Configuration for optimization orchestrator"""
    # Data collection targets
    git_mining_target: int = 30  # Target examples from git mining
    synthetic_target: int = 20   # Target examples from synthetic generation
    telemetry_target: int = 0    # Target examples from telemetry (disabled until production)

    # Quality thresholds
    min_quality_score: float = 70.0
    min_total_examples: int = 50  # Minimum dataset size before optimization

    # Optimization parameters
    mipro_trials: int = 50  # Number of MIPROv2 trials
    min_improvement_threshold: float = 0.05  # Minimum 5% improvement to deploy

    # Paths
    base_dir: str = "."
    training_data_dir: str = "training_data"
    output_dir: str = "/tmp/optimization_runs"

    # Signatures to optimize
    signatures: List[str] = None

    def __post_init__(self):
        if self.signatures is None:
            self.signatures = [
                'extract_requirements',
                'validate_intent',
                'validate_completeness',
                'validate_correctness',
                'generate_guidance'
            ]


@dataclass
class OptimizationResult:
    """Result of optimization run for a single signature"""
    signature_name: str
    dataset_version: str
    total_examples: int
    baseline_score: float
    optimized_score: float
    improvement: float
    should_deploy: bool
    output_path: str
    timestamp: str
    notes: str = ""


@dataclass
class OrchestrationRun:
    """Complete orchestration run summary"""
    run_id: str
    start_time: str
    end_time: str
    config: OptimizationConfig
    results: List[OptimizationResult]
    success: bool
    error_message: Optional[str] = None


class OptimizationOrchestrator:
    """
    Orchestrate the monthly optimization pipeline.

    Workflow:
    1. Collect new training data from all sources
    2. Validate and score data quality
    3. Filter through quality gates
    4. Create new dataset versions
    5. Run MIPROv2 optimization
    6. Evaluate results vs. baseline
    7. Deploy improvements above threshold
    """

    def __init__(self, config: Optional[OptimizationConfig] = None):
        self.config = config or OptimizationConfig()
        self.run_id = datetime.now().strftime("%Y%m%d_%H%M%S")
        self.start_time = datetime.now().isoformat()
        self.results: List[OptimizationResult] = []

        # Ensure output directory exists
        Path(self.config.output_dir).mkdir(parents=True, exist_ok=True)

        # Set up logging to file
        log_file = Path(self.config.output_dir) / f"orchestration_{self.run_id}.log"
        file_handler = logging.FileHandler(log_file)
        file_handler.setFormatter(logging.Formatter('%(asctime)s - %(levelname)s - %(message)s'))
        logger.addHandler(file_handler)

        logger.info(f"Starting optimization orchestration run: {self.run_id}")

    def run(self) -> OrchestrationRun:
        """
        Execute complete optimization pipeline.

        Returns:
            OrchestrationRun with summary of all results
        """
        try:
            # Phase 1: Data Collection
            logger.info("="*60)
            logger.info("PHASE 1: Data Collection")
            logger.info("="*60)
            collected_data = self._collect_data()

            # Phase 2: Quality Validation & Gating
            logger.info("\n" + "="*60)
            logger.info("PHASE 2: Quality Validation & Gating")
            logger.info("="*60)
            validated_data = self._validate_and_gate(collected_data)

            # Phase 3: Dataset Versioning
            logger.info("\n" + "="*60)
            logger.info("PHASE 3: Dataset Versioning")
            logger.info("="*60)
            dataset_versions = self._create_dataset_versions(validated_data)

            # Phase 4: Optimization
            logger.info("\n" + "="*60)
            logger.info("PHASE 4: MIPROv2 Optimization")
            logger.info("="*60)
            self._run_optimization(dataset_versions)

            # Phase 5: Deployment Decision
            logger.info("\n" + "="*60)
            logger.info("PHASE 5: Deployment Decision")
            logger.info("="*60)
            self._make_deployment_decisions()

            end_time = datetime.now().isoformat()

            run_summary = OrchestrationRun(
                run_id=self.run_id,
                start_time=self.start_time,
                end_time=end_time,
                config=self.config,
                results=self.results,
                success=True
            )

            self._save_run_summary(run_summary)
            self._print_summary(run_summary)

            return run_summary

        except Exception as e:
            logger.error(f"Orchestration failed: {e}", exc_info=True)
            end_time = datetime.now().isoformat()

            run_summary = OrchestrationRun(
                run_id=self.run_id,
                start_time=self.start_time,
                end_time=end_time,
                config=self.config,
                results=self.results,
                success=False,
                error_message=str(e)
            )

            self._save_run_summary(run_summary)
            raise

    def _collect_data(self) -> Dict[str, List[str]]:
        """
        Collect training data from all sources.

        Returns:
            Dict mapping signature -> list of raw data file paths
        """
        collected = {sig: [] for sig in self.config.signatures}

        # 1. Git mining
        if self.config.git_mining_target > 0:
            logger.info(f"Mining git history (target: {self.config.git_mining_target} examples)...")
            try:
                output_dir = Path(self.config.output_dir) / f"git_mined_{self.run_id}"
                cmd = [
                    "uv", "run", "python3",
                    "data_collection/git_mining_pipeline.py",
                    "--target", str(self.config.git_mining_target),
                    "--since-days", "90",
                    "--output", str(output_dir)
                ]
                subprocess.run(cmd, check=True, cwd=self.config.base_dir)

                # Collect output files
                for sig in self.config.signatures:
                    sig_file = output_dir / f"{sig}_git.json"
                    if sig_file.exists():
                        collected[sig].append(str(sig_file))
                        logger.info(f"  ✓ {sig}: git data collected")
            except subprocess.CalledProcessError as e:
                logger.warning(f"Git mining failed: {e}")

        # 2. Synthetic generation
        if self.config.synthetic_target > 0:
            logger.info(f"Generating synthetic data (target: {self.config.synthetic_target} examples)...")
            try:
                output_dir = Path(self.config.output_dir) / f"synthetic_{self.run_id}"
                cmd = [
                    "uv", "run", "python3",
                    "data_collection/synthetic_data_generator.py",
                    "--target", str(self.config.synthetic_target),
                    "--output", str(output_dir)
                ]
                subprocess.run(cmd, check=True, cwd=self.config.base_dir)

                # Collect output files
                for sig in self.config.signatures:
                    sig_file = output_dir / f"{sig}_synthetic.json"
                    if sig_file.exists():
                        collected[sig].append(str(sig_file))
                        logger.info(f"  ✓ {sig}: synthetic data generated")
            except subprocess.CalledProcessError as e:
                logger.warning(f"Synthetic generation failed: {e}")

        # 3. Telemetry aggregation
        if self.config.telemetry_target > 0:
            logger.info(f"Aggregating production telemetry (target: {self.config.telemetry_target} examples)...")
            try:
                # Read telemetry config to get log file path
                config_path = Path(self.config.base_dir) / "src" / "orchestration" / "monitoring_config.json"
                if not config_path.exists():
                    logger.warning("monitoring_config.json not found, using default log path")
                    log_file = "logs/dspy_production.jsonl"
                else:
                    with open(config_path, 'r') as f:
                        telemetry_config = json.load(f)
                        log_file = telemetry_config.get('telemetry', {}).get('log_file_path', 'logs/dspy_production.jsonl')

                # Run telemetry aggregator
                output_dir = Path(self.config.output_dir) / f"telemetry_{self.run_id}"
                output_dir.mkdir(parents=True, exist_ok=True)

                cmd = [
                    "uv", "run", "python3",
                    "data_collection/telemetry_aggregator.py",
                    "--log-file", log_file,
                    "--output-dir", str(output_dir),
                    "--min-quality-score", "0.70"
                ]
                subprocess.run(cmd, check=True, cwd=self.config.base_dir)

                # Collect output files from versioned datasets
                # TelemetryAggregator writes to training_data/<signature>/v<version>/dataset.json
                training_data_path = Path(self.config.base_dir) / self.config.training_data_dir
                for sig in self.config.signatures:
                    sig_dir = training_data_path / sig
                    if sig_dir.exists():
                        # Get latest version with telemetry provenance
                        latest_link = sig_dir / "latest"
                        if latest_link.exists() and latest_link.is_symlink():
                            version_dir = latest_link.resolve()
                            provenance_file = version_dir / "provenance.jsonl"

                            # Check if this version includes telemetry data
                            if provenance_file.exists():
                                with open(provenance_file, 'r') as f:
                                    for line in f:
                                        entry = json.loads(line)
                                        if entry.get('source') == 'telemetry':
                                            dataset_file = version_dir / "dataset.json"
                                            if dataset_file.exists():
                                                collected[sig].append(str(dataset_file))
                                                logger.info(f"  ✓ {sig}: telemetry data collected")
                                            break

            except subprocess.CalledProcessError as e:
                logger.warning(f"Telemetry aggregation failed: {e}")
            except Exception as e:
                logger.warning(f"Telemetry aggregation error: {e}")

        return collected

    def _validate_and_gate(self, collected_data: Dict[str, List[str]]) -> Dict[str, str]:
        """
        Validate quality and filter through quality gates.

        Args:
            collected_data: Dict mapping signature -> list of raw data files

        Returns:
            Dict mapping signature -> path to validated+gated data file
        """
        validated = {}

        for sig, data_files in collected_data.items():
            if not data_files:
                logger.warning(f"{sig}: No data collected, skipping")
                continue

            # Merge all data files for this signature
            merged_data = []
            for data_file in data_files:
                with open(data_file, 'r') as f:
                    data = json.load(f)
                    merged_data.extend(data)

            logger.info(f"{sig}: Merged {len(merged_data)} examples from {len(data_files)} sources")

            # Run through quality gates
            output_file = Path(self.config.output_dir) / f"{sig}_validated_{self.run_id}.json"

            try:
                cmd = [
                    "uv", "run", "python3",
                    "data_collection/quality_gates.py",
                    "--input", "-",  # stdin
                    "--signature", sig,
                    "--output", str(output_file),
                    "--min-quality", str(self.config.min_quality_score)
                ]

                proc = subprocess.run(
                    cmd,
                    input=json.dumps(merged_data),
                    text=True,
                    capture_output=True,
                    check=True,
                    cwd=self.config.base_dir
                )

                logger.info(f"  ✓ {sig}: Quality gates passed")
                validated[sig] = str(output_file)

            except subprocess.CalledProcessError as e:
                logger.warning(f"{sig}: Quality gates failed: {e.stderr}")

        return validated

    def _create_dataset_versions(self, validated_data: Dict[str, str]) -> Dict[str, str]:
        """
        Create new dataset versions with validated data.

        Args:
            validated_data: Dict mapping signature -> validated data file path

        Returns:
            Dict mapping signature -> dataset version identifier
        """
        versions = {}

        for sig, data_file in validated_data.items():
            try:
                # Load validated data
                with open(data_file, 'r') as f:
                    examples = json.load(f)

                if len(examples) < self.config.min_total_examples:
                    logger.warning(
                        f"{sig}: Only {len(examples)} examples, "
                        f"minimum {self.config.min_total_examples} required"
                    )
                    continue

                # Create new version via DatasetManager
                cmd = [
                    "uv", "run", "python3",
                    "data_collection/dataset_manager.py",
                    "add",
                    "--signature", sig,
                    "--input", data_file,
                    "--source", f"orchestration_{self.run_id}",
                    "--notes", f"Monthly optimization run {self.run_id}"
                ]

                result = subprocess.run(
                    cmd,
                    capture_output=True,
                    text=True,
                    check=True,
                    cwd=self.config.base_dir
                )

                # Extract version from output
                # Output format: "Created version: YYYYMMDD_HHMMSS"
                for line in result.stdout.split('\n'):
                    if 'Created version:' in line:
                        version = line.split(':')[1].strip()
                        versions[sig] = version
                        logger.info(f"  ✓ {sig}: Dataset version {version} created")
                        break

            except Exception as e:
                logger.warning(f"{sig}: Dataset versioning failed: {e}")

        return versions

    def _run_optimization(self, dataset_versions: Dict[str, str]):
        """
        Run MIPROv2 optimization for each signature.

        Args:
            dataset_versions: Dict mapping signature -> version identifier
        """
        for sig, version in dataset_versions.items():
            logger.info(f"\nOptimizing {sig}...")

            try:
                # Run baseline benchmark first
                baseline_output = Path(self.config.output_dir) / f"{sig}_baseline_{self.run_id}.json"
                baseline_score = self._run_baseline(sig, str(baseline_output))

                # Run MIPROv2 optimization
                optimized_output = Path(self.config.output_dir) / f"{sig}_optimized_{self.run_id}.json"
                optimized_score = self._run_mipro(sig, str(optimized_output))

                # Calculate improvement
                improvement = (optimized_score - baseline_score) / baseline_score if baseline_score > 0 else 0
                should_deploy = improvement >= self.config.min_improvement_threshold

                # Load dataset metadata for example count
                dataset_path = Path(self.config.training_data_dir) / sig / f"v{version}" / "metadata.json"
                with open(dataset_path, 'r') as f:
                    metadata = json.load(f)

                result = OptimizationResult(
                    signature_name=sig,
                    dataset_version=version,
                    total_examples=metadata['total_examples'],
                    baseline_score=baseline_score,
                    optimized_score=optimized_score,
                    improvement=improvement,
                    should_deploy=should_deploy,
                    output_path=str(optimized_output),
                    timestamp=datetime.now().isoformat(),
                    notes=f"Trials: {self.config.mipro_trials}"
                )

                self.results.append(result)
                logger.info(f"  ✓ {sig}: Optimization complete (improvement: {improvement:.1%})")

            except Exception as e:
                logger.error(f"{sig}: Optimization failed: {e}", exc_info=True)

    def _run_baseline(self, signature: str, output_path: str) -> float:
        """Run baseline benchmark and return score"""
        logger.info(f"  Running baseline benchmark for {signature}...")

        cmd = [
            "uv", "run", "python3",
            "baseline_benchmark.py",
            "--module", "reviewer",  # Adjust based on signature
            "--iterations", "3",
            "--output", output_path
        ]

        subprocess.run(cmd, check=True, cwd=self.config.base_dir, capture_output=True)

        # Parse baseline score from output
        with open(output_path, 'r') as f:
            result = json.load(f)
            # Assuming structure: {"composite_score": 0.75, ...}
            return result.get('composite_score', 0.0)

    def _run_mipro(self, signature: str, output_path: str) -> float:
        """Run MIPROv2 optimization and return score"""
        logger.info(f"  Running MIPROv2 optimization for {signature}...")

        cmd = [
            "uv", "run", "python3",
            f"optimize_{signature}.py",
            "--trials", str(self.config.mipro_trials),
            "--output", output_path
        ]

        subprocess.run(cmd, check=True, cwd=self.config.base_dir, capture_output=True)

        # Parse optimized score from output
        with open(output_path, 'r') as f:
            result = json.load(f)
            return result.get('best_score', 0.0)

    def _make_deployment_decisions(self):
        """
        Make deployment decisions based on optimization results.
        """
        deploy_count = sum(1 for r in self.results if r.should_deploy)

        logger.info(f"\nDeployment decisions: {deploy_count}/{len(self.results)} signatures approved")

        for result in self.results:
            if result.should_deploy:
                logger.info(
                    f"  ✓ DEPLOY {result.signature_name}: "
                    f"{result.baseline_score:.1%} → {result.optimized_score:.1%} "
                    f"(+{result.improvement:.1%})"
                )
            else:
                logger.info(
                    f"  ✗ SKIP {result.signature_name}: "
                    f"{result.baseline_score:.1%} → {result.optimized_score:.1%} "
                    f"(+{result.improvement:.1%}, below {self.config.min_improvement_threshold:.1%} threshold)"
                )

    def _save_run_summary(self, run: OrchestrationRun):
        """Save orchestration run summary to JSON"""
        output_file = Path(self.config.output_dir) / f"orchestration_summary_{self.run_id}.json"

        with open(output_file, 'w') as f:
            json.dump(asdict(run), f, indent=2)

        logger.info(f"\nRun summary saved to: {output_file}")

    def _print_summary(self, run: OrchestrationRun):
        """Print human-readable summary"""
        print("\n" + "="*60)
        print(f"OPTIMIZATION ORCHESTRATION SUMMARY")
        print("="*60)
        print(f"Run ID: {run.run_id}")
        print(f"Start: {run.start_time}")
        print(f"End: {run.end_time}")
        print(f"Status: {'✓ SUCCESS' if run.success else '✗ FAILED'}")

        if run.error_message:
            print(f"Error: {run.error_message}")

        print(f"\nResults ({len(run.results)} signatures):")
        for result in run.results:
            status = "✓ DEPLOY" if result.should_deploy else "✗ SKIP"
            print(
                f"  {status} {result.signature_name}: "
                f"{result.baseline_score:.1%} → {result.optimized_score:.1%} "
                f"(+{result.improvement:.1%})"
            )

        deploy_count = sum(1 for r in run.results if r.should_deploy)
        print(f"\nDeployment: {deploy_count}/{len(run.results)} signatures approved")
        print("="*60 + "\n")


def main():
    """CLI for optimization orchestrator"""
    import argparse

    parser = argparse.ArgumentParser(description="DSPy Monthly Optimization Orchestrator")
    parser.add_argument('--config', help='Config JSON file')
    parser.add_argument('--git-target', type=int, help='Git mining target')
    parser.add_argument('--synthetic-target', type=int, help='Synthetic generation target')
    parser.add_argument('--trials', type=int, help='MIPROv2 trials')
    parser.add_argument('--output-dir', help='Output directory')
    parser.add_argument('--signatures', nargs='+', help='Signatures to optimize')

    args = parser.parse_args()

    # Load config
    if args.config:
        with open(args.config, 'r') as f:
            config_dict = json.load(f)
        config = OptimizationConfig(**config_dict)
    else:
        config = OptimizationConfig()

    # Apply CLI overrides
    if args.git_target is not None:
        config.git_mining_target = args.git_target
    if args.synthetic_target is not None:
        config.synthetic_target = args.synthetic_target
    if args.trials is not None:
        config.mipro_trials = args.trials
    if args.output_dir:
        config.output_dir = args.output_dir
    if args.signatures:
        config.signatures = args.signatures

    # Run orchestration
    orchestrator = OptimizationOrchestrator(config)
    run = orchestrator.run()

    # Exit with appropriate code
    sys.exit(0 if run.success else 1)


if __name__ == '__main__':
    main()
