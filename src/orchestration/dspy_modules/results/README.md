# DSPy Optimization Results

This directory contains the results of MIPROv2 optimization runs for all ReviewerModule signatures.

## v1 Optimization Run (2025-11-03)

**Method**: Per-signature optimization (25 trials each)
**Model**: Claude Haiku 4.5 (`claude-haiku-4-5-20251001`)
**Training Data**: 20 examples per signature
**Rate Limiting**: `num_threads=2` (90k tokens/min limit)

### Results Summary

| Signature | Baseline | Optimized | Improvement | Status |
|-----------|----------|-----------|-------------|--------|
| `extract_requirements` | 36.7% | **56.0%** | **+52.4%** | ✅ Excellent |
| `validate_intent` | 100% | 100% | 0% | ✅ Already perfect |
| `validate_completeness` | 75% | 75% | 0% | ⚠️ Stagnant |
| `validate_correctness` | 75% | 75% | 0% | ⚠️ Stagnant |
| `generate_guidance` | 50% | 50% | 0% | ⚠️ Stagnant |

### Files

**Optimized Modules**:
- `extract_requirements_v1.json` - Optimized extract_requirements predictor
- `validate_intent_v1.json` - Optimized validate_intent predictor
- `validate_completeness_v1.json` - Optimized validate_completeness predictor
- `validate_correctness_v1.json` - Optimized validate_correctness predictor
- `generate_guidance_v1.json` - Optimized generate_guidance predictor
- `optimized_reviewer_v1.json` - **Aggregated module** (all 5 signatures combined)

**Results**:
- `*_v1.results.json` - Performance metrics for each optimization

###Analysis

**Success**: `extract_requirements` showed 52.4% improvement, demonstrating the optimization pipeline works correctly.

**Stagnant Signatures**: 3 signatures showed 0% improvement, likely due to:
1. **Insufficient training data**: 20 examples may not provide enough signal
2. **Strong baselines**: 75% is already quite good
3. **Low diversity**: Need more edge cases and failure modes

**Next Steps**: See [CONTINUOUS_IMPROVEMENT.md](../CONTINUOUS_IMPROVEMENT.md) for roadmap to improve stagnant signatures.

### Metrics

**Semantic F1 Score**: Used for `extract_requirements` - LLM-as-judge evaluates semantic similarity of predicted requirements vs. gold standard.

**Accuracy**: Used for validation signatures - exact match of boolean outcomes (is_complete, is_correct, intent_satisfied).

**Rate Limiting**: Optimizations configured with `num_threads=2` to stay under 90k tokens/min Anthropic API limit.

### Reproduction

To reproduce these results:

```bash
cd src/orchestration/dspy_modules

# Per-signature optimization (recommended)
env ANTHROPIC_API_KEY="..." uv run python3 optimize_extract_requirements.py --trials 25 --output /tmp/extract_requirements_v1.json

# Aggregate optimized modules
python3 aggregate_optimized.py --modules-dir /tmp --output reviewer_optimized_v1.json
```

### Known Issues

**Composite Metric Bug** (Fixed): Initial attempt used composite metric that caused all trials to score 0.0 due to field ambiguity. Resolved by switching to per-signature optimization.

See [OPTIMIZATION_ANALYSIS.md](../OPTIMIZATION_ANALYSIS.md) for detailed bug analysis and fix.

---

## Future Optimization Runs

### v2 (Planned)

**Goal**: Improve stagnant signatures by expanding training data

**Changes**:
- Expand training data from 20 → 50 examples per signature
- Focus on edge cases, failure modes, diverse domains
- Run 50 trials (vs 25 in v1) for better exploration

**Expected**: 5-15% improvement on stagnant signatures

### v3 (Planned)

**Goal**: Production-driven optimization with usage data

**Changes**:
- Incorporate 100+ production examples
- Monthly re-optimization cycle
- A/B testing validation

---

**Last Updated**: 2025-11-03
**Branch**: feature/dspy-integration
