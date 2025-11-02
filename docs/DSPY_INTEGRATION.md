# DSPy Integration Architecture

Complete guide to the DSPy integration in Mnemosyne, providing systematic prompt optimization for agents and semantic analysis.

## Overview

This integration replaces direct PyObject calls with a clean adapter pattern using DSPy for:
- **Reviewer Agent**: Intent validation, completeness checking, correctness verification ✅ **Phase 1 Complete**
- **Tier 3 Semantic Analysis**: Discourse, contradictions, pragmatics ✅ **Phase 1 Complete**
- **Optimizer Agent**: Context consolidation, skills discovery, context budget optimization ✅ **Phase 2 Complete**
- **Memory Evolution**: Cluster consolidation, importance recalibration, archival detection ✅ **Phase 3 Complete**

**Status**: Phases 1-3 implemented and tested. Phase 4 (optimization pipeline) planned.

## Architecture

### Layer 1: Python DSPy Modules

**Location**: `src/orchestration/dspy_modules/`

Two core modules implement DSPy signatures with ChainOfThought:

#### ReviewerModule (`reviewer_module.py`)
```python
class ReviewerModule(dspy.Module):
    def extract_requirements(user_intent, context) -> requirements
    def validate_intent(user_intent, implementation, execution_context) -> (satisfied, issues)
    def verify_completeness(requirements, implementation, execution_context) -> (complete, issues)
    def verify_correctness(implementation, execution_context) -> (correct, issues)
```

#### SemanticModule (`semantic_module.py`)
```python
class SemanticModule(dspy.Module):
    def analyze_discourse(text) -> (segments, coherence_score)
    def detect_contradictions(text) -> contradictions
    def extract_pragmatics(text) -> elements
```

#### OptimizerModule (`optimizer_module.py`)
```python
class OptimizerModule(dspy.Module):
    def consolidate_context(
        original_intent, execution_summaries, review_feedback,
        suggested_tests, review_attempt, consolidation_mode
    ) -> (consolidated_content, key_issues, strategic_guidance, estimated_tokens)

    def discover_skills_for_task(
        task_description, available_skills, max_skills, current_context_usage
    ) -> (selected_skills, relevance_scores, reasoning)

    def optimize_context_allocation(
        current_usage, loaded_resources, target_pct, work_priority
    ) -> (unload_skills, unload_memory_ids, optimization_rationale)
```

**Consolidation Modes**:
- `detailed` (attempt 1): Preserve all context, full reasoning
- `summary` (attempts 2-3): Key issues + patterns
- `compressed` (attempt 4+): Critical blockers only

#### MemoryEvolutionModule (`memory_evolution_module.py`)
```python
class MemoryEvolutionModule(dspy.Module):
    def consolidate_cluster(
        cluster_memories, avg_similarity, similarity_scores
    ) -> (action, primary_memory_id, secondary_memory_ids, rationale,
          preserved_content, confidence)

    def recalibrate_importance(
        memory_id, memory_summary, memory_type, current_importance,
        access_count, days_since_created, days_since_accessed,
        linked_memories_count, namespace
    ) -> (new_importance, adjustment_reason, recommended_action)

    def detect_archival_candidates(
        memories, archival_threshold_days, min_importance
    ) -> (archive_ids, keep_ids, rationale)
```

**Consolidation Actions**:
- `MERGE`: Combine into single comprehensive memory
- `SUPERSEDE`: Newer memory replaces older
- `KEEP`: Maintain all memories separately

**Key Features**:
- ChainOfThought for transparency
- Structured JSON outputs
- Optimizable via teleprompters (MIPROv2, GEPA)

### Layer 2: Generic Bridge (DSpyBridge)

**Location**: `src/orchestration/dspy_bridge.rs`

Generic Rust ↔ Python bridge with:
- `call_agent_module(agent_name, inputs: HashMap<String, Value>) -> HashMap<String, Value>`
- Module registration and listing
- Hot reloading support
- GIL management and async execution (spawn_blocking)

### Layer 3: Type-Safe Adapters

Two specialized adapters provide strongly-typed interfaces:

#### ReviewerDSpyAdapter (`orchestration/actors/reviewer_dspy_adapter.rs`)
```rust
impl ReviewerDSpyAdapter {
    async fn extract_requirements(&self, intent: &str, context: Option<&str>)
        -> Result<Vec<String>>

    async fn semantic_intent_check(&self, intent: &str, implementation: &str,
        execution_memories: Vec<Value>) -> Result<(bool, Vec<String>)>

    async fn verify_completeness(&self, requirements: &[String],
        implementation: &str, execution_memories: Vec<Value>)
        -> Result<(bool, Vec<String>)>

    async fn verify_correctness(&self, implementation: &str,
        execution_memories: Vec<Value>) -> Result<(bool, Vec<String>)>
}
```

#### DSpySemanticBridge (`ics/semantic_highlighter/tier3_analytical/dspy_integration.rs`)
```rust
impl DSpySemanticBridge {
    async fn analyze_discourse(&self, text: &str)
        -> Result<Vec<DiscourseSegment>>

    async fn detect_contradictions(&self, text: &str)
        -> Result<Vec<Contradiction>>

    async fn extract_pragmatics(&self, text: &str)
        -> Result<Vec<PragmaticElement>>
}
```

#### OptimizerDSpyAdapter (`orchestration/actors/optimizer_dspy_adapter.rs`)
```rust
impl OptimizerDSpyAdapter {
    async fn consolidate_context(
        &self,
        original_intent: &str,
        execution_summaries: Vec<String>,
        review_feedback: Vec<String>,
        suggested_tests: Vec<String>,
        review_attempt: u32,
        consolidation_mode: &str,
    ) -> Result<ConsolidatedContext>

    async fn discover_skills(
        &self,
        task_description: &str,
        available_skills: Vec<SkillMetadata>,
        max_skills: usize,
        current_context_usage: f64,
    ) -> Result<SkillDiscoveryResult>

    async fn optimize_context_budget(
        &self,
        current_usage: ContextUsage,
        loaded_resources: LoadedResources,
        target_pct: f64,
        work_priority: u8,
    ) -> Result<OptimizationPlan>
}
```

**Type Definitions**:
```rust
pub struct ConsolidatedContext {
    pub consolidated_content: String,
    pub key_issues: Vec<String>,
    pub strategic_guidance: String,
    pub estimated_tokens: usize,
}

pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub keywords: Vec<String>,
    pub domains: Vec<String>,
}

pub struct SkillDiscoveryResult {
    pub selected_skills: Vec<String>,
    pub relevance_scores: HashMap<String, f64>,
    pub reasoning: String,
}

pub struct ContextUsage {
    pub critical_pct: f64,
    pub skills_pct: f64,
    pub project_pct: f64,
    pub general_pct: f64,
    pub total_pct: f64,
}

pub struct OptimizationPlan {
    pub unload_skills: Vec<String>,
    pub unload_memory_ids: Vec<String>,
    pub optimization_rationale: String,
}
```

#### MemoryEvolutionDSpyAdapter (`evolution/memory_evolution_dspy_adapter.rs`)
```rust
impl MemoryEvolutionDSpyAdapter {
    async fn consolidate_cluster(
        &self,
        cluster: &MemoryCluster,
    ) -> Result<ConsolidationDecision>

    async fn recalibrate_importance(
        &self,
        memory: &MemoryNote,
    ) -> Result<ImportanceRecalibration>

    async fn detect_archival_candidates(
        &self,
        memories: &[MemoryNote],
        config: &ArchivalConfig,
    ) -> Result<ArchivalDecisions>
}
```

**Type Definitions**:
```rust
pub struct MemoryCluster {
    pub memories: Vec<MemoryNote>,
    pub similarity_scores: Vec<(MemoryId, MemoryId, f64)>,
    pub avg_similarity: f64,
}

pub enum ConsolidationAction {
    Merge,
    Supersede,
    Keep,
}

pub struct ConsolidationDecision {
    pub action: ConsolidationAction,
    pub primary_memory_id: Option<MemoryId>,
    pub secondary_memory_ids: Vec<MemoryId>,
    pub rationale: String,
    pub preserved_content: Option<String>,
    pub confidence: f64,
}

pub struct ImportanceRecalibration {
    pub new_importance: u8,
    pub adjustment_reason: String,
    pub recommended_action: RecommendedAction,
}

pub enum RecommendedAction {
    Keep,
    Archive,
    Delete,
}

pub struct ArchivalConfig {
    pub archival_threshold_days: i64,
    pub min_importance: u8,
}

pub struct ArchivalDecisions {
    pub archive_ids: Vec<MemoryId>,
    pub keep_ids: Vec<MemoryId>,
    pub rationale: String,
}
```

### Layer 4: Integration Points

**Reviewer Actor** (`orchestration/actors/reviewer.rs`):
```rust
impl ReviewerState {
    // New DSPy-based registration
    pub fn register_dspy_bridge(&mut self, bridge: Arc<DSpyBridge>)

    // Deprecated PyObject-based (backward compatibility)
    #[deprecated]
    pub fn register_py_reviewer(&mut self, py_reviewer: Arc<PyObject>)
}
```

**Tier 3 Analyzers**:
```rust
// discourse.rs, contradictions.rs, pragmatics.rs
impl Analyzer {
    pub fn with_dspy(llm_service: Arc<LlmService>,
        dspy_bridge: Arc<DSpySemanticBridge>) -> Self

    async fn analyze/detect/extract(&self, text: &str) -> Result<T> {
        if let Some(bridge) = &self.dspy_bridge {
            // Use DSPy
        } else {
            // Fallback error
        }
    }
}
```

**Optimizer Actor** (`orchestration/actors/optimizer.rs`):
```rust
impl OptimizerState {
    #[cfg(feature = "python")]
    pub fn register_optimizer_adapter(&mut self, adapter: Arc<OptimizerDSpyAdapter>)

    async fn consolidate_work_item_context(...) -> Result<(MemoryId, usize)> {
        #[cfg(feature = "python")]
        if let Some(adapter) = &state.optimizer_adapter {
            // Determine mode based on review attempt
            let consolidation_mode = match review_attempt {
                1 => "detailed",
                2..=3 => "summary",
                _ => "compressed",
            };

            // Try DSPy consolidation
            match adapter.consolidate_context(...).await {
                Ok(consolidated) => {
                    // Create memory with DSPy content
                    // Add "dspy_enhanced" tag
                    return Ok((memory_id, consolidated.estimated_tokens));
                }
                Err(e) => {
                    // Fall back to heuristics
                }
            }
        }
        // Heuristic-based consolidation (fallback)
    }
}
```

**Memory Evolution** (`evolution/consolidation.rs`):
```rust
impl ConsolidationJob {
    #[cfg(feature = "python")]
    pub fn with_dspy(
        storage: Arc<LibsqlStorage>,
        evolution_adapter: Arc<MemoryEvolutionDSpyAdapter>,
    ) -> Self

    #[cfg(feature = "python")]
    pub fn with_dspy_and_config(
        storage: Arc<LibsqlStorage>,
        evolution_adapter: Arc<MemoryEvolutionDSpyAdapter>,
        config: ConsolidationConfig,
    ) -> Self

    async fn make_llm_consolidation_decision(
        &self,
        cluster: &MemoryCluster,
    ) -> Result<ConsolidationDecision, JobError> {
        #[cfg(feature = "python")]
        if let Some(adapter) = &self.evolution_adapter {
            // Try DSPy consolidation
            match adapter.consolidate_cluster(cluster).await {
                Ok(dspy_decision) => {
                    // Convert to ConsolidationDecision
                    // Include confidence in rationale
                    return Ok(decision);
                }
                Err(e) => {
                    // Fall through to LLM fallback
                }
            }
        }
        // Fallback to direct LLM service
    }
}
```

## Data Flow

### Reviewer Validation Flow
```
User Intent
    ↓
ReviewerState::extract_requirements_from_intent()
    ↓
ReviewerDSpyAdapter::extract_requirements()
    ↓
DSpyBridge::call_agent_module("Reviewer", inputs)
    ↓
[Python GIL] ReviewerModule.extract_requirements()
    ↓
DSPy ChainOfThought → Claude API
    ↓
JSON Response {"requirements": [...]}
    ↓
Rust Vec<String>
```

### Semantic Analysis Flow
```
Text Content
    ↓
DiscourseAnalyzer::analyze() [with DSPy]
    ↓
DSpySemanticBridge::analyze_discourse()
    ↓
DSpyBridge::call_agent_module("Semantic", inputs)
    ↓
[Python GIL] SemanticModule.analyze_discourse()
    ↓
DSPy ChainOfThought → Claude API
    ↓
JSON Response {"segments": [...], "coherence_score": 0.8}
    ↓
Rust Vec<DiscourseSegment>
```

### Optimizer Context Consolidation Flow
```
Work Context (intent, summaries, feedback, review_attempt)
    ↓
OptimizerState::consolidate_work_item_context()
    ↓
Determine mode (detailed/summary/compressed)
    ↓
OptimizerDSpyAdapter::consolidate_context()
    ↓
DSpyBridge::call_agent_module("optimizer", inputs)
    ↓
[Python GIL] OptimizerModule.consolidate_context()
    ↓
DSPy ChainOfThought → Claude API
    ↓
JSON Response {
    "consolidated_content": "...",
    "key_issues": [...],
    "strategic_guidance": "...",
    "estimated_tokens": 1500
}
    ↓
Rust ConsolidatedContext
    ↓
Create memory with "dspy_enhanced" tag
```

### Memory Evolution Consolidation Flow
```
Memory Cluster (similar memories)
    ↓
ConsolidationJob::make_llm_consolidation_decision()
    ↓
MemoryEvolutionDSpyAdapter::consolidate_cluster()
    ↓
DSpyBridge::call_agent_module("memory_evolution", inputs)
    ↓
[Python GIL] MemoryEvolutionModule.consolidate_cluster()
    ↓
DSPy ChainOfThought → Claude API
    ↓
JSON Response {
    "action": "MERGE",
    "primary_memory_id": "mem-123",
    "rationale": "...",
    "confidence": 0.92
}
    ↓
Rust ConsolidationDecision
    ↓
Execute action (merge/supersede/keep)
```

## Benefits

### Type Safety
- **Before**: `PyObject.call_method1()` returns untyped PyObject
- **After**: Strongly-typed methods with Rust types

### Error Handling
- **Before**: Manual PyErr handling and retry macros
- **After**: Centralized error handling in bridge/adapter

### Maintainability
- **Before**: Python interop scattered across business logic
- **After**: Changes localized to adapters

### Optimization
- **Before**: Static prompts, no systematic optimization
- **After**: DSPy teleprompters can optimize all modules

### Testability
- **Before**: Hard to mock Python objects
- **After**: Adapters can be easily mocked

## Usage

### Initializing DSPy Integration

```rust
use mnemosyne_core::orchestration::dspy_service::DSpyService;
use mnemosyne_core::orchestration::dspy_bridge::DSpyBridge;
use mnemosyne_core::orchestration::actors::reviewer_dspy_adapter::ReviewerDSpyAdapter;

// Create DSPy service
let dspy_service = DSpyService::new().await?;

// Create generic bridge
let bridge = Arc::new(DSpyBridge::new(Arc::new(Mutex::new(
    dspy_service.into_py_object()
))));

// Create type-safe adapter
let reviewer_adapter = Arc::new(ReviewerDSpyAdapter::new(Arc::clone(&bridge)));

// Register with reviewer
reviewer_state.register_dspy_bridge(Arc::clone(&bridge));
```

### Using Reviewer Operations

```rust
// Extract requirements
let requirements = reviewer_adapter
    .extract_requirements(
        "Implement user authentication",
        Some("REST API with JWT tokens")
    )
    .await?;

// Validate intent
let (satisfied, issues) = reviewer_adapter
    .semantic_intent_check(
        "Add caching",
        "Implemented Redis caching layer",
        execution_memories
    )
    .await?;

// Check completeness
let (complete, issues) = reviewer_adapter
    .verify_completeness(
        &requirements,
        "Implementation details...",
        execution_memories
    )
    .await?;

// Verify correctness
let (correct, issues) = reviewer_adapter
    .verify_correctness(
        "Implementation code...",
        execution_memories
    )
    .await?;
```

### Using Semantic Analysis

```rust
use mnemosyne_core::ics::semantic_highlighter::tier3_analytical::dspy_integration::DSpySemanticBridge;

// Create semantic bridge
let semantic_bridge = Arc::new(DSpySemanticBridge::new(dspy_service));

// Create analyzer with DSPy
let discourse = DiscourseAnalyzer::with_dspy(llm_service, semantic_bridge);

// Analyze discourse
let segments = discourse.analyze("Text to analyze").await?;

// Detect contradictions
let contradictions = contradiction_detector
    .with_dspy(llm_service, semantic_bridge)
    .detect("Text with contradictions")
    .await?;

// Extract pragmatics
let elements = pragmatics_analyzer
    .with_dspy(llm_service, semantic_bridge)
    .analyze("Text with implied meanings")
    .await?;
```

### Using Optimizer Operations

```rust
use mnemosyne_core::orchestration::actors::optimizer_dspy_adapter::OptimizerDSpyAdapter;

// Create optimizer adapter
let optimizer_adapter = Arc::new(OptimizerDSpyAdapter::new(bridge));

// Register with optimizer state
optimizer_state.register_optimizer_adapter(Arc::clone(&optimizer_adapter));

// Consolidate context (automatic mode selection)
let consolidated = optimizer_adapter
    .consolidate_context(
        "Implement authentication",
        vec!["Created JWT module".to_string()],
        vec!["Missing token refresh".to_string()],
        vec!["Test expiration".to_string()],
        1, // First attempt -> "detailed" mode
        "detailed",
    )
    .await?;

println!("Consolidated: {}", consolidated.consolidated_content);
println!("Key issues: {:?}", consolidated.key_issues);
println!("Strategic guidance: {}", consolidated.strategic_guidance);
println!("Estimated tokens: {}", consolidated.estimated_tokens);

// Discover relevant skills
use mnemosyne_core::orchestration::actors::optimizer_dspy_adapter::SkillMetadata;

let skills = vec![
    SkillMetadata {
        name: "rust-async".to_string(),
        description: "Async Rust programming".to_string(),
        keywords: vec!["async".to_string(), "tokio".to_string()],
        domains: vec!["rust".to_string()],
    },
    SkillMetadata {
        name: "database-postgres".to_string(),
        description: "PostgreSQL database".to_string(),
        keywords: vec!["postgres".to_string(), "sql".to_string()],
        domains: vec!["database".to_string()],
    },
];

let discovery = optimizer_adapter
    .discover_skills(
        "Build async REST API with database",
        skills,
        3,
        0.5, // Current context usage
    )
    .await?;

println!("Selected skills: {:?}", discovery.selected_skills);
println!("Reasoning: {}", discovery.reasoning);

// Optimize context budget
use mnemosyne_core::orchestration::actors::optimizer_dspy_adapter::{
    ContextUsage, LoadedResources
};

let usage = ContextUsage {
    critical_pct: 0.40,
    skills_pct: 0.30,
    project_pct: 0.20,
    general_pct: 0.10,
    total_pct: 1.0,
};

let resources = LoadedResources {
    skill_names: vec!["skill-1".to_string(), "skill-2".to_string()],
    memory_ids: vec!["mem-1".to_string(), "mem-2".to_string()],
    memory_summaries: vec!["Summary 1".to_string(), "Summary 2".to_string()],
};

let plan = optimizer_adapter
    .optimize_context_budget(usage, resources, 0.75, 8)
    .await?;

println!("Unload skills: {:?}", plan.unload_skills);
println!("Unload memories: {:?}", plan.unload_memory_ids);
println!("Rationale: {}", plan.optimization_rationale);
```

### Using Memory Evolution

```rust
use mnemosyne_core::evolution::memory_evolution_dspy_adapter::{
    MemoryEvolutionDSpyAdapter, MemoryCluster, ArchivalConfig
};

// Create evolution adapter
let evolution_adapter = Arc::new(MemoryEvolutionDSpyAdapter::new(bridge));

// Create consolidation job with DSPy
let consolidation_job = ConsolidationJob::with_dspy(
    storage,
    Arc::clone(&evolution_adapter),
);

// Consolidate memory cluster
let cluster = MemoryCluster {
    memories: vec![memory1, memory2],
    similarity_scores: vec![(mem1.id, mem2.id, 0.92)],
    avg_similarity: 0.92,
};

let decision = evolution_adapter
    .consolidate_cluster(&cluster)
    .await?;

match decision.action {
    ConsolidationAction::Merge => {
        println!("Merging memories: {}", decision.rationale);
        // Execute merge
    }
    ConsolidationAction::Supersede => {
        println!("Superseding memory: {}", decision.rationale);
        // Execute supersede
    }
    ConsolidationAction::Keep => {
        println!("Keeping separate: {}", decision.rationale);
        // Keep separate
    }
}

println!("Confidence: {:.1}%", decision.confidence * 100.0);

// Recalibrate importance
let recalibration = evolution_adapter
    .recalibrate_importance(&memory)
    .await?;

println!("Old importance: {}", memory.importance);
println!("New importance: {}", recalibration.new_importance);
println!("Reason: {}", recalibration.adjustment_reason);

match recalibration.recommended_action {
    RecommendedAction::Keep => println!("Keep memory"),
    RecommendedAction::Archive => println!("Archive memory"),
    RecommendedAction::Delete => println!("Delete memory"),
}

// Detect archival candidates
let config = ArchivalConfig {
    archival_threshold_days: 90,
    min_importance: 8,
};

let archival_decisions = evolution_adapter
    .detect_archival_candidates(&memories, &config)
    .await?;

println!("Archive: {:?}", archival_decisions.archive_ids);
println!("Keep: {:?}", archival_decisions.keep_ids);
println!("Rationale: {}", archival_decisions.rationale);
```

## Testing

### Python Tests

```bash
# Test all DSPy modules
pytest src/orchestration/dspy_modules/ -v

# Run specific test files
pytest src/orchestration/dspy_modules/test_semantic_module.py -v
pytest src/orchestration/dspy_modules/test_reviewer_module.py -v
pytest src/orchestration/dspy_modules/test_optimizer_module.py -v
pytest src/orchestration/dspy_modules/test_memory_evolution_module.py -v
```

**Test Coverage**:
- **ReviewerModule**: Requirements extraction, intent validation, completeness, correctness
- **SemanticModule**: Discourse analysis, contradiction detection, pragmatics extraction
- **OptimizerModule**: Context consolidation (3 modes), skills discovery, budget optimization
- **MemoryEvolutionModule**: Cluster consolidation, importance recalibration, archival detection

### Rust Integration Tests

```bash
# Run all integration tests (requires Python environment)
cargo test --features python -- --ignored

# Run specific adapter tests
cargo test --features python dspy_bridge_integration_test -- --ignored
cargo test --features python reviewer_dspy_adapter_test -- --ignored
cargo test --features python dspy_semantic_bridge_test -- --ignored
cargo test --features python optimizer_dspy_adapter_test -- --ignored
cargo test --features python memory_evolution_dspy_adapter_test -- --ignored
```

**Test Coverage**:
- **ReviewerDSpyAdapter**: Full workflow validation, concurrent operations
- **DSpySemanticBridge**: Type conversion, error handling, concurrent operations
- **OptimizerDSpyAdapter**: All 3 consolidation modes, skills discovery, budget optimization, concurrent ops
- **MemoryEvolutionDSpyAdapter**: Cluster consolidation, importance recalibration, archival detection, concurrent ops

All Rust tests are marked `#[ignore]` and require:
- Python environment with DSPy
- `ANTHROPIC_API_KEY` environment variable
- `python` feature flag enabled

## Optimization

### Using Teleprompters

DSPy modules can be optimized using teleprompters like MIPROv2 or GEPA:

```python
from dspy.teleprompt import MIPROv2
from mnemosyne.orchestration.dspy_modules.reviewer_module import ReviewerModule

# Define metric
def review_quality(example, pred, trace=None):
    # Evaluate quality of review
    return score

# Optimize module
teleprompter = MIPROv2(metric=review_quality, num_candidates=10)
optimized_reviewer = teleprompter.compile(ReviewerModule(), trainset=examples)

# Save optimized module
optimized_reviewer.save("optimized_reviewer.json")
```

### Joint Optimization (GEPA)

Multiple modules can be optimized jointly:

```python
from dspy.teleprompt import GEPA

# Optimize Reviewer and Semantic together
modules = {
    "reviewer": ReviewerModule(),
    "semantic": SemanticModule()
}

optimized_modules = GEPA(modules=modules, trainset=examples)
```

## Migration Guide

### From Direct PyObject Calls

**Before**:
```rust
let py_reviewer = state.py_reviewer.clone();
retry_llm_operation!(&config, "operation", {
    Python::with_gil(|py| -> PyResult<T> {
        let reviewer = py_reviewer.as_ref().unwrap();
        let result = reviewer.call_method1(py, "method", (args,))?;
        result.extract(py)
    })
})
```

**After**:
```rust
let adapter = state.reviewer_adapter.as_ref().unwrap();
adapter.method(args).await?
```

### From Pattern Matching to DSPy

**Before** (pattern matching):
```rust
if content.contains("INTENT NOT SATISFIED") {
    issues.push("Intent not satisfied".to_string());
}
```

**After** (semantic understanding):
```rust
let (satisfied, issues) = adapter
    .semantic_intent_check(intent, implementation, context)
    .await?;
```

### From Heuristic Context Consolidation to DSPy

**Before** (heuristic-based):
```rust
fn consolidate_context(...) -> String {
    let mut consolidated = String::new();
    consolidated.push_str("## Original Intent\n");
    consolidated.push_str(original_intent);
    consolidated.push_str("\n\n## Execution Summary\n");
    for summary in execution_summaries {
        consolidated.push_str(&format!("- {}\n", summary));
    }
    // Manual string concatenation...
    consolidated
}
```

**After** (DSPy-based with automatic mode selection):
```rust
async fn consolidate_context(...) -> Result<ConsolidatedContext> {
    #[cfg(feature = "python")]
    if let Some(adapter) = &state.optimizer_adapter {
        let mode = match review_attempt {
            1 => "detailed",
            2..=3 => "summary",
            _ => "compressed",
        };

        let consolidated = adapter
            .consolidate_context(
                original_intent,
                execution_summaries,
                review_feedback,
                suggested_tests,
                review_attempt,
                mode,
            )
            .await?;

        // Returns structured ConsolidatedContext with:
        // - consolidated_content
        // - key_issues (Vec<String>)
        // - strategic_guidance
        // - estimated_tokens
        return Ok(consolidated);
    }
    // Fallback to heuristics
}
```

### From Direct LLM Calls to DSPy Memory Evolution

**Before** (direct LLM prompt):
```rust
async fn should_consolidate(&self, cluster: &MemoryCluster) -> Result<bool> {
    let prompt = format!(
        "Should these memories be consolidated?\n{}",
        cluster.memories.iter()
            .map(|m| format!("- {}", m.summary))
            .collect::<Vec<_>>()
            .join("\n")
    );

    let response = self.llm.generate(prompt).await?;
    Ok(response.contains("yes") || response.contains("consolidate"))
}
```

**After** (DSPy with structured decisions):
```rust
async fn make_consolidation_decision(
    &self,
    cluster: &MemoryCluster,
) -> Result<ConsolidationDecision> {
    #[cfg(feature = "python")]
    if let Some(adapter) = &self.evolution_adapter {
        let decision = adapter.consolidate_cluster(cluster).await?;

        // Returns structured ConsolidationDecision with:
        // - action: Merge | Supersede | Keep
        // - primary_memory_id
        // - secondary_memory_ids
        // - rationale (ChainOfThought reasoning)
        // - confidence (0.0-1.0)
        return Ok(decision);
    }
    // Fallback to direct LLM
}
```

## Configuration

### Feature Flags

- `python`: Enables Python integration and DSPy modules
- Compile without: `cargo build` (no Python dependency)
- Compile with: `cargo build --features python`

### Environment Variables

- `ANTHROPIC_API_KEY`: Required for DSPy Claude API calls
- `MNEMOSYNE_DSPY_MODEL`: Override default model (default: claude-3-5-sonnet-20241022)
- `MNEMOSYNE_DSPY_CACHE_DIR`: Cache directory for optimized modules

## Performance

### GIL Management

All Python calls use `tokio::spawn_blocking` to avoid blocking the async runtime:

```rust
tokio::task::spawn_blocking(move || {
    Python::with_gil(|py| {
        // Python operations
    })
}).await?
```

### Caching

- DSPy automatically caches identical prompts
- Semantic analysis results cached by content hash
- Optimized modules saved to disk for reuse

## Troubleshooting

### Python Module Not Found

```
Error: Failed to get semantic module: 'SemanticModule' not registered
```

**Solution**: Ensure DSPy service initialized with all modules:
```rust
let dspy_service = DSpyService::new().await?;
```

### GIL Deadlock

```
Error: Tokio spawn_blocking failed
```

**Solution**: Never call async Rust from within Python GIL:
```rust
// Wrong
Python::with_gil(|py| {
    some_async_function().await  // Deadlock!
});

// Correct
let result = tokio::spawn_blocking(move || {
    Python::with_gil(|py| {
        // Sync Python code only
    })
}).await?;
```

### Type Conversion Errors

```
Error: JSON parse error: expected struct `Vec<Value>`
```

**Solution**: Use proper JSON conversion:
```rust
let json_value = serde_json::to_value(&data)?;
```

## Future Work

### Phase 4: Optimization Pipeline (Planned)

1. **Training Data Collection**:
   - Collect real-world examples from Optimizer and Memory Evolution operations
   - Build labeled datasets for teleprompter training
   - Define quality metrics for each module

2. **MIPROv2 Optimization**:
   - Optimize OptimizerModule for context consolidation quality
   - Optimize MemoryEvolutionModule for consolidation accuracy
   - Run teleprompter training with collected datasets

3. **GEPA Joint Optimization**:
   - Jointly optimize all four modules (Reviewer, Semantic, Optimizer, Memory Evolution)
   - Coordinate prompts across agent boundaries
   - Maintain consistency in decision-making

4. **Prompt Versioning**:
   - Version control for optimized modules
   - Rollback mechanism for prompt changes
   - A/B testing framework for comparing versions

5. **Additional Integrations**:
   - Executor agent DSPy module for task decomposition
   - Orchestrator agent DSPy module for coordination
   - Skills discovery DSPy module for context optimization

### Completed Work

- ✅ **Phase 1**: Reviewer + Tier 3 Semantic (Oct 28-29, 2025)
  - ReviewerModule: 4 signatures, 598 LOC Python
  - SemanticModule: 3 signatures, 338 LOC Python
  - Type-safe Rust adapters integrated
  - Comprehensive Python + Rust tests

- ✅ **Phase 2**: Optimizer Agent DSPy integration (Oct 30-31, 2025)
  - OptimizerModule: 3 signatures, 253 LOC Python
  - OptimizerDSpyAdapter: 380 LOC Rust
  - Integrated with optimizer.rs
  - Progressive consolidation modes (detailed/summary/compressed)
  - Skills discovery and context budget optimization

- ✅ **Phase 3**: Memory Evolution DSPy integration (Oct 30-31, 2025)
  - MemoryEvolutionModule: 3 signatures, 281 LOC Python
  - MemoryEvolutionDSpyAdapter: 402 LOC Rust
  - Integrated with consolidation.rs
  - Cluster consolidation, importance recalibration, archival detection

- ✅ Comprehensive test coverage (Python + Rust)
- ✅ Progressive consolidation modes
- ✅ Type-safe Rust adapters
- ✅ Graceful fallback mechanisms

**Total Implementation**: ~1,470 LOC Python, ~1,056 LOC Rust adapters, 13 DSPy signatures, 660 tests passing

### What's Next (Phase 4)

**Phase 4: Optimization Pipeline** - Planned for next sprint

Priority tasks:
1. **Training Data Collection** (P0)
   - Collect 20-30 Optimizer consolidation examples
   - Collect 20-30 Memory Evolution decision examples
   - Label with quality scores and success criteria

2. **Performance Baseline Benchmarking** (P0)
   - Measure latency for all 4 modules
   - Track token usage per operation
   - Calculate cost per operation
   - Compare with manual prompt baselines

3. **MIPROv2 Optimization** (P1)
   - Optimize ReviewerModule prompts
   - Optimize OptimizerModule prompts
   - Optimize MemoryEvolutionModule prompts
   - Validate improvements against baselines

4. **GEPA Joint Optimization** (P2)
   - Optimize all modules together for synergistic improvements
   - Coordinate prompts across agent boundaries
   - Maintain consistency in decision-making

5. **A/B Testing Framework** (P2)
   - Feature flags for baseline vs optimized modules
   - Metrics collection and comparison
   - Statistical significance testing

6. **Prompt Versioning** (P3)
   - Version control for optimized modules (.claude/dspy-prompts/)
   - Rollback capability
   - Performance comparison dashboard

**Estimated Timeline**: 1.5-2 weeks

See `docs/DSPY_INTEGRATION_PLAN.md` for detailed Phase 4 roadmap.

## References

- [DSPy Documentation](https://dspy-docs.vercel.app/)
- [PyO3 Guide](https://pyo3.rs/)
- Reviewer LLM Guide: `docs/guides/llm-reviewer.md`
- ICS Architecture: `docs/ICS_ARCHITECTURE.md`
