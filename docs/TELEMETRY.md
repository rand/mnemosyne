# DSPy Telemetry System

Production telemetry sampling infrastructure for continuous DSPy module optimization.

## Overview

The telemetry system provides **10% hash-based deterministic sampling** of live DSPy module invocations to create high-quality training data from production usage. This enables continuous improvement of DSPy prompts through monthly optimization cycles.

## Architecture

```
Production DSPy Calls
      ↓ (10% sampled)
DSpyInstrumentation
      ↓ (JSON Lines logs)
logs/dspy_production.jsonl
      ↓ (monthly aggregation)
telemetry_aggregator.py
      ↓ (quality filtering + deduplication)
training_data/<signature>/v<N>/
      ↓ (monthly optimization)
MIPROv2 Optimizer
      ↓ (A/B testing)
Production Deployment
```

## Components

### 1. DSpyInstrumentation (Rust)

**Location**: `src/orchestration/dspy_instrumentation.rs`

**Purpose**: Wrapper around DSpyBridge providing:
- 10% deterministic sampling based on request_id hash
- Production logging in JSON Lines format
- Drop-in replacement API for existing adapters
- Zero overhead when sampling disabled

**Key Methods**:
```rust
pub async fn call_module_with_sampling(
    &self,
    module_name: &str,
    version: &ModuleVersion,
    inputs: HashMap<String, Value>,
    request_id: &str,
) -> Result<HashMap<String, Value>>

pub async fn call_agent_module(
    &self,
    module_name: &str,
    inputs: HashMap<String, Value>,
) -> Result<HashMap<String, Value>>
```

**Sampling Logic**:
```rust
let mut hasher = DefaultHasher::new();
request_id.hash(&mut hasher);
let hash_value = hasher.finish();
let should_sample = (hash_value % 100) < self.config.sampling_rate_pct;
```

### 2. Telemetry Aggregator (Python)

**Location**: `src/orchestration/dspy_modules/data_collection/telemetry_aggregator.py`

**Purpose**: Convert production logs to DSPy training data

**Features**:
- Parses JSON Lines logs into InteractionLog objects
- Filters by quality metrics (success, latency, cost)
- SHA256-based deduplication across inputs
- Integration with DatasetManager for versioned storage
- Provenance tracking (telemetry source)

**Usage**:
```bash
uv run python3 data_collection/telemetry_aggregator.py \
  --log-file logs/dspy_production.jsonl \
  --output-dir training_data \
  --min-quality-score 0.70 \
  --max-latency-ms 5000 \
  --max-cost-usd 0.10
```

### 3. Monthly Optimization Pipeline

**Location**: `src/orchestration/dspy_modules/data_collection/optimization_orchestrator.py`

**Purpose**: Orchestrate monthly optimization cycles

**Telemetry Integration** (Lines 253-307):
- Reads `monitoring_config.json` for log file path
- Invokes `telemetry_aggregator.py` with quality thresholds
- Collects versioned datasets from `training_data/<signature>/`
- Verifies provenance to identify telemetry-sourced data
- Comprehensive error handling for subprocess operations

## Configuration

### monitoring_config.json

**Location**: `src/orchestration/monitoring_config.json`

**Structure**:
```json
{
  "telemetry": {
    "enabled": true,
    "sampling_rate_pct": 10,
    "log_file_path": "logs/dspy_production.jsonl",
    "rotation": {
      "max_size_mb": 100,
      "max_age_days": 90,
      "compression": "gzip"
    }
  },
  "quality_thresholds": {
    "min_success_rate": 0.95,
    "max_latency_p99_ms": 3000,
    "max_cost_p99_usd": 0.05
  },
  "optimization": {
    "schedule": "monthly",
    "telemetry_target": 50,
    "git_target": 100,
    "synthetic_target": 20
  }
}
```

**Fields**:
- `telemetry.enabled`: Enable/disable telemetry sampling (default: `false`, enable in production)
- `telemetry.sampling_rate_pct`: Percentage of requests to sample (default: `10`)
- `telemetry.log_file_path`: Path to JSON Lines log file (default: `logs/dspy_production.jsonl`)
- `telemetry.rotation.max_size_mb`: Rotate log when exceeds size (default: `100`)
- `telemetry.rotation.max_age_days`: Delete logs older than days (default: `90`)
- `quality_thresholds.min_success_rate`: Alert if success rate drops below (default: `0.95`)
- `quality_thresholds.max_latency_p99_ms`: Alert if p99 latency exceeds (default: `3000`)
- `optimization.telemetry_target`: Target number of telemetry examples per signature (default: `0`, disabled until production)

## Log Format

### InteractionLog Structure

```json
{
  "module_name": "reviewer",
  "module_version": "baseline",
  "signature": "extract_requirements",
  "input": {
    "user_intent": "Implement user authentication",
    "context": "REST API project"
  },
  "output": {
    "requirements": [
      "JWT token generation",
      "Password hashing with bcrypt",
      "Session management"
    ]
  },
  "timestamp_ms": 1699564800000,
  "latency_ms": 250,
  "tokens": 120,
  "cost_usd": 0.0018,
  "model": "claude-haiku-4-5",
  "success": true,
  "error": null
}
```

**Required Fields**:
- `module_name`: DSPy module name (e.g., "reviewer", "optimizer")
- `module_version`: Version identifier ("baseline", "optimized_v1")
- `signature`: DSPy signature name (e.g., "extract_requirements")
- `input`: Input dictionary matching signature inputs
- `output`: Output dictionary matching signature outputs
- `timestamp_ms`: Unix timestamp in milliseconds
- `success`: Boolean indicating successful completion

**Optional Fields**:
- `latency_ms`: Request latency in milliseconds
- `tokens`: Total tokens consumed (input + output)
- `cost_usd`: Estimated cost in USD
- `model`: Model used for inference
- `error`: Error message if `success` is false

## Quality Filtering

### Metrics

1. **Success Rate**: Only successful invocations are included
2. **Latency**: Filter out high-latency requests (default: >5000ms)
3. **Cost**: Filter out expensive requests (default: >$0.10)
4. **Quality Score**: Composite metric (future: accuracy, helpfulness)

### Thresholds

**Default Thresholds**:
```python
min_quality_score = 0.70  # Future: semantic quality
max_latency_ms = 5000     # 5 seconds
max_cost_usd = 0.10       # 10 cents
```

**Configuration**:
```bash
telemetry_aggregator.py \
  --min-quality-score 0.70 \
  --max-latency-ms 5000 \
  --max-cost-usd 0.10
```

## Deduplication

### SHA256-Based Hashing

**Purpose**: Prevent duplicate training examples from repetitive usage patterns

**Implementation**:
```python
def compute_hash(input_dict: dict) -> str:
    """Compute SHA256 hash of sorted input JSON."""
    input_str = json.dumps(input_dict, sort_keys=True)
    return hashlib.sha256(input_str.encode()).hexdigest()
```

**Key Properties**:
- **Deterministic**: Same input always produces same hash
- **Order-Independent**: `{"a": 1, "b": 2}` == `{"b": 2, "a": 1}`
- **Collision-Resistant**: SHA256 provides 256-bit security

## Provenance Tracking

### Purpose

Track data lineage to understand training data composition and enable reproducibility.

### Format

**Location**: `training_data/<signature>/v<N>/provenance.jsonl`

**Structure**:
```json
{
  "example_id": "abc123",
  "source": "telemetry",
  "timestamp": "2025-11-04T12:00:00Z",
  "metadata": {
    "original_timestamp_ms": 1699564800000,
    "latency_ms": 250,
    "cost_usd": 0.0018,
    "model": "claude-haiku-4-5"
  }
}
```

**Source Types**:
- `telemetry`: Production telemetry logs
- `git`: Git commit mining
- `synthetic`: Synthetic data generation

## Monthly Optimization Pipeline

### Workflow

1. **Data Collection** (Day 1):
   - Aggregate telemetry logs from past month
   - Mine git commits for new examples
   - Generate synthetic edge cases
   - Store in versioned datasets

2. **Quality Gates** (Day 2):
   - Validate schema compliance
   - Check difficulty stratification
   - Deduplicate across sources
   - Flag low-quality examples

3. **Optimization** (Days 3-5):
   - Run MIPROv2 with 50 trials per signature
   - Generate optimized prompts and demonstrations
   - Save to `<signature>_optimized_v<N>.json`

4. **A/B Testing** (Days 6-30):
   - Deploy optimized version to 10% of traffic
   - Monitor success rate, latency, cost
   - Auto-rollback if metrics degrade
   - Full rollout if metrics improve

### Automation

**Cron Job** (Day 1 of month at 2 AM):
```bash
0 2 1 * * cd /path/to/mnemosyne && \
  uv run python3 src/orchestration/dspy_modules/data_collection/optimization_orchestrator.py \
    --config src/orchestration/monitoring_config.json \
    --output-dir /tmp/optimization_$(date +\%Y\%m) \
    2>&1 | tee logs/optimization_$(date +\%Y\%m).log
```

## Testing

### Unit Tests

**Location**: `src/orchestration/dspy_modules/data_collection/test_telemetry_aggregator.py`

**Coverage**:
- InteractionLog parsing from JSON Lines
- Quality filtering (success, latency, cost)
- SHA256-based deduplication
- DatasetManager integration
- Provenance tracking
- Error handling for malformed logs

**Run Tests**:
```bash
cd src/orchestration/dspy_modules/data_collection
uv run pytest test_telemetry_aggregator.py -v
```

### Integration Tests

**End-to-End Flow**:
1. Generate sample telemetry logs
2. Run aggregator
3. Verify versioned datasets created
4. Check provenance tracking
5. Validate schema compliance

## Monitoring

### Key Metrics

1. **Sampling Rate**: Actual percentage of sampled requests
2. **Log File Size**: Monitor disk usage
3. **Quality Distribution**: Success rate, latency, cost distributions
4. **Deduplication Rate**: Percentage of duplicates filtered
5. **Dataset Growth**: Examples per signature over time

### Alerts

**Success Rate Drop**:
```
Alert: DSPy module success rate < 95%
Module: reviewer
Signature: extract_requirements
Current: 89%
Threshold: 95%
Action: Investigate errors, consider rollback
```

**High Latency**:
```
Alert: DSPy module p99 latency > 3s
Module: optimizer
Signature: discover_skills
Current: 4200ms
Threshold: 3000ms
Action: Optimize prompt, reduce context
```

## Privacy & Security

### PII Handling

**Telemetry logs MAY contain sensitive user data. Implement:**

1. **Data Minimization**: Only log inputs/outputs needed for optimization
2. **Anonymization**: Remove personally identifiable information
3. **Encryption**: Encrypt logs at rest and in transit
4. **Retention**: Auto-delete logs after 90 days
5. **Access Control**: Restrict log access to authorized personnel

### Best Practices

```rust
// Before logging, sanitize sensitive fields
let sanitized_input = sanitize_user_input(&input);

// Log with anonymization
self.log_interaction(InteractionLog {
    input: sanitized_input,  // PII removed
    // ...
});
```

## Troubleshooting

### Common Issues

**1. No telemetry logs generated**
- Check `monitoring_config.json`: `telemetry.enabled = true`
- Verify log file path exists and is writable
- Confirm sampling rate > 0

**2. Low telemetry example count**
- Increase sampling rate (default: 10%)
- Extend collection period
- Check quality thresholds (may be too strict)

**3. Deduplication removes too many examples**
- Review input variations - may indicate repetitive usage
- Consider relaxing deduplication (hash only subset of inputs)
- Generate synthetic examples for diversity

**4. Optimization pipeline fails**
- Check telemetry log format (must be valid JSON Lines)
- Verify DatasetManager permissions
- Review error logs in `logs/optimization_*.log`

## Future Enhancements

1. **Semantic Quality Scoring**: Use LLM to score output quality
2. **Adaptive Sampling**: Increase sampling for low-performing signatures
3. **Real-time Monitoring**: Dashboard for telemetry metrics
4. **Federated Learning**: Aggregate telemetry across deployments
5. **Differential Privacy**: Noise injection for privacy preservation

## References

- [DSPy Documentation](https://dspy-docs.vercel.app/)
- [MIPROv2 Paper](https://arxiv.org/abs/2406.11695)
- [DatasetManager Implementation](../src/orchestration/dspy_modules/data_collection/dataset_manager.py)
- [DSpyInstrumentation Source](../src/orchestration/dspy_instrumentation.rs)
