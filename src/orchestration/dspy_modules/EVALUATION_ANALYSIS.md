# DSPy Evaluation Metrics Analysis

## Problem Statement

MIPROv2 optimization completed successfully but all evaluation scores are 0.0 for both baseline and optimized modules.

## Root Cause

**Fundamental metric design flaw**: Using exact set intersection F1 for natural language requirements.

### Example from Debug Run

**Expected (Gold)**:
```python
[
    "Add login endpoint accepting email and password",
    "Hash passwords using bcrypt before storage",
    "Generate JWT access tokens on successful login",
    "Implement token validation middleware for protected routes",
    "Add token expiration with configurable lifetime",
    "Store refresh tokens for session extension"
]
```

**Predicted**:
```python
[
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
]
```

**Intersection**: 0 (no exact string matches)

**Semantic overlap**: High! The predicted requirements are more detailed and cover the same concepts.

## Why Current Metrics Fail

### 1. Exact String Matching (Current Implementation)
```python
gold_reqs = set(example.requirements)
pred_reqs = set(pred.requirements)
intersection = len(gold_reqs & pred_reqs)
f1 = 2 * (precision * recall) / (precision + recall)
```

**Problem**: Natural language rarely matches exactly. The model is producing *semantically equivalent but differently worded* requirements, which is correct LLM behavior.

### 2. Token-Based F1 (DSPy Built-in)
```python
from dspy.evaluate.metrics import f1_score

# Normalizes text, tokenizes, counts common tokens
score = f1_score(prediction, ground_truth)
```

**Problem**: Better than exact matching, but still insufficient:
- "Add token expiration with configurable lifetime" vs
- "Generate JWT tokens containing user identity claims (user_id, username) with configurable expiration time"

Share tokens: `["token", "expiration", "configurable"]` = 3 tokens
Total tokens: ~15
F1 score: ~0.3

This undervalues semantically equivalent but more detailed requirements.

## Solutions (Ranked by Appropriateness)

### Solution 1: LLM-as-a-Judge (Recommended)
**Best for**: Semantic understanding tasks like requirement extraction

**Implementation**:
```python
class SemanticSimilarityJudge(dspy.Module):
    def __init__(self):
        super().__init__()
        self.judge = dspy.ChainOfThought(
            "gold_requirement, predicted_requirement -> reasoning, semantic_match, score"
        )

    def forward(self, gold, pred):
        result = self.judge(
            gold_requirement=gold,
            predicted_requirement=pred
        )
        # Returns score 0.0-1.0 based on semantic similarity
        return float(result.score)

def semantic_requirement_f1(example, pred, trace=None) -> float:
    judge = SemanticSimilarityJudge()
    gold_reqs = example.requirements
    pred_reqs = pred.requirements

    # Find best matches using bipartite matching
    scores = []
    for pred_req in pred_reqs:
        best_score = max(
            judge(gold_req, pred_req) for gold_req in gold_reqs
        )
        scores.append(best_score)

    precision = sum(scores) / len(pred_reqs) if pred_reqs else 0
    recall = sum(scores) / len(gold_reqs) if gold_reqs else 0

    if precision + recall == 0:
        return 0.0

    return 2 * (precision * recall) / (precision + recall)
```

**Pros**:
- Captures semantic equivalence
- Rewards detailed, correct requirements
- Can evaluate quality dimensions (specificity, completeness)
- Aligns with how humans would judge

**Cons**:
- More expensive (additional LLM calls)
- Slower evaluation
- Requires careful prompt engineering for judge

### Solution 2: Token-Based F1 with Relaxed Matching
**Best for**: Quick approximation when LLM calls are expensive

**Implementation**:
```python
from dspy.evaluate.metrics import f1_score

def relaxed_requirement_f1(example, pred, trace=None, threshold=0.3) -> float:
    """Use token-overlap F1 with threshold for partial credit."""
    gold_reqs = example.requirements
    pred_reqs = pred.requirements

    # Find best matches
    scores = []
    for pred_req in pred_reqs:
        best_score = max(
            f1_score(pred_req, gold_req) for gold_req in gold_reqs
        )
        # Give credit if above threshold
        scores.append(1.0 if best_score >= threshold else 0.0)

    precision = sum(scores) / len(pred_reqs) if pred_reqs else 0
    recall = len([s for s in scores if s > 0]) / len(gold_reqs) if gold_reqs else 0

    if precision + recall == 0:
        return 0.0

    return 2 * (precision * recall) / (precision + recall)
```

**Pros**:
- Fast (no additional LLM calls)
- Uses proven DSPy utilities
- Simple to implement

**Cons**:
- Still undervalues good requirements with different wording
- Threshold is arbitrary
- May not reward quality improvements

### Solution 3: Embedding-Based Similarity
**Best for**: When you need fast semantic similarity without LLM calls

**Implementation**:
```python
from sentence_transformers import SentenceTransformer

model = SentenceTransformer('all-MiniLM-L6-v2')

def embedding_similarity_f1(example, pred, trace=None, threshold=0.7) -> float:
    """Use sentence embeddings for semantic similarity."""
    gold_reqs = example.requirements
    pred_reqs = pred.requirements

    gold_embeddings = model.encode(gold_reqs)
    pred_embeddings = model.encode(pred_reqs)

    # Cosine similarity for each pair
    scores = []
    for pred_emb in pred_embeddings:
        best_sim = max(
            cosine_similarity(pred_emb, gold_emb) for gold_emb in gold_embeddings
        )
        scores.append(1.0 if best_sim >= threshold else 0.0)

    precision = sum(scores) / len(pred_reqs) if pred_reqs else 0
    recall = len([s for s in scores if s > 0]) / len(gold_reqs) if gold_reqs else 0

    if precision + recall == 0:
        return 0.0

    return 2 * (precision * recall) / (precision + recall)
```

**Pros**:
- Fast (GPU-accelerated)
- Good semantic understanding
- No additional LLM API costs

**Cons**:
- Requires additional dependency (sentence-transformers)
- May miss nuanced differences in specificity
- Threshold still somewhat arbitrary

## Recommended Approach

**Phase 1** (Immediate): Implement **LLM-as-a-Judge** for requirement extraction

**Phase 2** (Future): Extend to other signatures with task-specific judges

**Rationale**:
1. Requirement extraction is a semantic task where quality matters more than speed
2. LLM judges align with how humans evaluate requirements
3. Can measure multiple dimensions (specificity, testability, completeness, independence)
4. MIPROv2 optimization will learn to produce requirements that pass the judge

## Implementation Plan

1. ✅ Research DSPy evaluation patterns (completed)
2. ✅ Debug and understand root cause (completed)
3. Create LLM-as-a-Judge metric for requirements
4. Test on sample data and validate scores make sense
5. Update optimize_reviewer.py to use new metrics
6. Re-run 5-trial test to validate
7. Run full 50-trial optimization with correct metrics

## References

- DSPy Documentation: https://dspy.ai/
- LLM as a Judge: https://medium.com/@mayankkhulbe1903/llm-as-a-judge-evaluating-ai-agents-with-dspy-7223f0c76bcd
- DSPy Evaluation: https://llmshowto.com/llm-evaluation-using-dspy-to-decompose-an-llm-judge
- DSPy Metrics: https://arxiv.org/abs/2412.15298
