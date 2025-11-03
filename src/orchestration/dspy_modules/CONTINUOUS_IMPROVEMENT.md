# DSPy Continuous Improvement Guide

This document describes the continuous improvement process for DSPy-optimized ReviewerModule.

## Current State (2025-11-03)

**Optimization v1 Results**:
- **extract_requirements**: 36.7% → 56.0% (+52.4% improvement) ✨
- **validate_intent**: 100% → 100% (already perfect)
- **validate_completeness**: 75% → 75% (baseline strong, potential for improvement)
- **validate_correctness**: 75% → 75% (baseline strong, potential for improvement)
- **generate_guidance**: 50% → 50% (moderate baseline, room for improvement)

**Training Data**: 20 examples per signature
**Optimization Method**: MIPROv2 with 25 trials per signature
**Model**: Claude Haiku 4.5

---

## Quick Start: Expanding Training Data

The most effective way to improve optimization is to add more training examples.

### 1. View Templates

```bash
cd src/orchestration/dspy_modules

# View templates for a specific signature
python3 expand_training_data.py --signature validate_completeness --show-template

# View all templates
python3 expand_training_data.py --signature all --show-template
```

### 2. Create Examples Interactively

```bash
# Create 5 new examples interactively
python3 expand_training_data.py --signature validate_completeness --count 5 --interactive
```

The script will:
- Prompt you for inputs and outputs
- Show you the generated example
- Ask for confirmation before adding
- Automatically backup existing data
- Save the expanded dataset

### 3. Re-run Optimization

```bash
# Re-run with expanded dataset (now 25 examples instead of 20)
cd src/orchestration/dspy_modules
python3 optimize_validate_completeness.py --trials 25 --output /tmp/validate_completeness_v2.json

# Check improvement
python3 -m json.tool /tmp/validate_completeness_v2.results.json
```

### 4. Compare Results

```bash
# v1: 75% accuracy (20 examples, 25 trials)
# v2: expected 78-82% accuracy (25 examples, 25 trials)
```

---

## Phase 1: Immediate Improvements (Next 1-2 Weeks)

### Goal: Increase from 20 → 50 examples per signature

**Focus Signatures** (0% improvement in v1):
- `validate_completeness`
- `validate_correctness`
- `generate_guidance`

### Recommended Workflow

1. **Generate 10 examples manually** (2-3 hours per signature)
   - Use `expand_training_data.py --interactive`
   - Focus on diversity: easy/medium/hard, different categories
   - Include both positive and negative examples
   - Document edge cases and failure modes

2. **Re-optimize with 30 total examples** (30-45 min)
   ```bash
   python3 optimize_[signature].py --trials 25 --output /tmp/[signature]_v2.json
   ```

3. **Evaluate improvement**
   - Expected: 5-10% improvement with 30 examples
   - If improvement < 3%: Add 10 more examples
   - If improvement > 7%: Proceed to next signature

4. **Iterate to 50 examples**
   - Target: 10-15% total improvement vs baseline
   - Validate on held-out test set (see Phase 2)

### Creating High-Quality Examples

**Positive Examples** (is_complete=true, is_correct=true):
- Cover all requirements comprehensively
- Show different implementation styles
- Include testing and documentation
- Demonstrate best practices

**Negative Examples** (is_complete=false, is_correct=false):
- Missing critical requirements (security, testing, error handling)
- Edge case failures
- Performance issues
- Incorrect logic or algorithm

**Diversity Checklist**:
- [ ] Different domains (auth, API, database, caching, UI, etc.)
- [ ] Different complexity levels (easy, medium, hard)
- [ ] Different failure modes (security, correctness, completeness)
- [ ] Different implementation approaches (sync/async, patterns, architectures)

---

## Phase 2: Enhanced Metrics & Validation

### Held-Out Test Set

Create 10-20 examples that are NEVER used in training:

```bash
# Create held-out test set
cp training_data/validate_completeness.json training_data/validate_completeness_train.json
# Manually move 10 examples from _train.json to validate_completeness_test.json

# Update optimizer to use _train.json only
# Evaluate both baseline and optimized on _test.json
```

### Multi-Dimensional Scoring

Instead of single accuracy/F1 score, track:
- **Precision**: How many predicted positives are correct?
- **Recall**: How many actual positives were found?
- **F1**: Harmonic mean of precision and recall
- **Confidence**: How sure is the model?

Example implementation:
```python
def enhanced_metric(example, pred, trace=None) -> Dict[str, float]:
    """Multi-dimensional evaluation metric."""
    return {
        "accuracy": 1.0 if pred.is_complete == example.is_complete else 0.0,
        "explanation_quality": score_explanation(pred.explanation),
        "precision": precision_score(example, pred),
        "recall": recall_score(example, pred),
        "f1": f1_score(example, pred)
    }
```

---

## Phase 3: Longer Optimization Runs

### 50-100 Trial Runs

For signatures with more data (50+ examples), run longer optimization:

```bash
# 50-trial run (60-90 minutes)
python3 optimize_validate_completeness.py --trials 50 --output /tmp/validate_completeness_v3.json

# 100-trial run (2-3 hours) - for final production version
python3 optimize_validate_completeness.py --trials 100 --output /tmp/validate_completeness_v4.json
```

**Diminishing Returns**: Expect most gains in first 25-50 trials. Beyond 50 trials, improvement usually < 2% additional.

---

## Phase 4: Production Integration

### Deploy Optimized Module

1. **Load optimized module in Rust**:
   ```rust
   // src/orchestration/adapters/dspy_adapter.rs
   let optimized_module = ReviewerModule::load("reviewer_optimized_v1.json")?;
   ```

2. **A/B Testing**:
   ```rust
   let module = if rand::random::<bool>() {
       load_baseline()
   } else {
       load_optimized()
   };
   ```

3. **Metrics Collection**:
   ```rust
   log::info!(
       "ReviewerModule.validate_completeness: baseline={}, optimized={}, latency_ms={}",
       baseline_result, optimized_result, duration.as_millis()
   );
   ```

### Usage Data Collection

Collect production examples for future optimization:

```python
# production_logger.py
@app.post("/api/reviewer/validate_completeness")
async def validate_completeness(request: CompletenenessRequest):
    start = time.time()
    result = reviewer.validate_completeness(
        implementation=request.implementation,
        requirements=request.requirements
    )
    latency_ms = (time.time() - start) * 1000

    # Sample 10% of requests for training data
    if random.random() < 0.1:
        log_training_example({
            "signature": "validate_completeness",
            "inputs": {
                "implementation": request.implementation,
                "requirements": request.requirements
            },
            "outputs": {
                "is_complete": result.is_complete,
                "missing_requirements": result.missing_requirements,
                "explanation": result.explanation
            },
            "metadata": {
                "timestamp": datetime.now().isoformat(),
                "latency_ms": latency_ms,
                "model": "claude-haiku-4-5",
                "version": "reviewer_optimized_v1"
            }
        })

    return result
```

---

## Phase 5: Monthly Re-Optimization

### Continuous Improvement Loop

**Goal**: Grow training set from production usage, re-optimize monthly

**Schedule**:
- **Weekly**: Collect production examples (target 20-50 new examples/week)
- **Monthly**: Re-run optimization with expanded dataset
- **Quarterly**: Major optimization campaign with full dataset

**Process**:
```bash
#!/bin/bash
# continuous_optimization.sh

MONTH=$1
SIGNATURE=$2

echo "=== Monthly Re-Optimization: Month $MONTH, Signature $SIGNATURE ==="

# 1. Load existing training data
EXISTING_COUNT=$(python3 -c "import json; print(len(json.load(open('training_data/${SIGNATURE}.json'))))")
echo "Existing examples: $EXISTING_COUNT"

# 2. Import production examples
python3 import_production_examples.py --signature $SIGNATURE --month $MONTH

# 3. Count new total
NEW_COUNT=$(python3 -c "import json; print(len(json.load(open('training_data/${SIGNATURE}.json'))))")
echo "New total: $NEW_COUNT (+$((NEW_COUNT - EXISTING_COUNT)))"

# 4. Run optimization
python3 optimize_${SIGNATURE}.py --trials 50 --output /tmp/${SIGNATURE}_v${MONTH}.json

# 5. Compare to previous month
python3 compare_versions.py --baseline v$((MONTH-1)) --candidate v${MONTH} --signature $SIGNATURE

# 6. If improvement > 3%, deploy to production
# (A/B test, gradual rollout, monitoring)
```

---

## Tools Reference

### expand_training_data.py

```bash
# Show templates
python3 expand_training_data.py --signature <name> --show-template

# Interactive creation
python3 expand_training_data.py --signature <name> --count 10 --interactive

# Show positive/negative templates only
python3 expand_training_data.py --signature validate_completeness --show-template --template-type positive
```

### optimize_[signature].py

```bash
# Standard optimization (25 trials)
python3 optimize_validate_completeness.py --trials 25 --output /tmp/validate_completeness_v2.json

# Long optimization (100 trials)
python3 optimize_validate_completeness.py --trials 100 --output /tmp/validate_completeness_v2.json

# Test mode (5 trials, fast validation)
python3 optimize_validate_completeness.py --test-mode --output /tmp/test.json
```

### aggregate_optimized.py

```bash
# Combine optimized per-signature modules
python3 aggregate_optimized.py --modules-dir /tmp --output reviewer_optimized_v2.json
```

---

## Success Metrics

### Short-term (1-2 months)
- ✅ 50+ examples per signature
- ✅ 5-15% improvement on stagnant signatures (completeness, correctness, guidance)
- ✅ Deploy optimized module to production with telemetry
- ✅ Collect 100+ production examples

### Medium-term (3-6 months)
- ✅ 100+ examples per signature
- ✅ Continuous optimization pipeline running monthly
- ✅ 10-20% improvement across all signatures vs original baseline
- ✅ Production usage metrics: latency, cost, accuracy

### Long-term (6-12 months)
- ✅ 500+ examples per signature from production usage
- ✅ Fully automated optimization loop
- ✅ 20-30% improvement across all signatures
- ✅ Sub-second latency, <$0.01/operation cost
- ✅ A/B testing framework validating all improvements

---

## Troubleshooting

### "No improvement after adding examples"

**Possible causes**:
1. **Examples too similar**: Add more diversity (different domains, complexity levels)
2. **Metric not sensitive**: Review metric implementation, consider multi-dimensional scoring
3. **Examples mislabeled**: Audit training data quality
4. **Baseline already strong**: Accept current performance, focus on other signatures

**Solutions**:
- Audit 10 random examples for correctness
- Try longer optimization runs (50-100 trials)
- Improve metric to capture nuance
- Focus on edge cases and failure modes

### "Optimization taking too long"

**Expected times** (with num_threads=2, rate limit 90k tokens/min):
- 5 trials: 5-10 minutes
- 25 trials: 30-45 minutes
- 50 trials: 60-90 minutes
- 100 trials: 2-3 hours

**Speed optimizations**:
- Increase `num_threads` if rate limits allow
- Use smaller model (Haiku instead of Sonnet)
- Reduce `num_candidates` (default 10 → 5)
- Use minibatch optimization (already enabled)

### "Optimized worse than baseline"

**This should not happen**, but if it does:
1. Check logs for scoring errors
2. Validate metric implementation
3. Re-run optimization (may be unlucky trial)
4. Increase trials for more exploration
5. File issue with optimization logs

---

## Next Steps

1. **Immediate**: Use `expand_training_data.py` to add 10 examples to `validate_completeness`
2. **This week**: Re-optimize with 30 examples, validate improvement
3. **This month**: Expand all 3 stagnant signatures to 50 examples each
4. **Next month**: Deploy optimized module to production, start collecting usage data

Good luck optimizing!
