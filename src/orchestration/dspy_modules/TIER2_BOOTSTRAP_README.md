# Tier 2: BootstrapFewShot Optimizer

**Status**: ✅ READY FOR EXECUTION
**Created**: 2025-11-03
**Purpose**: Alternative optimizer better suited for datasets with <200 examples

---

## Overview

After Phase 3 v2 showed persistent stagnation (generate_guidance 50%→50%), research revealed that **MIPROv2 requires 200+ examples** to work effectively. Our datasets have only 50 examples per signature.

**BootstrapFewShot** is a simpler, faster optimizer specifically designed for small datasets (<200 examples). Research shows it often outperforms MIPROv2 when training data is limited.

---

## Research-Backed Rationale

### Why BootstrapFewShot?

1. **Dataset Size**: We have 50 examples/signature, but MIPROv2 needs 200+
2. **Simpler Approach**: BootstrapFewShot uses few-shot learning instead of complex instruction optimization
3. **Proven Track Record**: Often matches or beats MIPROv2 on small datasets
4. **Faster Execution**: 5-15 minutes vs 15-30 minutes for MIPROv2
5. **Lower Overfitting Risk**: Less prone to memorizing small training sets

### Key Differences from MIPROv2

| Aspect | MIPROv2 | BootstrapFewShot |
|--------|---------|------------------|
| Optimal Dataset Size | 200+ examples | <200 examples |
| Optimization Method | Bayesian search over instructions | Bootstrap few-shot demonstrations |
| Runtime | 15-30 min (25 trials) | 5-15 min |
| Complexity | High (3-stage optimization) | Low (single-pass bootstrapping) |
| Best For | Large diverse datasets | Small targeted datasets |

---

## Implementation

### Scripts Created

1. `bootstrap_validate_completeness.py` - BootstrapFewShot for completeness validation
2. `bootstrap_validate_correctness.py` - BootstrapFewShot for correctness validation
3. `bootstrap_generate_guidance.py` - BootstrapFewShot for guidance generation

### Key Configuration

```python
teleprompter = BootstrapFewShot(
    metric=<signature_metric>,
    max_bootstrapped_demos=8,    # Bootstrap 8 demonstrations
    max_labeled_demos=4,          # Use 4 labeled examples
    max_rounds=1,                 # Single-pass bootstrapping
    max_errors=5                  # Allow some errors during bootstrap
)
```

### Train/Test Split

**IMPORTANT**: Uses **20/80 split** (inverted from traditional ML)

- **Training**: 20% of data (10 examples) for bootstrapping
- **Validation**: 80% of data (40 examples) for evaluation

This is counterintuitive but **research-backed** for prompt optimization - more validation data helps prevent overfitting to specific prompt patterns.

---

## Usage

### Test Mode (Quick Validation)

```bash
cd /Users/rand/src/mnemosyne/src/orchestration/dspy_modules

# Test validate_completeness (~5 minutes)
env ANTHROPIC_API_KEY="..." uv run python3 bootstrap_validate_completeness.py \
  --test-mode --output /tmp/validate_completeness_bootstrap_test.json

# Test validate_correctness
env ANTHROPIC_API_KEY="..." uv run python3 bootstrap_validate_correctness.py \
  --test-mode --output /tmp/validate_correctness_bootstrap_test.json

# Test generate_guidance
env ANTHROPIC_API_KEY="..." uv run python3 bootstrap_generate_guidance.py \
  --test-mode --output /tmp/generate_guidance_bootstrap_test.json
```

### Full Optimization

```bash
# Run all three in parallel (~10-15 minutes total)
env ANTHROPIC_API_KEY="..." uv run python3 bootstrap_validate_completeness.py \
  --max-demos 8 --output /tmp/validate_completeness_bootstrap.json \
  2>&1 | tee /tmp/bootstrap_completeness.log &

env ANTHROPIC_API_KEY="..." uv run python3 bootstrap_validate_correctness.py \
  --max-demos 8 --output /tmp/validate_correctness_bootstrap.json \
  2>&1 | tee /tmp/bootstrap_correctness.log &

env ANTHROPIC_API_KEY="..." uv run python3 bootstrap_generate_guidance.py \
  --max-demos 8 --output /tmp/generate_guidance_bootstrap.json \
  2>&1 | tee /tmp/bootstrap_guidance.log &
```

---

## Expected Improvements

Based on research and v1 baselines:

| Signature | v1 MIPROv2 | BootstrapFewShot Target | Expected Gain |
|-----------|------------|------------------------|---------------|
| validate_completeness | 75% → 75% (stagnant) | 75% → 80-85% | +10-15% |
| validate_correctness | 75% → 75% (stagnant) | 75% → 80-85% | +10-15% |
| generate_guidance | 50% → 50% (stagnant) | 50% → 65-75% | +15-25% |
| **Average** | **67% → 67%** | **67% → 75-82%** | **+12-22%** |

**Hypothesis**: BootstrapFewShot will succeed where MIPROv2 failed because it's designed for small datasets and uses few-shot learning instead of instruction optimization.

---

## When to Use Tier 2

Execute Tier 2 BootstrapFewShot optimization if **either**:

1. **Phase 3 v2 shows <10% improvement** overall across the three signatures
2. **Any signature remains stagnant** (0% improvement) after v2

---

## Monitoring Progress

```bash
# Check logs
tail -f /tmp/bootstrap_completeness.log
tail -f /tmp/bootstrap_correctness.log
tail -f /tmp/bootstrap_guidance.log

# Check for completion
ls -lh /tmp/validate_*_bootstrap.json /tmp/generate_guidance_bootstrap.json

# Analyze results (once complete)
python3 /tmp/analyze_bootstrap_results.py
```

---

## Next Steps After Tier 2

1. **If BootstrapFewShot succeeds** (>10% improvement):
   - Aggregate optimized modules
   - Deploy to production
   - Document findings
   - Consider expanding to 200+ examples for future MIPROv2 run

2. **If BootstrapFewShot also stagnates** (<10% improvement):
   - Proceed to Tier 3: LLM-as-judge metrics
   - Metric redesign likely needed (current binary metrics too simple)
   - Consider multi-property evaluation

3. **If targeting 85%+ accuracy**:
   - Expand training data to 200-300 examples per signature
   - Re-run MIPROv2 with larger dataset
   - This aligns with best practices and should enable MIPROv2 to work as designed

---

## References

- Research: BootstrapFewShot effective for <200 examples
- Research: Prompt optimization requires 20/80 train/validation split
- Research: MIPROv2 designed for 200+ examples, struggles with <100
- v1 Results: extract_requirements succeeded (+52%) with perfect diversity
- v2 Results: generate_guidance stagnant (50%→50%) despite 150% category increase

---

## Files

**Scripts**:
- `bootstrap_validate_completeness.py` (ready)
- `bootstrap_validate_correctness.py` (ready)
- `bootstrap_generate_guidance.py` (ready)

**Outputs** (after execution):
- `/tmp/validate_completeness_bootstrap.json` (optimized module)
- `/tmp/validate_completeness_bootstrap.results.json` (metrics)
- `/tmp/validate_correctness_bootstrap.json`
- `/tmp/validate_correctness_bootstrap.results.json`
- `/tmp/generate_guidance_bootstrap.json`
- `/tmp/generate_guidance_bootstrap.results.json`

**Logs**:
- `/tmp/bootstrap_completeness.log`
- `/tmp/bootstrap_correctness.log`
- `/tmp/bootstrap_guidance.log`
