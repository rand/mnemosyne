# DSPy Integration Architecture

Complete guide to the DSPy integration in Mnemosyne, providing systematic prompt optimization for agents and semantic analysis.

## Overview

This integration replaces direct PyObject calls with a clean adapter pattern using DSPy for:
- **Reviewer Agent**: Intent validation, completeness checking, correctness verification
- **Tier 3 Semantic Analysis**: Discourse, contradictions, pragmatics

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

## Testing

### Python Tests

```bash
# Test DSPy modules directly
pytest src/orchestration/dspy_modules/ -v

# Run specific test file
pytest src/orchestration/dspy_modules/test_semantic_module.py -v
pytest src/orchestration/dspy_modules/test_reviewer_module.py -v
```

### Rust Integration Tests

```bash
# Run all integration tests (requires Python environment)
cargo test --features python -- --ignored

# Run specific test file
cargo test --features python dspy_bridge_integration_test -- --ignored
cargo test --features python reviewer_dspy_adapter_test -- --ignored
cargo test --features python dspy_semantic_bridge_test -- --ignored
```

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

1. **Improvement Guidance**: Implement in ReviewerModule (currently stubbed)
2. **Optimization Pipeline**: Automated teleprompter training
3. **Multi-Agent GEPA**: Joint optimization across all agents
4. **Prompt Versioning**: Track and rollback optimized prompts
5. **A/B Testing**: Compare optimized vs baseline modules

## References

- [DSPy Documentation](https://dspy-docs.vercel.app/)
- [PyO3 Guide](https://pyo3.rs/)
- Reviewer LLM Guide: `docs/guides/llm-reviewer.md`
- ICS Architecture: `docs/ICS_ARCHITECTURE.md`
