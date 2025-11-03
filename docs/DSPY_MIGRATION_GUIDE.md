# DSPy Integration Migration Guide

Comprehensive guide for migrating to the DSPy integration in Mnemosyne. This guide provides a step-by-step migration path from legacy implementations to the new DSPy-based architecture.

## Table of Contents

1. [Overview](#overview)
2. [Benefits of Migration](#benefits-of-migration)
3. [Prerequisites](#prerequisites)
4. [Migration Path](#migration-path)
5. [Module-Specific Migration](#module-specific-migration)
6. [Breaking Changes](#breaking-changes)
7. [Backward Compatibility](#backward-compatibility)
8. [Testing & Validation](#testing--validation)
9. [Rollback Plan](#rollback-plan)
10. [Common Issues](#common-issues)
11. [Support & Resources](#support--resources)

---

## Overview

The DSPy integration replaces direct PyObject calls and pattern-based heuristics with a clean adapter pattern that provides:

- **Type Safety**: Strongly-typed Rust interfaces replace untyped PyObject calls
- **Optimization**: Systematic prompt optimization via DSPy teleprompters (MIPROv2, GEPA)
- **Maintainability**: Changes localized to adapters, not scattered across business logic
- **Testability**: Easy mocking and testing with adapters
- **Performance**: Centralized error handling and retry logic

**Migration Timeline**: Estimated 1-2 weeks for full adoption across all modules.

**Status**: Production-ready as of Phase 7 completion (2025-11-03).

---

## Benefits of Migration

### Type Safety

**Before** (PyObject):
\`\`\`rust
let result = py_reviewer
    .as_ref()
    .unwrap()
    .call_method1(py, "extract_requirements", (intent, context))?;

// Untyped PyObject - manual extraction required
let requirements: Vec<String> = result.extract(py)?;
\`\`\`

**After** (DSPy Adapter):
\`\`\`rust
let requirements = reviewer_adapter
    .extract_requirements(intent, Some(context))
    .await?;

// Strongly-typed Vec<String> return
\`\`\`

### Optimization Results

**v1 Performance Improvements** (2025-11-03):

| Signature | Baseline | Optimized | Improvement |
|-----------|----------|-----------|-------------|
| extract_requirements | 36.7% | 56.0% | +52.4% |
| validate_intent | 100% | 100% | Already perfect |
| validate_completeness | 75% | 75% | Needs more data |
| validate_correctness | 75% | 75% | Needs more data |
| generate_guidance | 50% | 50% | Needs more data |

DSPy teleprompters proved effective where training data was sufficient (extract_requirements).

---

## Prerequisites

### Environment Setup

1. **Python Environment**:
\`\`\`bash
cd src/orchestration/dspy_modules
uv sync
\`\`\`

2. **Environment Variables**:
\`\`\`bash
export ANTHROPIC_API_KEY="your-api-key"
export MNEMOSYNE_DSPY_MODEL="claude-3-5-sonnet-20241022"  # Optional
\`\`\`

3. **Rust Feature Flags**:
\`\`\`bash
cargo build --features python
cargo test --features python -- --ignored
\`\`\`

---

## Migration Path

See [DSPY_INTEGRATION.md](./DSPY_INTEGRATION.md) for detailed architecture and code examples. This guide focuses on the organizational migration process.

### Phase 1: Setup & Validation (1-2 days)

**Tasks**:
- [ ] Install Python dependencies
- [ ] Verify API key configured
- [ ] Run Python test suite (145+ tests)
- [ ] Run Rust test suite
- [ ] Review documentation

**Verification**:
\`\`\`bash
cd src/orchestration/dspy_modules
uv run pytest test_*.py -v
cargo test --features python -- --ignored
\`\`\`

**Exit Criteria**: All tests passing, environment configured.

---

### Phase 2: Reviewer Migration (2-3 days)

**Scope**: Migrate Reviewer agent from PyObject to ReviewerDSpyAdapter.

**Code Changes**:
- Replace `py_reviewer.call_method1()` with `reviewer_adapter.extract_requirements()`
- Update imports and state structs
- Initialize DSPy service and adapters

**Testing**:
\`\`\`bash
cargo test --features python reviewer_dspy_adapter_test -- --ignored
\`\`\`

**Rollback Plan**: Keep PyObject fallback during transition.

---

### Phase 3: Semantic Analysis Migration (2-3 days)

**Scope**: Migrate Tier 3 semantic analyzers to DSpySemanticBridge.

**Affected Modules**:
- Discourse analyzer
- Contradiction detector
- Pragmatics analyzer

**Testing**:
\`\`\`bash
cargo test --features python dspy_semantic_bridge_test -- --ignored
\`\`\`

---

### Phase 4: Optimizer Migration (2-3 days)

**Scope**: Migrate context consolidation to OptimizerDSpyAdapter.

**Benefits**:
- Progressive consolidation modes (detailed/summary/compressed)
- Skills discovery
- Context budget optimization

**Testing**:
\`\`\`bash
cargo test --features python optimizer_dspy_adapter_test -- --ignored
\`\`\`

---

### Phase 5: Memory Evolution Migration (2-3 days)

**Scope**: Migrate memory consolidation to MemoryEvolutionDSpyAdapter.

**Benefits**:
- Cluster consolidation with confidence scores
- Importance recalibration
- Archival detection

**Testing**:
\`\`\`bash
cargo test --features python memory_evolution_dspy_adapter_test -- --ignored
\`\`\`

---

## Module-Specific Migration

### Reviewer Module

| Operation | API Method |
|-----------|-----------|
| Extract Requirements | `reviewer_adapter.extract_requirements()` |
| Validate Intent | `reviewer_adapter.semantic_intent_check()` |
| Verify Completeness | `reviewer_adapter.verify_completeness()` |
| Verify Correctness | `reviewer_adapter.verify_correctness()` |

**See**: [DSPY_INTEGRATION.md Lines 493-551](./DSPY_INTEGRATION.md#using-reviewer-operations)

### Semantic Module

| Operation | API Method |
|-----------|-----------|
| Discourse Analysis | `semantic_bridge.analyze_discourse()` |
| Contradiction Detection | `semantic_bridge.detect_contradictions()` |
| Pragmatics Extraction | `semantic_bridge.extract_pragmatics()` |

**See**: [DSPY_INTEGRATION.md Lines 553-579](./DSPY_INTEGRATION.md#using-semantic-analysis)

### Optimizer Module

| Operation | API Method |
|-----------|-----------|
| Context Consolidation | `optimizer_adapter.consolidate_context()` |
| Skills Discovery | `optimizer_adapter.discover_skills()` |
| Context Budget Optimization | `optimizer_adapter.optimize_context_budget()` |

**See**: [DSPY_INTEGRATION.md Lines 581-665](./DSPY_INTEGRATION.md#using-optimizer-operations)

### Memory Evolution Module

| Operation | API Method |
|-----------|-----------|
| Cluster Consolidation | `evolution_adapter.consolidate_cluster()` |
| Importance Recalibration | `evolution_adapter.recalibrate_importance()` |
| Archival Detection | `evolution_adapter.detect_archival_candidates()` |

**See**: [DSPY_INTEGRATION.md Lines 667-739](./DSPY_INTEGRATION.md#using-memory-evolution)

---

## Breaking Changes

### API Changes

1. **Reviewer Registration**:
   - Deprecated: `register_py_reviewer(py_reviewer: Arc<PyObject>)`
   - New: `register_dspy_bridge(bridge: Arc<DSpyBridge>)`

2. **Analyzer Constructors**:
   - Old: `DiscourseAnalyzer::new(llm_service)`
   - New: `DiscourseAnalyzer::with_dspy(llm_service, dspy_bridge)`

3. **Consolidation Job Creation**:
   - Old: `ConsolidationJob::new(storage)`
   - New: `ConsolidationJob::with_dspy(storage, evolution_adapter)`

### Configuration Changes

1. **Environment Variables** (new):
   - `ANTHROPIC_API_KEY` - Required for DSPy modules
   - `MNEMOSYNE_DSPY_MODEL` - Optional model override

2. **Feature Flags**:
   - Must build with `--features python` for DSPy integration

### Behavior Changes

1. **Error Messages**: More descriptive from adapters
2. **Retry Logic**: Centralized, no manual retry macros
3. **Logging**: Operations logged with module name
4. **Performance**: Slight adapter overhead (negligible)

---

## Backward Compatibility

### Gradual Migration Strategy

DSPy integration supports gradual migration via fallback mechanisms:

\`\`\`rust
// Try DSPy first, fallback to PyObject
if let Some(adapter) = &self.reviewer_adapter {
    return adapter.extract_requirements(intent, None).await;
}

// Fallback to deprecated PyObject
if let Some(py_reviewer) = &self.py_reviewer {
    return retry_llm_operation!(...);
}

Err(anyhow!("No reviewer configured"))
\`\`\`

### Deprecation Timeline

- **Now**: DSPy production-ready, recommended for new code
- **Months 1-2**: Gradual migration, both systems supported
- **Months 3-4**: DSPy primary, PyObject deprecated warnings
- **Months 5-6**: Remove PyObject support, DSPy only

---

## Testing & Validation

### Pre-Migration

\`\`\`bash
# Python tests
cd src/orchestration/dspy_modules
uv run pytest test_*.py -v

# Rust tests
cargo test --features python -- --ignored
\`\`\`

**Expected**: 145+ Python tests passing, all Rust integration tests passing, 80% coverage.

### Post-Migration

\`\`\`bash
# Smoke tests
cargo test --features python reviewer_dspy_adapter_test -- --ignored
cargo test --features python dspy_semantic_bridge_test -- --ignored
cargo test --features python optimizer_dspy_adapter_test -- --ignored
cargo test --features python memory_evolution_dspy_adapter_test -- --ignored

# Full test suite
cargo test --workspace --all-features
\`\`\`

### Performance Benchmarks

\`\`\`bash
cd src/orchestration/dspy_modules
uv run python3 baseline_benchmark.py --module reviewer --iterations 10 --output /tmp/baseline.json
cat /tmp/baseline.json | jq '.operations[].latency'
\`\`\`

**Expected Performance**:
- p50 latency: 100-200ms
- p95 latency: 200-400ms
- Success rate: 95%+

---

## Rollback Plan

### Emergency Rollback

If critical issues arise:

\`\`\`rust
// Disable DSPy via environment variable
let use_dspy = std::env::var("ENABLE_DSPY")
    .unwrap_or("false".to_string()) == "true";

if use_dspy {
    // Initialize DSPy adapters
} else {
    // Use PyObject fallback
    log::warn!("DSPy disabled, using legacy integration");
}
\`\`\`

### Rollback Verification

\`\`\`bash
# Run tests
cargo test --workspace

# Check metrics
tail -f logs/mnemosyne.log | grep -E "ERROR|WARN"
\`\`\`

**Target Metrics**:
- Error rate < 1%
- Latency < 500ms p95
- Success rate > 95%

---

## Common Issues

### Issue 1: Python Module Not Found

**Error**: `Failed to get semantic module: 'SemanticModule' not registered`

**Solution**: Ensure DSPy service initialized:
\`\`\`rust
let dspy_service = DSpyService::new().await?;
let modules = dspy_service.list_modules().await?;
assert!(modules.contains(&"Reviewer".to_string()));
\`\`\`

### Issue 2: GIL Deadlock

**Error**: `Tokio spawn_blocking failed`

**Solution**: Always use `spawn_blocking` for GIL operations:
\`\`\`rust
let result = tokio::spawn_blocking(move || {
    Python::with_gil(|py| {
        // Sync Python code only
    })
}).await?;
\`\`\`

### Issue 3: Missing API Key

**Error**: `ANTHROPIC_API_KEY environment variable not set`

**Solution**:
\`\`\`bash
export ANTHROPIC_API_KEY="your-api-key"
echo $ANTHROPIC_API_KEY  # Verify
\`\`\`

---

## Support & Resources

### Documentation

- **Architecture**: [DSPY_INTEGRATION.md](./DSPY_INTEGRATION.md) - Complete architecture guide
- **Testing**: [TESTING.md](./TESTING.md) - 145+ test cases, 80% coverage
- **Continuous Improvement**: [../src/orchestration/dspy_modules/CONTINUOUS_IMPROVEMENT.md](../src/orchestration/dspy_modules/CONTINUOUS_IMPROVEMENT.md)
- **Optimization Analysis**: [../src/orchestration/dspy_modules/OPTIMIZATION_ANALYSIS.md](../src/orchestration/dspy_modules/OPTIMIZATION_ANALYSIS.md)

### Code Examples

- **Reviewer Adapter**: `src/orchestration/actors/reviewer_dspy_adapter.rs` (src/orchestration/actors/reviewer_dspy_adapter.rs:1)
- **Semantic Bridge**: `src/ics/semantic_highlighter/tier3_analytical/dspy_integration.rs` (src/ics/semantic_highlighter/tier3_analytical/dspy_integration.rs:1)
- **Optimizer Adapter**: `src/orchestration/actors/optimizer_dspy_adapter.rs` (src/orchestration/actors/optimizer_dspy_adapter.rs:1)
- **Memory Evolution Adapter**: `src/evolution/memory_evolution_dspy_adapter.rs` (src/evolution/memory_evolution_dspy_adapter.rs:1)

### Tests

- **Python Tests**: `src/orchestration/dspy_modules/test_*.py` - 145+ tests
- **Rust Tests**: Search for `#[ignore]` in adapter files

---

## Appendix: Migration Checklist

### Pre-Migration

- [ ] Python environment configured
- [ ] API key set
- [ ] Python tests passing (145+)
- [ ] Rust tests passing
- [ ] Baseline metrics collected

### Phase 2: Reviewer

- [ ] Imports updated
- [ ] State struct updated
- [ ] DSPy service initialized
- [ ] All operations migrated
- [ ] Tests passing

### Phase 3: Semantic

- [ ] Semantic bridge created
- [ ] All analyzers migrated
- [ ] Tests passing

### Phase 4: Optimizer

- [ ] Optimizer adapter created
- [ ] All operations migrated
- [ ] Tests passing

### Phase 5: Memory Evolution

- [ ] Evolution adapter created
- [ ] All operations migrated
- [ ] Tests passing

### Post-Migration

- [ ] All modules migrated
- [ ] All tests passing
- [ ] Performance acceptable
- [ ] A/B testing shows no regression
- [ ] Documentation updated
- [ ] Team trained
- [ ] Rollback plan validated

---

## Summary

This migration guide provides a comprehensive path for adopting the DSPy integration. Follow the phased approach, validate at each step, and maintain rollback capability throughout.

**Timeline**: 1-2 weeks for full migration.

**Status**: Production-ready (Phase 7 complete, 2025-11-03).

For questions, refer to linked documentation or review test suites for implementation examples.
