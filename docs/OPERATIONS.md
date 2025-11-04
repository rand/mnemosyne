# DSPy Integration Operations Runbook

Production operations guide for the Mnemosyne DSPy integration. This runbook covers deployment, monitoring, troubleshooting, and maintenance procedures.

## Table of Contents

1. [Production Deployment](#production-deployment)
2. [Monitoring & Alerting](#monitoring--alerting)
3. [Performance Tuning](#performance-tuning)
4. [Troubleshooting](#troubleshooting)
5. [Maintenance Tasks](#maintenance-tasks)
6. [Incident Response](#incident-response)
7. [Backup & Recovery](#backup--recovery)
8. [Security](#security)
9. [Continuous Optimization](#continuous-optimization)
10. [Scaling](#scaling)

---

## Production Deployment

### Pre-Deployment Checklist

**Environment Verification**:
- [ ] Python 3.11+ installed
- [ ] Rust 1.70+ installed
- [ ] `ANTHROPIC_API_KEY` configured
- [ ] Storage backend accessible
- [ ] Network connectivity verified

**Dependency Check**:
```bash
# Python dependencies
cd src/orchestration/dspy_modules
uv sync
uv run python3 -c "import dspy; print(f'DSPy {dspy.__version__}')"

# Rust dependencies
cargo --version
cargo build --features python --release
```

**Test Suite Verification**:
```bash
# Python tests
uv run pytest test_*.py -v

# Rust tests
cargo test --features python -- --ignored

# Expected: 145+ tests passing, 80% coverage
```

### Deployment Procedure

**Step 1: Build Production Binaries**:
```bash
# Build with optimizations
cargo build --release --features python

# Verify binary
./target/release/mnemosyne --version
```

**Step 2: Deploy Optimized Modules**:
```bash
# Copy optimized modules to production
cp src/orchestration/dspy_modules/results/reviewer_optimized_v1.json \
   /etc/mnemosyne/modules/

# Verify module integrity
jq '.' /etc/mnemosyne/modules/reviewer_optimized_v1.json > /dev/null
```

**Step 3: Configure Environment**:
```bash
# Production environment
cat > /etc/mnemosyne/dspy.env << 'EOF'
ANTHROPIC_API_KEY=<production-key>
MNEMOSYNE_DSPY_MODEL=claude-3-5-sonnet-20241022
MNEMOSYNE_DSPY_CACHE_DIR=/var/cache/mnemosyne/dspy
EOF

# Secure environment file
chmod 600 /etc/mnemosyne/dspy.env
chown mnemosyne:mnemosyne /etc/mnemosyne/dspy.env
```

**Step 4: Start Services**:
```bash
# Start with systemd (recommended)
systemctl start mnemosyne
systemctl enable mnemosyne

# Verify startup
systemctl status mnemosyne
journalctl -u mnemosyne -f
```

**Step 5: Smoke Tests**:
```bash
# Test basic operations
mnemosyne-cli reviewer extract "Implement user authentication"

# Expected: Returns list of requirements
```

### A/B Testing Deployment

For gradual rollout of optimized modules:

**Traffic Split Configuration**:
```bash
# Start with 10% traffic
export DSPY_OPTIMIZED_TRAFFIC=0.10

# Monitor for 24h, then increase
export DSPY_OPTIMIZED_TRAFFIC=0.50

# If stable, full rollout
export DSPY_OPTIMIZED_TRAFFIC=1.00
```

**Rollback Procedure**:
```bash
# Immediate rollback to baseline
export DSPY_OPTIMIZED_TRAFFIC=0.00
systemctl restart mnemosyne

# Verify metrics
tail -f /var/log/mnemosyne/dspy.log | grep "module_version"
```

---

## Monitoring & Alerting

### Key Metrics

**Performance Metrics**:
| Metric | Target | Alert Threshold |
|--------|--------|-----------------|
| Request Latency (p50) | < 200ms | > 500ms |
| Request Latency (p95) | < 400ms | > 1000ms |
| Request Latency (p99) | < 1000ms | > 2000ms |
| Success Rate | > 95% | < 90% |
| Token Usage per Request | < 1000 | > 2000 |
| Cost per Request | < $0.01 | > $0.05 |

**System Metrics**:
| Metric | Target | Alert Threshold |
|--------|--------|-----------------|
| CPU Usage | < 60% | > 80% |
| Memory Usage | < 70% | > 85% |
| Disk Usage | < 80% | > 90% |
| Python GIL Contention | < 10% | > 25% |

### Prometheus Metrics

**Exported Metrics**:
```
# Request metrics
dspy_requests_total{module="reviewer",signature="extract_requirements"} 1234
dspy_request_duration_seconds{module="reviewer",quantile="0.95"} 0.250
dspy_request_errors_total{module="reviewer",error_type="timeout"} 5

# Token metrics
dspy_tokens_total{module="reviewer",type="input"} 50000
dspy_tokens_total{module="reviewer",type="output"} 25000

# Cost metrics
dspy_cost_usd_total{module="reviewer"} 5.25

# Module metrics
dspy_module_version{module="reviewer",version="v1"} 1
```

**Prometheus Configuration**:
```yaml
scrape_configs:
  - job_name: 'mnemosyne-dspy'
    static_configs:
      - targets: ['localhost:9090']
    scrape_interval: 15s
```

### Alert Rules

**Critical Alerts** (PagerDuty):
```yaml
groups:
  - name: dspy_critical
    rules:
      - alert: DSPyHighErrorRate
        expr: rate(dspy_request_errors_total[5m]) > 0.10
        for: 5m
        annotations:
          summary: "DSPy error rate > 10%"
          
      - alert: DSPyHighLatency
        expr: dspy_request_duration_seconds{quantile="0.95"} > 2.0
        for: 10m
        annotations:
          summary: "DSPy p95 latency > 2s"
```

**Warning Alerts** (Slack):
```yaml
  - name: dspy_warnings
    rules:
      - alert: DSPyElevatedLatency
        expr: dspy_request_duration_seconds{quantile="0.95"} > 0.5
        for: 15m
        annotations:
          summary: "DSPy p95 latency elevated"
          
      - alert: DSPyHighCost
        expr: rate(dspy_cost_usd_total[1h]) > 10.0
        for: 30m
        annotations:
          summary: "DSPy cost > $10/hour"
```

### Logging

**Log Levels**:
```bash
# Production: INFO
export RUST_LOG=info,mnemosyne_dspy=info

# Debugging: DEBUG
export RUST_LOG=debug,mnemosyne_dspy=debug

# Troubleshooting: TRACE
export RUST_LOG=trace,mnemosyne_dspy=trace
```

**Log Aggregation** (JSON format):
```json
{
  "timestamp": "2025-11-03T12:00:00Z",
  "level": "INFO",
  "module": "reviewer",
  "signature": "extract_requirements",
  "latency_ms": 150,
  "tokens": {"input": 100, "output": 50},
  "cost_usd": 0.001,
  "success": true
}
```

**Query Examples**:
```bash
# Error rate (last hour)
jq 'select(.success == false)' /var/log/mnemosyne/dspy.log | wc -l

# Average latency
jq -s 'map(.latency_ms) | add/length' /var/log/mnemosyne/dspy.log

# Cost summary
jq -s 'map(.cost_usd) | add' /var/log/mnemosyne/dspy.log
```

---

## Performance Tuning

### Latency Optimization

**Python GIL Optimization**:
```rust
// Increase tokio thread pool
let runtime = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(8)  // Adjust based on CPU cores
    .enable_all()
    .build()?;
```

**Connection Pooling**:
```python
# DSPy LM configuration
import dspy
dspy.configure(lm=dspy.Claude(
    model="claude-3-5-sonnet-20241022",
    max_tokens=2000,
    temperature=0.0,
    # Connection pool settings
    max_retries=3,
    timeout=30.0,
))
```

**Caching Strategy**:
```bash
# Enable DSPy caching
export DSPY_CACHE_ENABLED=true
export DSPY_CACHE_TTL=3600  # 1 hour

# Cache directory
export MNEMOSYNE_DSPY_CACHE_DIR=/var/cache/mnemosyne/dspy
```

### Cost Optimization

**Model Selection**:
```bash
# Use Haiku for non-critical operations
export MNEMOSYNE_DSPY_MODEL_FALLBACK=claude-haiku-4-5-20251001

# Cost comparison (per 1M tokens):
# Sonnet: $3 input / $15 output
# Haiku:  $0.80 input / $4 output
```

**Request Batching**:
```rust
// Batch requests where possible
let requirements = tokio::try_join!(
    adapter.extract_requirements(intent1, None),
    adapter.extract_requirements(intent2, None),
    adapter.extract_requirements(intent3, None),
)?;
```

**Token Limits**:
```python
# Limit output tokens
dspy.configure(lm=dspy.Claude(
    model="claude-3-5-sonnet-20241022",
    max_tokens=500,  # Reduce for predictable operations
))
```

### Throughput Optimization

**Concurrency Tuning**:
```bash
# Adjust based on Anthropic API rate limits
export DSPY_MAX_CONCURRENT=10
export DSPY_RATE_LIMIT_RPM=90000  # 90k tokens/min
```

**Load Balancing**:
```nginx
upstream mnemosyne_dspy {
    least_conn;
    server 10.0.1.1:8080;
    server 10.0.1.2:8080;
    server 10.0.1.3:8080;
}
```

---

## Troubleshooting

### Common Issues

#### Issue 1: High Latency

**Symptoms**:
- p95 latency > 1000ms
- User complaints about slow responses

**Diagnosis**:
```bash
# Check latency distribution
jq '.latency_ms' /var/log/mnemosyne/dspy.log | sort -n | tail -100

# Identify slow signatures
jq 'select(.latency_ms > 1000) | .signature' /var/log/mnemosyne/dspy.log | sort | uniq -c
```

**Resolution**:
1. Check Anthropic API status
2. Review token usage (reduce if excessive)
3. Enable caching
4. Consider model downgrade (Sonnet → Haiku)

#### Issue 2: High Error Rate

**Symptoms**:
- Success rate < 90%
- Errors in logs

**Diagnosis**:
```bash
# Error breakdown
jq 'select(.success == false) | .error_type' /var/log/mnemosyne/dspy.log | sort | uniq -c

# Common error types:
# - timeout: Increase timeout
# - rate_limit: Reduce concurrency
# - invalid_request: Check input validation
# - api_error: Check Anthropic status
```

**Resolution**:
```bash
# Timeout errors
export DSPY_TIMEOUT=60  # Increase from 30s

# Rate limit errors
export DSPY_MAX_CONCURRENT=5  # Reduce concurrency

# Invalid request errors
# Review input validation in adapters
```

#### Issue 3: High Cost

**Symptoms**:
- Cost > $10/hour
- Budget alerts triggering

**Diagnosis**:
```bash
# Cost by signature
jq 'group_by(.signature) | map({signature: .[0].signature, cost: map(.cost_usd) | add})' /var/log/mnemosyne/dspy.log

# Token usage analysis
jq '.tokens | {input, output}' /var/log/mnemosyne/dspy.log | jq -s 'map(.input) | add'
```

**Resolution**:
1. Switch to Haiku for non-critical operations
2. Reduce max_tokens limits
3. Enable aggressive caching
4. Review training data (remove duplicates)

#### Issue 4: Memory Leaks

**Symptoms**:
- Memory usage growing over time
- OOM errors

**Diagnosis**:
```bash
# Memory profiling
valgrind --leak-check=full ./target/release/mnemosyne

# Python GIL monitoring
py-spy top --pid $(pgrep mnemosyne)
```

**Resolution**:
1. Check for unclosed Python objects
2. Review tokio task cleanup
3. Enable periodic GC
4. Restart service if needed

### Debug Mode

**Enable Verbose Logging**:
```bash
# Rust debug
export RUST_LOG=trace,mnemosyne_dspy=trace

# Python debug
export DSPY_DEBUG=1

# Restart service
systemctl restart mnemosyne

# Monitor logs
journalctl -u mnemosyne -f | grep -E "ERROR|WARN|DEBUG"
```

---

## Maintenance Tasks

### Daily Tasks

**Health Check**:
```bash
#!/bin/bash
# /usr/local/bin/dspy-health-check.sh

# Check service status
systemctl is-active mnemosyne || exit 1

# Check error rate (last hour)
error_rate=$(jq -s 'map(select(.success == false)) | length' \
  /var/log/mnemosyne/dspy-$(date +%Y-%m-%d).log)

if [ "$error_rate" -gt 100 ]; then
  echo "High error rate: $error_rate errors in last hour"
  exit 1
fi

# Check latency (p95 < 1s)
p95_latency=$(jq -s 'map(.latency_ms) | sort | .[length*0.95|floor]' \
  /var/log/mnemosyne/dspy-$(date +%Y-%m-%d).log)

if [ "$p95_latency" -gt 1000 ]; then
  echo "High latency: p95 = ${p95_latency}ms"
  exit 1
fi

echo "Health check passed"
```

**Log Rotation**:
```bash
# /etc/logrotate.d/mnemosyne-dspy
/var/log/mnemosyne/dspy*.log {
    daily
    rotate 30
    compress
    delaycompress
    notifempty
    create 0640 mnemosyne mnemosyne
    sharedscripts
    postrotate
        systemctl reload mnemosyne
    endscript
}
```

### Weekly Tasks

**Performance Review**:
```bash
#!/bin/bash
# /usr/local/bin/dspy-weekly-report.sh

week_start=$(date -d '7 days ago' +%Y-%m-%d)

echo "=== DSPy Weekly Performance Report ==="
echo "Week starting: $week_start"

# Request volume
echo -n "Total requests: "
jq -s 'length' /var/log/mnemosyne/dspy-*.log | awk '{sum+=$1} END {print sum}'

# Average latency
echo -n "Average latency: "
jq -s 'map(.latency_ms) | add/length' /var/log/mnemosyne/dspy-*.log | xargs printf "%.0fms\n"

# Total cost
echo -n "Total cost: $"
jq -s 'map(.cost_usd) | add' /var/log/mnemosyne/dspy-*.log

# Error rate
echo -n "Error rate: "
jq -s 'map(select(.success == false)) | length / (map(1) | length) * 100' \
  /var/log/mnemosyne/dspy-*.log | xargs printf "%.2f%%\n"
```

**Training Data Collection**:
```bash
# Export successful interactions for training
cd src/orchestration/dspy_modules
python3 import_production_logs.py \
  --input /var/log/mnemosyne/dspy-production.jsonl \
  --output training_data/production_$(date +%Y-%m-%d).json \
  --min-success-rate 0.95
```

### Monthly Tasks

**Continuous Optimization**:
```bash
# Run optimization pipeline
cd src/orchestration/dspy_modules

# Import production data
python3 continuous_optimize.py \
  --module reviewer \
  --production-logs /var/log/mnemosyne/dspy-production.jsonl \
  --trials 50 \
  --min-improvement 0.02

# If improved, deploy via A/B test
# See "Continuous Optimization" section
```

**Capacity Planning**:
```bash
# Analyze growth trends
echo "=== Monthly Growth Analysis ==="

# Request volume trend
echo "Request volume (last 3 months):"
for month in {2..0}; do
  start_date=$(date -d "$month months ago" +%Y-%m-01)
  echo -n "$start_date: "
  jq -s 'length' /var/log/mnemosyne/dspy-$start_date*.log | awk '{sum+=$1} END {print sum}'
done

# Cost trend
echo "Cost (last 3 months):"
for month in {2..0}; do
  start_date=$(date -d "$month months ago" +%Y-%m-01)
  echo -n "$start_date: $"
  jq -s 'map(.cost_usd) | add' /var/log/mnemosyne/dspy-$start_date*.log
done
```

---

## Incident Response

### Severity Levels

**P0 - Critical** (Immediate response):
- Service down (> 50% error rate)
- Data loss
- Security breach

**P1 - High** (< 1 hour response):
- Degraded performance (p95 > 2s)
- Elevated error rate (10-50%)
- Cost spike (> 2x baseline)

**P2 - Medium** (< 4 hours response):
- Minor performance issues
- Isolated errors
- Configuration issues

**P3 - Low** (< 24 hours response):
- Documentation errors
- Feature requests
- Optimization opportunities

### Incident Response Procedure

**Step 1: Triage**:
```bash
# Assess severity
./scripts/dspy-health-check.sh

# Check recent changes
git log --since="1 hour ago" --oneline

# Review recent deploys
tail -100 /var/log/mnemosyne/deploy.log
```

**Step 2: Contain**:
```bash
# Rollback if recent deploy
systemctl stop mnemosyne
git checkout <previous-stable-commit>
cargo build --release --features python
systemctl start mnemosyne

# Or disable DSPy integration
export ENABLE_DSPY=false
systemctl restart mnemosyne
```

**Step 3: Investigate**:
```bash
# Collect diagnostic data
mkdir -p /tmp/incident-$(date +%Y%m%d-%H%M%S)
cp /var/log/mnemosyne/dspy.log /tmp/incident-*/
journalctl -u mnemosyne --since "1 hour ago" > /tmp/incident-*/journal.log

# Analyze errors
jq 'select(.success == false)' /var/log/mnemosyne/dspy.log | tail -100
```

**Step 4: Resolve**:
- Apply fix
- Test in staging
- Deploy to production
- Monitor for 30 minutes

**Step 5: Postmortem**:
```markdown
# Incident Postmortem Template

## Summary
- Date/Time:
- Duration:
- Severity:
- Impact:

## Timeline
- HH:MM - Incident detected
- HH:MM - Response initiated
- HH:MM - Root cause identified
- HH:MM - Fix deployed
- HH:MM - Incident resolved

## Root Cause
[Detailed analysis]

## Resolution
[Steps taken to resolve]

## Prevention
[Measures to prevent recurrence]

## Action Items
- [ ] Update monitoring
- [ ] Add alerting
- [ ] Document runbook
```

---

## Backup & Recovery

### Backup Procedures

**What to Back Up**:
- Optimized module files (`*.json`)
- Training data (`training_data/*.json`)
- Production logs (last 30 days)
- Configuration files

**Backup Schedule**:
```bash
# Daily backup script
#!/bin/bash
# /usr/local/bin/dspy-backup.sh

backup_dir=/backup/mnemosyne/$(date +%Y-%m-%d)
mkdir -p $backup_dir

# Modules
cp -r src/orchestration/dspy_modules/results/*.json $backup_dir/

# Training data
cp -r src/orchestration/dspy_modules/training_data/*.json $backup_dir/

# Logs (last 7 days)
find /var/log/mnemosyne -name "dspy-*.log" -mtime -7 -exec cp {} $backup_dir/ \;

# Compress
tar -czf $backup_dir.tar.gz $backup_dir
rm -rf $backup_dir

# Retention (keep 30 days)
find /backup/mnemosyne -name "*.tar.gz" -mtime +30 -delete
```

### Recovery Procedures

**Module Recovery**:
```bash
# Restore optimized modules
tar -xzf /backup/mnemosyne/2025-11-03.tar.gz
cp 2025-11-03/results/*.json src/orchestration/dspy_modules/results/

# Verify integrity
jq '.' src/orchestration/dspy_modules/results/reviewer_optimized_v1.json
```

**Training Data Recovery**:
```bash
# Restore training data
cp 2025-11-03/training_data/*.json src/orchestration/dspy_modules/training_data/

# Re-train if needed
cd src/orchestration/dspy_modules
python3 optimize_reviewer.py --trials 25 --output results/reviewer_recovered.json
```

---

## Security

### API Key Management

**Rotation Policy**:
```bash
# Rotate API keys monthly
# 1. Generate new key in Anthropic Console
# 2. Update environment
echo "ANTHROPIC_API_KEY=<new-key>" > /etc/mnemosyne/dspy.env

# 3. Restart service
systemctl restart mnemosyne

# 4. Verify
tail -f /var/log/mnemosyne/dspy.log | grep "authentication"

# 5. Revoke old key in Anthropic Console
```

**Key Storage**:
```bash
# Never commit keys to git
echo "*.env" >> .gitignore

# Use secrets management
# AWS Secrets Manager, HashiCorp Vault, etc.
```

### Network Security

**Firewall Rules**:
```bash
# Allow only necessary connections
ufw allow from 10.0.0.0/8 to any port 8080  # Internal only
ufw deny from any to any port 8080  # Deny external
```

**TLS Configuration**:
```nginx
# HTTPS only
server {
    listen 443 ssl http2;
    ssl_certificate /etc/ssl/certs/mnemosyne.crt;
    ssl_certificate_key /etc/ssl/private/mnemosyne.key;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
}
```

### Audit Logging

**Enable Audit Trail**:
```bash
# Log all DSPy operations
export DSPY_AUDIT_LOG=/var/log/mnemosyne/audit.log

# Audit log format (immutable)
{
  "timestamp": "2025-11-03T12:00:00Z",
  "user": "system",
  "operation": "extract_requirements",
  "input_hash": "sha256:...",
  "output_hash": "sha256:...",
  "success": true
}
```

---

## Continuous Optimization

### Monthly Optimization Cycle

**Week 1: Data Collection**:
```bash
# Export production logs
python3 import_production_logs.py \
  --input /var/log/mnemosyne/dspy-production.jsonl \
  --output training_data/monthly_$(date +%Y-%m).json \
  --merge \
  --deduplicate
```

**Week 2: Baseline Benchmarking**:
```bash
# Measure current performance
python3 baseline_benchmark.py \
  --module reviewer \
  --iterations 100 \
  --output results/baseline_$(date +%Y-%m).json
```

**Week 3: Optimization**:
```bash
# Run MIPROv2 optimization
python3 continuous_optimize.py \
  --module reviewer \
  --production-logs /var/log/mnemosyne/dspy-production.jsonl \
  --trials 50 \
  --min-improvement 0.02 \
  --dry-run  # Review before deploying
```

**Week 4: A/B Testing & Deployment**:
```bash
# Deploy with 10% traffic
export DSPY_OPTIMIZED_TRAFFIC=0.10
systemctl restart mnemosyne

# Monitor for 48h
# If metrics improved, increase to 50%
# If metrics degraded, rollback
```

---

## Scaling

### Horizontal Scaling

**Load Balancer Configuration**:
```yaml
# kubernetes deployment
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mnemosyne-dspy
spec:
  replicas: 3  # Adjust based on load
  selector:
    matchLabels:
      app: mnemosyne-dspy
  template:
    metadata:
      labels:
        app: mnemosyne-dspy
    spec:
      containers:
      - name: mnemosyne
        image: mnemosyne:latest
        env:
        - name: ANTHROPIC_API_KEY
          valueFrom:
            secretKeyRef:
              name: dspy-secrets
              key: api-key
```

### Vertical Scaling

**Resource Allocation**:
```yaml
resources:
  requests:
    memory: "2Gi"
    cpu: "1000m"
  limits:
    memory: "4Gi"
    cpu: "2000m"
```

### Auto-Scaling

**HPA Configuration**:
```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: mnemosyne-dspy-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: mnemosyne-dspy
  minReplicas: 2
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Pods
    pods:
      metric:
        name: dspy_request_duration_seconds
      target:
        type: AverageValue
        averageValue: "0.5"
```

---

## Appendix: Quick Reference

### Common Commands

```bash
# Health check
systemctl status mnemosyne

# View logs
journalctl -u mnemosyne -f

# Test operation
mnemosyne-cli reviewer extract "Test intent"

# Check metrics
curl localhost:9090/metrics | grep dspy_

# Rotate API key
vi /etc/mnemosyne/dspy.env && systemctl restart mnemosyne

# Backup
/usr/local/bin/dspy-backup.sh

# Emergency rollback
systemctl stop mnemosyne && \
git checkout <stable-commit> && \
cargo build --release --features python && \
systemctl start mnemosyne
```

### Support Contacts

- **On-Call**: PagerDuty rotation
- **Slack**: #mnemosyne-ops
- **Documentation**: docs/DSPY_INTEGRATION.md
- **Issue Tracker**: GitHub Issues

---

## Summary

This operations runbook provides comprehensive procedures for production deployment, monitoring, troubleshooting, and maintenance of the DSPy integration.

**Key Takeaways**:
- Follow deployment checklist rigorously
- Monitor key metrics continuously
- Respond to incidents promptly
- Maintain regular optimization cycle
- Scale proactively based on trends

For questions or issues not covered in this runbook, consult [DSPY_INTEGRATION.md](./DSPY_INTEGRATION.md) or contact the ops team.
