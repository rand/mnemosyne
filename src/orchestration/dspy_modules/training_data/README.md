# ReviewerModule Training Data

This directory contains labeled training examples for optimizing the ReviewerModule using DSPy teleprompters (MIPROv2, GEPA).

## Structure

- `extract_requirements.json` - Examples for ExtractRequirements signature
- `validate_intent.json` - Examples for ValidateIntentSatisfaction signature
- `validate_completeness.json` - Examples for ValidateCompleteness signature
- `validate_correctness.json` - Examples for ValidateCorrectness signature
- `generate_guidance.json` - Examples for GenerateImprovementGuidance signature

## Training Data Format

Each JSON file contains an array of labeled examples with this structure:

```json
{
  "inputs": {
    "input_field1": "value1",
    "input_field2": "value2"
  },
  "outputs": {
    "output_field1": "expected_value1",
    "output_field2": "expected_value2"
  },
  "metadata": {
    "source": "real_session | synthetic",
    "difficulty": "easy | medium | hard",
    "category": "authentication | database | UI | etc",
    "notes": "Any additional context"
  }
}
```

## Quality Criteria

Good training examples should:

1. **Cover diverse scenarios**: Authentication, database, UI, API, infrastructure, etc.
2. **Span difficulty levels**: Easy (obvious requirements) to hard (ambiguous intent)
3. **Include edge cases**: Vague requests, conflicting requirements, implicit expectations
4. **Be realistic**: Based on actual development scenarios
5. **Have accurate labels**: Ground truth validated by human review

## Target Coverage

- **Minimum**: 20 examples per signature (100 total)
- **Target**: 30 examples per signature (150 total)
- **Ideal**: 50 examples per signature (250 total)

## Usage

### Loading Training Data

```python
import json

# Load examples for ExtractRequirements
with open('training_data/extract_requirements.json') as f:
    examples = json.load(f)

# Convert to DSPy format
import dspy
trainset = [dspy.Example(**ex).with_inputs('user_intent', 'context') for ex in examples]
```

### Optimization Process

```python
from dspy.teleprompt import MIPROv2
from reviewer_module import ReviewerModule

# Define metric
def requirement_quality(example, pred, trace=None):
    # Check if requirements are specific, testable, complete
    gold_requirements = set(example.requirements)
    pred_requirements = set(pred.requirements)

    # F1 score between predicted and gold requirements
    if not pred_requirements:
        return 0.0

    intersection = len(gold_requirements & pred_requirements)
    precision = intersection / len(pred_requirements)
    recall = intersection / len(gold_requirements)

    if precision + recall == 0:
        return 0.0

    return 2 * (precision * recall) / (precision + recall)

# Optimize using MIPROv2
teleprompter = MIPROv2(
    metric=requirement_quality,
    num_candidates=10,
    init_temperature=1.0
)

optimized = teleprompter.compile(
    ReviewerModule(),
    trainset=trainset,
    num_trials=100
)

# Save optimized module
optimized.save('optimized_reviewer_v1.json')
```

## Creating New Examples

### From Real Sessions

1. Review Claude Code session logs
2. Identify user intents and implementation outcomes
3. Extract requirements manually (ground truth)
4. Format as training example
5. Add metadata (source: real_session, category, difficulty)

### Synthetic Generation

1. Create realistic user intent scenarios
2. Generate corresponding implementations
3. Label expected outputs manually
4. Validate with domain expert
5. Add metadata (source: synthetic)

### Example Sources

- Real project work (authentication, databases, APIs)
- Common development tasks (add logging, fix bug, refactor)
- Edge cases (vague requests, conflicting requirements)
- Domain-specific scenarios (web, mobile, infrastructure)

## Validation

Before using training data:

1. **Completeness**: All required fields present
2. **Accuracy**: Labels match expected outputs
3. **Diversity**: Covers multiple categories and difficulties
4. **Balance**: Similar distribution across categories
5. **Realism**: Examples reflect actual development scenarios

## Current Status

### ReviewerModule
- **ExtractRequirements**: 20/20 examples ✅ **COMPLETE**
- **ValidateIntentSatisfaction**: 20/20 examples ✅ **COMPLETE**
- **ValidateCompleteness**: 20/20 examples ✅ **COMPLETE**
- **ValidateCorrectness**: 20/20 examples ✅ **COMPLETE**
- **GenerateImprovementGuidance**: 20/20 examples ✅ **COMPLETE**

**ReviewerModule Total**: 100/100 examples (100%, ALL signatures complete) ✅

### SemanticModule
- **AnalyzeDiscourse**: 20/20 examples ✅ **COMPLETE**
- **DetectContradictions**: 20/20 examples ✅ **COMPLETE**
- **ExtractPragmatics**: 20/20 examples ✅ **COMPLETE**

**SemanticModule Total**: 60/60 examples (100%, ALL signatures complete) ✅

### OptimizerModule
- **DiscoverSkills**: 21/20 examples ✅ **MINIMUM MET** (exceeds by 1)
- **ConsolidateContext**: 22/20 examples ✅ **MINIMUM MET** (exceeds by 2)
- **OptimizeContextBudget**: 20/20 examples ✅ **MINIMUM MET**

**OptimizerModule Total**: 63/60 examples (105%, ALL signatures complete) ✅

### Overall Total
**223/220 minimum examples (101% complete)** - ALL MODULES COMPLETE ✅✅✅
- ReviewerModule: 100/100 (100% - COMPLETE ✅)
- SemanticModule: 60/60 (100% - COMPLETE ✅)
- OptimizerModule: 63/60 (105% - COMPLETE ✅)

## Next Steps

**All training data collection COMPLETE!** ✅

Next phase: DSPy optimization with MIPROv2

1. Run baseline performance evaluation for all modules
2. Configure MIPROv2 teleprompter with appropriate metrics
3. Optimize ReviewerModule prompts (100 examples available)
4. Optimize SemanticModule prompts (60 examples available)
5. Optimize OptimizerModule prompts (63 examples available)
6. Measure performance improvement vs baseline
7. A/B test optimized vs baseline prompts
8. Deploy optimized modules to production

## References

- [DSPy Teleprompters](https://dspy-docs.vercel.app/docs/building-blocks/teleprompters)
- [MIPROv2 Paper](https://arxiv.org/abs/2305.15023)
- [Training Data Best Practices](https://dspy-docs.vercel.app/docs/deep-dive/teleprompter/teleprompt-optimizer)
