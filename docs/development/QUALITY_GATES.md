# Quality Gates

This document defines what "done" means for different types of work in the dual-track feature branch.

---

## General Quality Gates

**All work must satisfy**:

1. **Compiles cleanly**
   - No compilation errors
   - Zero new compiler warnings
   - Feature flags respected (`#[cfg(feature = "python")]`)

2. **Tests pass**
   - All existing tests continue passing (660+ baseline)
   - New tests added for new functionality
   - Test coverage ≥ 70% for new code

3. **Code quality**
   - No new Clippy warnings
   - Code formatted with `cargo fmt`
   - No debug code (println!, dbg!, unwrap() in production paths)

4. **Documentation**
   - Public APIs have doc comments
   - Examples updated if behavior changed
   - README/guides reflect current state

5. **Git hygiene**
   - Atomic commits (one logical change)
   - Descriptive commit messages with track prefix
   - Clean working tree after commit

---

## Track-Specific Quality Gates

### [DSPy] Track Quality Gates

#### For Python DSPy Modules

- [ ] **Signature defined** using `dspy.Signature`
- [ ] **Module implemented** using `dspy.Module` with `ChainOfThought`
- [ ] **Python tests pass** (`pytest src/orchestration/dspy_modules/test_*.py`)
- [ ] **JSON I/O validated** (inputs/outputs match schema)
- [ ] **Error handling** implemented (invalid inputs, API failures)
- [ ] **Documentation** includes usage example

#### For Rust DSPy Adapters

- [ ] **Type-safe interface** (no raw PyObject leaks)
- [ ] **Async-safe** (uses `spawn_blocking` for GIL operations)
- [ ] **Error handling** (`Result<T, E>` for all fallible operations)
- [ ] **Integration test** marked `#[ignore]` (requires Python env)
- [ ] **Graceful fallback** maintained (works without Python)
- [ ] **Concurrent operations** tested (Arc<Adapter> works)

#### For DSPy Integration Points

- [ ] **Optional dependency** (code compiles without Python)
- [ ] **Runtime check** before DSPy usage (`if let Some(adapter)`)
- [ ] **Fallback documented** (what happens when DSPy unavailable)
- [ ] **Performance acceptable** (<10% overhead vs manual prompts)

---

### [SpecFlow] Track Quality Gates

#### For Artifact Types

- [ ] **Round-trip serialization** (`to_markdown` ↔ `from_markdown`)
- [ ] **YAML frontmatter** complete with all required fields
- [ ] **Builder API** fluent and ergonomic
- [ ] **Unit tests** cover parsing edge cases
- [ ] **Example updated** to demonstrate artifact

#### For CLI Commands

- [ ] **Help text** clear and complete
- [ ] **Error messages** actionable (not just "failed")
- [ ] **Dry-run mode** available for destructive operations
- [ ] **Examples** in documentation
- [ ] **Integration tested** manually

#### For Workflow Integration

- [ ] **Memory created** for artifact
- [ ] **Graph links** established (constitution → spec → plan)
- [ ] **File written** to correct location
- [ ] **Namespace** correctly inferred or specified
- [ ] **Concurrent operations** safe (no race conditions)

---

### [Integration] Track Quality Gates

- [ ] **Both tracks tested** independently
- [ ] **Integration points verified** (DSPy validates SpecFlow artifacts)
- [ ] **No cross-contamination** (DSPy changes don't break SpecFlow, vice versa)
- [ ] **Shared infrastructure** backward compatible
- [ ] **Documentation** explains integration clearly

---

### [Infra] Track Quality Gates

- [ ] **No breaking changes** to existing APIs
- [ ] **Migration path** documented (if breaking change unavoidable)
- [ ] **All tracks tested** after infrastructure change
- [ ] **Performance impact** measured and acceptable
- [ ] **Security implications** considered

---

## Phase-Specific Quality Gates

### Phase 1: Foundation ✅ Complete

- [x] Documentation comprehensive
- [x] Development infrastructure established
- [x] Beads tracking initialized

### Phase 2: Baselines

- [ ] Test coverage baseline documented
- [ ] Performance benchmarks established
- [ ] Code quality baseline recorded
- [ ] Documentation audit complete

### Phase 3: Sprint Planning

- [ ] Priority matrix created
- [ ] Work sequenced by dependencies
- [ ] Beads issues prioritized
- [ ] Success criteria defined

### Phase 4: Execution (DSPy)

**Training Data Collection**:
- [ ] ≥20 labeled examples per module
- [ ] Quality criteria documented
- [ ] Data schema validated
- [ ] README explains dataset

**Performance Baseline Benchmarking**:
- [ ] All 4 modules benchmarked
- [ ] Latency measured (p50, p95, p99)
- [ ] Token usage tracked
- [ ] Cost calculated
- [ ] Comparison with manual prompts

**MIPROv2 Optimization**:
- [ ] Optimized modules tested
- [ ] Improvements ≥5% vs baseline
- [ ] No quality regression
- [ ] Optimized prompts versioned

**GEPA Joint Optimization**:
- [ ] Cross-module consistency validated
- [ ] Joint improvements measured
- [ ] Synergistic effects documented

### Phase 4: Execution (SpecFlow)

**Clarification Parsing**:
- [ ] `from_markdown` implemented
- [ ] Round-trip test passes
- [ ] Example updated
- [ ] 6/6 artifacts complete

**Slash Commands**:
- [ ] Command file created (`.claude/commands/*.md`)
- [ ] Interactive Q&A working
- [ ] Artifacts created correctly
- [ ] Error handling robust
- [ ] Documentation updated

**Beads Integration**:
- [ ] Export function implemented
- [ ] Bidirectional sync working
- [ ] Completion status reflected
- [ ] No data loss

---

## Performance Quality Gates

### Latency Targets

- **DSPy operations**: <2x manual prompt latency
- **Artifact serialization**: <100ms per artifact
- **CLI commands**: <500ms response time
- **Memory operations**: <200ms (existing target)

### Resource Targets

- **Memory usage**: No >20% increase
- **Binary size**: No >10% increase
- **Compile time**: No >15% increase

### Cost Targets

- **DSPy API calls**: Track cost per operation
- **Optimization runs**: Budget defined before execution
- **A/B testing**: Compare total cost optimized vs baseline

---

## Test Coverage Targets

### By Component Type

- **Critical path** (core functionality): ≥90%
- **Business logic** (DSPy modules, artifacts): ≥80%
- **CLI/UI** (commands, examples): ≥60%
- **Infrastructure** (shared utilities): ≥70%

### By Test Type

- **Unit tests**: All pure functions tested
- **Integration tests**: All cross-module interactions tested
- **E2E tests**: At least 1 per major workflow
- **Property tests**: For complex algorithms (if applicable)

---

## Documentation Quality Gates

### Code Documentation

- [ ] All `pub` functions have doc comments
- [ ] Complex algorithms explained
- [ ] Safety invariants documented (`unsafe` code)
- [ ] Examples in doc comments for non-obvious usage

### User Documentation

- [ ] README.md up to date
- [ ] Getting started guide current
- [ ] CLI help text complete
- [ ] Migration guides (if breaking changes)

### Developer Documentation

- [ ] Architecture decisions documented (ADRs)
- [ ] Integration points mapped
- [ ] Development workflows explained
- [ ] Troubleshooting guides

---

## Merge Readiness Quality Gates

### Required for Main Merge

- [ ] **All P0 and P1 tasks complete**
- [ ] **Test suite passing** (660+ tests, zero regressions)
- [ ] **Zero new Clippy warnings**
- [ ] **All documentation current**
- [ ] **All examples working**
- [ ] **Beads state clean** (no critical blockers)
- [ ] **Performance benchmarks acceptable** (within targets)
- [ ] **Integration tests passing**
- [ ] **PR description comprehensive** (both tracks explained)
- [ ] **Code review complete** (self-review + external review)
- [ ] **Migration path documented** (if breaking changes)
- [ ] **Rollback plan** (how to undo if issues found)

### Recommended (but not blocking)

- [ ] P2 tasks complete
- [ ] GEPA optimization run
- [ ] A/B testing results positive
- [ ] Prompt versioning infrastructure
- [ ] Advanced workflow automation

---

## Enforcement

### Automated Checks (CI/CD)

When CI is configured:
- Compilation must pass
- All tests must pass
- Clippy must not fail
- Formatting must be correct

### Manual Review Checks

Before approving PR:
- Documentation read for clarity
- Examples run manually
- Integration points tested
- Performance impacts understood

### Self-Review Protocol

Use `COMMIT_CHECKLIST.md` before every commit
Use this document when marking tasks complete
Update quality gates as project evolves

---

## Exceptions

### When Quality Gates May Be Relaxed

1. **Exploratory work**: Mark commits with `[WIP]`, document in commit message
2. **Emergency fixes**: Document why gates skipped, plan to address later
3. **Documentation updates**: Skip compile/test gates
4. **Refactoring**: May temporarily reduce coverage (plan to restore)

### Exception Documentation

If skipping a quality gate:
- Document **why** in commit message
- Create **Beads issue** to address later
- Update **FEATURE_BRANCH_STATUS.md** with caveat
- Plan **checkpoint** to resolve

---

## Living Document

This document should evolve:
- Add gates when issues arise
- Remove gates that prove unnecessary
- Adjust targets based on experience
- Document all changes

**Principle**: Quality gates protect us from ourselves. Better to update them than to bypass them silently.

---

**Last Updated**: 2025-11-02
**Next Review**: After Sprint 1 completion
