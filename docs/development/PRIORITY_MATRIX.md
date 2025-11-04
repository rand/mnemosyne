# Priority Matrix - Dual-Track Feature Branch

**Branch**: feature/dspy-integration
**Date**: 2025-11-02
**Status**: Phase 3 - Sprint Planning

---

## Executive Summary

**Total Remaining Work**: 19 tasks across 2 tracks
- **DSPy Track**: Phase 4 (Optimization Pipeline) - 10 tasks
- **SpecFlow Track**: Phase 3 (Interactive Workflow) - 5 tasks
- **Integration Track**: 4 tasks

**Recommended Sprint 1 Scope**: 7 P0-P1 tasks (~24-34 hours)

---

## Prioritization Framework

### Scoring Model

**Value Score (1-10)**:
- 10 = Critical blocker (prevents other work)
- 7-9 = High value (major feature completion)
- 4-6 = Medium value (quality improvement)
- 1-3 = Low value (nice-to-have)

**Effort Score (1-10)**:
- 1-3 = Small (<4 hours)
- 4-6 = Medium (4-8 hours)
- 7-10 = Large (>8 hours)

**Risk Score (1-10)**:
- 1-3 = Low risk (well-understood, similar to past work)
- 4-6 = Medium risk (some unknowns, moderate complexity)
- 7-10 = High risk (many unknowns, high complexity)

**Priority Calculation**:
```
Priority = (Value × 2 + (11 - Effort) + (11 - Risk)) / 4
Higher score = Higher priority
```

**Priority Levels**:
- **P0**: Priority ≥ 8.0 (Foundation, critical path)
- **P1**: Priority 6.0-7.9 (Core features)
- **P2**: Priority 4.0-5.9 (Enhancements)
- **P3**: Priority < 4.0 (Future work)

---

## Priority Matrix

### Track: SpecFlow (Phase 3 - Interactive Workflow)

| ID | Task | Value | Effort | Risk | Priority | Level | Time |
|----|------|-------|--------|------|----------|-------|------|
| SF-1 | Complete Clarification parsing | 9 | 3 | 2 | **8.25** | P0 | 2-3h |
| SF-2 | Implement /feature-specify slash command | 8 | 4 | 3 | **7.50** | P1 | 3-4h |
| SF-3 | Implement /feature-clarify slash command | 8 | 5 | 4 | **7.00** | P1 | 4-5h |
| SF-4 | Implement /feature-plan slash command | 7 | 4 | 3 | **7.00** | P1 | 3-4h |
| SF-5 | Implement /feature-tasks slash command | 7 | 4 | 3 | **7.00** | P1 | 3-4h |
| SF-6 | Implement /feature-checklist slash command | 7 | 4 | 3 | **7.00** | P1 | 3-4h |
| SF-7 | Beads integration (export tasks) | 6 | 5 | 4 | **6.00** | P2 | 4-6h |
| SF-8 | Beads bidirectional sync | 5 | 6 | 5 | **5.00** | P2 | 3-4h |
| SF-9 | Advanced workflow integration tests | 5 | 4 | 3 | **5.50** | P2 | 3-4h |

**Subtotal**: 9 tasks, 28-42 hours

---

### Track: DSPy (Phase 4 - Optimization Pipeline)

| ID | Task | Value | Effort | Risk | Priority | Level | Time |
|----|------|-------|---|------|----------|-------|------|
| DS-1 | Create training data for ReviewerModule | 9 | 6 | 4 | **7.25** | P1 | 4-6h |
| DS-2 | Create training data for SemanticModule | 8 | 6 | 4 | **6.75** | P1 | 4-6h |
| DS-3 | Create training data for OptimizerModule* | 7 | 6 | 5 | **6.00** | P2 | 4-6h |
| DS-4 | Create training data for MemoryEvolutionModule* | 7 | 6 | 5 | **6.00** | P2 | 4-6h |
| DS-5 | Establish performance baselines (all modules) | 8 | 4 | 3 | **7.50** | P1 | 2-3h |
| DS-6 | Run MIPROv2 optimization (ReviewerModule) | 7 | 5 | 6 | **6.25** | P2 | 6-8h |
| DS-7 | Run MIPROv2 optimization (SemanticModule) | 7 | 5 | 6 | **6.25** | P2 | 6-8h |
| DS-8 | Run MIPROv2 optimization (OptimizerModule)* | 6 | 5 | 6 | **5.75** | P2 | 6-8h |
| DS-9 | Run MIPROv2 optimization (MemoryEvolutionModule)* | 6 | 5 | 6 | **5.75** | P2 | 6-8h |
| DS-10 | GEPA joint optimization (exploratory) | 5 | 8 | 8 | **4.25** | P3 | 8-12h |
| DS-11 | A/B testing framework | 6 | 6 | 5 | **5.50** | P2 | 4-6h |
| DS-12 | Prompt versioning infrastructure | 5 | 5 | 4 | **5.25** | P2 | 2-3h |

**Note**: Tasks marked with * depend on Optimizer and Memory Evolution modules being implemented first (not yet started).

**Subtotal**: 12 tasks, 52-80 hours

---

### Track: Integration (DSPy + SpecFlow)

| ID | Task | Value | Effort | Risk | Priority | Level | Time |
|----|------|-------|--------|------|----------|-------|------|
| INT-1 | Use ReviewerModule to validate feature specs | 8 | 3 | 3 | **7.75** | P1 | 2-3h |
| INT-2 | Use OptimizerModule for task generation* | 7 | 4 | 4 | **6.75** | P1 | 3-4h |
| INT-3 | Use MemoryEvolutionModule for spec consolidation* | 6 | 4 | 4 | **6.25** | P2 | 3-4h |
| INT-4 | End-to-end integration test (spec → implement) | 7 | 5 | 4 | **6.50** | P2 | 4-6h |

**Note**: Tasks marked with * depend on modules not yet implemented.

**Subtotal**: 4 tasks, 12-17 hours

---

## Total Workload Summary

| Priority | Count | Time Estimate |
|----------|-------|---------------|
| **P0** | 1 | 2-3h |
| **P1** | 10 | 29-41h |
| **P2** | 11 | 54-77h |
| **P3** | 1 | 8-12h |
| **Total** | 23 | 93-133h |

---

## Dependencies Graph

```
Foundation (P0)
├─ SF-1: Clarification parsing [2-3h]
   │
P1 Core Features
├─ SpecFlow Slash Commands (Parallel)
│  ├─ SF-2: /feature-specify [3-4h]
│  ├─ SF-3: /feature-clarify [4-5h] ← Depends on SF-1
│  ├─ SF-4: /feature-plan [3-4h]
│  ├─ SF-5: /feature-tasks [3-4h]
│  └─ SF-6: /feature-checklist [3-4h]
│
├─ DSPy Training Data (Parallel)
│  ├─ DS-1: ReviewerModule training [4-6h]
│  └─ DS-2: SemanticModule training [4-6h]
│
└─ DSPy Baselines (After training data)
   ├─ DS-5: Performance baselines [2-3h] ← Depends on DS-1, DS-2
   └─ INT-1: Reviewer validates specs [2-3h] ← Depends on DS-1

P2 Enhancements
├─ SpecFlow Advanced
│  ├─ SF-7: Beads export [4-6h] ← Depends on SF-5
│  ├─ SF-8: Beads bidirectional sync [3-4h] ← Depends on SF-7
│  └─ SF-9: Workflow integration tests [3-4h] ← Depends on SF-2,3,4,5,6
│
└─ DSPy Optimization
   ├─ DS-6: MIPROv2 ReviewerModule [6-8h] ← Depends on DS-5
   ├─ DS-7: MIPROv2 SemanticModule [6-8h] ← Depends on DS-5
   ├─ DS-11: A/B testing framework [4-6h] ← Depends on DS-6, DS-7
   └─ DS-12: Prompt versioning [2-3h] ← Depends on DS-6, DS-7

P3 Future Work
└─ DS-10: GEPA joint optimization [8-12h] ← Depends on DS-6, DS-7
```

---

## Sprint 1 Scope (Recommended)

### Goals
1. Complete SpecFlow Phase 3 foundation (Clarification parsing)
2. Deliver core SpecFlow slash commands
3. Establish DSPy training data and baselines
4. Integrate Reviewer with SpecFlow

### Included Tasks (7 tasks, 24-34 hours)

**Foundation (P0)**:
- [x] SF-1: Complete Clarification parsing (2-3h)

**Core Features (P1)**:
- [ ] SF-2: Implement /feature-specify (3-4h)
- [ ] SF-3: Implement /feature-clarify (4-5h)
- [ ] SF-4: Implement /feature-plan (3-4h)
- [ ] DS-1: Create ReviewerModule training data (4-6h)
- [ ] DS-5: Establish performance baselines (2-3h)
- [ ] INT-1: Reviewer validates specs (2-3h)

**Timeline**: 5-7 working days (assuming 5h/day focused work)

**Success Criteria**:
- [ ] All 6 artifact types have complete round-trip serialization
- [ ] 3+ slash commands functional (/specify, /clarify, /plan)
- [ ] ReviewerModule training data created (≥20 labeled examples)
- [ ] Performance baselines documented
- [ ] Reviewer validates feature specs
- [ ] All P0-P1 tasks complete
- [ ] Zero test regressions (≥657 passing)

---

## Sprint 2 Scope (Tentative)

### Goals
1. Complete remaining SpecFlow slash commands
2. Implement Beads integration
3. Run MIPROv2 optimization for Reviewer and Semantic modules
4. Create SemanticModule training data

### Included Tasks (9 tasks, 32-47 hours)

**Core Features (P1)**:
- [ ] SF-5: Implement /feature-tasks (3-4h)
- [ ] SF-6: Implement /feature-checklist (3-4h)
- [ ] DS-2: Create SemanticModule training data (4-6h)

**Enhancements (P2)**:
- [ ] SF-7: Beads export (4-6h)
- [ ] SF-8: Beads bidirectional sync (3-4h)
- [ ] SF-9: Workflow integration tests (3-4h)
- [ ] DS-6: MIPROv2 ReviewerModule optimization (6-8h)
- [ ] DS-7: MIPROv2 SemanticModule optimization (6-8h)
- [ ] DS-12: Prompt versioning (2-3h)

**Timeline**: 6-9 working days

---

## Sprint 3 Scope (Tentative)

### Goals
1. Complete DSPy optimization infrastructure
2. Integrate remaining DSPy modules with SpecFlow
3. Prepare for merge to main

### Included Tasks (6 tasks, 25-37 hours)

**Integration (P1-P2)**:
- [ ] INT-2: OptimizerModule for task generation (3-4h) *
- [ ] INT-3: MemoryEvolutionModule for spec consolidation (3-4h) *
- [ ] INT-4: End-to-end integration test (4-6h)

**Optimization (P2-P3)**:
- [ ] DS-3: Create OptimizerModule training data (4-6h) *
- [ ] DS-4: Create MemoryEvolutionModule training data (4-6h) *
- [ ] DS-11: A/B testing framework (4-6h)

**Conditional Work** (if Optimizer and Memory Evolution modules implemented):
- [ ] DS-8: MIPROv2 OptimizerModule (6-8h) *
- [ ] DS-9: MIPROv2 MemoryEvolutionModule (6-8h) *
- [ ] DS-10: GEPA joint optimization (8-12h)

**Note**: Tasks marked with * require Optimizer and Memory Evolution modules to be implemented first (not part of current dual-track branch).

**Timeline**: 5-7 working days + conditional work

---

## Risk Analysis

### High-Risk Tasks

**DS-6, DS-7, DS-8, DS-9: MIPROv2 Optimization** (Risk: 6)
- **Concern**: Teleprompter optimization is exploratory
- **Mitigation**: Start with ReviewerModule (well-understood domain)
- **Fallback**: Skip optimization if improvements <5%

**DS-10: GEPA Joint Optimization** (Risk: 8)
- **Concern**: Multi-module optimization highly experimental
- **Mitigation**: Treat as P3 (exploratory only)
- **Fallback**: Skip if time-constrained

**SF-7, SF-8: Beads Integration** (Risk: 4)
- **Concern**: Bidirectional sync complexity
- **Mitigation**: Start with one-way export (SF-7) first
- **Fallback**: Manual Beads creation acceptable

### Medium-Risk Tasks

**SF-3: /feature-clarify** (Risk: 4)
- **Concern**: Interactive Q&A flow complexity
- **Mitigation**: Limit to max 3 clarifications
- **Fallback**: Simple prompt-response model

**DS-1, DS-2: Training Data Creation** (Risk: 4)
- **Concern**: Quality and quantity of training examples
- **Mitigation**: Start with ≥20 examples, iterate
- **Fallback**: Use existing test cases as starting point

### Low-Risk Tasks

**SF-1: Clarification parsing** (Risk: 2)
- **Concern**: Minimal, similar to other artifact parsing
- **Mitigation**: Use existing pattern from other artifacts

**SF-2, SF-4, SF-5, SF-6: Slash Commands** (Risk: 3)
- **Concern**: Low, slash command infrastructure exists
- **Mitigation**: Follow established patterns

---

## Blockers and Dependencies

### Current Blockers

**None** - All Sprint 1 tasks are unblocked and ready to start.

### Future Blockers

**Sprint 2-3**:
- INT-2, INT-3 blocked until OptimizerModule and MemoryEvolutionModule implemented
- DS-3, DS-4, DS-8, DS-9 similarly blocked

**Resolution**: These modules are part of DSPy Phase 2-3, not yet started. Can either:
1. Implement modules as part of Sprint 2-3 (adds 16-24 hours)
2. Defer integration tasks to post-merge work
3. Reduce Sprint 3 scope to only completed modules

**Recommendation**: Option 2 (defer), focus on ReviewerModule and SemanticModule optimization.

---

## Sequencing Strategy

### Parallel Workstreams

**Stream A: SpecFlow Commands** (Independent)
- SF-1 → SF-2, SF-4, SF-5, SF-6 (parallel) → SF-3 (after SF-1)

**Stream B: DSPy Training & Optimization** (Sequential with parallel substeps)
- DS-1, DS-2 (parallel) → DS-5 → DS-6, DS-7 (parallel) → DS-11, DS-12

**Stream C: Integration** (Depends on A + B)
- INT-1 (after DS-1) → INT-4 (after SF-2,3,4,5,6)

### Execution Order (Sprint 1)

**Week 1**:
1. **Day 1-2**: SF-1 (Clarification parsing) + SF-2 (/feature-specify) - Parallel
2. **Day 2-4**: DS-1 (Reviewer training data) + SF-4 (/feature-plan) - Parallel
3. **Day 4-5**: SF-3 (/feature-clarify) sequential after SF-1
4. **Day 5**: DS-5 (Performance baselines) after DS-1
5. **Day 5-6**: INT-1 (Reviewer validates specs) after DS-1

**Checkpoint**: After Week 1, evaluate progress and adjust Sprint 1 scope if needed.

---

## Quality Gates (Sprint 1)

### Must Pass
- [ ] All existing tests still passing (≥657)
- [ ] Zero new compiler warnings (≤26 total)
- [ ] Zero new clippy warnings (≤9 total)
- [ ] All new code has unit tests
- [ ] All slash commands have examples
- [ ] Training data meets quality threshold (validated by manual review)

### Should Pass
- [ ] Documentation updated (slash commands, training data process)
- [ ] Commit messages follow template
- [ ] Code formatted with rustfmt/black

### Nice to Have
- [ ] Examples demonstrate end-to-end workflows
- [ ] Performance improvements documented

---

## Success Metrics

### Sprint 1
- **Feature Completeness**: 3/6 slash commands (50%)
- **Artifact Round-Trip**: 6/6 types (100%)
- **Training Data**: 1/2 modules (ReviewerModule)
- **Integration**: 1/4 tasks (Reviewer validates specs)

### Sprint 2
- **Feature Completeness**: 6/6 slash commands (100%)
- **Beads Integration**: Export functional, sync implemented
- **Training Data**: 2/2 modules (Reviewer + Semantic)
- **Optimization**: 2/4 modules optimized (Reviewer + Semantic)

### Sprint 3
- **Integration**: 2-4/4 tasks (depending on module availability)
- **Optimization Infrastructure**: A/B testing, versioning complete
- **Merge Readiness**: All quality gates pass

---

## Next Steps

### Immediate (Today)
1. ✅ Create PRIORITY_MATRIX.md (this document)
2. [ ] Update Beads issues with priorities from matrix
3. [ ] Create Sprint 1 milestone in Beads
4. [ ] Commit priority matrix

### Tomorrow (Start Sprint 1)
1. [ ] Begin SF-1: Complete Clarification parsing
2. [ ] Begin SF-2: Implement /feature-specify
3. [ ] Begin DS-1: Create ReviewerModule training data

---

**Priority Matrix Approved**: 2025-11-02
**Next Review**: After Sprint 1 completion (Week 1 checkpoint)
