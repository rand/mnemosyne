#!/usr/bin/env python3
"""
DSPy Optimization Pipeline Monitoring and Alerting System

Provides comprehensive monitoring and alerting for:
- Orchestration run health (success/failure/performance)
- A/B test status (active experiments, rollbacks)
- Dataset quality trends (degradation detection)
- Optimization performance (improvement tracking)

Alert destinations:
- Email (SMTP)
- Slack webhooks
- File-based alerts (for cron integration)
- Stdout/logs

Usage:
    # Monitor orchestration runs
    python monitoring.py check-orchestration --run-id <run_id>

    # Monitor A/B tests
    python monitoring.py check-ab-tests

    # Monitor dataset quality
    python monitoring.py check-dataset-quality --signature <name>

    # Run all checks (for cron)
    python monitoring.py check-all

    # Dashboard view
    python monitoring.py dashboard
"""

import argparse
import json
import logging
import smtplib
import sys
from dataclasses import dataclass, asdict
from datetime import datetime, timedelta
from email.mime.text import MIMEText
from email.mime.multipart import MIMEMultipart
from enum import Enum
from pathlib import Path
from typing import Dict, List, Optional, Any
import requests

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='[%(asctime)s] %(levelname)s: %(message)s',
    datefmt='%Y-%m-%d %H:%M:%S'
)
logger = logging.getLogger(__name__)


class AlertLevel(str, Enum):
    """Alert severity levels"""
    INFO = "info"
    WARNING = "warning"
    ERROR = "error"
    CRITICAL = "critical"


class AlertChannel(str, Enum):
    """Alert delivery channels"""
    EMAIL = "email"
    SLACK = "slack"
    FILE = "file"
    LOG = "log"


@dataclass
class Alert:
    """Alert message"""
    level: AlertLevel
    title: str
    message: str
    timestamp: str
    details: Dict[str, Any]
    channels: List[AlertChannel]


@dataclass
class MonitoringConfig:
    """Monitoring configuration"""
    # Alert thresholds
    orchestration_failure_threshold: int = 2  # Consecutive failures before alert
    ab_test_rollback_alert: bool = True
    dataset_quality_drop_threshold: float = 0.10  # 10% drop triggers alert
    optimization_no_improvement_threshold: int = 3  # Months without improvement

    # Alert destinations
    email_enabled: bool = False
    email_smtp_server: str = "localhost"
    email_smtp_port: int = 587
    email_from: str = "dspy-monitoring@localhost"
    email_to: List[str] = None
    email_username: Optional[str] = None
    email_password: Optional[str] = None

    slack_enabled: bool = False
    slack_webhook_url: Optional[str] = None

    file_alerts_enabled: bool = True
    file_alerts_dir: str = "/tmp/dspy_alerts"

    # Data sources
    orchestration_output_dir: str = "/tmp/optimization_runs"
    ab_experiments_dir: str = ".ab_experiments"
    training_data_dir: str = "training_data"

    # State tracking
    state_file: str = ".monitoring_state.json"

    def __post_init__(self):
        if self.email_to is None:
            self.email_to = []


class MonitoringState:
    """Persistent state for monitoring"""

    def __init__(self, state_file: str):
        self.state_file = Path(state_file)
        self.state = self._load()

    def _load(self) -> Dict[str, Any]:
        """Load state from file"""
        if self.state_file.exists():
            try:
                with open(self.state_file) as f:
                    return json.load(f)
            except Exception as e:
                logger.warning(f"Failed to load state: {e}")

        # Default state
        return {
            "last_check": None,
            "consecutive_orchestration_failures": 0,
            "last_orchestration_run": None,
            "last_successful_optimization": None,
            "dataset_quality_baselines": {},
            "ab_test_snapshots": {},
            "alerts_sent": []
        }

    def save(self):
        """Save state to file"""
        try:
            with open(self.state_file, 'w') as f:
                json.dump(self.state, f, indent=2)
        except Exception as e:
            logger.error(f"Failed to save state: {e}")

    def get(self, key: str, default: Any = None) -> Any:
        """Get state value"""
        return self.state.get(key, default)

    def set(self, key: str, value: Any):
        """Set state value"""
        self.state[key] = value
        self.save()

    def update(self, updates: Dict[str, Any]):
        """Update multiple state values"""
        self.state.update(updates)
        self.save()


class AlertManager:
    """Manages alert delivery to multiple channels"""

    def __init__(self, config: MonitoringConfig):
        self.config = config

        # Create alerts directory
        if config.file_alerts_enabled:
            Path(config.file_alerts_dir).mkdir(parents=True, exist_ok=True)

    def send_alert(self, alert: Alert):
        """Send alert to configured channels"""
        logger.info(f"Sending {alert.level.upper()} alert: {alert.title}")

        for channel in alert.channels:
            try:
                if channel == AlertChannel.EMAIL and self.config.email_enabled:
                    self._send_email(alert)
                elif channel == AlertChannel.SLACK and self.config.slack_enabled:
                    self._send_slack(alert)
                elif channel == AlertChannel.FILE and self.config.file_alerts_enabled:
                    self._write_file(alert)
                elif channel == AlertChannel.LOG:
                    self._log_alert(alert)
            except Exception as e:
                logger.error(f"Failed to send alert via {channel}: {e}")

    def _send_email(self, alert: Alert):
        """Send email alert"""
        if not self.config.email_to:
            logger.warning("No email recipients configured")
            return

        msg = MIMEMultipart()
        msg['From'] = self.config.email_from
        msg['To'] = ", ".join(self.config.email_to)
        msg['Subject'] = f"[{alert.level.upper()}] {alert.title}"

        body = f"""
{alert.message}

Timestamp: {alert.timestamp}
Level: {alert.level.upper()}

Details:
{json.dumps(alert.details, indent=2)}
"""
        msg.attach(MIMEText(body, 'plain'))

        server = smtplib.SMTP(self.config.email_smtp_server, self.config.email_smtp_port)
        server.starttls()

        if self.config.email_username and self.config.email_password:
            server.login(self.config.email_username, self.config.email_password)

        server.send_message(msg)
        server.quit()

        logger.info(f"Email sent to {len(self.config.email_to)} recipients")

    def _send_slack(self, alert: Alert):
        """Send Slack webhook alert"""
        if not self.config.slack_webhook_url:
            logger.warning("No Slack webhook URL configured")
            return

        # Color mapping
        colors = {
            AlertLevel.INFO: "#36a64f",
            AlertLevel.WARNING: "#ff9900",
            AlertLevel.ERROR: "#ff0000",
            AlertLevel.CRITICAL: "#990000"
        }

        payload = {
            "attachments": [{
                "color": colors.get(alert.level, "#808080"),
                "title": alert.title,
                "text": alert.message,
                "fields": [
                    {"title": key, "value": str(value), "short": True}
                    for key, value in alert.details.items()
                ],
                "footer": "DSPy Monitoring",
                "ts": int(datetime.fromisoformat(alert.timestamp).timestamp())
            }]
        }

        response = requests.post(self.config.slack_webhook_url, json=payload)
        response.raise_for_status()

        logger.info("Slack webhook sent successfully")

    def _write_file(self, alert: Alert):
        """Write alert to file"""
        timestamp = datetime.fromisoformat(alert.timestamp).strftime("%Y%m%d_%H%M%S")
        filename = f"alert_{alert.level}_{timestamp}.json"
        filepath = Path(self.config.file_alerts_dir) / filename

        with open(filepath, 'w') as f:
            json.dump(asdict(alert), f, indent=2)

        logger.info(f"Alert written to {filepath}")

    def _log_alert(self, alert: Alert):
        """Log alert to stdout/logging"""
        log_level = {
            AlertLevel.INFO: logging.INFO,
            AlertLevel.WARNING: logging.WARNING,
            AlertLevel.ERROR: logging.ERROR,
            AlertLevel.CRITICAL: logging.CRITICAL
        }.get(alert.level, logging.INFO)

        logger.log(log_level, f"{alert.title}: {alert.message}")


class PipelineMonitor:
    """Monitors DSPy optimization pipeline health"""

    def __init__(self, config: MonitoringConfig):
        self.config = config
        self.state = MonitoringState(config.state_file)
        self.alert_manager = AlertManager(config)

    def check_orchestration_runs(self) -> List[Alert]:
        """Check orchestration run health"""
        alerts = []
        output_dir = Path(self.config.orchestration_output_dir)

        if not output_dir.exists():
            return alerts

        # Find recent orchestration summaries
        summaries = sorted(
            output_dir.glob("orchestration_summary_*.json"),
            key=lambda p: p.stat().st_mtime,
            reverse=True
        )

        if not summaries:
            logger.info("No orchestration runs found")
            return alerts

        # Check most recent run
        latest = summaries[0]
        try:
            with open(latest) as f:
                summary = json.load(f)

            run_id = summary.get("run_id", "unknown")
            success = summary.get("success", False)
            timestamp = summary.get("timestamp", "unknown")

            if success:
                # Reset failure counter
                self.state.set("consecutive_orchestration_failures", 0)
                self.state.set("last_successful_optimization", timestamp)

                # Check for improvements
                results = summary.get("optimization_results", {})
                improvements = {
                    sig: res for sig, res in results.items()
                    if res.get("deployed", False)
                }

                if improvements:
                    alerts.append(Alert(
                        level=AlertLevel.INFO,
                        title="Optimization Improvements Deployed",
                        message=f"Successfully deployed improvements for {len(improvements)} signatures",
                        timestamp=datetime.now().isoformat(),
                        details={
                            "run_id": run_id,
                            "signatures": list(improvements.keys()),
                            "improvements": {
                                sig: f"{res.get('improvement_percent', 0):.1f}%"
                                for sig, res in improvements.items()
                            }
                        },
                        channels=[AlertChannel.LOG, AlertChannel.FILE]
                    ))
            else:
                # Increment failure counter
                failures = self.state.get("consecutive_orchestration_failures", 0) + 1
                self.state.set("consecutive_orchestration_failures", failures)

                error = summary.get("error", "Unknown error")

                if failures >= self.config.orchestration_failure_threshold:
                    alerts.append(Alert(
                        level=AlertLevel.ERROR,
                        title="Orchestration Run Failures",
                        message=f"Orchestration has failed {failures} consecutive times",
                        timestamp=datetime.now().isoformat(),
                        details={
                            "run_id": run_id,
                            "consecutive_failures": failures,
                            "last_error": error,
                            "timestamp": timestamp
                        },
                        channels=[AlertChannel.EMAIL, AlertChannel.SLACK, AlertChannel.FILE, AlertChannel.LOG]
                    ))

            self.state.set("last_orchestration_run", run_id)

        except Exception as e:
            logger.error(f"Failed to check orchestration run: {e}")
            alerts.append(Alert(
                level=AlertLevel.WARNING,
                title="Orchestration Monitoring Error",
                message=f"Failed to parse orchestration summary: {e}",
                timestamp=datetime.now().isoformat(),
                details={"error": str(e), "file": str(latest)},
                channels=[AlertChannel.LOG, AlertChannel.FILE]
            ))

        # Check for stale optimizations (no improvement in N months)
        last_success = self.state.get("last_successful_optimization")
        if last_success:
            last_dt = datetime.fromisoformat(last_success)
            months_since = (datetime.now() - last_dt).days / 30

            if months_since >= self.config.optimization_no_improvement_threshold:
                alerts.append(Alert(
                    level=AlertLevel.WARNING,
                    title="No Recent Optimization Improvements",
                    message=f"No optimization improvements in {months_since:.1f} months",
                    timestamp=datetime.now().isoformat(),
                    details={
                        "last_success": last_success,
                        "months_since": f"{months_since:.1f}"
                    },
                    channels=[AlertChannel.EMAIL, AlertChannel.LOG, AlertChannel.FILE]
                ))

        return alerts

    def check_ab_tests(self) -> List[Alert]:
        """Check A/B test status"""
        alerts = []
        experiments_dir = Path(self.config.ab_experiments_dir)

        if not experiments_dir.exists():
            return alerts

        # Find active experiments
        experiment_files = list(experiments_dir.glob("*.json"))

        for exp_file in experiment_files:
            try:
                with open(exp_file) as f:
                    experiment = json.load(f)

                exp_id = experiment.get("experiment_id", exp_file.stem)
                status = experiment.get("status", "unknown")
                signature = experiment.get("signature_name", "unknown")

                # Check for rollbacks
                if status == "rolled_back" and self.config.ab_test_rollback_alert:
                    # Check if we've already alerted
                    alerted_key = f"ab_rollback_{exp_id}"
                    if alerted_key not in self.state.get("alerts_sent", []):
                        rollback_reason = experiment.get("rollback_reason", "Unknown")

                        alerts.append(Alert(
                            level=AlertLevel.WARNING,
                            title="A/B Test Rolled Back",
                            message=f"Experiment {exp_id} for {signature} was rolled back",
                            timestamp=datetime.now().isoformat(),
                            details={
                                "experiment_id": exp_id,
                                "signature": signature,
                                "reason": rollback_reason,
                                "final_traffic_percent": experiment.get("candidate_traffic_percent", 0)
                            },
                            channels=[AlertChannel.EMAIL, AlertChannel.SLACK, AlertChannel.FILE, AlertChannel.LOG]
                        ))

                        # Mark as alerted
                        alerted = self.state.get("alerts_sent", [])
                        alerted.append(alerted_key)
                        self.state.set("alerts_sent", alerted)

                # Check for long-running experiments (>7 days)
                created_at = experiment.get("created_at")
                if created_at and status == "running":
                    created_dt = datetime.fromisoformat(created_at)
                    age_days = (datetime.now() - created_dt).days

                    if age_days > 7:
                        alerts.append(Alert(
                            level=AlertLevel.INFO,
                            title="Long-Running A/B Test",
                            message=f"Experiment {exp_id} has been running for {age_days} days",
                            timestamp=datetime.now().isoformat(),
                            details={
                                "experiment_id": exp_id,
                                "signature": signature,
                                "age_days": age_days,
                                "traffic_percent": experiment.get("candidate_traffic_percent", 0)
                            },
                            channels=[AlertChannel.LOG, AlertChannel.FILE]
                        ))

            except Exception as e:
                logger.error(f"Failed to check A/B test {exp_file}: {e}")

        return alerts

    def check_dataset_quality(self, signature_name: Optional[str] = None) -> List[Alert]:
        """Check dataset quality trends"""
        alerts = []
        training_data_dir = Path(self.config.training_data_dir)

        if not training_data_dir.exists():
            return alerts

        # Determine which signatures to check
        if signature_name:
            signatures = [signature_name]
        else:
            signatures = [
                d.name for d in training_data_dir.iterdir()
                if d.is_dir() and not d.name.startswith('.')
            ]

        for sig in signatures:
            sig_dir = training_data_dir / sig

            # Find version directories
            versions = sorted(
                [d for d in sig_dir.iterdir() if d.is_dir() and d.name.startswith('v')],
                key=lambda d: d.name,
                reverse=True
            )

            if len(versions) < 2:
                continue  # Need at least 2 versions to compare

            # Compare latest vs. previous
            latest = versions[0]
            previous = versions[1]

            try:
                # Load metadata
                with open(latest / "metadata.json") as f:
                    latest_meta = json.load(f)

                with open(previous / "metadata.json") as f:
                    previous_meta = json.load(f)

                # Extract quality metrics
                latest_quality = latest_meta.get("statistics", {}).get("quality_scores", {}).get("mean", 0)
                previous_quality = previous_meta.get("statistics", {}).get("quality_scores", {}).get("mean", 0)

                # Check for degradation
                if previous_quality > 0:
                    drop_percent = (previous_quality - latest_quality) / previous_quality

                    if drop_percent >= self.config.dataset_quality_drop_threshold:
                        alerts.append(Alert(
                            level=AlertLevel.WARNING,
                            title="Dataset Quality Degradation",
                            message=f"Quality score dropped {drop_percent*100:.1f}% for {sig}",
                            timestamp=datetime.now().isoformat(),
                            details={
                                "signature": sig,
                                "latest_version": latest.name,
                                "previous_version": previous.name,
                                "latest_quality": f"{latest_quality:.2f}",
                                "previous_quality": f"{previous_quality:.2f}",
                                "drop_percent": f"{drop_percent*100:.1f}%"
                            },
                            channels=[AlertChannel.EMAIL, AlertChannel.LOG, AlertChannel.FILE]
                        ))

                # Track baseline
                baselines = self.state.get("dataset_quality_baselines", {})
                baselines[sig] = latest_quality
                self.state.set("dataset_quality_baselines", baselines)

            except Exception as e:
                logger.error(f"Failed to check dataset quality for {sig}: {e}")

        return alerts

    def check_all(self) -> List[Alert]:
        """Run all monitoring checks"""
        all_alerts = []

        logger.info("Running orchestration checks...")
        all_alerts.extend(self.check_orchestration_runs())

        logger.info("Running A/B test checks...")
        all_alerts.extend(self.check_ab_tests())

        logger.info("Running dataset quality checks...")
        all_alerts.extend(self.check_dataset_quality())

        # Update last check time
        self.state.set("last_check", datetime.now().isoformat())

        return all_alerts

    def get_dashboard(self) -> Dict[str, Any]:
        """Get monitoring dashboard data"""
        output_dir = Path(self.config.orchestration_output_dir)
        experiments_dir = Path(self.config.ab_experiments_dir)
        training_data_dir = Path(self.config.training_data_dir)

        dashboard = {
            "timestamp": datetime.now().isoformat(),
            "orchestration": {
                "last_run": self.state.get("last_orchestration_run", "Never"),
                "last_success": self.state.get("last_successful_optimization", "Never"),
                "consecutive_failures": self.state.get("consecutive_orchestration_failures", 0)
            },
            "ab_tests": {
                "active": 0,
                "paused": 0,
                "completed": 0,
                "rolled_back": 0
            },
            "datasets": {},
            "alerts": {
                "last_check": self.state.get("last_check", "Never"),
                "recent_alerts": []
            }
        }

        # Count A/B experiments
        if experiments_dir.exists():
            for exp_file in experiments_dir.glob("*.json"):
                try:
                    with open(exp_file) as f:
                        exp = json.load(f)
                        status = exp.get("status", "unknown")
                        if status in dashboard["ab_tests"]:
                            dashboard["ab_tests"][status] += 1
                except:
                    pass

        # Dataset summaries
        if training_data_dir.exists():
            for sig_dir in training_data_dir.iterdir():
                if not sig_dir.is_dir() or sig_dir.name.startswith('.'):
                    continue

                versions = [d for d in sig_dir.iterdir() if d.is_dir() and d.name.startswith('v')]
                latest_version = sorted(versions, key=lambda d: d.name, reverse=True)[0] if versions else None

                if latest_version:
                    try:
                        with open(latest_version / "metadata.json") as f:
                            meta = json.load(f)

                        dashboard["datasets"][sig_dir.name] = {
                            "version": latest_version.name,
                            "examples": meta.get("example_count", 0),
                            "quality": meta.get("statistics", {}).get("quality_scores", {}).get("mean", 0),
                            "sources": list(meta.get("statistics", {}).get("sources", {}).keys())
                        }
                    except:
                        pass

        # Recent alerts
        if Path(self.config.file_alerts_dir).exists():
            alert_files = sorted(
                Path(self.config.file_alerts_dir).glob("alert_*.json"),
                key=lambda p: p.stat().st_mtime,
                reverse=True
            )[:10]  # Last 10 alerts

            for alert_file in alert_files:
                try:
                    with open(alert_file) as f:
                        alert_data = json.load(f)
                        dashboard["alerts"]["recent_alerts"].append({
                            "level": alert_data.get("level"),
                            "title": alert_data.get("title"),
                            "timestamp": alert_data.get("timestamp")
                        })
                except:
                    pass

        return dashboard


def main():
    parser = argparse.ArgumentParser(
        description="DSPy Optimization Pipeline Monitoring and Alerting"
    )

    subparsers = parser.add_subparsers(dest='command', help='Command to run')

    # check-orchestration
    check_orch = subparsers.add_parser('check-orchestration', help='Check orchestration runs')

    # check-ab-tests
    check_ab = subparsers.add_parser('check-ab-tests', help='Check A/B test status')

    # check-dataset-quality
    check_dataset = subparsers.add_parser('check-dataset-quality', help='Check dataset quality')
    check_dataset.add_argument('--signature', help='Specific signature to check')

    # check-all
    check_all_cmd = subparsers.add_parser('check-all', help='Run all checks')

    # dashboard
    dashboard_cmd = subparsers.add_parser('dashboard', help='Show monitoring dashboard')

    # Configuration options (apply to all commands)
    parser.add_argument('--config', help='Configuration file (JSON)')
    parser.add_argument('--email', action='store_true', help='Enable email alerts')
    parser.add_argument('--slack-webhook', help='Slack webhook URL')

    args = parser.parse_args()

    # Load configuration
    if args.config and Path(args.config).exists():
        with open(args.config) as f:
            config_dict = json.load(f)
        config = MonitoringConfig(**config_dict)
    else:
        config = MonitoringConfig()

    # Apply CLI overrides
    if args.email:
        config.email_enabled = True
    if args.slack_webhook:
        config.slack_enabled = True
        config.slack_webhook_url = args.slack_webhook

    # Create monitor
    monitor = PipelineMonitor(config)

    # Execute command
    if args.command == 'check-orchestration':
        alerts = monitor.check_orchestration_runs()
    elif args.command == 'check-ab-tests':
        alerts = monitor.check_ab_tests()
    elif args.command == 'check-dataset-quality':
        alerts = monitor.check_dataset_quality(args.signature)
    elif args.command == 'check-all':
        alerts = monitor.check_all()
    elif args.command == 'dashboard':
        dashboard = monitor.get_dashboard()
        print(json.dumps(dashboard, indent=2))
        sys.exit(0)
    else:
        parser.print_help()
        sys.exit(1)

    # Send alerts
    for alert in alerts:
        monitor.alert_manager.send_alert(alert)

    # Summary
    if alerts:
        logger.info(f"Generated {len(alerts)} alerts")
        by_level = {}
        for alert in alerts:
            by_level[alert.level] = by_level.get(alert.level, 0) + 1

        for level, count in sorted(by_level.items()):
            logger.info(f"  {level.upper()}: {count}")
    else:
        logger.info("No alerts generated")

    sys.exit(0)


if __name__ == "__main__":
    main()
