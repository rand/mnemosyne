# DSPy Training Data Collection & Continuous Optimization System

**Status**: Production-Ready (90% Complete - 9/10 Phases)

This system provides end-to-end automation for continuously improving DSPy module performance through systematic training data collection, quality validation, optimization, and safe deployment.

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Components](#components)
4. [Setup & Deployment](#setup--deployment)
5. [Usage](#usage)
6. [Monitoring & Alerting](#monitoring--alerting)
7. [Troubleshooting](#troubleshooting)
8. [Operational Runbook](#operational-runbook)

---

## Overview

### Problem Solved

DSPy's MIPROv2 optimizer requires 200-300 training examples per signature for effective prompt optimization. Manual data collection is:
- Time-consuming and unsustainable
- Inconsistent in quality
- Unable to capture production edge cases
- Difficult to version and track

### Solution

This system automates the complete optimization lifecycle:

```
Data Collection → Validation → Quality Gates → Versioning →
Optimization (MIPROv2) → Evaluation → A/B Testing → Deployment → Monitoring
```

**Key Benefits**:
- Continuous improvement through monthly automated cycles
- Safe deployment via gradual rollout with auto-rollback
- Quality assurance through multi-layer validation
- Operational visibility via real-time monitoring
- Provenance tracking for all training data

---

## Architecture

### Data Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                      DATA COLLECTION                             │
├─────────────────────────────────────────────────────────────────┤
│ Git Mining ──┐                                                   │
│              ├──→ Raw Examples                                   │
│ Synthetic ───┤                                                   │
│              │                                                    │
│ Telemetry* ──┘                                                   │
└─────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│                    DATA VALIDATION                               │
├─────────────────────────────────────────────────────────────────┤
│ Schema Validation → Quality Scoring → Deduplication             │
└─────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│                     QUALITY GATES                                │
├─────────────────────────────────────────────────────────────────┤
│ Min Quality: 70-80 | Difficulty: 30/50/20 | Diversity Checks    │
└─────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│                  VERSIONED STORAGE                               │
├─────────────────────────────────────────────────────────────────┤
│ training_data/                                                   │
│ ├── extract_requirements/                                        │
│ │   ├── v20251104_120000/ (dataset.json, metadata, provenance)  │
│ │   └── latest -> v20251104_120000                              │
│ └── validate_intent/ ...                                         │
└─────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│                  OPTIMIZATION (MIPROv2)                          │
├─────────────────────────────────────────────────────────────────┤
│ 50 Trials | Prompt Tuning | Evaluation | Improvement Tracking   │
└─────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│                    A/B TESTING                                   │
├─────────────────────────────────────────────────────────────────┤
│ Gradual Rollout: 10% → 25% → 50% → 75% → 100%                   │
│ Auto-Rollback: Error Rate +5% | Latency +500ms | Quality <70%   │
└─────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│                    MONITORING                                    │
├─────────────────────────────────────────────────────────────────┤
│ Orchestration Health | A/B Status | Dataset Quality | Alerts    │
└─────────────────────────────────────────────────────────────────┘

* Phase 1.3 (Production Telemetry) pending - requires production system integration
```

### Directory Structure

```
data_collection/
├── README.md                          # This file
├── git_mining_pipeline.py             # Extract examples from git commits
├── synthetic_data_generator.py        # Generate synthetic examples
├── data_validator.py                  # Schema validation & quality scoring
├── dataset_manager.py                 # Versioned dataset storage
├── quality_gates.py                   # Quality threshold enforcement
├── optimization_orchestrator.py       # End-to-end orchestration
├── ab_testing_framework.py            # A/B testing with auto-rollback
├── monitoring.py                      # Monitoring & alerting system
├── cron_monthly_optimization.sh       # Monthly automation script
├── cron_cleanup.sh                    # Weekly maintenance script
├── cron_monitoring.sh                 # Hourly monitoring script
└── monitoring_config.json.example     # Sample monitoring config

training_data/                         # Versioned datasets (created by system)
├── extract_requirements/
│   ├── v20251104_120000/
│   │   ├── dataset.json              # Training examples
│   │   ├── metadata.json             # Statistics & provenance
│   │   └── provenance.jsonl          # Per-example source tracking
│   └── latest -> v20251104_120000    # Symlink to latest version
├── validate_intent/
├── validate_completeness/
├── validate_correctness/
└── generate_guidance/

.ab_experiments/                       # A/B test state (created by system)
├── routing_config.json               # Current traffic routing
└── experiment_*.json                 # Experiment snapshots

.monitoring_state.json                # Monitoring state (created by system)
```

---

## Components

### 1. Data Collection

#### Git Mining Pipeline (`git_mining_pipeline.py`)

Extracts training examples from git commit history.

```bash
python git_mining_pipeline.py \
  --target 30 \
  --since-days 90 \
  --output /tmp/git_mined_data
```

**Features**:
- Extracts code changes, commit messages, PR descriptions
- Filters by file patterns and commit types
- Deduplicates similar commits
- Assigns difficulty scores based on change complexity
- Tracks provenance (commit SHA, author, timestamp)

**Configuration**:
- `--target`: Number of examples to collect
- `--since-days`: How far back to search (default: 90)
- `--output`: Output directory for collected examples
- `--repo-path`: Repository to mine (default: current repo)

#### Synthetic Data Generator (`synthetic_data_generator.py`)

Generates high-quality synthetic training examples.

```bash
python synthetic_data_generator.py \
  --target 20 \
  --output /tmp/synthetic_data
```

**Features**:
- Template-based generation with variation
- Quality validation before output
- Difficulty stratification (easy/medium/hard)
- Category diversity enforcement
- Automatic quality scoring

**Configuration**:
- `--target`: Number of examples to generate
- `--output`: Output directory
- `--signatures`: Specific signatures to generate (default: all)

#### Production Telemetry (Phase 1.3 - TODO)

*Pending production system integration*

Will capture 10% sample of production DSPy calls with:
- Real user inputs and outputs
- Performance metrics (latency, token usage)
- Quality scores from downstream validation
- Edge cases and failure modes

### 2. Data Validation & Quality

#### Data Validator (`data_validator.py`)

Validates schema and scores quality.

```bash
python data_validator.py \
  --input /tmp/raw_data \
  --output /tmp/validated_data
```

**Validation Checks**:
- Schema compliance (required fields, types)
- Quality scoring (0-100 scale based on completeness, clarity, relevance)
- Deduplication (fuzzy matching on inputs)
- Difficulty assignment (easy/medium/hard)
- Category validation

**Quality Scoring Criteria**:
- Completeness: All required fields present
- Clarity: Clear, well-formed inputs/outputs
- Relevance: Appropriate for signature
- Diversity: Not overly similar to existing examples

#### Quality Gates (`quality_gates.py`)

Enforces quality thresholds before dataset inclusion.

```bash
python quality_gates.py \
  --input /tmp/validated_data \
  --signature extract_requirements \
  --strict
```

**Quality Thresholds** (per signature):

| Signature | Min Quality | Difficulty Distribution | Completeness |
|-----------|-------------|-------------------------|--------------|
| extract_requirements | 70.0 | 30% easy / 50% medium / 20% hard | - |
| validate_intent | 70.0 | 30/50/20 | - |
| validate_completeness | 75.0 | 30/50/20 | 40% complete / 60% incomplete |
| validate_correctness | 75.0 | 30/50/20 | - |
| generate_guidance | 80.0 | 30/50/20 | - |

**Modes**:
- `--strict`: Reject batch if thresholds not met (for production)
- Default: Warn but accept (for development)

**Checks**:
- Individual quality scores ≥ threshold
- Difficulty distribution ±10% of target
- Completeness distribution (where applicable)
- Category diversity (≥3 categories)
- Source diversity (no single source >80%)

### 3. Dataset Management

#### Dataset Manager (`dataset_manager.py`)

Manages versioned dataset storage with provenance tracking.

```bash
python dataset_manager.py \
  --signature extract_requirements \
  --examples /tmp/validated_data/*.json \
  --source git_mining \
  --notes "Monthly collection Nov 2025"
```

**Features**:
- Timestamp-based versioning (YYYYMMDD_HHMMSS)
- Automatic metadata generation (statistics, sources, quality)
- Provenance tracking (per-example source, timestamp, quality)
- Incremental updates (add to latest version)
- Symlink to latest version for easy access

**Storage Structure**:
```
training_data/extract_requirements/v20251104_120000/
├── dataset.json      # Training examples
├── metadata.json     # {version, created, example_count, statistics}
└── provenance.jsonl  # Per-example: {example_id, source, timestamp, quality}
```

**CLI**:
```bash
# Create new version
dataset_manager.py create --signature <sig> --examples <files> --source <src>

# Add to latest version
dataset_manager.py add --signature <sig> --examples <files> --source <src>

# List versions
dataset_manager.py list --signature <sig>

# Load dataset
dataset_manager.py load --signature <sig> --version <ver>
```

### 4. Optimization Orchestrator

#### Optimization Orchestrator (`optimization_orchestrator.py`)

End-to-end automation of the optimization pipeline.

```bash
python optimization_orchestrator.py \
  --git-target 30 \
  --synthetic-target 20 \
  --trials 50 \
  --output-dir /tmp/optimization_runs
```

**Pipeline Phases**:

1. **Data Collection**: Git mining + synthetic generation
2. **Validation**: Schema validation + quality scoring
3. **Quality Gates**: Threshold enforcement with strict mode
4. **Versioning**: Create new dataset versions with provenance
5. **Optimization**: Run MIPROv2 for 50 trials per signature
6. **Evaluation**: Compare against baseline, calculate improvements
7. **Deployment Decision**: Deploy if improvement ≥5%

**Output**:
```
/tmp/optimization_runs/run_20251104_120000/
├── orchestration_summary.json  # Overall results
├── collected_data/             # Raw data from collection
├── validated_data/             # Post-validation data
├── quality_reports/            # Quality gate results
└── optimization_results/       # MIPROv2 outputs
```

**Configuration**:
- `--git-target`: Examples to collect from git (default: 30)
- `--synthetic-target`: Synthetic examples to generate (default: 20)
- `--trials`: MIPROv2 trials (default: 50)
- `--output-dir`: Output directory (default: /tmp/optimization_runs)
- `--min-improvement`: Minimum improvement % for deployment (default: 5%)

### 5. A/B Testing Framework

#### A/B Testing (`ab_testing_framework.py`)

Gradual rollout with automatic rollback on degradation.

```bash
# Start A/B test
python ab_testing_framework.py start \
  --signature extract_requirements \
  --baseline reviewer_baseline.json \
  --candidate reviewer_optimized_v1.json \
  --experiment-id extract_reqs_v1

# Monitor (daemon mode with auto-promote/rollback)
python ab_testing_framework.py monitor \
  --experiment-id extract_reqs_v1 \
  --daemon

# Manual operations
python ab_testing_framework.py pause --experiment-id extract_reqs_v1
python ab_testing_framework.py resume --experiment-id extract_reqs_v1
python ab_testing_framework.py promote --experiment-id extract_reqs_v1
python ab_testing_framework.py rollback --experiment-id extract_reqs_v1 --reason "High error rate"
python ab_testing_framework.py status --experiment-id extract_reqs_v1
```

**Rollout Stages**:
1. 10% candidate traffic (60 min, ≥100 requests)
2. 25% candidate traffic (60 min, ≥100 requests)
3. 50% candidate traffic (60 min, ≥100 requests)
4. 75% candidate traffic (60 min, ≥100 requests)
5. 100% candidate traffic (deployment complete)

**Auto-Rollback Triggers**:
- Error rate increased >5%
- Latency increased >500ms
- Quality score <70%

**Integration**: Writes `routing_config.json` for Rust DSPy adapter to consume.

### 6. Monitoring & Alerting

#### Monitoring System (`monitoring.py`)

Comprehensive monitoring with multi-channel alerting.

```bash
# Run all checks
python monitoring.py check-all

# Individual checks
python monitoring.py check-orchestration
python monitoring.py check-ab-tests
python monitoring.py check-dataset-quality --signature extract_requirements

# Dashboard
python monitoring.py dashboard
```

**Monitoring Coverage**:

1. **Orchestration Health**:
   - Success/failure rates
   - Consecutive failures (alert after 2)
   - Last successful optimization
   - Improvement trends
   - Stale optimizations (warning if no improvement in 3 months)

2. **A/B Test Status**:
   - Active experiments
   - Rollback events (immediate alert with reason)
   - Long-running experiments (info alert after 7 days)
   - Traffic distribution

3. **Dataset Quality**:
   - Quality score trends
   - Version-to-version degradation (alert on >10% drop)
   - Example count growth
   - Source diversity

**Alert Channels**:
- **Email**: SMTP-based with configurable recipients
- **Slack**: Webhook integration with color-coded severity
- **File**: JSON alerts written to `/tmp/dspy_alerts/`
- **Log**: Structured logging to stdout/logs

**Alert Levels**:
- INFO: Successful deployments, normal events
- WARNING: Degradation trends, long-running experiments, stale optimizations
- ERROR: Consecutive failures, quality drops
- CRITICAL: System-level failures

**Configuration** (`monitoring_config.json`):
```json
{
  "orchestration_failure_threshold": 2,
  "ab_test_rollback_alert": true,
  "dataset_quality_drop_threshold": 0.10,
  "optimization_no_improvement_threshold": 3,
  "email_enabled": false,
  "slack_enabled": false,
  "file_alerts_enabled": true
}
```

---

## Setup & Deployment

### Prerequisites

1. **Python 3.11+** with `uv` package manager
2. **Anthropic API key** (for DSPy optimization)
3. **Git repository** with commit history (for git mining)
4. **Cron access** (for scheduled automation)

### Installation

```bash
# 1. Install dependencies (handled by uv)
cd /path/to/mnemosyne/src/orchestration/dspy_modules
uv sync

# 2. Configure API key
export ANTHROPIC_API_KEY="sk-ant-..."
# Or use mnemosyne secrets:
# cargo run --bin mnemosyne -- secrets set anthropic_api_key <key>

# 3. Create required directories
mkdir -p /var/log/dspy_optimization
mkdir -p /tmp/optimization_runs
mkdir -p /tmp/dspy_alerts

# 4. Test individual components
uv run python3 data_collection/git_mining_pipeline.py --target 5 --output /tmp/test_git
uv run python3 data_collection/synthetic_data_generator.py --target 5 --output /tmp/test_synthetic
uv run python3 data_collection/monitoring.py dashboard

# 5. Configure monitoring (optional)
cp data_collection/monitoring_config.json.example data_collection/monitoring_config.json
# Edit monitoring_config.json with email/Slack credentials
```

### Cron Setup

```bash
# Edit crontab
crontab -e

# Add these entries:

# Monthly optimization (1st of month at 2 AM)
0 2 1 * * /path/to/mnemosyne/src/orchestration/dspy_modules/data_collection/cron_monthly_optimization.sh >> /var/log/dspy_optimization.log 2>&1

# Hourly monitoring
0 * * * * /path/to/mnemosyne/src/orchestration/dspy_modules/data_collection/cron_monitoring.sh >> /var/log/dspy_monitoring.log 2>&1

# Weekly cleanup (Sunday at 3 AM)
0 3 * * 0 /path/to/mnemosyne/src/orchestration/dspy_modules/data_collection/cron_cleanup.sh >> /var/log/dspy_cleanup.log 2>&1
```

**Cron Environment Variables** (add to crontab):
```bash
ANTHROPIC_API_KEY=sk-ant-...
GIT_MINING_TARGET=30
SYNTHETIC_TARGET=20
MIPRO_TRIALS=50
OUTPUT_DIR=/tmp/optimization_runs
```

### Manual First Run

Before enabling cron, run the orchestrator manually to verify:

```bash
cd /path/to/mnemosyne/src/orchestration/dspy_modules

env PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 \
    ANTHROPIC_API_KEY="sk-ant-..." \
    uv run python3 data_collection/optimization_orchestrator.py \
    --git-target 30 \
    --synthetic-target 20 \
    --trials 50 \
    --output-dir /tmp/optimization_runs_test
```

Check results:
```bash
# View summary
cat /tmp/optimization_runs_test/run_*/orchestration_summary.json | jq .

# Check monitoring dashboard
uv run python3 data_collection/monitoring.py dashboard | jq .
```

---

## Usage

### Monthly Optimization Cycle (Automated)

The cron job handles this automatically, but you can trigger manually:

```bash
./data_collection/cron_monthly_optimization.sh
```

**What Happens**:
1. Collects 30 git examples + 20 synthetic examples
2. Validates and scores quality
3. Filters through quality gates (strict mode)
4. Creates new dataset versions with provenance
5. Runs MIPROv2 optimization (50 trials per signature)
6. Evaluates improvements vs. baseline
7. Deploys optimized modules if improvement ≥5%
8. Writes orchestration summary to output directory

**Success Criteria**:
- Exit code 0
- Orchestration summary shows `"success": true`
- At least one signature deployed with improvement ≥5%

### A/B Testing (Manual)

After optimization produces a candidate module:

```bash
# Start gradual rollout
python ab_testing_framework.py start \
  --signature extract_requirements \
  --baseline optimized_modules/extract_reqs_baseline.json \
  --candidate optimized_modules/extract_reqs_v2.json \
  --experiment-id extract_reqs_v2_test

# Monitor in daemon mode (auto-promote/rollback)
python ab_testing_framework.py monitor \
  --experiment-id extract_reqs_v2_test \
  --daemon &

# Check status
python ab_testing_framework.py status --experiment-id extract_reqs_v2_test

# Manual intervention if needed
python ab_testing_framework.py pause --experiment-id extract_reqs_v2_test
python ab_testing_framework.py rollback --experiment-id extract_reqs_v2_test --reason "Manual rollback"
```

### Monitoring (Automated)

The hourly cron runs all checks. View recent alerts:

```bash
# Dashboard
python monitoring.py dashboard | jq .

# Recent file alerts
ls -ltr /tmp/dspy_alerts/

# View specific alert
cat /tmp/dspy_alerts/alert_warning_20251104_120000.json | jq .
```

### Dataset Management (Manual)

```bash
# List all versions for a signature
python dataset_manager.py list --signature extract_requirements

# Load specific version
python dataset_manager.py load --signature extract_requirements --version v20251104_120000

# Add examples to latest version
python dataset_manager.py add \
  --signature extract_requirements \
  --examples /tmp/new_examples/*.json \
  --source manual_curation
```

---

## Monitoring & Alerting

### Dashboard Metrics

```bash
python monitoring.py dashboard
```

**Output Structure**:
```json
{
  "timestamp": "2025-11-04T12:00:00",
  "orchestration": {
    "last_run": "run_20251104_020000",
    "last_success": "2025-11-04T02:00:00",
    "consecutive_failures": 0
  },
  "ab_tests": {
    "active": 1,
    "paused": 0,
    "completed": 3,
    "rolled_back": 0
  },
  "datasets": {
    "extract_requirements": {
      "version": "v20251104_020000",
      "examples": 95,
      "quality": 78.5,
      "sources": ["git", "synthetic"]
    }
  },
  "alerts": {
    "last_check": "2025-11-04T12:00:00",
    "recent_alerts": []
  }
}
```

### Alert Examples

**Orchestration Failure**:
```json
{
  "level": "error",
  "title": "Orchestration Run Failures",
  "message": "Orchestration has failed 2 consecutive times",
  "timestamp": "2025-11-04T12:00:00",
  "details": {
    "run_id": "run_20251104_020000",
    "consecutive_failures": 2,
    "last_error": "Quality gates failed: extract_requirements quality=65.0 < 70.0"
  }
}
```

**A/B Test Rollback**:
```json
{
  "level": "warning",
  "title": "A/B Test Rolled Back",
  "message": "Experiment extract_reqs_v2 was rolled back",
  "timestamp": "2025-11-04T12:00:00",
  "details": {
    "experiment_id": "extract_reqs_v2",
    "signature": "extract_requirements",
    "reason": "Error rate increased >5%",
    "final_traffic_percent": 25
  }
}
```

**Dataset Quality Degradation**:
```json
{
  "level": "warning",
  "title": "Dataset Quality Degradation",
  "message": "Quality score dropped 15.2% for extract_requirements",
  "timestamp": "2025-11-04T12:00:00",
  "details": {
    "signature": "extract_requirements",
    "latest_version": "v20251104_020000",
    "previous_version": "v20251003_020000",
    "latest_quality": 65.8,
    "previous_quality": 77.8,
    "drop_percent": 15.2
  }
}
```

### Configuring Alerts

Edit `monitoring_config.json`:

```json
{
  "email_enabled": true,
  "email_smtp_server": "smtp.gmail.com",
  "email_smtp_port": 587,
  "email_from": "dspy-monitoring@example.com",
  "email_to": ["team@example.com"],
  "email_username": "your-email@gmail.com",
  "email_password": "your-app-password",

  "slack_enabled": true,
  "slack_webhook_url": "https://hooks.slack.com/services/YOUR/WEBHOOK/URL"
}
```

Then update cron to use config:
```bash
MONITORING_CONFIG=/path/to/monitoring_config.json
```

---

## Troubleshooting

### Orchestration Failures

**Symptom**: Consecutive orchestration failures

**Diagnosis**:
```bash
# Check latest orchestration summary
cat /tmp/optimization_runs/run_*/orchestration_summary.json | jq .

# Check orchestration log
tail -100 /var/log/dspy_optimization.log
```

**Common Causes**:

1. **Quality Gates Failed**:
   - **Symptom**: `quality_gates: reject, reason: quality_score=X < Y`
   - **Fix**: Lower quality thresholds in `quality_gates.py` or improve data collection
   - **Temporary**: Use `--no-strict` mode to proceed with warnings

2. **Insufficient Data**:
   - **Symptom**: `collected: 10 examples, target: 30`
   - **Fix**: Increase `--since-days` for git mining or `--target` for synthetic
   - **Check**: `git log --since="90 days ago" --oneline | wc -l` for available commits

3. **API Key Issues**:
   - **Symptom**: `anthropic.AuthenticationError`
   - **Fix**: Verify `ANTHROPIC_API_KEY` environment variable
   - **Check**: `echo $ANTHROPIC_API_KEY` or `mnemosyne secrets get anthropic_api_key`

4. **MIPROv2 Optimization Timeout**:
   - **Symptom**: `optimization timed out after 2 hours`
   - **Fix**: Reduce `--trials` (e.g., 25 instead of 50) or increase timeout

### A/B Test Rollbacks

**Symptom**: Experiments automatically rolled back

**Diagnosis**:
```bash
# Check experiment state
cat .ab_experiments/experiment_*.json | jq '.rollback_reason'

# Check monitoring alerts
cat /tmp/dspy_alerts/alert_warning_*.json | jq 'select(.title | contains("Rollback"))'
```

**Common Causes**:

1. **Error Rate Spike**:
   - **Symptom**: `rollback_reason: "error_rate_delta > 0.05"`
   - **Root Cause**: Candidate module producing errors
   - **Fix**: Review candidate module prompts, check edge cases in test data

2. **Latency Increase**:
   - **Symptom**: `rollback_reason: "latency_delta_ms > 500"`
   - **Root Cause**: Candidate module slower (longer prompts, more tokens)
   - **Fix**: Optimize prompts for conciseness, consider faster model tier

3. **Quality Degradation**:
   - **Symptom**: `rollback_reason: "quality_score < 0.70"`
   - **Root Cause**: Candidate module producing lower-quality outputs
   - **Fix**: Re-optimize with more training data, adjust evaluation metrics

### Dataset Quality Issues

**Symptom**: Quality scores consistently low

**Diagnosis**:
```bash
# Check dataset metadata
cat training_data/extract_requirements/latest/metadata.json | jq '.statistics.quality_scores'

# Check provenance
cat training_data/extract_requirements/latest/provenance.jsonl | jq -s 'map(.quality_score) | add/length'
```

**Common Causes**:

1. **Git Mining Low Quality**:
   - **Symptom**: Git-sourced examples have quality <60
   - **Fix**: Adjust git mining filters (file patterns, commit types)
   - **Tune**: `--min-commit-message-length`, `--exclude-merge-commits`

2. **Synthetic Generation Issues**:
   - **Symptom**: Synthetic examples repetitive or unrealistic
   - **Fix**: Improve synthetic templates, increase variation
   - **Review**: `synthetic_data_generator.py` templates

3. **Deduplication Too Aggressive**:
   - **Symptom**: Many examples rejected as duplicates
   - **Fix**: Tune fuzzy matching threshold in `data_validator.py`
   - **Check**: Duplicate rejection rate in validation logs

### Monitoring Alerts Not Triggering

**Symptom**: No alerts despite issues

**Diagnosis**:
```bash
# Check monitoring state
cat .monitoring_state.json | jq .

# Run manual check
python monitoring.py check-all
```

**Common Causes**:

1. **Alert Channels Disabled**:
   - **Symptom**: `email_enabled: false, slack_enabled: false`
   - **Fix**: Enable at least file alerts in config

2. **Thresholds Too High**:
   - **Symptom**: Issues don't exceed thresholds
   - **Fix**: Lower thresholds in `monitoring_config.json`

3. **State File Corruption**:
   - **Symptom**: Monitoring behaves inconsistently
   - **Fix**: Delete `.monitoring_state.json` to reset

---

## Operational Runbook

### Weekly Tasks

**Objective**: Verify system health

```bash
# 1. Check recent orchestration runs
ls -lt /tmp/optimization_runs/ | head -5

# 2. Review monitoring dashboard
python monitoring.py dashboard | jq .

# 3. Check for rollbacks
grep -r "rolled_back" .ab_experiments/*.json

# 4. Verify disk usage
du -sh /tmp/optimization_runs training_data /var/log/dspy_*

# 5. Review recent alerts
ls -ltr /tmp/dspy_alerts/ | tail -10
```

### Monthly Tasks

**Objective**: Validate improvements, review trends

```bash
# 1. Compare month-over-month dataset growth
for sig in extract_requirements validate_intent validate_completeness validate_correctness generate_guidance; do
    echo "=== $sig ==="
    ls -1 training_data/$sig/ | grep -E "^v" | tail -3
done

# 2. Review optimization improvements
grep "improvement_percent" /tmp/optimization_runs/*/orchestration_summary.json | jq .

# 3. Check A/B test history
cat .ab_experiments/*.json | jq -s 'map({id, status, created_at, final_status})'

# 4. Analyze data source breakdown
for sig in extract_requirements validate_intent; do
    echo "=== $sig ==="
    cat training_data/$sig/latest/metadata.json | jq '.statistics.sources'
done

# 5. Review quality trends
for sig in extract_requirements validate_intent; do
    echo "=== $sig ==="
    for ver in $(ls -1 training_data/$sig/ | grep "^v" | tail -3); do
        echo -n "$ver: "
        cat training_data/$sig/$ver/metadata.json | jq '.statistics.quality_scores.mean'
    done
done
```

### Quarterly Tasks

**Objective**: System optimization, capacity planning

```bash
# 1. Archive old dataset versions
# (Handled automatically by cron_cleanup.sh for versions >90 days)

# 2. Review and tune quality thresholds
# Compare quality gate pass rates:
grep -r "quality_gates" /tmp/optimization_runs/*/orchestration_summary.json

# 3. Evaluate data source effectiveness
# Which sources produce highest quality?
cat training_data/*/latest/provenance.jsonl | jq -s 'group_by(.source) | map({source: .[0].source, avg_quality: (map(.quality_score) | add/length)})'

# 4. Capacity planning
# Project disk usage growth rate
du -sh training_data /tmp/optimization_runs /var/log/dspy_* | \
  awk '{sum+=$1} END {print "Total usage:", sum}'

# 5. Review optimization trial counts
# Are 50 trials sufficient? Over-kill?
# Check plateau analysis in optimization logs
```

### Emergency Procedures

#### Immediate Rollback of All A/B Tests

```bash
for exp in .ab_experiments/experiment_*.json; do
    exp_id=$(jq -r '.experiment_id' "$exp")
    python ab_testing_framework.py rollback \
      --experiment-id "$exp_id" \
      --reason "Emergency rollback - system-wide issue"
done
```

#### Disable Automated Optimization

```bash
# Comment out cron job
crontab -e
# Add # before the monthly optimization line

# Or kill running optimization
pkill -f "optimization_orchestrator.py"
```

#### Clear Corrupted State

```bash
# Backup current state
cp .monitoring_state.json .monitoring_state.json.backup
cp -r .ab_experiments .ab_experiments.backup

# Reset monitoring
rm .monitoring_state.json

# Cancel all A/B tests
rm -rf .ab_experiments/*.json
```

#### Restore from Backup

```bash
# Restore dataset version
cd training_data/extract_requirements
tar -xzf v20251003_020000.tar.gz
rm latest
ln -s v20251003_020000 latest

# Restore monitoring state
cp .monitoring_state.json.backup .monitoring_state.json

# Restore A/B tests
cp -r .ab_experiments.backup .ab_experiments
```

---

## Performance & Scalability

### Current Capacity

**Tested With**:
- 50 examples per signature (5 signatures = 250 total examples)
- 50 MIPROv2 trials per signature
- 5 dataset versions per signature (automatic archival)
- 30-day log retention

**Resource Usage** (typical monthly run):
- CPU: 2-4 hours (optimization), <1 min (monitoring)
- Memory: ~2GB peak (optimization), ~100MB (monitoring)
- Disk: ~500MB/month (datasets), ~100MB/month (logs)
- API Costs: ~$5-10/month (50 trials × 5 signatures)

### Scaling Considerations

**To 200+ Examples Per Signature**:
- Increase `--git-target` and `--synthetic-target`
- Consider parallel data collection (git mining + synthetic)
- No changes needed to quality gates or versioning

**To 100+ MIPROv2 Trials**:
- Increase `--trials` parameter
- Consider parallel optimization per signature
- Monitor API rate limits and costs

**To 10+ Signatures**:
- Current architecture supports unlimited signatures
- Linear scaling in time and resources
- Consider distributed optimization (future work)

**To Production Telemetry** (Phase 1.3):
- Requires Rust integration for sampling production calls
- Estimated 1000-10000 examples/month at 10% sampling
- Automatic deduplication handles volume

---

## Future Enhancements

### Phase 1.3: Production Telemetry

**Status**: Pending production system integration

**Requirements**:
- Rust DSPy adapter captures 10% of production calls
- Write samples to telemetry buffer
- Telemetry collector reads buffer, validates, versions
- Automatic quality scoring based on downstream success

**Integration Points**:
1. Modify Rust DSPy adapter to sample calls
2. Write samples to `/tmp/dspy_telemetry/*.jsonl`
3. Add telemetry collector to orchestrator
4. Merge with git/synthetic data streams

### Distributed Optimization

**Goal**: Parallelize MIPROv2 across signatures

**Approach**:
- Launch 5 optimization processes concurrently
- Use message queue for work distribution
- Aggregate results at end

**Benefits**: 5× speedup (30min instead of 2.5hrs for 50 trials)

### Advanced A/B Testing

**Features**:
- Multi-armed bandit allocation (adaptive traffic routing)
- Automatic champion/challenger rotation
- Performance-based traffic allocation

### Enhanced Monitoring

**Metrics**:
- Cost tracking (API tokens, compute)
- Performance trends (latency percentiles, error rates)
- Prediction: Estimated improvement from next optimization

---

## References

- **DSPy Documentation**: https://dspy-docs.vercel.app/
- **MIPROv2 Paper**: https://arxiv.org/abs/2406.11695
- **Mnemosyne Repository**: /Users/rand/src/mnemosyne
- **Training Data Location**: `/Users/rand/src/mnemosyne/src/orchestration/dspy_modules/training_data/`

---

## Support

For issues or questions:
1. Check troubleshooting section above
2. Review logs in `/var/log/dspy_optimization/`
3. Check monitoring dashboard: `python monitoring.py dashboard`
4. Review orchestration summaries in `/tmp/optimization_runs/`

**Common Log Locations**:
- Monthly optimization: `/var/log/dspy_optimization.log`
- Hourly monitoring: `/var/log/dspy_monitoring.log`
- Weekly cleanup: `/var/log/dspy_cleanup.log`
- File alerts: `/tmp/dspy_alerts/*.json`
