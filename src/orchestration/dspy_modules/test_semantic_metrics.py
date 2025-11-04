#!/usr/bin/env python3
"""Test suite for semantic evaluation metrics.

Validates that LLM-as-a-Judge metrics produce reasonable scores for:
- Exact matches → ~1.0
- Semantic equivalents with different wording → 0.7-0.9
- Partially overlapping requirements → 0.4-0.6
- Unrelated requirements → ~0.0

This ensures metrics are working correctly before using them in optimization.
"""

import dspy
import os
import json
from pathlib import Path
from semantic_metrics import (
    SemanticSimilarityJudge,
    semantic_requirement_f1,
    semantic_requirement_f1_weighted,
)

# Configure DSPy
api_key = os.getenv("ANTHROPIC_API_KEY")
if not api_key:
    print("ERROR: ANTHROPIC_API_KEY not set")
    exit(1)

dspy.configure(lm=dspy.LM('anthropic/claude-haiku-4-5-20251001', api_key=api_key))
print("DSPy configured with Claude Haiku 4.5\n")


# =============================================================================
# Test Cases: Known Similarity Scores
# =============================================================================

def test_pairwise_similarity():
    """Test individual requirement similarity judgments."""
    print("=" * 80)
    print("TEST 1: Pairwise Similarity Judgments")
    print("=" * 80)
    print()

    judge = SemanticSimilarityJudge()

    # Test cases: (gold, pred, expected_min, expected_max, description)
    test_pairs = [
        # Exact match (same wording)
        (
            "Add login endpoint accepting email and password",
            "Add login endpoint accepting email and password",
            0.95, 1.0,
            "Exact match"
        ),

        # Semantic equivalent (different wording, same meaning)
        (
            "Add login endpoint accepting email and password",
            "Implement authentication endpoint that accepts user credentials (email/password)",
            0.75, 0.95,
            "Semantic equivalent with different wording"
        ),

        # More detailed version
        (
            "Generate JWT access tokens on successful login",
            "Generate JWT tokens containing user identity claims (user_id, username) with configurable expiration time",
            0.70, 0.90,
            "More detailed but same core intent"
        ),

        # Partially overlapping (related but different scope)
        (
            "Add token expiration with configurable lifetime",
            "Verify token expiration and reject expired tokens",
            0.40, 0.70,
            "Related (both about expiration) but different scope"
        ),

        # Tangentially related
        (
            "Hash passwords using bcrypt before storage",
            "Implement token refresh mechanism to issue new tokens before expiration",
            0.10, 0.40,
            "Same domain (auth) but different focus"
        ),

        # Unrelated
        (
            "Add login endpoint accepting email and password",
            "Implement real-time WebSocket notifications for user events",
            0.0, 0.20,
            "Completely unrelated"
        ),
    ]

    passed = 0
    failed = 0

    for gold, pred, min_score, max_score, description in test_pairs:
        score = judge(gold, pred)

        # Check if score is in expected range
        in_range = min_score <= score <= max_score
        status = "✓" if in_range else "✗"

        print(f"{status} {description}")
        print(f"  Gold: {gold}")
        print(f"  Pred: {pred}")
        print(f"  Score: {score:.3f} (expected {min_score:.2f}-{max_score:.2f})")

        if in_range:
            passed += 1
        else:
            failed += 1
            print(f"  WARNING: Score outside expected range!")
        print()

    print(f"Pairwise tests: {passed} passed, {failed} failed")
    print()
    return failed == 0


# =============================================================================
# Test Cases: Requirement Set F1
# =============================================================================

def test_requirement_set_f1():
    """Test F1 computation on full requirement sets."""
    print("=" * 80)
    print("TEST 2: Requirement Set F1 Scoring")
    print("=" * 80)
    print()

    # Load actual training example
    data_path = Path(__file__).parent / "training_data" / "extract_requirements.json"
    with open(data_path) as f:
        training_data = json.load(f)

    raw_example = training_data[0]  # JWT authentication example

    # Create DSPy example
    example = dspy.Example(
        **raw_example['inputs'],
        **raw_example['outputs']
    ).with_inputs(*raw_example['inputs'].keys())

    print("Test Case: JWT Authentication")
    print(f"Gold requirements ({len(example.requirements)}):")
    for i, req in enumerate(example.requirements, 1):
        print(f"  {i}. {req}")
    print()

    # Test case 1: Perfect match
    print("Scenario 1: Perfect Match")
    pred_perfect = dspy.Prediction(requirements=example.requirements)
    score_perfect = semantic_requirement_f1(example, pred_perfect)
    print(f"  F1 Score: {score_perfect:.3f}")
    print(f"  Expected: ~1.0 ({'✓' if score_perfect >= 0.95 else '✗'})")
    print()

    # Test case 2: Semantically equivalent but differently worded
    pred_reworked = dspy.Prediction(requirements=[
        "Implement authentication endpoint accepting email and password credentials",
        "Hash user passwords using bcrypt algorithm before database storage",
        "Generate JWT tokens upon successful authentication",
        "Create middleware to validate JWT tokens on protected routes",
        "Configure JWT token expiration with adjustable lifetime",
        "Persist refresh tokens to enable session extension"
    ])
    score_reworked = semantic_requirement_f1(example, pred_reworked)
    print("Scenario 2: Semantically Equivalent (Different Wording)")
    print(f"  Predicted requirements ({len(pred_reworked.requirements)}):")
    for i, req in enumerate(pred_reworked.requirements, 1):
        print(f"    {i}. {req}")
    print(f"  F1 Score: {score_reworked:.3f}")
    print(f"  Expected: 0.75-0.95 ({'✓' if 0.75 <= score_reworked <= 0.95 else '✗'})")
    print()

    # Test case 3: More detailed (10 reqs vs 6 gold)
    pred_detailed = dspy.Prediction(requirements=[
        "Generate JWT tokens containing user identity claims (user_id, username) with configurable expiration time",
        "Validate JWT token signatures using a secure secret key",
        "Verify token expiration and reject expired tokens",
        "Extract and decode user claims from valid JWT tokens",
        "Implement token refresh mechanism to issue new tokens before expiration",
        "Handle authentication errors with appropriate error types and messages",
        "Integrate JWT authentication with user.rs for user lookup and validation",
        "Implement secure secret key management (not hardcoded in source)",
        "Support token revocation or blacklisting mechanism",
        "Provide middleware/decorator for protecting authenticated endpoints"
    ])
    score_detailed = semantic_requirement_f1(example, pred_detailed)
    print("Scenario 3: More Detailed (10 reqs covering same ground as 6)")
    print(f"  Predicted requirements ({len(pred_detailed.requirements)}):")
    for i, req in enumerate(pred_detailed.requirements, 1):
        print(f"    {i}. {req}")
    print(f"  F1 Score: {score_detailed:.3f}")
    print(f"  Expected: 0.60-0.85 (more reqs → lower precision) ({'✓' if 0.60 <= score_detailed <= 0.85 else '✗'})")
    print()

    # Test case 4: Partially overlapping
    pred_partial = dspy.Prediction(requirements=[
        "Add login endpoint accepting email and password",
        "Generate JWT access tokens on successful login",
        "Implement OAuth2 social login (Google, GitHub)",
        "Add two-factor authentication with TOTP"
    ])
    score_partial = semantic_requirement_f1(example, pred_partial)
    print("Scenario 4: Partially Overlapping (2/6 gold covered, 2 extras)")
    print(f"  F1 Score: {score_partial:.3f}")
    print(f"  Expected: 0.30-0.60 ({'✓' if 0.30 <= score_partial <= 0.60 else '✗'})")
    print()

    # Test case 5: Completely unrelated
    pred_unrelated = dspy.Prediction(requirements=[
        "Implement WebSocket connection handling",
        "Add real-time notification broadcasting",
        "Configure Redis pub/sub for event distribution"
    ])
    score_unrelated = semantic_requirement_f1(example, pred_unrelated)
    print("Scenario 5: Completely Unrelated")
    print(f"  F1 Score: {score_unrelated:.3f}")
    print(f"  Expected: 0.0-0.20 ({'✓' if score_unrelated <= 0.20 else '✗'})")
    print()

    # Summary
    all_passed = (
        score_perfect >= 0.95 and
        0.75 <= score_reworked <= 0.95 and
        0.60 <= score_detailed <= 0.85 and
        0.30 <= score_partial <= 0.60 and
        score_unrelated <= 0.20
    )

    print(f"Requirement set F1 tests: {'All passed ✓' if all_passed else 'Some failed ✗'}")
    print()
    return all_passed


# =============================================================================
# Test Cases: Weighted vs Unweighted F1
# =============================================================================

def test_weighted_vs_unweighted():
    """Compare weighted and unweighted F1 scoring."""
    print("=" * 80)
    print("TEST 3: Weighted vs Unweighted F1")
    print("=" * 80)
    print()

    # Load example
    data_path = Path(__file__).parent / "training_data" / "extract_requirements.json"
    with open(data_path) as f:
        training_data = json.load(f)

    example = dspy.Example(
        **training_data[0]['inputs'],
        **training_data[0]['outputs']
    ).with_inputs(*training_data[0]['inputs'].keys())

    # Prediction with varying quality
    pred = dspy.Prediction(requirements=[
        "Add login endpoint accepting email and password",  # Exact match
        "Generate JWT tokens with configurable expiration",  # Close match
        "Implement secure password hashing",  # Related but less specific
        "Add session management functionality"  # Tangentially related
    ])

    score_unweighted = semantic_requirement_f1(example, pred, threshold=0.5)
    score_weighted = semantic_requirement_f1_weighted(example, pred, threshold=0.5)

    print(f"Unweighted F1: {score_unweighted:.3f} (binary match/no-match)")
    print(f"Weighted F1:   {score_weighted:.3f} (uses similarity scores)")
    print()
    print("Weighted should be ≤ unweighted (partial credit for imperfect matches)")
    print(f"Relationship check: {'✓' if score_weighted <= score_unweighted + 0.05 else '✗'}")
    print()

    return score_weighted <= score_unweighted + 0.05


# =============================================================================
# Main Test Runner
# =============================================================================

def main():
    print("\n" + "=" * 80)
    print("SEMANTIC METRICS VALIDATION TEST SUITE")
    print("=" * 80)
    print()

    results = []

    # Run tests
    results.append(("Pairwise Similarity", test_pairwise_similarity()))
    results.append(("Requirement Set F1", test_requirement_set_f1()))
    results.append(("Weighted vs Unweighted", test_weighted_vs_unweighted()))

    # Summary
    print("=" * 80)
    print("FINAL RESULTS")
    print("=" * 80)
    for name, passed in results:
        status = "✓ PASS" if passed else "✗ FAIL"
        print(f"{status}: {name}")

    all_passed = all(passed for _, passed in results)
    print()
    if all_passed:
        print("✓ All tests passed! Metrics are ready for optimization.")
    else:
        print("✗ Some tests failed. Review metric implementation before proceeding.")

    return 0 if all_passed else 1


if __name__ == "__main__":
    exit(main())
