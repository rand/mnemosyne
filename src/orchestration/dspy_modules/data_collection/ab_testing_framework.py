#!/usr/bin/env python3
"""
A/B Testing Framework for DSPy Module Deployment

Enables safe production deployment of optimized DSPy modules through:
- Gradual traffic rollout (e.g., 10% → 50% → 100%)
- Real-time performance monitoring
- Automatic rollback on degradation
- Manual control and emergency stops
- Experiment state tracking and history

Integrates with Rust DSPy adapter for production routing.
"""

import json
import os
import sys
import time
from dataclasses import dataclass, asdict
from datetime import datetime, timedelta
from pathlib import Path
from typing import List, Dict, Any, Optional, Literal
from enum import Enum
import logging

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)


class ExperimentStatus(str, Enum):
    """Experiment lifecycle states"""
    PENDING = "pending"         # Configured but not started
    RUNNING = "running"         # Active traffic split
    PAUSED = "paused"          # Temporarily suspended
    COMPLETED = "completed"     # Successfully promoted to 100%
    ROLLED_BACK = "rolled_back" # Degradation detected, reverted
    FAILED = "failed"          # Setup or execution failure


@dataclass
class ExperimentConfig:
    """Configuration for A/B testing experiment"""
    experiment_id: str
    signature_name: str

    # Module versions
    baseline_module_path: str  # Path to baseline DSPy module JSON
    candidate_module_path: str  # Path to optimized DSPy module JSON

    # Traffic split
    candidate_traffic_percent: float = 10.0  # Start with 10% traffic

    # Rollout schedule (gradual increase)
    rollout_stages: List[float] = None  # e.g., [10, 25, 50, 100]
    stage_duration_minutes: int = 60    # Time at each stage before promotion

    # Safety thresholds
    min_requests_per_stage: int = 100   # Minimum requests before evaluation
    max_error_rate_delta: float = 0.05  # Max 5% increase in errors
    max_latency_delta_ms: float = 500   # Max 500ms latency increase
    min_quality_score: float = 0.70     # Minimum quality score threshold

    # Monitoring
    metrics_window_minutes: int = 10    # Rolling window for metrics
    check_interval_seconds: int = 60    # How often to check metrics

    def __post_init__(self):
        if self.rollout_stages is None:
            self.rollout_stages = [10.0, 25.0, 50.0, 75.0, 100.0]


@dataclass
class MetricsSnapshot:
    """Performance metrics at a point in time"""
    timestamp: str
    variant: Literal["baseline", "candidate"]

    # Volume
    request_count: int

    # Quality
    success_count: int
    error_count: int
    error_rate: float

    # Latency
    avg_latency_ms: float
    p50_latency_ms: float
    p95_latency_ms: float
    p99_latency_ms: float

    # Task-specific quality (if available)
    avg_quality_score: Optional[float] = None


@dataclass
class ExperimentSnapshot:
    """Complete state snapshot of experiment"""
    experiment_id: str
    status: ExperimentStatus
    current_stage: int
    candidate_traffic_percent: float

    # Metrics
    baseline_metrics: Optional[MetricsSnapshot]
    candidate_metrics: Optional[MetricsSnapshot]

    # Comparisons
    error_rate_delta: Optional[float]
    latency_delta_ms: Optional[float]
    quality_score_delta: Optional[float]

    # Decision
    should_promote: bool
    should_rollback: bool
    rollback_reason: Optional[str]

    timestamp: str


@dataclass
class ExperimentHistory:
    """Complete history of experiment lifecycle"""
    config: ExperimentConfig
    snapshots: List[ExperimentSnapshot]
    final_status: ExperimentStatus
    start_time: str
    end_time: Optional[str]
    notes: str = ""


class ABTestingFramework:
    """
    A/B testing framework for DSPy module deployment.

    Usage:
        # Create experiment
        config = ExperimentConfig(
            experiment_id="reviewer_v1_test",
            signature_name="validate_intent",
            baseline_module_path="modules/reviewer_baseline.json",
            candidate_module_path="modules/reviewer_optimized_v1.json"
        )

        framework = ABTestingFramework(config)

        # Start experiment
        framework.start_experiment()

        # Monitor (in loop or separate process)
        while framework.is_running():
            framework.check_and_update()
            time.sleep(60)

        # Or manual control
        framework.pause_experiment()
        framework.resume_experiment()
        framework.promote_candidate()
        framework.rollback_experiment("Manual rollback")
    """

    def __init__(self, config: ExperimentConfig, state_dir: str = ".ab_experiments"):
        self.config = config
        self.state_dir = Path(state_dir)
        self.state_dir.mkdir(exist_ok=True)

        # State
        self.status = ExperimentStatus.PENDING
        self.current_stage = 0
        self.stage_start_time: Optional[datetime] = None
        self.snapshots: List[ExperimentSnapshot] = []
        self.start_time: Optional[str] = None

        # Load existing state if resuming
        self._load_state()

    def _get_state_file(self) -> Path:
        """Get path to experiment state file"""
        return self.state_dir / f"{self.config.experiment_id}.json"

    def _save_state(self):
        """Persist experiment state to disk"""
        history = ExperimentHistory(
            config=self.config,
            snapshots=self.snapshots,
            final_status=self.status,
            start_time=self.start_time or datetime.now().isoformat(),
            end_time=datetime.now().isoformat() if self.status in [
                ExperimentStatus.COMPLETED,
                ExperimentStatus.ROLLED_BACK,
                ExperimentStatus.FAILED
            ] else None
        )

        with open(self._get_state_file(), 'w') as f:
            json.dump(asdict(history), f, indent=2)

    def _load_state(self):
        """Load existing experiment state if resuming"""
        state_file = self._get_state_file()
        if not state_file.exists():
            return

        with open(state_file, 'r') as f:
            data = json.load(f)
            history = ExperimentHistory(**data)

            self.config = history.config
            self.snapshots = history.snapshots
            self.status = ExperimentStatus(history.final_status)
            self.start_time = history.start_time

            if history.snapshots:
                latest = history.snapshots[-1]
                self.current_stage = latest.current_stage

            logger.info(f"Loaded experiment state: {self.status}, stage {self.current_stage}")

    def start_experiment(self):
        """Start the A/B test"""
        if self.status not in [ExperimentStatus.PENDING, ExperimentStatus.PAUSED]:
            raise ValueError(f"Cannot start experiment in status: {self.status}")

        self.status = ExperimentStatus.RUNNING
        self.start_time = datetime.now().isoformat()
        self.current_stage = 0
        self.stage_start_time = datetime.now()

        # Set initial traffic split
        self.config.candidate_traffic_percent = self.config.rollout_stages[0]

        # Update routing configuration (integration point with Rust)
        self._update_routing_config()

        self._save_state()
        logger.info(
            f"Started experiment {self.config.experiment_id}: "
            f"{self.config.candidate_traffic_percent}% to candidate"
        )

    def check_and_update(self) -> Optional[ExperimentSnapshot]:
        """
        Check metrics and update experiment state.

        Returns:
            Latest snapshot if check was performed, None otherwise
        """
        if self.status != ExperimentStatus.RUNNING:
            return None

        # Collect metrics
        baseline_metrics = self._collect_metrics("baseline")
        candidate_metrics = self._collect_metrics("candidate")

        # Check if we have minimum requests
        if candidate_metrics.request_count < self.config.min_requests_per_stage:
            logger.info(
                f"Insufficient requests: {candidate_metrics.request_count} < "
                f"{self.config.min_requests_per_stage}"
            )
            return None

        # Compare metrics
        error_rate_delta = candidate_metrics.error_rate - baseline_metrics.error_rate
        latency_delta_ms = candidate_metrics.avg_latency_ms - baseline_metrics.avg_latency_ms
        quality_score_delta = None

        if (candidate_metrics.avg_quality_score is not None and
            baseline_metrics.avg_quality_score is not None):
            quality_score_delta = (
                candidate_metrics.avg_quality_score - baseline_metrics.avg_quality_score
            )

        # Decide: promote, rollback, or continue
        should_rollback = False
        rollback_reason = None
        should_promote = False

        # Check for degradation
        if error_rate_delta > self.config.max_error_rate_delta:
            should_rollback = True
            rollback_reason = (
                f"Error rate increased by {error_rate_delta:.1%} "
                f"(threshold: {self.config.max_error_rate_delta:.1%})"
            )
        elif latency_delta_ms > self.config.max_latency_delta_ms:
            should_rollback = True
            rollback_reason = (
                f"Latency increased by {latency_delta_ms:.0f}ms "
                f"(threshold: {self.config.max_latency_delta_ms:.0f}ms)"
            )
        elif (candidate_metrics.avg_quality_score is not None and
              candidate_metrics.avg_quality_score < self.config.min_quality_score):
            should_rollback = True
            rollback_reason = (
                f"Quality score {candidate_metrics.avg_quality_score:.1%} below "
                f"threshold {self.config.min_quality_score:.1%}"
            )

        # Check if stage duration elapsed and no issues
        stage_elapsed = (datetime.now() - self.stage_start_time).total_seconds() / 60
        if not should_rollback and stage_elapsed >= self.config.stage_duration_minutes:
            should_promote = True

        # Create snapshot
        snapshot = ExperimentSnapshot(
            experiment_id=self.config.experiment_id,
            status=self.status,
            current_stage=self.current_stage,
            candidate_traffic_percent=self.config.candidate_traffic_percent,
            baseline_metrics=baseline_metrics,
            candidate_metrics=candidate_metrics,
            error_rate_delta=error_rate_delta,
            latency_delta_ms=latency_delta_ms,
            quality_score_delta=quality_score_delta,
            should_promote=should_promote,
            should_rollback=should_rollback,
            rollback_reason=rollback_reason,
            timestamp=datetime.now().isoformat()
        )

        self.snapshots.append(snapshot)

        # Take action
        if should_rollback:
            self.rollback_experiment(rollback_reason)
        elif should_promote:
            self._promote_to_next_stage()

        self._save_state()

        return snapshot

    def _promote_to_next_stage(self):
        """Promote candidate to next traffic percentage stage"""
        if self.current_stage >= len(self.config.rollout_stages) - 1:
            # Already at 100%, mark complete
            self.status = ExperimentStatus.COMPLETED
            logger.info(f"Experiment {self.config.experiment_id} completed successfully!")
            return

        self.current_stage += 1
        self.config.candidate_traffic_percent = self.config.rollout_stages[self.current_stage]
        self.stage_start_time = datetime.now()

        self._update_routing_config()

        logger.info(
            f"Promoted to stage {self.current_stage}: "
            f"{self.config.candidate_traffic_percent}% to candidate"
        )

    def promote_candidate(self):
        """Manually promote candidate to 100% (emergency override)"""
        if self.status != ExperimentStatus.RUNNING:
            raise ValueError(f"Cannot promote in status: {self.status}")

        self.current_stage = len(self.config.rollout_stages) - 1
        self.config.candidate_traffic_percent = 100.0
        self.status = ExperimentStatus.COMPLETED

        self._update_routing_config()
        self._save_state()

        logger.info(f"Manually promoted candidate to 100%")

    def rollback_experiment(self, reason: str):
        """Rollback to baseline (0% candidate traffic)"""
        if self.status == ExperimentStatus.ROLLED_BACK:
            logger.warning("Already rolled back")
            return

        self.status = ExperimentStatus.ROLLED_BACK
        self.config.candidate_traffic_percent = 0.0

        # Revert routing to baseline
        self._update_routing_config()
        self._save_state()

        logger.error(f"ROLLBACK: {reason}")

    def pause_experiment(self):
        """Pause experiment (stop traffic split, freeze state)"""
        if self.status != ExperimentStatus.RUNNING:
            raise ValueError(f"Cannot pause from status: {self.status}")

        self.status = ExperimentStatus.PAUSED
        self._save_state()

        logger.info("Experiment paused")

    def resume_experiment(self):
        """Resume paused experiment"""
        if self.status != ExperimentStatus.PAUSED:
            raise ValueError(f"Cannot resume from status: {self.status}")

        self.status = ExperimentStatus.RUNNING
        self.stage_start_time = datetime.now()  # Reset stage timer

        self._save_state()

        logger.info("Experiment resumed")

    def is_running(self) -> bool:
        """Check if experiment is actively running"""
        return self.status == ExperimentStatus.RUNNING

    def _collect_metrics(self, variant: Literal["baseline", "candidate"]) -> MetricsSnapshot:
        """
        Collect metrics for a variant from production system.

        Integration point: This should query the Rust telemetry system
        for actual production metrics within the metrics window.

        For now, returns mock data structure.
        """
        # TODO: Integrate with Rust telemetry/metrics system
        # Query metrics from last N minutes for this variant

        # Mock implementation
        return MetricsSnapshot(
            timestamp=datetime.now().isoformat(),
            variant=variant,
            request_count=0,
            success_count=0,
            error_count=0,
            error_rate=0.0,
            avg_latency_ms=0.0,
            p50_latency_ms=0.0,
            p95_latency_ms=0.0,
            p99_latency_ms=0.0,
            avg_quality_score=None
        )

    def _update_routing_config(self):
        """
        Update routing configuration in production system.

        Integration point: This should call into the Rust DSPy adapter
        to update the traffic split configuration.

        Expected format for Rust side:
        {
            "experiment_id": "...",
            "signature": "...",
            "baseline_module": "path/to/module.json",
            "candidate_module": "path/to/module.json",
            "candidate_traffic_percent": 10.0
        }
        """
        routing_config = {
            "experiment_id": self.config.experiment_id,
            "signature": self.config.signature_name,
            "baseline_module": self.config.baseline_module_path,
            "candidate_module": self.config.candidate_module_path,
            "candidate_traffic_percent": self.config.candidate_traffic_percent,
            "status": self.status.value
        }

        # Write to file that Rust can monitor
        routing_file = self.state_dir / "routing_config.json"
        with open(routing_file, 'w') as f:
            json.dump(routing_config, f, indent=2)

        logger.info(f"Updated routing config: {self.config.candidate_traffic_percent}% to candidate")


def main():
    """CLI for A/B testing framework"""
    import argparse

    parser = argparse.ArgumentParser(description="DSPy A/B Testing Framework")
    subparsers = parser.add_subparsers(dest='command', help='Command to run')

    # Start experiment
    start_parser = subparsers.add_parser('start', help='Start A/B test')
    start_parser.add_argument('--experiment-id', required=True, help='Unique experiment ID')
    start_parser.add_argument('--signature', required=True, help='DSPy signature name')
    start_parser.add_argument('--baseline', required=True, help='Baseline module JSON path')
    start_parser.add_argument('--candidate', required=True, help='Candidate module JSON path')
    start_parser.add_argument('--config', help='Config JSON file (optional)')

    # Monitor experiment
    monitor_parser = subparsers.add_parser('monitor', help='Monitor running experiment')
    monitor_parser.add_argument('--experiment-id', required=True, help='Experiment ID')
    monitor_parser.add_argument('--daemon', action='store_true', help='Run as daemon')

    # Control commands
    pause_parser = subparsers.add_parser('pause', help='Pause experiment')
    pause_parser.add_argument('--experiment-id', required=True, help='Experiment ID')

    resume_parser = subparsers.add_parser('resume', help='Resume experiment')
    resume_parser.add_argument('--experiment-id', required=True, help='Experiment ID')

    promote_parser = subparsers.add_parser('promote', help='Promote candidate to 100%')
    promote_parser.add_argument('--experiment-id', required=True, help='Experiment ID')

    rollback_parser = subparsers.add_parser('rollback', help='Rollback to baseline')
    rollback_parser.add_argument('--experiment-id', required=True, help='Experiment ID')
    rollback_parser.add_argument('--reason', required=True, help='Rollback reason')

    # Status
    status_parser = subparsers.add_parser('status', help='Show experiment status')
    status_parser.add_argument('--experiment-id', required=True, help='Experiment ID')

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        sys.exit(1)

    if args.command == 'start':
        # Create config
        if args.config:
            with open(args.config, 'r') as f:
                config_dict = json.load(f)
            config = ExperimentConfig(**config_dict)
        else:
            config = ExperimentConfig(
                experiment_id=args.experiment_id,
                signature_name=args.signature,
                baseline_module_path=args.baseline,
                candidate_module_path=args.candidate
            )

        framework = ABTestingFramework(config)
        framework.start_experiment()
        print(f"✓ Started experiment: {args.experiment_id}")

    elif args.command == 'monitor':
        # Load experiment
        config = ExperimentConfig(
            experiment_id=args.experiment_id,
            signature_name="",  # Will be loaded from state
            baseline_module_path="",
            candidate_module_path=""
        )
        framework = ABTestingFramework(config)

        if args.daemon:
            # Run continuous monitoring
            print(f"Monitoring experiment: {args.experiment_id}")
            while framework.is_running():
                snapshot = framework.check_and_update()
                if snapshot:
                    print(f"[{snapshot.timestamp}] Stage {snapshot.current_stage}: "
                          f"{snapshot.candidate_traffic_percent}% candidate")
                    if snapshot.should_rollback:
                        print(f"  ⚠ ROLLBACK: {snapshot.rollback_reason}")
                        break
                    elif snapshot.should_promote:
                        print(f"  ✓ Promoting to next stage")

                time.sleep(framework.config.check_interval_seconds)
        else:
            # Single check
            snapshot = framework.check_and_update()
            if snapshot:
                print(json.dumps(asdict(snapshot), indent=2))

    elif args.command == 'pause':
        config = ExperimentConfig(experiment_id=args.experiment_id, signature_name="",
                                  baseline_module_path="", candidate_module_path="")
        framework = ABTestingFramework(config)
        framework.pause_experiment()
        print(f"✓ Paused experiment: {args.experiment_id}")

    elif args.command == 'resume':
        config = ExperimentConfig(experiment_id=args.experiment_id, signature_name="",
                                  baseline_module_path="", candidate_module_path="")
        framework = ABTestingFramework(config)
        framework.resume_experiment()
        print(f"✓ Resumed experiment: {args.experiment_id}")

    elif args.command == 'promote':
        config = ExperimentConfig(experiment_id=args.experiment_id, signature_name="",
                                  baseline_module_path="", candidate_module_path="")
        framework = ABTestingFramework(config)
        framework.promote_candidate()
        print(f"✓ Promoted candidate to 100%: {args.experiment_id}")

    elif args.command == 'rollback':
        config = ExperimentConfig(experiment_id=args.experiment_id, signature_name="",
                                  baseline_module_path="", candidate_module_path="")
        framework = ABTestingFramework(config)
        framework.rollback_experiment(args.reason)
        print(f"✓ Rolled back to baseline: {args.experiment_id}")

    elif args.command == 'status':
        config = ExperimentConfig(experiment_id=args.experiment_id, signature_name="",
                                  baseline_module_path="", candidate_module_path="")
        framework = ABTestingFramework(config)

        print(f"Experiment: {framework.config.experiment_id}")
        print(f"Status: {framework.status.value}")
        print(f"Stage: {framework.current_stage}/{len(framework.config.rollout_stages)}")
        print(f"Candidate traffic: {framework.config.candidate_traffic_percent}%")
        print(f"Snapshots: {len(framework.snapshots)}")

        if framework.snapshots:
            latest = framework.snapshots[-1]
            print(f"\nLatest metrics:")
            print(f"  Error rate delta: {latest.error_rate_delta:+.2%}")
            print(f"  Latency delta: {latest.latency_delta_ms:+.0f}ms")
            if latest.quality_score_delta is not None:
                print(f"  Quality delta: {latest.quality_score_delta:+.2%}")


if __name__ == '__main__':
    main()
