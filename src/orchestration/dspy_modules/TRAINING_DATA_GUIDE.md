# DSPy Training Data Expansion Guide

Guide for expanding training data from 20 to 50 examples per signature to unblock v2 optimization.

## Current Status

**v1 Optimization Results** (2025-11-03, 20 examples each):
- ✅ **extract_requirements**: 36.7% → 56.0% (+52.4% improvement)
- ⚠️ **validate_intent**: 100% → 100% (already perfect, no room for improvement)
- ❌ **validate_completeness**: 75% → 75% (stagnant)
- ❌ **validate_correctness**: 75% → 75% (stagnant)
- ❌ **generate_guidance**: 50% → 50% (stagnant)

**Hypothesis**: Stagnant signatures need more diverse training examples to improve. MIPROv2 had insufficient data to find better prompts.

---

## Training Data Structure

All training examples use this JSON structure:

```json
{
  "inputs": {
    "<param1>": "value",
    "<param2>": "value"
  },
  "outputs": {
    "<output1>": "value",
    "<output2>": "value"
  },
  "metadata": {
    "source": "synthetic | production | manual",
    "difficulty": "easy | medium | hard",
    "category": "authentication | database | api | etc",
    "notes": "Explanation of what makes this example interesting"
  }
}
```

---

## Signature 1: validate_completeness (PRIORITY: HIGH)

**Current**: 75% accuracy, 20 examples
**Target**: 50 examples
**Need**: 30 more examples

### Input/Output Schema

```json
{
  "inputs": {
    "implementation": "Description of what was implemented (files, lines, tests)",
    "requirements": ["req1", "req2", "req3"]
  },
  "outputs": {
    "is_complete": true/false,
    "missing_requirements": ["missing1", "missing2"],
    "explanation": "Detailed explanation of completeness assessment"
  }
}
```

### What Makes a Good Example

**Diversity Needed**:
1. **Completion Levels**: Fully complete (100%), mostly complete (80%), partially complete (50%), barely complete (20%)
2. **Requirement Types**: Functional, security, performance, scalability, testing, documentation
3. **Missing Patterns**:
   - Obvious omissions (forgot to implement X)
   - Subtle omissions (implemented X but not edge case Y)
   - Hidden requirements (no tests, no error handling, no logging)
4. **Complexity**: Simple (3 reqs), medium (5-7 reqs), complex (10+ reqs)

### Example Template

```json
{
  "inputs": {
    "implementation": "Created REST API with GET /users and POST /users endpoints. Added validation for email format. Wrote 10 unit tests. Files: api.rs (300 lines), validation.rs (50 lines).",
    "requirements": [
      "User listing endpoint",
      "User creation endpoint",
      "Email validation",
      "Password hashing for new users",
      "Rate limiting on endpoints",
      "Integration tests"
    ]
  },
  "outputs": {
    "is_complete": false,
    "missing_requirements": [
      "Password hashing for new users - no evidence of bcrypt/argon2 usage",
      "Rate limiting on endpoints - no middleware or rate limiter configured",
      "Integration tests - only unit tests present, no end-to-end API tests"
    ],
    "explanation": "Implementation covers 3/6 requirements (listing, creation, email validation). Missing critical security (password hashing), performance (rate limiting), and testing (integration tests). Without password hashing, user credentials at risk. Without rate limiting, API vulnerable to abuse. Without integration tests, endpoint interactions untested."
  },
  "metadata": {
    "source": "synthetic",
    "difficulty": "medium",
    "category": "api",
    "notes": "Mixed completion - basic functionality present, security and testing gaps"
  }
}
```

### Coverage Checklist (aim for 50 examples)

- [ ] 10 examples: Fully complete implementations (all requirements met)
- [ ] 15 examples: Partially complete (3-5 requirements missing)
- [ ] 10 examples: Barely complete (most requirements missing)
- [ ] 5 examples: Edge cases (ambiguous requirements, over-implemented)
- [ ] 5 examples: Security requirements (passwords, auth, encryption)
- [ ] 5 examples: Performance requirements (caching, optimization, rate limiting)

---

## Signature 2: validate_correctness (PRIORITY: HIGH)

**Current**: 75% accuracy, 20 examples
**Target**: 50 examples
**Need**: 30 more examples

### Input/Output Schema

```json
{
  "inputs": {
    "implementation": "Description of implementation details",
    "execution_context": "Phase: X, Agent: Y, Status: Z, Previous attempts: N"
  },
  "outputs": {
    "is_correct": true/false,
    "issues": ["issue1", "issue2"],
    "explanation": "Detailed analysis of correctness"
  }
}
```

### What Makes a Good Example

**Diversity Needed**:
1. **Issue Types**: Logic errors, security vulnerabilities, performance problems, edge cases unhandled
2. **Correctness Levels**: Fully correct, mostly correct (minor issues), incorrect (major bugs)
3. **Context Variations**: Different phases (specification, implementation, review), different attempt counts
4. **Code Patterns**: Algorithms, data structures, APIs, database operations, concurrent code

### Example Template

```json
{
  "inputs": {
    "implementation": "Implemented user password validation: checks length >= 8, requires one uppercase letter. Function: `fn validate_password(pwd: &str) -> bool { pwd.len() >= 8 && pwd.chars().any(|c| c.is_uppercase()) }`",
    "execution_context": "Phase: Implementation, Agent: Executor, Status: In Progress, Previous attempts: 0"
  },
  "outputs": {
    "is_correct": false,
    "issues": [
      "Weak password policy - missing requirements for numbers and special characters",
      "No protection against common passwords (e.g., 'Password123')",
      "No Unicode handling - is_uppercase() may not work correctly for all languages"
    ],
    "explanation": "Password validation implements basic requirements but has security gaps. Modern password policies require numbers and special characters. No check against common/breached password lists. Unicode handling may cause issues for international users. Recommendation: Use established password validation library (e.g., zxcvbn) instead of custom implementation."
  },
  "metadata": {
    "source": "synthetic",
    "difficulty": "medium",
    "category": "security",
    "notes": "Subtle correctness issues - code works but security insufficient"
  }
}
```

### Coverage Checklist (aim for 50 examples)

- [ ] 10 examples: Fully correct implementations
- [ ] 15 examples: Minor issues (edge cases, performance, style)
- [ ] 15 examples: Major issues (logic errors, security vulnerabilities)
- [ ] 5 examples: Algorithms (sorting, searching, graph traversal)
- [ ] 5 examples: Concurrency (race conditions, deadlocks)

---

## Signature 3: generate_guidance (PRIORITY: MEDIUM)

**Current**: 50% accuracy, 20 examples
**Target**: 50 examples
**Need**: 30 more examples

### Input/Output Schema

```json
{
  "inputs": {
    "user_intent": "What user wants to achieve",
    "requirements": ["req1", "req2"],
    "issues": ["issue1", "issue2"],
    "attempt_number": 1-5
  },
  "outputs": {
    "guidance": "Detailed, actionable guidance for next attempt",
    "priority_order": [1, 2, 3],
    "warnings": ["warning1", "warning2"]
  }
}
```

### What Makes a Good Example

**Diversity Needed**:
1. **Issue Complexity**: Simple fixes, moderate refactoring, major redesign needed
2. **Attempt Progression**: First attempt (general guidance), later attempts (specific fixes)
3. **Guidance Types**: Architecture advice, code suggestions, testing strategies, debugging help
4. **Domains**: Authentication, APIs, databases, algorithms, UI, infrastructure

### Example Template

```json
{
  "inputs": {
    "user_intent": "Implement caching layer for database queries",
    "requirements": [
      "Cache frequent queries",
      "Invalidate cache on updates",
      "Handle cache misses gracefully"
    ],
    "issues": [
      "Current implementation caches everything indefinitely",
      "No cache invalidation strategy",
      "Memory usage grows unbounded"
    ],
    "attempt_number": 2
  },
  "outputs": {
    "guidance": "**Cache Strategy Improvements Needed:**\n\n1. **Add TTL (Time-To-Live)**: Set expiration times for cached entries (e.g., 5 minutes for user data, 1 hour for static content). Use `ttl` parameter in cache.set().\n\n2. **Implement Cache Invalidation**: On UPDATE/DELETE operations, call `cache.invalidate(key)` for affected entries. Consider invalidating related keys (e.g., user update → invalidate user list).\n\n3. **Add Size Limits**: Use LRU (Least Recently Used) eviction policy. Set max cache size (e.g., 1000 entries or 100MB). Library: `lru-cache` or Redis with maxmemory-policy.\n\n4. **Handle Cache Stampede**: When cache misses, use request coalescing to prevent thundering herd. Lock pattern: check cache → if miss, acquire lock → fetch DB → update cache → release lock.\n\n**Implementation Order**: TTL first (prevents unbounded growth), then invalidation (correctness), then size limits (safety), finally stampede protection (performance).\n\n**Testing**: Add tests for cache expiration, invalidation on updates, and memory limits.",
    "priority_order": [1, 2, 3, 4],
    "warnings": [
      "Without TTL, memory usage will eventually cause OOM crashes",
      "Without proper invalidation, users will see stale data after updates"
    ]
  },
  "metadata": {
    "source": "synthetic",
    "difficulty": "hard",
    "category": "caching",
    "notes": "Second attempt - specific technical guidance with prioritized action items"
  }
}
```

### Coverage Checklist (aim for 50 examples)

- [ ] 10 examples: First attempts (general architecture guidance)
- [ ] 15 examples: Second attempts (specific implementation advice)
- [ ] 10 examples: Third+ attempts (debugging and fixing specific issues)
- [ ] 5 examples: Architecture decisions (design patterns, trade-offs)
- [ ] 5 examples: Performance optimization
- [ ] 5 examples: Security hardening

---

## Best Practices for Creating Training Data

### 1. Diversity is Key

The most important factor for optimization success is **diverse examples**. Avoid:
- ❌ All examples from same domain (e.g., only authentication)
- ❌ All examples at same difficulty level
- ❌ Similar input/output patterns

Aim for:
- ✅ Multiple domains (auth, API, database, algorithms, UI, infrastructure)
- ✅ Mix of difficulty levels (easy, medium, hard)
- ✅ Varied input lengths and output structures
- ✅ Different edge cases and failure modes

### 2. Quality Over Quantity

20 high-quality diverse examples beat 50 low-quality similar examples. Each example should:
- ✅ Be realistic (could occur in real development)
- ✅ Have clear, unambiguous labels
- ✅ Include detailed explanations in outputs
- ✅ Test a specific aspect of the signature's behavior

### 3. Learn from Production

If you have production logs (see `production_logger.py`), use real interactions as training data:
1. Extract high-quality interactions (clear intent, good outcomes)
2. Anonymize sensitive data
3. Manually label/verify outputs
4. Add as training examples with `source: "production"`

### 4. Incremental Expansion

Don't try to create all 30 examples at once:
1. Start with 10 examples (varied domains)
2. Run optimization with 30 total examples
3. Analyze results - which examples helped?
4. Add 10 more examples (target weak areas)
5. Run optimization with 40 total examples
6. Add final 10 examples
7. Run optimization with 50 total examples

---

## Validation Before Optimization

Before running optimization with expanded training data:

### 1. Check JSON Format

```bash
for file in training_data/{validate_completeness,validate_correctness,generate_guidance}.json; do
  echo "Validating: $file"
  jq empty $file && echo "✓ Valid JSON" || echo "✗ Invalid JSON"
done
```

### 2. Check Example Count

```bash
for file in training_data/{validate_completeness,validate_correctness,generate_guidance}.json; do
  count=$(jq 'length' $file)
  echo "$(basename $file .json): $count examples"
done
```

### 3. Check Diversity

```bash
# Check category diversity
jq '[.[].metadata.category] | unique' training_data/validate_completeness.json

# Check difficulty distribution
jq 'group_by(.metadata.difficulty) | map({difficulty: .[0].metadata.difficulty, count: length})' training_data/validate_completeness.json
```

### 4. Spot Check Examples

```bash
# Random sample of 3 examples
jq -c '.[] | select(.metadata.category)' training_data/validate_completeness.json | shuf -n 3 | jq '.'
```

---

## Helper Script Usage

Use the provided helper script to create training examples interactively:

```bash
cd src/orchestration/dspy_modules
python3 add_training_example.py --signature validate_completeness
```

The script will:
1. Show current example count
2. Prompt for inputs (with examples)
3. Prompt for outputs (with validation)
4. Prompt for metadata
5. Validate JSON structure
6. Append to training data file
7. Offer to create another example

---

## Timeline and Effort

**Per Signature** (30 examples):
- Research/planning: 30 minutes
- Creating examples: 3-4 hours (6-8 min per example)
- Validation/testing: 30 minutes
- **Total**: ~4-5 hours per signature

**All 3 Signatures**:
- **Total effort**: 12-15 hours
- **Recommended schedule**: 2-3 hours per day over 5-7 days

---

## Next Steps After Training Data Expansion

Once training data is expanded to 50 examples:

### Phase 3: Re-Optimization

```bash
# Re-optimize each signature with 50 trials
cd src/orchestration/dspy_modules

# validate_completeness
python3 optimize_validate_completeness.py --trials 50 --output /tmp/validate_completeness_v2.json

# validate_correctness
python3 optimize_validate_correctness.py --trials 50 --output /tmp/validate_correctness_v2.json

# generate_guidance
python3 optimize_generate_guidance.py --trials 50 --output /tmp/generate_guidance_v2.json

# Aggregate v2 optimized module
python3 aggregate_optimized_module.py \
  --extract results/extract_requirements_optimized_v1.json \
  --validate-intent results/validate_intent_optimized_v1.json \
  --validate-completeness /tmp/validate_completeness_v2.json \
  --validate-correctness /tmp/validate_correctness_v2.json \
  --generate-guidance /tmp/generate_guidance_v2.json \
  --output results/reviewer_optimized_v2.json
```

### Phase 4: A/B Testing

Deploy v2 module with gradual rollout (following `docs/OPERATIONS.md`):
1. 10% traffic → monitor for 24 hours
2. 50% traffic → monitor for 24 hours
3. 100% traffic → full deployment

---

## Questions & Support

**Need help?**
- Review existing examples in `training_data/*.json`
- Check `test_*.py` for validation examples
- See `DSPY_INTEGRATION.md` for architecture context
- See `OPERATIONS.md` for deployment guidance

**Common Issues**:
- **"How do I know if an implementation is complete?"**: Look for evidence in the implementation description (files, lines, tests). If a requirement isn't mentioned, it's likely missing.
- **"How do I determine correctness?"**: Consider logic errors, security vulnerabilities, edge cases, and performance issues. Correct code should handle all requirements properly.
- **"How detailed should guidance be?"**: Guidance should be actionable (user can implement directly) but not prescriptive (don't write the code for them). Include architecture advice, specific techniques, and prioritization.

**Ready to start?** Use the `add_training_example.py` script and aim for 10 examples per day!
