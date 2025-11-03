# MIPROv2 Optimization Analysis

## Observation: Zero Improvement After 5 Trials

The 5-trial optimization test completed successfully with functional semantic metrics, but showed **0.0 improvement** across all signatures.

### Results Summary

```json
{
  "baseline_scores": {
    "extract_requirements": 0.71,
    "validate_intent": 1.00,
    "validate_completeness": 0.75,
    "validate_correctness": 0.75,
    "generate_guidance": 0.00
  },
  "optimized_scores": {
    "extract_requirements": 0.71,
    "validate_intent": 1.00,
    "validate_completeness": 0.75,
    "validate_correctness": 0.75,
    "generate_guidance": 0.00
  }
}
```

## Hypotheses for Zero Improvement

### H1: Baseline Prompts Already Near-Optimal
**Likelihood: HIGH**

The baseline prompts for ReviewerModule are already sophisticated and production-tested:
- `extract_requirements`: 71% semantic F1 (already quite good)
- `validate_intent`: 100% accuracy (perfect on test set)
- `validate_completeness`: 75% accuracy (solid baseline)
- `validate_correctness`: 75% accuracy (solid baseline)

These are strong baselines. MIPROv2 may need many more trials to find meaningful improvements when starting from already-good prompts.

### H2: Insufficient Trials for Convergence
**Likelihood: HIGH**

- Only 5 trials with 10 instruction candidates and 10 fewshot sets
- MIPROv2 typically needs 20-50+ trials to find improvements
- Test mode intentionally uses fewer trials for validation, not optimization

### H3: Test Set Too Small for Variance
**Likelihood: MEDIUM**

- Only 4 test examples per signature (80/20 split from 20 examples)
- Small test sets reduce ability to detect small improvements
- Random variance could mask small gains

### H4: Composite Metric Issues
**Likelihood: MEDIUM**

The `composite_metric` in `optimize_reviewer.py` (lines 276-292) routes to different signature-specific metrics based on example fields. This approach may:
- Dilute optimization signal across multiple signatures
- Make it harder for MIPROv2 to find improvements for any single signature
- Mix examples with different evaluation criteria in single batch

**Current Implementation:**
```python
def composite_metric(example, pred, trace=None) -> float:
    """Composite metric across all ReviewerModule operations."""
    # Determine which operation to evaluate based on example fields
    if hasattr(example, 'requirements') and hasattr(example, 'user_intent'):
        return requirement_extraction_metric(example, pred, trace)
    elif hasattr(example, 'intent_satisfied'):
        return intent_validation_metric(example, pred, trace)
    # ... etc
```

### H5: Module-Level vs Signature-Level Optimization
**Likelihood: LOW**

MIPROv2 is optimizing the entire `ReviewerModule` (all 5 signatures together) rather than individual signatures. This is intentional but may make optimization harder.

## Recommended Next Steps (Principled Approach)

### Option 1: Run Full 50-Trial Optimization (Baseline Approach)
**Rationale**: The standard approach is to run more trials. 5 trials is only for infrastructure validation.

**Expected Outcome**: May see 5-15% improvement on extract_requirements and generate_guidance (the two signatures with room for improvement).

**Time Investment**: ~30-60 minutes

**Command**:
```bash
python optimize_reviewer.py --trials 50 --output optimized_reviewer_v1.json
```

### Option 2: Signature-Specific Optimization (Focused Approach)
**Rationale**: Optimize each signature independently to isolate signal.

**Expected Outcome**: Higher likelihood of finding improvements for individual signatures, especially extract_requirements (currently 71%) and generate_guidance (currently 0%).

**Time Investment**: ~2-3 hours total (5 signatures × 30-50 min each)

**Implementation Required**: Create per-signature optimization scripts.

### Option 3: Increase Training Data (Data-Driven Approach)
**Rationale**: More examples provide stronger optimization signal.

**Expected Outcome**: Better generalization and more reliable improvements.

**Time Investment**: 1-2 hours to create 20+ additional examples per signature.

**Current State**: 20 examples per signature (16 train / 4 test)

### Option 4: Hybrid Approach (Recommended)
**Rationale**: Combine approaches for maximum learning and improvement.

**Steps**:
1. Run full 50-trial module-level optimization (establish baseline)
2. Analyze which signatures improved and which didn't
3. For signatures that didn't improve, try signature-specific optimization
4. If still no improvement, expand training data for those signatures

## Decision Framework

**If goal is to validate the optimization pipeline works**:
→ Option 1 (Run 50 trials now)

**If goal is to maximize quality improvements**:
→ Option 4 (Hybrid approach)

**If goal is to understand optimization dynamics**:
→ Option 2 (Signature-specific optimization with detailed analysis)

**If goal is to ship production-ready prompts quickly**:
→ Option 1, accept whatever improvements we get, iterate based on production metrics

## Conclusion

Zero improvement after 5 trials is **expected and not concerning**. The metrics are working correctly (we see meaningful non-zero scores), the infrastructure is functional, and the baseline is already strong.

**Recommendation**: Proceed with Option 1 (50-trial optimization) as the principled next step. This follows standard DSPy optimization practices and will provide enough trials for MIPROv2 to explore the prompt space effectively.

If 50 trials still shows minimal improvement, that's valuable information: it means the baseline prompts are already near-optimal, which is a success indicator for the manual prompt engineering that went into ReviewerModule.

---

## Update: Rate Limiting Issue (2025-11-03)

### Problem
First 50-trial optimization attempt hit Anthropic API rate limits:
```
litellm.RateLimitError: This request would exceed the rate limit for your organization
(f009ea24-3dd8-4960-b4ec-201a731c3cf6) of 90,000 output tokens per minute.
```

MIPROv2 was making too many parallel API calls during evaluation, exceeding the organization's rate limit.

### Root Cause
Default `num_threads` parameter in MIPROv2 allows high parallelism (likely 8+ threads). When evaluating multiple instruction candidates × fewshot sets × examples in parallel, token usage rate exceeded 90k tokens/minute.

### Solution
Added `num_threads=2` parameter to MIPROv2 configuration in `optimize_reviewer.py`:

```python
teleprompter = MIPROv2(
    metric=composite_metric,
    auto=None,
    num_candidates=10,
    init_temperature=1.0,
    verbose=True,
    num_threads=2  # Limit parallelism to avoid rate limits (90k tokens/min)
)
```

**Impact**: Reduces parallel API calls by ~75%, keeping token usage under rate limit. May increase optimization time from 30-60 min to 60-120 min, but allows completion without errors.

**Trade-off**: Acceptable - correctness over speed. A slightly slower optimization that completes is better than a fast one that fails.

---

## Update: 50-Trial Optimization - CRITICAL BUG DISCOVERED (2025-11-03)

### INCORRECT PREVIOUS ANALYSIS

**The previous analysis claiming "baseline prompts are near-optimal" was WRONG.**

Upon critical investigation, the zero improvement was not due to optimal baselines, but due to a **critical bug in the composite_metric function** that caused ALL trials to score 0.0.

### The Bug: Broken Composite Metric Dispatch

**Location**: `optimize_reviewer.py` lines 276-292

**Root Cause**: The `composite_metric` function attempts to route examples to the correct metric by checking field presence:

```python
def composite_metric(example, pred, trace=None) -> float:
    # Check for output fields that indicate the operation type
    if hasattr(example, 'requirements') and hasattr(example, 'user_intent'):
        return requirement_extraction_metric(example, pred, trace)
    elif hasattr(example, 'intent_satisfied'):
        return intent_validation_metric(example, pred, trace)
    # ...
```

**The Fatal Flaw**: This logic does not distinguish between INPUT and OUTPUT fields:

- **extract_requirements** examples:
  - Inputs: `user_intent`, `context`
  - Outputs: `requirements`, `priorities`

- **validate_intent** examples:
  - Inputs: `user_intent`, `work_item`, `implementation`, `requirements`
  - Outputs: `intent_satisfied`, `explanation`, `missing_aspects`

**Both signatures have `requirements` AND `user_intent` fields**, but:
- For `extract_requirements`: `requirements` is an OUTPUT
- For `validate_intent`: `requirements` is an INPUT

The composite_metric misroutes `validate_intent` examples to `requirement_extraction_metric`, which returns 0.0 because it expects `requirements` as an output field that isn't there in the prediction.

### Evidence

**From optimization logs (`/tmp/mipro_50trial_v2.log`)**:

1. **ALL 50 trials scored 0.0**:
   ```
   Minibatch scores so far: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, ...]
   Full eval scores so far: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
   Best full score so far: 0.0
   ```

2. **920 lines with "Score: 0.0"** in the log

3. **JSON parsing failures**: Model generating XML-like output that doesn't match expected format

4. **All optimization trials scored identically**: Not just baseline, but EVERY variant explored scored 0.0

### Impact

**The 50-trial optimization was completely invalid**:
- No meaningful signal for MIPROv2 to learn from
- Every prompt variant scored 0.0 regardless of quality
- "Zero improvement" was due to broken metrics, not optimal prompts
- Wasted ~49 minutes of API time and ~$X in costs
- Led to incorrect conclusion about baseline quality

### Lessons Learned

1. **Always inspect optimization logs critically**: Check that scores vary during optimization
2. **Validate metrics independently**: Run metrics on known examples before optimization
3. **Don't trust zero variance**: If all trials score the same, investigate immediately
4. **Check for field ambiguity**: When combining multiple signatures, ensure clean dispatch logic
5. **Question convenient narratives**: "Already optimal" is suspicious without evidence

### Correct Fix: Per-Signature Optimization

**Approach**: Abandon composite metric entirely. Create separate optimization scripts for each signature:

1. `optimize_extract_requirements.py` - Loads only extract_requirements training data
2. `optimize_validate_intent.py` - Loads only validate_intent training data
3. `optimize_validate_completeness.py` - Loads only validate_completeness training data
4. `optimize_validate_correctness.py` - Loads only validate_correctness training data
5. `optimize_generate_guidance.py` - Loads only generate_guidance training data

**Advantages**:
- No dispatch ambiguity - each script optimizes one signature
- Clean, focused metrics - no composite routing logic
- Parallel execution possible - 5 signatures can optimize concurrently
- Easier debugging - single responsibility per script
- True optimization signal - MIPROv2 gets meaningful feedback

**Expected Outcomes**:
- **extract_requirements**: 5-15% improvement (baseline 0.71 F1)
- **validate_intent**: 0-5% improvement (baseline 1.00 - already excellent)
- **validate_completeness**: 5-10% improvement (baseline 0.75)
- **validate_correctness**: 5-10% improvement (baseline 0.75)
- **generate_guidance**: Should finally score >0.0 and show improvement

### Status

**Task mnemosyne-32 (DS-6: MIPROv2 optimization) is REOPENED.**

The previous closure was premature. The optimization needs to be re-run with fixed methodology.

### Next Actions

1. **Document bug**: ✓ (this section)
2. **Revert incorrect commits**: Update git history noting the error
3. **Create per-signature optimizers**: 5 independent scripts
4. **Run corrected optimizations**: 20-30 trials per signature
5. **Validate improvements**: Compare against true baseline on held-out test set
