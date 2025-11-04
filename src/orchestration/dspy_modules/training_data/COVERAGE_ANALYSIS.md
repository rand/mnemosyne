# Training Data Coverage Analysis (v1 → v2)

**Date**: 2025-11-03
**Purpose**: Systematic analysis of existing 20-example training data to guide expansion to 50 examples

## Executive Summary

**Findings**:
- ✅ validate_completeness: Perfect category distribution (1-each), but difficulty skewed toward hard
- ⚠️ validate_correctness: Security over-represented (20%), many categories missing
- ✅ generate_guidance: Perfect category distribution (1-each), but difficulty skewed toward medium

**Strategy for v2**: Add 30 examples per signature targeting:
1. Difficulty rebalancing (30% easy, 40% medium, 30% hard)
2. Category expansion (add 15+ underrepresented domains)
3. Edge case coverage (boundary conditions, unusual patterns)

---

## validate_completeness Analysis

### Current State (20 examples)

**Category Distribution**: ✅ Perfect (1 example each across 20 categories)
```
authentication, accessibility, backup_recovery, batch_processing, caching,
ci_cd, compliance, database_migration, documentation, error_handling,
file_upload, internationalization, logging, mobile_optimization, monitoring,
multi_tenancy, rate_limiting, search, security, security_migration
```

**Difficulty Distribution**: ⚠️ Skewed toward hard
```
easy: 3 (15%)   ← Target: 30% (15 of 50)
medium: 7 (35%) ← Target: 40% (20 of 50)
hard: 10 (50%)  ← Target: 30% (15 of 50)
```

**Completeness Levels Represented**:
- Fully complete: ~20%
- Partially complete (50-80%): ~40%
- Barely complete (20-50%): ~30%
- Edge cases: ~10%

### Coverage Gaps

**Missing Categories** (need 30 new categories):
1. API development (REST endpoints, versioning)
2. Database queries & ORM
3. Async/await patterns
4. GraphQL implementation
5. WebSocket real-time features
6. Container/Docker setup
7. Configuration management
8. State management (Redux, MobX)
9. Data structures & algorithms
10. Stream processing
11. Message queues (RabbitMQ, Kafka)
12. Service mesh (Istio, Linkerd)
13. Feature flags
14. A/B testing infrastructure
15. Secrets management
16. CLI tools
17. Webhooks
18. SSO/OAuth integration
19. Data migration scripts
20. Performance profiling
21. Load balancing
22. DNS configuration
23. SSL/TLS setup
24. Reverse proxy (Nginx, Traefik)
25. CDN integration
26. Database indexing
27. Query optimization
28. Middleware development
29. Plugin architecture
30. Microservices communication

**Difficulty Rebalancing** (for 30 new examples):
- Add 6 easy examples → Total 9 easy (18% → 30%)
- Add 13 medium examples → Total 20 medium (35% → 40%)
- Add 11 hard examples → Total 21 hard (50% → 42%, close to 30%)

**Completeness Patterns to Add**:
- Over-implemented (implemented more than required): 3 examples
- Ambiguous requirements (unclear what constitutes "complete"): 3 examples
- Partial implementations with good test coverage: 4 examples
- Complete but poor quality implementations: 3 examples

---

## validate_correctness Analysis

### Current State (20 examples)

**Category Distribution**: ⚠️ Security over-represented
```
security: 4 (20%) ← Too many, reduce to 2-3
concurrency: 2 (10%)
All others: 1 each (14 categories)
```

**Difficulty Distribution**: ✅ Well-balanced
```
easy: 5 (25%)   ← Target: 30% (close enough)
medium: 10 (50%) ← Target: 40% (close)
hard: 5 (25%)   ← Target: 30% (close enough)
```

**Issue Types Represented**:
- Security flaws: 20% (over-represented)
- Race conditions/concurrency: 10%
- Logic errors: ~15%
- Performance issues: ~10%
- Edge case handling: ~10%
- Other: ~35%

### Coverage Gaps

**Missing Categories** (reduce security, add 28-29 new):
1. API correctness (response formats, status codes)
2. Database transaction handling
3. Async/await correctness (deadlocks, cancellation)
4. Memory leaks & resource cleanup
5. Type system violations (unsafe code)
6. Lifetime & borrow checker issues
7. Testing anti-patterns
8. GraphQL resolver correctness
9. WebSocket message ordering
10. Configuration parsing errors
11. State machine correctness
12. Algorithm correctness (sorting, searching)
13. Data structure invariants
14. Serialization/deserialization bugs
15. Time/date handling (timezones, DST)
16. Unicode handling
17. Regex correctness
18. Networking errors (timeouts, retries)
19. Error propagation
20. Logging correctness (PII leakage)
21. Metrics accuracy
22. Circuit breaker correctness
23. Queue processing (ordering, dedup)
24. Batch job correctness
25. Event sourcing bugs
26. CQRS synchronization
27. Distributed transaction correctness
28. Eventual consistency issues
29. Idempotency violations
30. Graceful degradation failures

**Difficulty Rebalancing** (for 30 new examples):
- Add 10 easy → Total 15 easy (25% → 30%)
- Add 10 medium → Total 20 medium (50% → 40%)
- Add 10 hard → Total 15 hard (25% → 30%)

**Issue Patterns to Add**:
- Subtle bugs (off-by-one, boundary conditions): 8 examples
- Performance bugs (O(n²) instead of O(n)): 5 examples
- Correctness under load: 4 examples
- Distributed system bugs: 5 examples
- Configuration bugs: 4 examples
- Integration bugs: 4 examples

---

## generate_guidance Analysis

### Current State (20 examples)

**Category Distribution**: ✅ Perfect (1 example each across 20 categories)
```
authentication, accessibility, cache_consistency, ci_cd, concurrency,
documentation, edge_cases, file_io, file_upload, financial, internationalization,
monitoring, multi_tenancy, pagination, rate_limiting, resilience, security,
security_critical, security_emergency, security_migration
```

**Difficulty Distribution**: ⚠️ Skewed toward medium
```
easy: 3 (15%)   ← Target: 30% (15 of 50)
medium: 12 (60%) ← Target: 40% (20 of 50)
hard: 3 (15%)   ← Target: 30% (15 of 50)
high: 2 (10%)   ← Inconsistency: should be "hard"
```

**Attempt Distribution Represented**:
- 1st attempt (general guidance): ~40%
- 2nd attempt (specific fixes): ~40%
- 3rd+ attempt (debugging): ~20%

**Priority Distribution**:
- Critical: ~20%
- High: ~35%
- Medium: ~35%
- Low: ~10%

### Coverage Gaps

**Missing Categories** (need 30 new categories):
1. API design guidance
2. Database schema design
3. Async architecture guidance
4. GraphQL schema guidance
5. Real-time architecture
6. Container optimization
7. Configuration best practices
8. State management patterns
9. Algorithm selection
10. Data structure choices
11. Message queue patterns
12. Service mesh configuration
13. Feature flag strategies
14. A/B testing setup
15. Secrets rotation procedures
16. CLI UX improvements
17. Webhook reliability
18. OAuth flow guidance
19. Migration strategies
20. Profiling techniques
21. Load balancing strategies
22. DNS optimization
23. TLS configuration
24. Reverse proxy optimization
25. CDN strategies
26. Index design
27. Query rewriting
28. Middleware patterns
29. Plugin API design
30. Microservice boundaries

**Difficulty Rebalancing** (for 30 new examples):
- Add 12 easy → Total 15 easy (15% → 30%)
- Add 8 medium → Total 20 medium (60% → 40%)
- Add 10 hard → Total 15 hard (15% → 30%)
- Fix 2 "high" → "hard" for consistency

**Attempt Distribution to Target**:
- 1st attempt (architecture/design): 12 of 30 new
- 2nd attempt (implementation fixes): 10 of 30 new
- 3rd+ attempt (debugging): 8 of 30 new

**Guidance Patterns to Add**:
- Architecture trade-off guidance: 6 examples
- Debugging strategy guidance: 6 examples
- Performance optimization paths: 5 examples
- Security hardening steps: 5 examples
- Testing strategy guidance: 4 examples
- Refactoring guidance: 4 examples

---

## Diversity Matrix for v2 Expansion

### Target Distribution (50 examples per signature)

**Difficulty**:
- Easy: 15 (30%)
- Medium: 20 (40%)
- Hard: 15 (30%)

**Categories**:
- 20 existing + 30 new = 50 unique categories per signature
- Maximum 2 examples per category (for variety)

**Signature-Specific Dimensions**:

**validate_completeness**:
- Completeness levels: 0% (5), 25% (7), 50% (10), 75% (10), 100% (8), over-implemented (5), ambiguous (5)

**validate_correctness**:
- Correctness: fully correct (8), minor issues (12), major bugs (15), critical flaws (10), subtle bugs (5)
- Issue types: logic (10), security (8), performance (8), edge cases (8), concurrency (8), other (8)

**generate_guidance**:
- Attempts: 1st (20), 2nd (18), 3rd+ (12)
- Priority: critical (12), high (18), medium (15), low (5)
- Guidance type: architecture (15), debugging (10), optimization (10), security (8), testing (7)

---

## Implementation Strategy

### Batch Approach (10 examples per batch)

**Day 1** (6 hours):
- Batch 1a: validate_completeness (10 examples)
- Batch 1b: validate_correctness (10 examples)
- Batch 1c: generate_guidance (10 examples)

**Day 2** (4 hours):
- Batch 2a: validate_completeness (10 examples)
- Batch 2b: validate_correctness (10 examples)
- Batch 2c: generate_guidance (10 examples)

**Day 3** (5 hours):
- Batch 3a: validate_completeness (10 examples)
- Batch 3b: validate_correctness (10 examples)
- Batch 3c: generate_guidance (10 examples)
- Validation (1 hour)

### Quality Checks (every 10 examples)

1. **JSON validation**: `jq empty file.json`
2. **Category audit**: Are we hitting new categories?
3. **Difficulty check**: Tracking toward target distribution?
4. **Realism check**: Could these scenarios happen?
5. **Uniqueness check**: Any duplicate patterns?

---

## Success Criteria

**Coverage**:
- ✅ 50 examples per signature
- ✅ 50 unique categories per signature (or max 2 per category)
- ✅ Target difficulty distribution achieved (30/40/30)

**Quality**:
- ✅ All examples JSON-valid
- ✅ Realistic scenarios based on real development
- ✅ Detailed, unambiguous labels
- ✅ Comprehensive explanations

**Diversity**:
- ✅ No category over-represented (max 2 per category)
- ✅ Full spectrum of completeness/correctness/guidance patterns
- ✅ Multiple attempt numbers, priorities, issue types represented

---

## Expected Impact

Based on extract_requirements precedent (+52.4% with 20 diverse examples):

**Conservative Estimates** (with 2.5× more training data):
- validate_completeness: 75% → 83-88% (+10-17%)
- validate_correctness: 75% → 83-88% (+10-17%)
- generate_guidance: 50% → 65-75% (+30-50%)

**Overall Reviewer Accuracy**: 71% → 80-85% (+13-20%)
