# DSPy Integration Plan - Comprehensive Roadmap

## Overview

Complete plan for DSPy integration across all Mnemosyne systems requiring LLM-based operations.

**Status**: Phase 1 (Reviewer + Tier 3 Semantic) complete. Phases 2-4 in progress.

## Architecture Principles

1. **Layered Design**: Python DSPy modules → Generic DSpyBridge → Type-safe Rust adapters → Integration points
2. **Conditional Compilation**: All DSPy integration behind `#[cfg(feature = "python")]`
3. **Graceful Degradation**: Clear error messages when DSPy not available
4. **Optimization Ready**: All modules designed for teleprompter optimization (MIPROv2, GEPA)
5. **Type Safety**: Strongly-typed Rust interfaces, validated JSON conversion

---

## Phase 1: Foundation (Reviewer + Tier 3 Semantic) ✅ COMPLETE

### Stream A: Reviewer Integration ✅
- **Python Module**: `ReviewerModule` with 4 DSPy signatures
  - `extract_requirements(user_intent, context) -> requirements`
  - `validate_intent(user_intent, implementation, execution_context) -> (satisfied, issues)`
  - `verify_completeness(requirements, implementation, execution_context) -> (complete, issues)`
  - `verify_correctness(implementation, execution_context) -> (correct, issues)`
- **Rust Adapter**: `ReviewerDSpyAdapter` (type-safe wrapper)
- **Integration**: `reviewer.rs` updated to use adapter
- **Status**: ✅ Complete, tested, documented

### Stream B: Tier 3 Semantic Integration ✅
- **Python Module**: `SemanticModule` with 3 DSPy signatures
  - `analyze_discourse(text) -> (segments, coherence_score)`
  - `detect_contradictions(text) -> contradictions`
  - `extract_pragmatics(text) -> elements`
- **Rust Bridge**: `DSpySemanticBridge` (direct PyO3 integration)
- **Integration**: `discourse.rs`, `contradictions.rs`, `pragmatics.rs`
- **Status**: ✅ Complete, tested, documented

### Documentation & Testing ✅
- ✅ `docs/DSPY_INTEGRATION.md` (463 lines)
- ✅ Python tests: `test_reviewer_module.py`, `test_semantic_module.py`
- ✅ Rust integration tests: 3 test files (1,363 total lines)
- ✅ All tests marked `#[ignore]` requiring Python environment

---

## Phase 2: Optimizer Agent Enhancement 🚧 IN PROGRESS

### Overview
Optimizer currently uses heuristic-based context consolidation and skills discovery. DSPy integration will enable intelligent, optimizable operations.

### Stream C: Context Consolidation DSPy Enhancement

**Current State** (`optimizer.rs:321-526`):
- Progressive consolidation: detailed → summary → compressed
- String-based heuristics for deciding consolidation level
- Manual template-based content formatting

**Target State**:
- DSPy-guided intelligent context summarization
- Optimizable consolidation strategies via teleprompters
- Semantic understanding of review feedback importance

**Implementation Tasks**:

#### C1: Create OptimizerModule (Python)
**File**: `src/orchestration/dspy_modules/optimizer_module.py`

**Signatures**:
```python
class ConsolidateContext(dspy.Signature):
    """Consolidate work item context based on review feedback."""

    original_intent = dspy.InputField(desc="User's original work intent")
    execution_summaries = dspy.InputField(desc="List of execution memory summaries")
    review_feedback = dspy.InputField(desc="List of review issues")
    suggested_tests = dspy.InputField(desc="List of suggested test improvements")
    review_attempt = dspy.InputField(desc="Review attempt number (1-N)")
    consolidation_mode = dspy.InputField(desc="detailed|summary|compressed")

    consolidated_content = dspy.OutputField(desc="Consolidated context content")
    key_issues = dspy.OutputField(desc="List of critical issues to address")
    strategic_guidance = dspy.OutputField(desc="Strategic recommendations")
    estimated_tokens = dspy.OutputField(desc="Estimated token count")

class DiscoverSkills(dspy.Signature):
    """Discover relevant skills for a task description."""

    task_description = dspy.InputField(desc="Description of task to perform")
    available_skills = dspy.InputField(desc="List of available skill metadata")
    max_skills = dspy.InputField(desc="Maximum number of skills to return")
    current_context_usage = dspy.InputField(desc="Current context usage percentage")

    selected_skills = dspy.OutputField(desc="List of skill names to load")
    relevance_scores = dspy.OutputField(desc="Relevance score for each skill")
    reasoning = dspy.OutputField(desc="Why these skills were selected")

class OptimizeContextBudget(dspy.Signature):
    """Optimize context allocation across different resource types."""

    current_usage = dspy.InputField(desc="Current context usage breakdown")
    loaded_resources = dspy.InputField(desc="Currently loaded resources")
    target_pct = dspy.InputField(desc="Target context percentage")
    work_priority = dspy.InputField(desc="Current work item priority")

    unload_skills = dspy.OutputField(desc="Skills to unload")
    unload_memories = dspy.OutputField(desc="Memory IDs to unload")
    optimization_rationale = dspy.OutputField(desc="Explanation of decisions")
```

**Module Implementation**:
```python
class OptimizerModule(dspy.Module):
    def __init__(self):
        super().__init__()
        self.consolidate = dspy.ChainOfThought(ConsolidateContext)
        self.discover_skills = dspy.ChainOfThought(DiscoverSkills)
        self.optimize_budget = dspy.ChainOfThought(OptimizeContextBudget)

    def consolidate_context(self, **kwargs) -> dspy.Prediction:
        return self.consolidate(**kwargs)

    def discover_skills_for_task(self, **kwargs) -> dspy.Prediction:
        return self.discover_skills(**kwargs)

    def optimize_context_allocation(self, **kwargs) -> dspy.Prediction:
        return self.optimize_budget(**kwargs)
```

**Dependencies**: None (greenfield)
**Parallel**: Can implement alongside Stream D

#### C2: Create OptimizerDSpyAdapter (Rust)
**File**: `src/orchestration/actors/optimizer_dspy_adapter.rs`

**Interface**:
```rust
pub struct OptimizerDSpyAdapter {
    bridge: Arc<DSpyBridge>,
}

impl OptimizerDSpyAdapter {
    pub fn new(bridge: Arc<DSpyBridge>) -> Self;

    pub async fn consolidate_context(
        &self,
        original_intent: &str,
        execution_summaries: Vec<String>,
        review_feedback: Vec<String>,
        suggested_tests: Vec<String>,
        review_attempt: u32,
        consolidation_mode: &str,
    ) -> Result<ConsolidatedContext>;

    pub async fn discover_skills(
        &self,
        task_description: &str,
        available_skills: Vec<SkillMetadata>,
        max_skills: usize,
        current_context_usage: f32,
    ) -> Result<SkillDiscoveryResult>;

    pub async fn optimize_context_budget(
        &self,
        current_usage: ContextUsage,
        loaded_resources: LoadedResources,
        target_pct: f32,
        work_priority: u8,
    ) -> Result<OptimizationPlan>;
}
```

**Dependencies**: Requires DSpyBridge (already exists)
**Parallel**: Can implement alongside Stream D

#### C3: Integrate with optimizer.rs
**File**: `src/orchestration/actors/optimizer.rs`

**Changes**:
1. Add optional `optimizer_adapter: Option<Arc<OptimizerDSpyAdapter>>` field
2. Update `consolidate_work_item_context()` to use adapter when available
3. Update `discover_skills()` to use adapter for intelligent matching
4. Add `compact_context()` DSPy-guided optimization
5. Keep heuristic fallbacks for backward compatibility

**Dependencies**: Requires C1 and C2 complete
**Parallel**: Final integration step after C1, C2

#### C4: Testing
- Python tests: `test_optimizer_module.py`
- Rust integration tests: `optimizer_dspy_adapter_test.rs`
- End-to-end context consolidation scenarios

---

### Stream D: Skills Discovery Enhancement

**Note**: Partially overlaps with Stream C (OptimizerModule.discover_skills), but focuses on filesystem-based discovery integration.

**Current State** (`optimizer.rs:118-153`):
- Uses `SkillsDiscovery` engine with keyword matching
- Loads skill metadata from filesystem
- Scores relevance based on patterns

**Target State**:
- DSPy-enhanced semantic skill matching
- Context-aware skill selection
- Budget-conscious loading strategies

**Implementation**:
- Leverage `DiscoverSkills` signature from OptimizerModule (C1)
- Create skill metadata extractor for filesystem skills
- Integrate with existing `SkillsDiscovery` as enhancement layer

**Dependencies**: Requires Stream C complete
**Parallel**: Can plan alongside C, implement after C1-C2

---

## Phase 3: Memory Evolution System Enhancement 🚧 IN PROGRESS

### Overview
Memory evolution (consolidation, archival, importance recalibration) currently uses heuristic decisions for duplicate detection and merging. DSPy integration enables intelligent, semantic-aware consolidation.

### Stream E: Consolidation Job DSPy Enhancement

**Current State** (`evolution/consolidation.rs`):
- ✅ Already has LLM integration (`make_llm_consolidation_decision`)
- ✅ Four decision modes: Heuristic, LlmAlways, LlmSelective, LlmWithFallback
- ❌ Uses manual prompt building (`build_cluster_prompt`)
- ❌ Manual JSON parsing (`parse_llm_cluster_response`)
- ❌ Direct LlmService calls (no optimization framework)

**Target State**:
- DSPy signatures replace manual prompts
- Structured outputs via DSPy (no manual parsing)
- Teleprompter-optimizable consolidation decisions
- Systematic prompt improvement via MIPROv2

**Implementation Tasks**:

#### E1: Create MemoryEvolutionModule (Python)
**File**: `src/orchestration/dspy_modules/memory_evolution_module.py`

**Signatures**:
```python
class ConsolidateMemoryCluster(dspy.Signature):
    """Analyze memory cluster and decide consolidation strategy."""

    cluster_memories = dspy.InputField(desc="List of memory metadata (id, created, summary, content, keywords)")
    avg_similarity = dspy.InputField(desc="Average similarity score in cluster")
    similarity_scores = dspy.InputField(desc="Pairwise similarity scores")

    action = dspy.OutputField(desc="MERGE|SUPERSEDE|KEEP")
    primary_memory_id = dspy.OutputField(desc="ID of memory to keep/enhance")
    secondary_memory_ids = dspy.OutputField(desc="IDs of memories to merge/supersede")
    rationale = dspy.OutputField(desc="Explanation of decision")
    preserved_content = dspy.OutputField(desc="Key facts to preserve from secondary memories")
    confidence = dspy.OutputField(desc="Confidence score 0.0-1.0")

class RecalibrateImportance(dspy.Signature):
    """Recalibrate memory importance based on access patterns and age."""

    memory_id = dspy.InputField(desc="Memory ID")
    current_importance = dspy.InputField(desc="Current importance score")
    access_count = dspy.InputField(desc="Number of times accessed")
    days_since_created = dspy.InputField(desc="Age in days")
    days_since_accessed = dspy.InputField(desc="Days since last access")
    memory_type = dspy.InputField(desc="Type of memory")
    linked_memories_count = dspy.InputField(desc="Number of linked memories")

    new_importance = dspy.OutputField(desc="Recalibrated importance (1-10)")
    adjustment_reason = dspy.OutputField(desc="Why importance changed")
    recommended_action = dspy.OutputField(desc="KEEP|ARCHIVE|DELETE")

class DetectArchivalCandidates(dspy.Signature):
    """Identify memories suitable for archival."""

    memories = dspy.InputField(desc="List of memory metadata")
    archival_threshold_days = dspy.InputField(desc="Age threshold for archival consideration")
    min_importance = dspy.InputField(desc="Minimum importance to keep active")

    archive_ids = dspy.OutputField(desc="Memory IDs to archive")
    keep_ids = dspy.OutputField(desc="Memory IDs to keep active")
    rationale = dspy.OutputField(desc="Archival decision reasoning")
```

**Module Implementation**:
```python
class MemoryEvolutionModule(dspy.Module):
    def __init__(self):
        super().__init__()
        self.consolidate = dspy.ChainOfThought(ConsolidateMemoryCluster)
        self.recalibrate = dspy.ChainOfThought(RecalibrateImportance)
        self.detect_archival = dspy.ChainOfThought(DetectArchivalCandidates)

    def consolidate_cluster(self, **kwargs) -> dspy.Prediction:
        return self.consolidate(**kwargs)

    def recalibrate_importance(self, **kwargs) -> dspy.Prediction:
        return self.recalibrate(**kwargs)

    def detect_archival_candidates(self, **kwargs) -> dspy.Prediction:
        return self.detect_archival(**kwargs)
```

**Dependencies**: None (greenfield)
**Parallel**: ✅ Can implement alongside Stream C

#### E2: Create MemoryEvolutionDSpyAdapter (Rust)
**File**: `src/evolution/memory_evolution_dspy_adapter.rs`

**Interface**:
```rust
pub struct MemoryEvolutionDSpyAdapter {
    bridge: Arc<DSpyBridge>,
}

impl MemoryEvolutionDSpyAdapter {
    pub fn new(bridge: Arc<DSpyBridge>) -> Self;

    pub async fn consolidate_cluster(
        &self,
        cluster: &MemoryCluster,
    ) -> Result<ConsolidationDecision>;

    pub async fn recalibrate_importance(
        &self,
        memory: &MemoryNote,
    ) -> Result<ImportanceRecalibration>;

    pub async fn detect_archival_candidates(
        &self,
        memories: &[MemoryNote],
        config: &ArchivalConfig,
    ) -> Result<ArchivalDecisions>;
}

#[derive(Debug)]
pub struct ImportanceRecalibration {
    pub new_importance: u8,
    pub adjustment_reason: String,
    pub recommended_action: RecommendedAction,
}

#[derive(Debug)]
pub enum RecommendedAction {
    Keep,
    Archive,
    Delete,
}

#[derive(Debug)]
pub struct ArchivalDecisions {
    pub archive_ids: Vec<MemoryId>,
    pub keep_ids: Vec<MemoryId>,
    pub rationale: String,
}
```

**Dependencies**: Requires DSpyBridge
**Parallel**: ✅ Can implement alongside Stream C

#### E3: Integrate with consolidation.rs
**File**: `src/evolution/consolidation.rs`

**Changes**:
1. Add optional `evolution_adapter: Option<Arc<MemoryEvolutionDSpyAdapter>>` field to `ConsolidationJob`
2. Add `with_dspy()` constructor
3. Update `make_llm_consolidation_decision()` to use adapter when available:
   ```rust
   #[cfg(feature = "python")]
   if let Some(adapter) = &self.evolution_adapter {
       return adapter.consolidate_cluster(cluster).await;
   }
   ```
4. Keep existing `build_cluster_prompt()` and `parse_llm_cluster_response()` as fallback
5. Deprecate but maintain backward compatibility with direct LLM service calls

**Dependencies**: Requires E1 and E2 complete
**Parallel**: Final integration after E1, E2

#### E4: Testing
- Python tests: `test_memory_evolution_module.py`
- Rust integration tests: `memory_evolution_dspy_adapter_test.rs`
- Consolidation decision accuracy tests
- Importance recalibration validation tests

---

## Phase 4: Optimization Pipeline 🔮 FUTURE

### Overview
After all DSPy modules are integrated, optimize prompts using teleprompters.

### Tasks

#### F1: Individual Module Optimization
**Goal**: Optimize each module independently using MIPROv2

**Process**:
1. Create training sets for each module:
   - Reviewer: Intent validation examples with ground truth
   - Semantic: Discourse/contradiction/pragmatics labeled examples
   - Optimizer: Context consolidation quality examples
   - Memory Evolution: Consolidation decision examples with expert labels
2. Define metrics for each module
3. Run MIPROv2 teleprompter
4. Save optimized modules

**Script**: `scripts/optimize_dspy_modules.py`

#### F2: Joint Optimization (GEPA)
**Goal**: Optimize multiple modules together for synergistic improvements

**Approach**:
- Optimize Reviewer + Optimizer jointly (context flow)
- Optimize Semantic analyzers jointly (consistency)
- Optimize Memory Evolution operations jointly

**Script**: `scripts/joint_optimize_dspy.py`

#### F3: A/B Testing Framework
**Goal**: Compare optimized vs baseline modules

**Implementation**:
- Feature flags for baseline vs optimized modules
- Metrics collection for both versions
- Statistical significance testing

#### F4: Prompt Versioning
**Goal**: Track and manage optimized prompt versions

**Storage**: `.claude/dspy-prompts/`
- Version control for optimized modules
- Rollback capability
- Performance comparison dashboard

---

## Parallel Work Streams Summary

### Currently Safe to Parallelize

**Stream C (Optimizer Context Consolidation)**:
- **C1**: OptimizerModule (Python) - ✅ Independent
- **C2**: OptimizerDSpyAdapter (Rust) - ✅ Independent
- **C3**: optimizer.rs integration - ⚠️ Depends on C1+C2
- **C4**: Testing - ⚠️ Depends on C3

**Stream E (Memory Evolution Consolidation)**:
- **E1**: MemoryEvolutionModule (Python) - ✅ Independent
- **E2**: MemoryEvolutionDSpyAdapter (Rust) - ✅ Independent
- **E3**: consolidation.rs integration - ⚠️ Depends on E1+E2
- **E4**: Testing - ⚠️ Depends on E3

### Execution Strategy

**Round 1** (Parallel):
- C1: Create OptimizerModule
- E1: Create MemoryEvolutionModule

**Round 2** (Parallel):
- C2: Create OptimizerDSpyAdapter
- E2: Create MemoryEvolutionDSpyAdapter

**Round 3** (Sequential per stream, streams still parallel):
- C3: Integrate optimizer.rs
- E3: Integrate consolidation.rs

**Round 4** (Parallel):
- C4: Optimizer testing
- E4: Memory evolution testing

**Round 5** (Final):
- Update DSPY_INTEGRATION.md with new modules
- Create PR with all Phase 2-3 work

---

## Dependencies Graph

```
Phase 1 (✅ Complete)
├─ Stream A: Reviewer
│  ├─ ReviewerModule (Python)
│  ├─ ReviewerDSpyAdapter (Rust)
│  └─ reviewer.rs integration
└─ Stream B: Tier 3 Semantic
   ├─ SemanticModule (Python)
   ├─ DSpySemanticBridge (Rust)
   └─ tier3 analyzers integration

Phase 2 (🚧 In Progress)
├─ Stream C: Optimizer Context
│  ├─ C1: OptimizerModule (Python) [Independent]
│  ├─ C2: OptimizerDSpyAdapter (Rust) [Independent]
│  ├─ C3: optimizer.rs integration [Depends: C1, C2]
│  └─ C4: Testing [Depends: C3]
└─ Stream D: Skills Discovery
   └─ Uses Stream C OptimizerModule [Depends: C1]

Phase 3 (🚧 In Progress)
└─ Stream E: Memory Evolution
   ├─ E1: MemoryEvolutionModule (Python) [Independent]
   ├─ E2: MemoryEvolutionDSpyAdapter (Rust) [Independent]
   ├─ E3: consolidation.rs integration [Depends: E1, E2]
   └─ E4: Testing [Depends: E3]

Phase 4 (🔮 Future)
└─ Optimization Pipeline
   ├─ F1: Individual module optimization [Depends: All Phase 2-3]
   ├─ F2: Joint optimization (GEPA) [Depends: F1]
   ├─ F3: A/B testing framework [Depends: F1]
   └─ F4: Prompt versioning [Depends: F1]
```

---

## Benefits Summary

### Optimizer Enhancements
- **Intelligent Context Consolidation**: Semantic understanding of review feedback
- **Adaptive Skill Discovery**: Context-aware skill matching beyond keyword patterns
- **Optimizable Strategies**: Teleprompter-tunable context management

### Memory Evolution Enhancements
- **Semantic Consolidation**: Understanding nuanced differences in similar memories
- **Intelligent Importance Scoring**: Access patterns + semantic value
- **Archival Intelligence**: Preserve valuable old memories, archive noise

### System-Wide Benefits
- **Consistency**: All LLM operations through DSPy framework
- **Optimization**: Systematic prompt improvement via teleprompters
- **Maintainability**: Centralized prompt management
- **Type Safety**: Structured outputs, validated conversions

---

## Timeline Estimate

- **Phase 2 (Optimizer)**: 2-3 days
  - C1: 4 hours
  - C2: 4 hours
  - C3: 4 hours
  - C4: 4 hours

- **Phase 3 (Memory Evolution)**: 2-3 days
  - E1: 4 hours
  - E2: 4 hours
  - E3: 4 hours
  - E4: 4 hours

- **Phase 4 (Optimization)**: 1-2 weeks
  - Training data creation: 3 days
  - Module optimization: 2 days
  - Joint optimization: 2 days
  - A/B testing framework: 2 days
  - Versioning infrastructure: 1 day

**Total**: ~2 weeks for Phases 2-3, additional 1-2 weeks for Phase 4

---

## Success Criteria

### Phase 2 (Optimizer)
- [ ] OptimizerModule with 3 signatures implemented and tested
- [ ] OptimizerDSpyAdapter provides type-safe interface
- [ ] optimizer.rs successfully uses DSPy when available
- [ ] Heuristic fallbacks maintained for backward compatibility
- [ ] All tests passing (Python + Rust)

### Phase 3 (Memory Evolution)
- [ ] MemoryEvolutionModule with 3 signatures implemented and tested
- [ ] MemoryEvolutionDSpyAdapter provides type-safe interface
- [ ] consolidation.rs uses DSPy for LLM decisions
- [ ] Existing decision modes (Heuristic, LlmAlways, etc.) preserved
- [ ] All tests passing (Python + Rust)

### Phase 4 (Optimization)
- [ ] Training datasets created for all modules
- [ ] MIPROv2 optimization demonstrates improvement
- [ ] GEPA joint optimization successful
- [ ] A/B testing shows statistical significance
- [ ] Prompt versioning operational

---

## Risk Mitigation

### Risk: Breaking existing functionality
**Mitigation**:
- Conditional compilation (`#[cfg(feature = "python")]`)
- Maintain heuristic fallbacks
- Comprehensive testing before integration

### Risk: PyO3 API compatibility issues
**Mitigation**:
- Use established patterns from Phase 1
- Test with actual Python environment early
- Document version requirements

### Risk: Context budget overflow with additional modules
**Mitigation**:
- Monitor context usage during development
- Design efficient signatures (minimal I/O)
- Implement lazy loading where appropriate

### Risk: Performance degradation from additional LLM calls
**Mitigation**:
- Cache DSPy predictions where appropriate
- Use async/await for non-blocking calls
- Benchmark before/after integration

---

## Next Steps (Immediate)

**Round 1 - Parallel Implementation**:
1. Start Stream C1: Create `OptimizerModule` (Python)
2. Start Stream E1: Create `MemoryEvolutionModule` (Python)

Both are independent and can be implemented in parallel.
