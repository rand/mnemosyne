# TODO Tracking for v2.0 Production Readiness

**Generated**: 2025-10-30
**Total TODOs**: 17
**Status**: All must be completed for v2.0 merge to main

---

## Category A: Evaluation System (13 TODOs) - **CRITICAL**

### Feature Extractor (9 TODOs)
**File**: `src/evaluation/feature_extractor.rs`

| Line | TODO | Priority | Estimated Time |
|------|------|----------|----------------|
| 115 | Namespace matching logic | High | 30min |
| 246 | Memory lookup implementation | High | 1h |
| 251 | File stat lookup implementation | Medium | 1h |
| 266 | Access frequency calculation | High | 1h |
| 281 | Last accessed timing | High | 45min |
| 324 | Historical success queries | High | 1.5h |
| 338 | Co-occurrence tracking | Medium | 1.5h |
| 371 | Feature persistence (database insert) | High | 1h |
| 380 | Feature retrieval (database query) | High | 1h |

**Subtotal**: ~9 hours

### Relevance Scorer (2 TODOs)
**File**: `src/evaluation/relevance_scorer.rs`

| Line | TODO | Priority | Estimated Time |
|------|------|----------|----------------|
| 641 | Hierarchical propagation (session → project → global) | High | 2h |
| 712 | Metrics calculation (precision, recall, F1) | Medium | 1h |

**Subtotal**: ~3 hours

### Python Evaluation Bindings (2 TODOs)
**File**: `src/python_bindings/evaluation.rs`

| Line | TODO | Priority | Estimated Time |
|------|------|----------|----------------|
| 241 | Actual weight lookup from database | High | 1h |
| 260 | Weight update implementation | High | 1h |

**Subtotal**: ~2 hours

**Category A Total**: ~14 hours

---

## Category B: Evolution System (1 TODO) - **HIGH**

**File**: `src/evolution/links.rs`

| Line | TODO | Priority | Estimated Time |
|------|------|----------|----------------|
| 104 | Add last_traversed_at from database | Medium | 1h |

**Category B Total**: ~1 hour

---

## Category C: Orchestration System (1 TODO) - **MEDIUM**

**File**: `src/orchestration/actors/reviewer.rs`

| Line | TODO | Priority | Estimated Time |
|------|------|----------|----------------|
| 626 | Check if all work items in current phase are complete | Medium | 1h |

**Category C Total**: ~1 hour

---

## Category D: ICS Completion (2 TODOs) - **MANDATORY**

### Vim Mode (1 TODO)
**File**: `src/ics/editor/buffer.rs`

| Line | TODO | Priority | Estimated Time |
|------|------|----------|----------------|
| 287 | Implement other movement commands (w, b, f, t, text objects) | High | 4-6h |

### Syntax Highlighting (1 TODO)
**File**: `src/ics/editor/highlight.rs`

| Line | TODO | Priority | Estimated Time |
|------|------|----------|----------------|
| 103 | Add more language support (8+ languages) | High | 6-8h |

**Category D Total**: ~10-14 hours

---

## Summary by Priority

### Critical Path (Must Complete First)
1. **Evaluation System** (14h) - Required for memory learning
2. **ICS Syntax Highlighting** (6-8h) - User requirement
3. **ICS Vim Mode** (4-6h) - User requirement

### High Priority (Complete Next)
4. **Evolution Links** (1h) - Completes evolution system
5. **Orchestration Reviewer** (1h) - Completes orchestration

---

## Implementation Order

Based on dependencies and effort:

1. **Phase 2.2: Evaluation System** (14h)
   - Start with database schema updates if needed
   - Implement feature_extractor TODOs (9)
   - Implement relevance_scorer TODOs (2)
   - Implement Python bindings TODOs (2)
   - Write comprehensive tests

2. **Phase 2.1: ICS Completion** (10-14h)
   - Syntax highlighting first (6-8h) - tree-sitter integration
   - Vim mode second (4-6h) - editor commands
   - Test with multiple languages

3. **Phase 2.3: Evolution System** (1h)
   - Implement last_traversed_at tracking
   - Update LinkDecayJob

4. **Phase 2.4: Orchestration Reviewer** (1h)
   - Implement phase completion check
   - Add tests

---

## Progress Tracking

- [x] Phase 0.1: GitHub Issue #4 fixed ✓
- [ ] Phase 0.2: TODO audit complete (in progress)
- [ ] Evaluation System: 0/13 TODOs complete
- [ ] ICS: 0/2 TODOs complete
- [ ] Evolution: 0/1 TODOs complete
- [ ] Orchestration: 0/1 TODOs complete

**Total Progress**: 0/17 TODOs complete (0%)

---

## Notes

- All TODOs represent legitimate pending work (not documentation)
- Total estimated time: 26-30 hours of focused implementation
- Does not include testing time (add 50% for comprehensive tests)
- Final estimate with tests: 39-45 hours

---

## Next Steps

1. Complete Phase 0.2 (this audit) ✓
2. Begin Phase 1: Infrastructure (README, installation)
3. Begin Phase 2.2: Evaluation System implementation
4. Track progress in this file as TODOs are completed
