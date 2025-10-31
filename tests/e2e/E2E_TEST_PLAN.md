# Expanded E2E Test Suite Plan - Mnemosyne v2.2+

**Status**: Infrastructure Complete (Phase 0) ✅
**Next**: Phase 1 - User Journey Tests (12 tests)
**Goal**: Comprehensive coverage across all capabilities and user types

---

## Executive Summary

### Coverage Goals
- **Total Tests**: 80 (20 existing + 60 new)
- **Baseline Tests** (Real LLM API): 25 tests
- **Regression Tests** (Mocked LLM): 55 tests
- **User Personas**: 8 different user types
- **Feature Coverage**: >80% of core functionality

### Cost Budget
- **Baseline Suite**: ~$2-5 per run (75-125 API calls)
- **Regression Suite**: $0 (mocked responses)
- **Annual Budget**: ~$100-260 (weekly baseline + releases)

### Timeline
- **Phase 0**: Infrastructure ✅ (Complete)
- **Phase 1-3**: Test Development (6 weeks)
- **Phase 4**: Baseline Validation (1 week)
- **Phase 5**: CI/CD Integration (1 week)
- **Total**: 8 weeks

---

## Infrastructure (Phase 0) ✅ Complete

### Test Libraries Created

#### 1. `lib/llm_mocking.sh`
**Purpose**: Mock LLM responses for regression testing

**Functions**:
- `is_baseline_mode()` - Check if using real API or mocks
- `mock_enrichment_response(content, importance, type)` - Mock memory enrichment
- `mock_consolidation_response(mem1, mem2)` - Mock consolidation decision
- `mock_reviewer_antipatterns(code)` - Mock anti-pattern detection
- `mock_reviewer_requirements(intent)` - Mock requirement extraction
- `mock_importance_recalibration(id, current, links, accesses)` - Mock scoring
- `mock_semantic_highlighting_tier3(text)` - Mock Tier 3 highlighting
- `mock_skill_relevance(task, skill)` - Mock context optimization
- `generate_mock_embedding(content)` - Deterministic embeddings
- `mnemosyne_with_mocking(bin, db, args...)` - Wrapper with auto-mocking

**Usage**:
```bash
export MNEMOSYNE_TEST_MODE=baseline   # Real LLM API
export MNEMOSYNE_TEST_MODE=regression # Mocked responses
```

#### 2. `lib/baseline_validators.sh`
**Purpose**: Validate real LLM response quality

**Validators**:
- `validate_enrichment_quality(json)` - Summary ≥20 chars, confidence ≥0.7
- `validate_consolidation_quality(json)` - Rationale ≥30 chars, valid decision
- `validate_requirements_quality(json)` - Non-empty, reasonable length
- `validate_antipattern_quality(json)` - Proper structure, issue details
- `validate_importance_quality(json)` - Score in range 1-10
- `validate_semantic_highlighting_quality(json)` - Tier 3 features present
- `validate_baseline_run(results_dir)` - Aggregate validation

**Example**:
```bash
validate_enrichment_quality "$enrichment_response"
# ✓ Summary length: 145 chars
# ✓ Keywords count: 5
# ✓ Confidence: 0.92
# ✓ Embedding dimension: 1536
# PASS Enrichment quality validated
```

#### 3. `lib/assertions.sh`
**Purpose**: Rich test assertions beyond basic pass/fail

**Categories** (40+ functions):
- **JSON**: `assert_valid_json`, `assert_json_field_exists`, `assert_json_array_length`
- **String**: `assert_contains`, `assert_matches`, `assert_equals`, `assert_not_empty`
- **Numeric**: `assert_greater_than`, `assert_in_range`, `assert_numeric_equals`
- **File**: `assert_file_exists`, `assert_file_contains`, `assert_file_size`
- **Command**: `assert_command_succeeds`, `assert_output_contains`, `assert_completes_within`
- **Database**: `assert_memory_exists`, `assert_memory_count`, `assert_link_exists`
- **Timing**: `time_operation(threshold_ms, command, args...)`

**Example**:
```bash
assert_memory_exists "$TEST_DB" "$memory_id"
assert_json_array_length "$response" ".keywords" 5
assert_completes_within 2 "$BIN" search "architecture"  # 2s timeout
```

#### 4. `lib/personas.sh`
**Purpose**: Setup/teardown for 8 user personas

**Personas**:
1. **Solo Developer** - Personal projects, individual CLI usage
2. **Team Lead** - Multi-namespace coordination, team workflows
3. **AI Agent (Single)** - Session-based context preservation
4. **Multi-Agent System** - 4 agents (Orchestrator, Optimizer, Reviewer, Executor)
5. **Python Developer** - PyO3 bindings integration
6. **API Consumer** - HTTP API with auto-start server
7. **ICS Power User** - Editor workflows with CRDT collaboration
8. **Dashboard Observer** - Monitoring and event streaming

**Usage**:
```bash
TEST_DATA=$(setup_persona "solo_developer" "my_test")
# ... run tests ...
cleanup_persona "solo_developer" "$TEST_DATA"
```

#### 5. `lib/data_generators.sh`
**Purpose**: Generate realistic test data

**Generators**:
- `generate_memory_content(type)` - Insight, architecture, decision, task, reference
- `generate_memory_batch(count, namespace, db)` - Batch creation
- `generate_keyword_memories(keywords, namespace, db)` - Search testing
- `generate_duplicate_memories(base, count, namespace, db)` - Consolidation testing
- `generate_namespace_hierarchy(base, depth, db)` - Multi-level namespaces
- `generate_memory_links(db, count)` - Graph structures
- `generate_realistic_project(name, db)` - Complete project setup
- `generate_stress_data(db, size)` - 100-10,000 memories

**Example**:
```bash
generate_realistic_project "myapp" "$TEST_DB"
# ✓ Realistic project 'myapp' generated
# - 1 architecture decision
# - 1 team decision
# - 3 active tasks
# - 5 insights
# - 1 reference
```

---

## Test Organization

### Directory Structure
```
tests/e2e/
├── lib/                                # Infrastructure (✅ Complete)
│   ├── common.sh                       # Existing common helpers
│   ├── llm_mocking.sh                  # LLM response mocking
│   ├── baseline_validators.sh          # Quality validation
│   ├── assertions.sh                   # Rich assertions
│   ├── personas.sh                     # User persona helpers
│   └── data_generators.sh              # Test data generation
├── fixtures/                           # Test data (Phase 0.4)
│   ├── memories/                       # Sample memories JSON
│   ├── projects/                       # Sample project structures
│   ├── workflows/                      # Workflow definitions
│   └── llm_responses/                  # Mocked LLM responses
├── results/                            # Test results (gitignored)
│   ├── baseline/                       # Baseline run results
│   └── regression/                     # Regression run results
├── E2E_TEST_PLAN.md                    # This file
├── README.md                           # Existing test documentation
├── run_all.sh                          # Test runner (updated)
├── run_baseline.sh                     # Baseline suite only (new)
├── run_regression.sh                   # Regression suite only (new)
│
├── solo_dev_*.sh                       # Phase 1.1 (4 tests)
├── team_lead_*.sh                      # Phase 1.2 (4 tests)
├── power_user_*.sh                     # Phase 1.3 (4 tests)
├── memory_types_*.sh                   # Phase 2.1 (5 tests)
├── namespaces_*.sh                     # Phase 2.2 (5 tests)
├── storage_*.sh                        # Phase 2.3 (3 tests)
├── llm_config_*.sh                     # Phase 2.4 (3 tests)
├── orchestration_*.sh                  # Phase 3.1 (4 tests)
├── work_queue_*.sh                     # Phase 3.2 (4 tests)
├── mcp_*.sh                            # Phase 4.1 (4 tests)
├── python_*.sh                         # Phase 4.2 (3 tests)
├── api_*.sh                            # Phase 4.3 (3 tests)
├── ics_editor_*.sh                     # Phase 5.1 (4 tests)
├── ics_collab_*.sh                     # Phase 5.2 (2 tests)
├── ics_panels_*.sh                     # Phase 5.3 (2 tests)
├── evolution_consolidate_*.sh          # Phase 6.1 (2 tests)
├── evolution_importance.sh             # Phase 6.2 (1 test)
├── evolution_decay.sh                  # Phase 6.2 (1 test)
├── evolution_archival.sh               # Phase 6.3 (1 test)
└── evolution_supersede.sh              # Phase 6.3 (1 test)
```

---

## Baseline Test Suite (25 tests with real LLM)

### Cost Analysis
- **Per Test**: 3-5 API calls avg
- **Total**: 75-125 API calls per baseline run
- **Tokens**: 150k-250k tokens (avg 2k per call)
- **Cost**: ~$2-5 per run (Claude Sonnet 3.5 pricing)

### Execution Schedule
- **Weekly**: Automated baseline run (52/year = ~$100-260/year)
- **Pre-Release**: Manual baseline validation
- **LLM Prompt Changes**: Validation run
- **CI/CD**: Regression only (free)

### Baseline Test List

| # | Test Name | Category | LLM Features Validated |
|---|-----------|----------|----------------------|
| 1 | `solo_dev_1_onboarding.sh` | User Journey | Memory enrichment (summary, keywords, embeddings) |
| 2 | `team_lead_2_coordinate_work.sh` | User Journey | Multi-memory enrichment, namespace handling |
| 3 | `power_user_1_advanced_search.sh` | User Journey | Vector search, semantic ranking |
| 4 | `memory_types_1_insight.sh` | Feature | Insight-specific enrichment patterns |
| 5 | `memory_types_2_architecture.sh` | Feature | Architecture decision analysis |
| 6 | `namespaces_3_session.sh` | Feature | Session→project consolidation |
| 7 | `storage_2_libsql.sh` | Feature | LibSQL vector embeddings |
| 8 | `llm_config_1_enrichment_enabled.sh` | Feature | Full LLM enrichment pipeline |
| 9 | `orchestration_2_parallel.sh` | Orchestration | Parallel context optimization |
| 10 | `orchestration_3_review_loop.sh` | Orchestration | Reviewer quality gates (anti-patterns, requirements) |
| 11 | `work_queue_3_deadlock.sh` | Orchestration | Deadlock resolution with state management |
| 12 | `mcp_1_ooda_observe.sh` | Integration | MCP recall/list with embeddings |
| 13 | `mcp_3_ooda_decide.sh` | Integration | MCP decision creation with enrichment |
| 14 | `python_1_storage_api.sh` | Integration | PyStorage with enrichment |
| 15 | `api_1_sse_events.sh` | Integration | API SSE streaming with real events |
| 16 | `ics_editor_4_semantic_highlighting.sh` | ICS | Tier 3 semantic highlighting (LLM-powered) |
| 17 | `ics_collab_1_multi_user.sh` | ICS | CRDT synchronization with real conflicts |
| 18 | `evolution_consolidate_1_auto.sh` | Evolution | Auto-detect duplicates with LLM analysis |
| 19 | `evolution_consolidate_2_manual.sh` | Evolution | Manual consolidation with LLM suggestions |
| 20 | `evolution_importance.sh` | Evolution | Graph-based importance with real data |
| 21 | `evolution_archival.sh` | Evolution | Auto-archive with real importance scores |
| 22-25 | *(Reserved for future baseline tests)* | - | TBD |

### Baseline Quality Thresholds

**Enrichment**:
- Summary: ≥20 chars, <500 chars
- Keywords: 3-10 items
- Confidence: ≥0.7
- Embedding: 1536 dimensions (if present)

**Consolidation**:
- Confidence: ≥0.6
- Rationale: ≥30 chars
- Consolidated content: ≥20 chars (if recommended)

**Requirements**:
- Count: ≥0 (may be empty for simple tasks)
- Length: ≥10 chars each
- No empty requirements

**Anti-Patterns**:
- Proper boolean: true/false
- Issues array: type and severity for each
- Consistency: found=true ↔ issues non-empty

**Importance**:
- Score range: 1-10
- Factors: link_count, access_count, graph_centrality

**Semantic Highlighting**:
- Tier: 3 (analytical)
- Features: discourse_markers, pragmatics, contradictions
- Processing time: <10,000ms

---

## Regression Test Suite (55 tests with mocked LLM)

### Characteristics
- **Cost**: $0 (no API calls)
- **Speed**: Fast (<30 min full suite)
- **Isolation**: Fully offline, reproducible
- **CI/CD**: Runs on every commit

### Mocking Strategy

**Deterministic Responses**:
- Embeddings: Generated from SHA256 hash of content
- Summaries: First 100 chars + "..."
- Keywords: Simple word extraction (4-15 char words, top 5)
- Confidence: Based on content length and complexity
- Consolidation: Jaccard similarity on words (threshold 0.3)

**Quality Simulation**:
- Simulates realistic LLM behavior patterns
- Maintains test validity for logic/flow validation
- Detects regressions in non-LLM code paths
- Fast feedback loop for developers

### Regression Test Categories

| Category | Count | Mocked Features |
|----------|-------|----------------|
| User Journeys | 9 | Enrichment, consolidation |
| Feature Permutations | 11 | Enrichment, namespaces |
| Orchestration | 4 | Context optimization |
| Integration | 6 | MCP, Python, API (no LLM paths) |
| ICS | 6 | Editing, panels (Tier 1-2 only) |
| Evolution | 2 | Decay, supersede (no LLM) |

---

## Test Implementation Phases

### Phase 1: User Journey Tests (12 tests) - Weeks 1-2
**Goal**: Complete workflows for 3 persona types

#### Phase 1.1: Solo Developer (4 tests)
1. ✅ **`solo_dev_1_onboarding.sh`** [BASELINE]
   - **Purpose**: First-time setup with real enrichment
   - **LLM Features**: Memory enrichment (summary, keywords, embeddings)
   - **Coverage**: CLI onboarding, database init, basic CRUD
   - **Validation**: Enrichment quality, memory retrieval
   - **Estimated Effort**: 3 hours

2. **`solo_dev_2_daily_workflow.sh`** [REGRESSION]
   - **Purpose**: Typical daily usage pattern
   - **Coverage**: Store decisions, recall context, export notes
   - **Mocked**: Enrichment responses
   - **Estimated Effort**: 2 hours

3. **`solo_dev_3_project_evolution.sh`** [REGRESSION]
   - **Purpose**: Project lifecycle management
   - **Coverage**: Consolidate memories, archive old content
   - **Mocked**: Consolidation recommendations
   - **Estimated Effort**: 2 hours

4. **`solo_dev_4_cross_project.sh`** [REGRESSION]
   - **Purpose**: Multi-project workflow
   - **Coverage**: Namespace switching, global vs project memories
   - **No LLM**: Namespace isolation testing
   - **Estimated Effort**: 2 hours

#### Phase 1.2: Team Lead (4 tests)
1. **`team_lead_1_setup_namespaces.sh`** [REGRESSION]
   - **Purpose**: Team structure setup
   - **Coverage**: Configure team/project/feature namespaces
   - **No LLM**: Namespace creation
   - **Estimated Effort**: 2 hours

2. ✅ **`team_lead_2_coordinate_work.sh`** [BASELINE]
   - **Purpose**: Work assignment and tracking
   - **LLM Features**: Multi-memory enrichment, work item analysis
   - **Coverage**: Assign tasks, track progress via memories
   - **Validation**: Batch enrichment quality
   - **Estimated Effort**: 3 hours

3. **`team_lead_3_consolidate_team_knowledge.sh`** [REGRESSION]
   - **Purpose**: Knowledge management
   - **Coverage**: Merge duplicate team decisions
   - **Mocked**: Consolidation suggestions
   - **Estimated Effort**: 2 hours

4. **`team_lead_4_generate_reports.sh`** [REGRESSION]
   - **Purpose**: Reporting and visualization
   - **Coverage**: Export memories, graph visualization
   - **No LLM**: Export formats, graph generation
   - **Estimated Effort**: 2 hours

#### Phase 1.3: Power User (4 tests)
1. ✅ **`power_user_1_advanced_search.sh`** [BASELINE]
   - **Purpose**: Complex search scenarios
   - **LLM Features**: Vector search, semantic ranking
   - **Coverage**: Hybrid search (keyword + graph + vector)
   - **Validation**: Search relevance, ranking quality
   - **Estimated Effort**: 3 hours

2. **`power_user_2_bulk_operations.sh`** [REGRESSION]
   - **Purpose**: Large-scale operations
   - **Coverage**: Import/export 1000+ memories, batch updates
   - **No LLM**: Bulk processing performance
   - **Estimated Effort**: 2 hours

3. **`power_user_3_custom_workflows.sh`** [REGRESSION]
   - **Purpose**: Scripted automation
   - **Coverage**: Shell scripting with mnemosyne CLI
   - **No LLM**: CLI composability
   - **Estimated Effort**: 2 hours

4. **`power_user_4_performance_optimization.sh`** [REGRESSION]
   - **Purpose**: Performance tuning
   - **Coverage**: Large datasets (10k+ memories), caching
   - **No LLM**: Query performance, index optimization
   - **Estimated Effort**: 3 hours

**Phase 1 Total**: 12 tests, 28 hours estimated

---

### Phase 2: Feature Permutation Tests (16 tests) - Weeks 3-4
**Goal**: Test all feature combinations

#### Phase 2.1: Memory Type Permutations (5 tests)
1. ✅ **`memory_types_1_insight.sh`** [BASELINE]
   - **LLM**: Insight-specific enrichment patterns
   - **Estimated Effort**: 2 hours

2. ✅ **`memory_types_2_architecture.sh`** [BASELINE]
   - **LLM**: Architecture decision analysis (rationale, trade-offs)
   - **Estimated Effort**: 2 hours

3. **`memory_types_3_decision.sh`** [REGRESSION]
   - **Mocked**: Decision log enrichment
   - **Estimated Effort**: 2 hours

4. **`memory_types_4_task.sh`** [REGRESSION]
   - **No LLM**: Task tracking with dependencies
   - **Estimated Effort**: 2 hours

5. **`memory_types_5_reference.sh`** [REGRESSION]
   - **Mocked**: Reference material categorization
   - **Estimated Effort**: 2 hours

#### Phase 2.2: Namespace Permutations (5 tests)
1. **`namespaces_1_global.sh`** [REGRESSION]
   - **Coverage**: Global user preferences
   - **Estimated Effort**: 1.5 hours

2. **`namespaces_2_project.sh`** [REGRESSION]
   - **Coverage**: Project-specific memories
   - **Estimated Effort**: 1.5 hours

3. ✅ **`namespaces_3_session.sh`** [BASELINE]
   - **LLM**: Session→project consolidation with analysis
   - **Estimated Effort**: 3 hours

4. **`namespaces_4_hierarchical.sh`** [REGRESSION]
   - **Coverage**: session→project→global propagation
   - **Estimated Effort**: 2 hours

5. **`namespaces_5_isolation.sh`** [REGRESSION]
   - **Coverage**: Cross-namespace access controls
   - **Estimated Effort**: 2 hours

#### Phase 2.3: Storage Backend Permutations (3 tests)
1. **`storage_1_local_sqlite.sh`** [REGRESSION]
   - **Coverage**: Local SQLite operations
   - **Estimated Effort**: 1.5 hours

2. ✅ **`storage_2_libsql.sh`** [BASELINE]
   - **LLM**: LibSQL vector search with real embeddings
   - **Estimated Effort**: 3 hours

3. **`storage_3_turso_cloud.sh`** [REGRESSION]
   - **Coverage**: Turso cloud sync (requires credentials)
   - **Mocked**: Cloud sync operations
   - **Estimated Effort**: 2 hours

#### Phase 2.4: LLM Configuration Permutations (3 tests)
1. ✅ **`llm_config_1_enrichment_enabled.sh`** [BASELINE]
   - **LLM**: Full enrichment pipeline validation
   - **Estimated Effort**: 2 hours

2. **`llm_config_2_enrichment_disabled.sh`** [REGRESSION]
   - **Coverage**: Graceful degradation when LLM unavailable
   - **Estimated Effort**: 1.5 hours

3. **`llm_config_3_partial_features.sh`** [REGRESSION]
   - **Coverage**: Consolidation only, no enrichment
   - **Estimated Effort**: 1.5 hours

**Phase 2 Total**: 16 tests, 30 hours estimated

---

### Phase 3: Multi-Agent Orchestration Tests (8 tests) - Weeks 5-6
**Goal**: Test agent coordination patterns

#### Phase 3.1: Agent Interaction Patterns (4 tests)
1. **`orchestration_1_sequential.sh`** [REGRESSION]
   - **Coverage**: Sequential agent execution (Orchestrator→Executor)
   - **Estimated Effort**: 2 hours

2. ✅ **`orchestration_2_parallel.sh`** [BASELINE]
   - **LLM**: Parallel work with real context optimization
   - **Estimated Effort**: 4 hours

3. ✅ **`orchestration_3_review_loop.sh`** [BASELINE]
   - **LLM**: Executor→Reviewer feedback loop with validation
   - **Estimated Effort**: 4 hours

4. **`orchestration_4_context_optimization.sh`** [REGRESSION]
   - **Mocked**: Optimizer context management
   - **Estimated Effort**: 2 hours

#### Phase 3.2: Work Queue Scenarios (4 tests)
1. **`work_queue_1_dependencies.sh`** [REGRESSION]
   - **Coverage**: Dependency-aware scheduling
   - **Estimated Effort**: 2 hours

2. **`work_queue_2_priorities.sh`** [REGRESSION]
   - **Coverage**: Priority-based execution order
   - **Estimated Effort**: 2 hours

3. ✅ **`work_queue_3_deadlock.sh`** [BASELINE]
   - **LLM**: Deadlock resolution with real state management
   - **Estimated Effort**: 3 hours

4. **`work_queue_4_branch_isolation.sh`** [REGRESSION]
   - **Coverage**: Git branch isolation for parallel work
   - **Estimated Effort**: 2 hours

**Phase 3 Total**: 8 tests, 21 hours estimated

---

### Phase 4-6: Integration, ICS, Evolution Tests
*(Remaining 34 tests, details in separate sections)*

**Phase 4**: Integration Tests (10 tests, 20 hours)
**Phase 5**: ICS Tests (8 tests, 16 hours)
**Phase 6**: Evolution Tests (6 tests, 12 hours)

**Phases 4-6 Total**: 34 tests, 48 hours estimated

---

## Test Template

### Standard Test Structure
```bash
#!/usr/bin/env bash
set -uo pipefail

# Test: <Test Name>
#
# Purpose: <What this test validates>
# Persona: <User type>
# Mode: [BASELINE|REGRESSION]
# LLM Features: <Features validated> (if baseline)
#
# Success Criteria:
# - <Criterion 1>
# - <Criterion 2>

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source test infrastructure
source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/llm_mocking.sh"
source "$SCRIPT_DIR/lib/baseline_validators.sh"
source "$SCRIPT_DIR/lib/assertions.sh"
source "$SCRIPT_DIR/lib/personas.sh"
source "$SCRIPT_DIR/lib/data_generators.sh"

# Test configuration
TEST_NAME="<test_name>"
PERSONA="<persona_type>"

section "E2E Test: <Test Description>"

# Setup persona environment
print_cyan "[SETUP] Initializing $PERSONA persona..."
TEST_DATA=$(setup_persona "$PERSONA" "$TEST_NAME")

# Extract database path (format depends on persona)
TEST_DB="${TEST_DATA%:*}"  # Or just $TEST_DATA for simple personas

# ===================================================================
# TEST EXECUTION
# ===================================================================

section "Test 1: <First Test Case>"

# Perform action
OUTPUT1=$("$BIN" command --args 2>&1)

# Validate result
if assert_contains "$OUTPUT1" "expected string"; then
    pass "Test 1 succeeded"
else
    fail "Test 1 failed"
fi

# For baseline mode only: Validate LLM response quality
if is_baseline_mode; then
    section "[BASELINE] Validating LLM Response Quality"

    ENRICHMENT=$(echo "$OUTPUT1" | jq '.enrichment')
    if validate_enrichment_quality "$ENRICHMENT"; then
        pass "LLM response quality validated"
    else
        fail "LLM response quality below threshold"
    fi
fi

# ===================================================================
# CLEANUP
# ===================================================================

section "[CLEANUP] Tearing down test environment"
cleanup_persona "$PERSONA" "$TEST_DATA"

# ===================================================================
# SUMMARY
# ===================================================================

section "Test Summary"
echo "Test: <Test Name>"
echo "Mode: $(is_baseline_mode && echo 'BASELINE (Real LLM)' || echo 'REGRESSION (Mocked)')"
echo "Passed: $PASSED"
echo "Failed: $FAILED"

if [ "$FAILED" -eq 0 ]; then
    print_green "✓ ALL TESTS PASSED"
    exit 0
else
    print_red "✗ SOME TESTS FAILED"
    exit 1
fi
```

---

## Running Tests

### Individual Test
```bash
# Regression mode (default)
./tests/e2e/solo_dev_1_onboarding.sh

# Baseline mode (real LLM)
export MNEMOSYNE_TEST_MODE=baseline
./tests/e2e/solo_dev_1_onboarding.sh
```

### Test Suites
```bash
# All tests (baseline mode)
export MNEMOSYNE_TEST_MODE=baseline
./tests/e2e/run_all.sh

# Baseline suite only
./tests/e2e/run_baseline.sh

# Regression suite only
./tests/e2e/run_regression.sh --fast

# Specific category
./tests/e2e/run_all.sh --category user_journey
./tests/e2e/run_all.sh --category orchestration
```

### CI/CD
```bash
# Fast regression (for CI)
export MNEMOSYNE_TEST_MODE=regression
./tests/e2e/run_regression.sh --parallel

# Weekly baseline
export MNEMOSYNE_TEST_MODE=baseline
export ANTHROPIC_API_KEY=$SECRET
./tests/e2e/run_baseline.sh --report results/baseline-$(date +%Y%m%d).json
```

---

## Success Metrics

### Quantitative
- **Coverage**: >80% of core features tested
- **Pass Rate**: 100% for baseline and regression
- **Flakiness**: <5% (max 4 flaky tests)
- **Performance**:
  - Baseline suite: <2 hours
  - Regression suite: <30 minutes
  - Individual test: <5 minutes avg

### Qualitative
- Each user persona has complete workflows
- All feature combinations tested
- Real LLM quality validated
- Error paths covered
- Performance benchmarks established
- Clear documentation

---

## Next Steps

### Immediate (This Week)
1. ✅ Complete Phase 0 infrastructure
2. Create test runners (run_baseline.sh, run_regression.sh)
3. Begin Phase 1.1: Solo Developer tests (4 tests)

### Short-term (Next 2 Weeks)
4. Complete Phase 1: User Journey Tests (12 tests)
5. Begin Phase 2: Feature Permutation Tests (16 tests)

### Medium-term (Weeks 5-6)
6. Complete Phases 2-3 (24 tests total)
7. Begin Phase 4-6: Integration/ICS/Evolution (34 tests)

### Long-term (Weeks 7-8)
8. Baseline validation runs (3x to establish reliability)
9. Create regression mocks from baseline responses
10. CI/CD integration (GitHub Actions)
11. Documentation and gap analysis

---

## Appendix

### Environment Variables
- `MNEMOSYNE_TEST_MODE`: `baseline` (real API) or `regression` (mocked, default)
- `ANTHROPIC_API_KEY`: API key for baseline runs
- `DATABASE_URL`: Test database path
- `MNEMOSYNE_API_PORT`: API server port (default: 3000)

### Test Data Locations
- **Test Databases**: `/tmp/mnemosyne_<persona>_<test>_<timestamp>.db`
- **Results**: `tests/e2e/results/<mode>/<date>/`
- **Fixtures**: `tests/e2e/fixtures/`
- **Logs**: `tests/e2e/logs/` (gitignored)

### References
- [E2E Test README](README.md) - Existing test documentation
- [Common Test Library](lib/common.sh) - Existing helpers
- [Mnemosyne Documentation](../../README.md) - Project overview
