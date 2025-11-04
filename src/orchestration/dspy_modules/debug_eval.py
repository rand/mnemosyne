#!/usr/bin/env python3
"""Debug evaluation metrics for ReviewerModule.

Tests a single example to understand the evaluation issue.
"""

import dspy
import os
import json
from pathlib import Path
from reviewer_module import ReviewerModule

# Configure DSPy
api_key = os.getenv("ANTHROPIC_API_KEY")
if not api_key:
    print("ERROR: ANTHROPIC_API_KEY not set")
    exit(1)

dspy.configure(lm=dspy.LM('anthropic/claude-haiku-4-5-20251001', api_key=api_key))
print("DSPy configured with Claude Haiku 4.5\n")

# Load one training example
data_path = Path(__file__).parent / "training_data" / "extract_requirements.json"
with open(data_path) as f:
    training_data = json.load(f)

# Take first example
raw_example = training_data[0]
print("=" * 60)
print("TRAINING EXAMPLE")
print("=" * 60)
print(f"Inputs: {json.dumps(raw_example['inputs'], indent=2)}")
print(f"Expected outputs: {json.dumps(raw_example['outputs'], indent=2)}")
print()

# Convert to DSPy Example
example = dspy.Example(
    **raw_example['inputs'],
    **raw_example['outputs']
).with_inputs(*raw_example['inputs'].keys())

print("=" * 60)
print("DSPY EXAMPLE OBJECT")
print("=" * 60)
print(f"Example type: {type(example)}")
print(f"Example dict: {example.__dict__}")
print(f"Has requirements attr: {hasattr(example, 'requirements')}")
if hasattr(example, 'requirements'):
    print(f"Requirements type: {type(example.requirements)}")
    print(f"Requirements value: {example.requirements}")
print()

# Run prediction
print("=" * 60)
print("RUNNING PREDICTION")
print("=" * 60)
module = ReviewerModule()
pred = module.extract_requirements(
    user_intent=example.user_intent,
    context=example.context
)

print(f"Prediction type: {type(pred)}")
print(f"Prediction dict: {pred.__dict__}")
print(f"Has requirements attr: {hasattr(pred, 'requirements')}")
if hasattr(pred, 'requirements'):
    print(f"Requirements type: {type(pred.requirements)}")
    print(f"Requirements value: {pred.requirements}")
print()

# Test metric
print("=" * 60)
print("TESTING METRIC")
print("=" * 60)

try:
    gold_reqs = set(example.requirements)
    pred_reqs = set(pred.requirements) if hasattr(pred, 'requirements') else set()

    print(f"Gold requirements (set): {gold_reqs}")
    print(f"Pred requirements (set): {pred_reqs}")
    print()

    if not pred_reqs:
        print("ERROR: No predicted requirements!")
        score = 0.0
    else:
        intersection = len(gold_reqs & pred_reqs)
        precision = intersection / len(pred_reqs) if pred_reqs else 0
        recall = intersection / len(gold_reqs) if gold_reqs else 0

        print(f"Intersection: {intersection}")
        print(f"Precision: {precision:.3f}")
        print(f"Recall: {recall:.3f}")

        if precision + recall == 0:
            score = 0.0
        else:
            f1 = 2 * (precision * recall) / (precision + recall)
            score = f1

        print(f"F1 Score: {score:.3f}")

except Exception as e:
    print(f"ERROR in metric: {e}")
    import traceback
    traceback.print_exc()
