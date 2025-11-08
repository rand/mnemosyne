# Phase A: Complete DSPy Integration

**Created**: 2025-11-07
**Status**: Ready to Start
**Estimated Duration**: 12-15 hours (1-2 weeks)
**Priority**: CRITICAL

---

## Context

Mnemosyne v2.1.2 has excellent infrastructure (90% complete, 726 tests passing) but incomplete multi-agent orchestration (60% complete). The primary blocker is **incomplete DSPy integration**:

- ✅ **Phase 1 Complete**: Reviewer module fully optimized (56% improvement)
- 🟡 **Phase 2 Partial**: Optimizer skills discovery done, context consolidation pending
- ❌ **Phase 3 Missing**: Memory Evolution adapter not implemented

## Objective

Complete DSPy integration for all 3 core modules, deploy proven optimizations, and validate improvements.

---

## Tasks

### A1. Memory Evolution DSPy Adapter (4-6 hours) ⭐ CRITICAL

**Goal**: Enable intelligent memory consolidation via DSPy

**Status**: Python module exists, Rust adapter MISSING

**Files**:
- ✅ Python module: `src/orchestration/dspy_modules/memory_evolution_module.py`
- ❌ Rust adapter: `src/evolution/memory_evolution_dspy_adapter.rs` (CREATE)
- 🔧 Integration: `src/evolution/consolidation.rs` (UPDATE)

**Implementation Steps**:
1. Create `memory_evolution_dspy_adapter.rs`
   - Follow pattern from `reviewer_dspy_adapter.rs`
   - Implement PyO3 bridge for 3 signatures:
     - `consolidate_memory_cluster`
     - `recalibrate_importance`
     - `detect_archival_candidates`
   - Add error handling and graceful degradation

2. Integrate in `consolidation.rs`
   - Replace manual LLM calls with DSPy adapter
   - Use structured `ConsolidationDecision` outputs
   - Maintain backward compatibility

3. Write integration tests
   - Test consolidation decisions
   - Validate JSON parsing
   - Test error handling

**Success Criteria**:
- ✅ Rust adapter compiles and passes tests
- ✅ Consolidation uses DSPy (not manual LLM calls)
- ✅ Integration tests pass
- ✅ Quality improvement measurable

---

### A2. Optimizer Context Consolidation (3-4 hours) ⭐ HIGH

**Goal**: Use DSPy for intelligent context summarization

**Status**: DSPy signature exists, not integrated in Rust

**Files**:
- ✅ Python module: `src/orchestration/dspy_modules/optimizer_module.py`
- ✅ Rust adapter: `src/orchestration/optimizer_dspy_adapter.rs` (EXISTS, needs enhancement)
- 🔧 Integration: `src/orchestration/actors/optimizer.rs` (UPDATE)

**Implementation Steps**:
1. Enhance `optimizer_dspy_adapter.rs`
   - Add `consolidate_context` method
   - Bridge to Python `ConsolidateContext` signature
   - Return structured context summary

2. Update `optimizer.rs`
   - Replace heuristic consolidation (line ~551)
   - Use DSPy adapter for context summarization
   - Validate quality vs baseline

3. Test integration
   - Compare DSPy vs heuristic quality
   - Measure context reduction ratio
   - Validate preserved information

**Success Criteria**:
- ✅ Optimizer uses DSPy for consolidation
- ✅ Context quality improved vs heuristics
- ✅ Integration tests pass

---

### A3. Deploy v2 Optimization Results (2-3 hours) ⭐ HIGH

**Goal**: Use optimized prompts in production

**Status**: v2 runs complete, results not deployed

**Files**:
- 📊 Results: `src/orchestration/dspy_modules/optimized/reviewer_v2_results.json` (EXTRACT)
- 🔧 Config: `src/orchestration/dspy_modules/optimized/reviewer_optimized_v2.json` (UPDATE)
- 🔧 Loader: `src/orchestration/actors/reviewer.rs` (UPDATE to load v2)

**Implementation Steps**:
1. Extract v2 optimization results
   - Check `scripts/optimize_dspy.sh` logs
   - Parse trial results for best prompts
   - Extract optimized prompts for:
     - `validate_completeness`
     - `validate_correctness`
     - `generate_guidance`

2. Update configuration
   - Create `reviewer_optimized_v2.json`
   - Include optimized prompts
   - Add metadata (version, date, metrics)

3. Update module loader
   - Modify `reviewer.rs` to prefer v2
   - Add fallback to v1 if v2 missing
   - Log which version loaded

4. Measure production improvement
   - Run baseline tests with v1
   - Run tests with v2
   - Compare quality metrics
   - Document improvement

**Success Criteria**:
- ✅ v2 prompts deployed
- ✅ Measurable quality improvement (>10%)
- ✅ Graceful fallback to v1

---

### A4. Close Completed Beads Tasks (30 minutes)

**Goal**: Accurate project tracking

**Tasks to Close**:
- ✅ **mnemosyne-33**: INT-2 OptimizerModule skills discovery (COMPLETE)
- ✅ **mnemosyne-34**: Expand training data 20→50 examples (COMPLETE)

**Tasks to Update**:
- 🔄 **mnemosyne-35**: Re-optimize stagnant signatures (v2 running → results pending)

**Commands**:
```bash
bd close mnemosyne-33 --reason "Complete"
bd close mnemosyne-34 --reason "Complete"
bd update mnemosyne-35 --comment "v2 optimization runs complete, awaiting result extraction"
```

**Success Criteria**:
- ✅ Beads state accurate
- ✅ Project tracking reflects reality

---

## Dependencies & Prerequisites

### Required Tools
- Rust 1.75+
- Python 3.10-3.13 (PyO3 compatibility)
- `uv` for Python package management
- Beads (`bd`) for task tracking

### Environment Setup
```bash
# Ensure Python environment ready
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo test --features python

# Verify DSPy modules load
cd src/orchestration/dspy_modules
uv run python -c "from memory_evolution_module import MemoryEvolutionModule; print('OK')"
```

### Build & Test
```bash
# Fast rebuild for development
./scripts/rebuild-and-update-install.sh

# Run tests
cargo test --lib
PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo test --features python
```

---

## Quality Gates

### Code Quality
- [ ] Zero compiler warnings
- [ ] All tests passing (target: 730+)
- [ ] Clippy lints addressed
- [ ] Code formatted (`cargo fmt`)

### Integration Quality
- [ ] DSPy adapter follows established patterns
- [ ] Error handling comprehensive
- [ ] Graceful degradation on LLM failure
- [ ] Type-safe Rust↔Python bridge

### Performance
- [ ] No regression vs baseline
- [ ] Latency acceptable (<500ms p95)
- [ ] Memory usage stable

### Documentation
- [ ] Inline code comments for complex logic
- [ ] Integration tests document expected behavior
- [ ] CHANGELOG.md updated
- [ ] ROADMAP.md updated

---

## Risk Mitigation

### Risk 1: PyO3 Python Environment Issues
- **Likelihood**: Medium
- **Impact**: High (blocks development)
- **Mitigation**: Use `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1`, verify Python 3.10-3.13

### Risk 2: DSPy Module Import Failures
- **Likelihood**: Low
- **Impact**: High
- **Mitigation**: Test imports before coding, validate `uv` dependencies

### Risk 3: v2 Results Missing or Incomplete
- **Likelihood**: Low
- **Impact**: Medium
- **Mitigation**: Extract from logs, fall back to manual optimization if needed

### Risk 4: Quality Regression
- **Likelihood**: Low
- **Impact**: High
- **Mitigation**: A/B testing, keep v1 as fallback

---

## Success Metrics

### Phase A Complete When:
1. ✅ Memory Evolution using DSPy (adapter implemented, tests passing)
2. ✅ Optimizer using DSPy for context consolidation
3. ✅ v2 optimized prompts deployed and validated
4. ✅ Beads tasks accurately reflect status
5. ✅ Quality improvement measured (>10% vs baseline)
6. ✅ All tests passing (730+ target)
7. ✅ Zero compiler warnings

### Measurement Baseline
- Current tests passing: 726
- Current quality (Reviewer): validate_completeness 75%, validate_correctness 75%
- Target quality: 85%+ (based on v2 optimization showing 56% improvement on extract_requirements)

---

## Next Steps After Phase A

### Phase B: Validation & Documentation (8-10 hours)
1. End-to-end workflow test (Phase 1→2→3→4)
2. Agent workflow documentation
3. Troubleshooting guide

### Phase C: Production Deployment (6-8 hours)
1. A/B testing framework
2. Production telemetry
3. Safe rollout with monitoring

---

## References

### Key Files
- **Reviewer adapter** (pattern to follow): `src/orchestration/reviewer_dspy_adapter.rs`
- **Python modules**: `src/orchestration/dspy_modules/*.py`
- **Consolidation logic**: `src/evolution/consolidation.rs`
- **Optimizer logic**: `src/orchestration/actors/optimizer.rs`

### Documentation
- **Agent Guide**: `AGENT_GUIDE.md`
- **Architecture**: `ARCHITECTURE.md`
- **TODO Tracking**: `TODO_TRACKING.md`
- **DSPy Integration**: `docs/guides/llm-reviewer.md`

### Commands
```bash
# Start MCP server
cargo run --bin mnemosyne -- serve --with-api

# Run optimization
./scripts/optimize_dspy.sh --signature validate_completeness --trials 50

# Check Beads status
bd ready --json --limit 10
```

---

**Last Updated**: 2025-11-07
**Status**: Ready to execute
**Estimated Completion**: Week of 2025-11-18
