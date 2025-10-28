# Privacy-Preserving Evaluation System

**Adaptive context relevance learning for the Optimizer agent**

---

## Overview

Mnemosyne's evaluation system helps the Optimizer agent learn which context (skills, memories, files) is most relevant over time. It tracks implicit feedback signals and adapts relevance scoring at session, project, and global levels.

**Key Privacy Feature**: All evaluation data is stored locally with privacy-preserving design. No telemetry, no network calls, no raw content stored.

---

## Why Evaluation?

Without evaluation, the Optimizer agent uses basic keyword matching to score skill relevance:

```python
# Without evaluation (basic)
score = keyword_overlap(task, skill) + filename_bonus
```

With evaluation, the Optimizer learns from experience:

```python
# With evaluation (learned)
weights = get_learned_weights(session, project, task_type, work_phase)
score = (
    keyword_overlap * weights['keyword_match'] +
    recency * weights['recency'] +
    access_patterns * weights['access_patterns'] +
    historical_success * weights['historical_success']
)
```

**Result**: Better context selection → More relevant skills → Higher quality assistance.

---

## Privacy Design

### Core Guarantees

1. **Local-Only Storage**: All data in `.mnemosyne/project.db` (gitignored)
2. **Hashed Tasks**: SHA256 hash of task descriptions (16 chars only)
3. **Limited Keywords**: Max 10 generic keywords, no sensitive terms
4. **Statistical Features**: Only computed metrics, never raw content
5. **No Network Calls**: Uses existing Anthropic API calls, no separate requests
6. **Graceful Degradation**: System works perfectly when disabled

### What IS Stored

```rust
// Evaluation record
ContextEvaluation {
    id: "uuid",
    session_id: "uuid",
    agent_role: "optimizer",
    context_type: "skill",
    context_id: "rust-async.md",
    task_hash: "a3f8e9d1..." // SHA256, 16 chars
    task_keywords: ["rust", "async", "tokio"], // Max 10, generic only
    task_type: Some(Feature),
    work_phase: Some(Implementation),
    file_types: Some([".rs"]),
    error_context: Some(None),
    // Feedback signals
    was_accessed: true,
    access_count: 3,
    time_to_first_access_ms: Some(5000),
    was_edited: false,
    was_committed: false,
    was_cited_in_response: true,
}

// Statistical features
RelevanceFeatures {
    evaluation_id: "uuid",
    keyword_overlap_score: 0.75, // Jaccard similarity
    recency_days: 7.2,
    access_frequency: 0.5,
    work_phase_match: true,
    task_type_match: true,
    agent_role_affinity: 0.9,
    historical_success_rate: Some(0.85),
    was_useful: true, // Ground truth
}
```

### What IS NOT Stored

```
✗ Raw task descriptions
✗ Full file contents
✗ Actual code snippets
✗ Sensitive variable names
✗ API keys or secrets
✗ Personal identifiable information (PII)
✗ Business logic details
```

**Privacy guarantee**: Only statistical features are persisted. No content reconstruction possible.

---

## Architecture

```
┌─────────────────────────────────────────────────────┐
│ Optimizer Agent (Python)                            │
│                                                      │
│  1. analyze_task()                                  │
│  2. discover_skills()                               │
│  3. score_skill_relevance() ◄─── Learned Weights   │
│  4. record_context_provided()                       │
└───────────────────┬─────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────────────┐
│ Evaluation System (Rust, PyO3)                      │
│                                                      │
│  ┌────────────────────────────────────────────┐    │
│  │ FeedbackCollector                          │    │
│  │ - Record context provided                  │    │
│  │ - Track access, edits, commits             │    │
│  │ - Compute task hash (SHA256, 16 chars)     │    │
│  │ - Extract keywords (max 10, generic)       │    │
│  └────────────────────────────────────────────┘    │
│                    │                                 │
│                    ▼                                 │
│  ┌────────────────────────────────────────────┐    │
│  │ FeatureExtractor                           │    │
│  │ - Compute keyword overlap (Jaccard)        │    │
│  │ - Compute recency, access frequency        │    │
│  │ - Extract contextual features              │    │
│  │ - Determine usefulness (ground truth)      │    │
│  └────────────────────────────────────────────┘    │
│                    │                                 │
│                    ▼                                 │
│  ┌────────────────────────────────────────────┐    │
│  │ RelevanceScorer                            │    │
│  │ - Online learning algorithm                │    │
│  │ - Hierarchical weights (session/project)   │    │
│  │ - Fallback on generic weights              │    │
│  │ - Context-aware (task_type, work_phase)    │    │
│  └────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────────────┐
│ Local SQLite Database                               │
│ .mnemosyne/project.db (gitignored)                  │
│                                                      │
│  - context_evaluations (feedback signals)           │
│  - relevance_features (statistical features)        │
│  - learned_weights (session/project/global)         │
└─────────────────────────────────────────────────────┘
```

---

## Hierarchical Learning

Weights are learned at three levels with different learning rates:

### 1. Session-Level (α=0.3)

**Scope**: Current session only
**Learning rate**: Fast (α=0.3) for immediate adaptation
**Use case**: Rapidly adapt to current task context

```
Session start: Use default weights
Task 1: Rust async programming
  → Provide: rust-async.md, tokio.md, memory-safety.md
  → Feedback: rust-async.md accessed, edited, committed
  → Update: Increase weight for "rust + async + implementation"
Task 2: Rust async debugging
  → Use learned session weights
  → Better skill selection based on recent success
```

### 2. Project-Level (α=0.1)

**Scope**: Current project (git repository)
**Learning rate**: Moderate (α=0.1) for project patterns
**Use case**: Learn project-specific skill preferences

```
Project: Mnemosyne (Rust + Python + Database)
Over weeks:
  → Rust skills consistently useful for implementation
  → Python skills useful for orchestration
  → Database skills useful for debugging
  → Weights reflect project-specific patterns
```

### 3. Global-Level (α=0.03)

**Scope**: All projects (user-wide)
**Learning rate**: Slow (α=0.03) for universal patterns
**Use case**: Learn general skill utility across domains

```
Across all projects:
  → Testing skills consistently useful during debugging
  → API design skills useful during planning
  → Performance skills useful for optimization tasks
  → Weights capture universal patterns
```

### Weight Lookup (Hierarchical Fallback)

Weights are fetched with hierarchical specificity:

```rust
// 1. Most specific: session + task_type + work_phase + error_context
weights = get_weights(
    scope: Session,
    scope_id: "current-session-uuid",
    work_phase: Some(Implementation),
    task_type: Some(Feature),
    error_context: Some(None)
)

// 2. If not found, try: session + task_type + work_phase
weights = get_weights(
    scope: Session,
    work_phase: Some(Implementation),
    task_type: Some(Feature),
    error_context: None
)

// 3. If not found, try: session + work_phase
weights = get_weights(
    scope: Session,
    work_phase: Some(Implementation),
    task_type: None,
    error_context: None
)

// 4. If not found, try: project-level
weights = get_weights(scope: Project, ...)

// 5. If not found, try: global-level
weights = get_weights(scope: Global, ...)

// 6. Fallback: default weights
weights = DEFAULT_WEIGHTS
```

**Result**: Context-aware learning with graceful degradation.

---

## Feedback Signals

### Implicit Signals (Automatic)

The evaluation system tracks implicit signals about context usefulness:

**Access tracking**:
```rust
was_accessed: bool,         // Did agent/user access this context?
access_count: u32,          // How many times?
time_to_first_access_ms: Option<i64>, // How quickly?
total_time_accessed_ms: i64, // How long?
```

**Usage tracking**:
```rust
was_edited: bool,           // Was context file edited?
was_committed: bool,        // Were changes committed?
was_cited_in_response: bool, // Did agent cite this in response?
```

**Task outcome**:
```rust
task_completed: bool,       // Was task completed?
task_success_score: Option<f32>, // How successful? (0.0-1.0)
```

**Usefulness heuristic**:
```rust
// Context is "useful" if:
// 1. Accessed AND (edited OR committed OR cited)
// 2. Accessed multiple times (≥2)
// 3. Explicit positive rating

fn determine_usefulness(eval: &ContextEvaluation) -> bool {
    // Explicit rating overrides
    if let Some(rating) = eval.user_rating {
        return rating > 0;
    }

    // Implicit signals
    let was_used = eval.was_accessed &&
        (eval.was_edited || eval.was_committed || eval.was_cited_in_response);

    let frequently_accessed = eval.access_count >= 2;

    was_used || frequently_accessed
}
```

### Explicit Signals (Optional)

Users can provide explicit feedback:

```python
# Rate context usefulness
feedback_collector.record_user_rating(eval_id, rating=1)  # Useful
feedback_collector.record_user_rating(eval_id, rating=-1) # Not useful
feedback_collector.record_user_rating(eval_id, rating=0)  # Neutral
```

**Note**: Explicit ratings are optional. System works well with implicit signals alone.

---

## Statistical Features

### Feature Extraction

The `FeatureExtractor` computes privacy-preserving statistical features:

**Keyword overlap** (Jaccard similarity):
```rust
// Compute overlap score, discard keywords
let task_keywords = ["rust", "async", "tokio"];
let context_keywords = ["rust", "async", "await"];
let score = jaccard_similarity(task_keywords, context_keywords);
// score = 0.67 (2 common / 3 unique)
// STORED: score (0.67)
// DISCARDED: keywords themselves
```

**Recency features**:
```rust
recency_days: f32,          // Days since context created
access_frequency: f32,      // Accesses per day
last_used_days_ago: Option<f32>, // Days since last access
```

**Contextual match**:
```rust
work_phase_match: bool,     // Does work phase match?
task_type_match: bool,      // Does task type match?
file_type_match: bool,      // Do file types match?
namespace_match: bool,      // Does namespace match?
```

**Agent affinity**:
```rust
agent_role_affinity: f32,   // How well does context suit this agent?
// Example: optimizer + skill = 0.9, executor + file = 0.9
```

**Historical features**:
```rust
historical_success_rate: Option<f32>, // Past success rate (0.0-1.0)
co_occurrence_score: Option<f32>,     // Co-occurrence with useful contexts
```

### Feature Storage

Features are stored in `relevance_features` table:

```sql
CREATE TABLE relevance_features (
    evaluation_id TEXT PRIMARY KEY,
    -- Statistical features (privacy-preserving)
    keyword_overlap_score REAL,
    semantic_similarity REAL,
    recency_days REAL,
    access_frequency REAL,
    last_used_days_ago REAL,
    -- Contextual match
    work_phase_match BOOLEAN,
    task_type_match BOOLEAN,
    agent_role_affinity REAL,
    namespace_match BOOLEAN,
    file_type_match BOOLEAN,
    -- Historical
    historical_success_rate REAL,
    co_occurrence_score REAL,
    -- Ground truth
    was_useful BOOLEAN,
    FOREIGN KEY (evaluation_id) REFERENCES context_evaluations(id)
);
```

**Privacy guarantee**: Only statistical features stored. No raw content.

---

## Online Learning Algorithm

### Weight Updates

When feedback is collected, weights are updated using online learning:

```rust
// Online gradient descent
fn update_weights(
    weights: &mut WeightSet,
    features: &RelevanceFeatures,
    predicted_score: f32,
    ground_truth: bool,
    learning_rate: f32
) {
    let ground_truth_score = if ground_truth { 1.0 } else { 0.0 };
    let error = ground_truth_score - predicted_score;

    // Update each weight proportionally to its feature
    weights.keyword_match += learning_rate * error * features.keyword_overlap_score;
    weights.recency += learning_rate * error * features.recency_days;
    weights.access_patterns += learning_rate * error * features.access_frequency;
    weights.historical_success += learning_rate * error * features.historical_success_rate.unwrap_or(0.0);
    weights.file_type_match += learning_rate * error * (if features.file_type_match { 1.0 } else { 0.0 });

    // Normalize weights to sum to 1.0
    weights.normalize();
}
```

**Learning rates**:
- Session: α=0.3 (fast adaptation)
- Project: α=0.1 (moderate adaptation)
- Global: α=0.03 (slow, stable adaptation)

### Relevance Scoring

When scoring a skill, the Optimizer uses learned weights:

```rust
// Get learned weights (with fallback)
let weights = relevance_scorer.get_weights(
    scope: Session,
    scope_id: session_id,
    work_phase: Some(Implementation),
    task_type: Some(Feature),
    error_context: Some(None)
);

// Compute weighted score
let score = (
    features.keyword_overlap_score * weights.keyword_match +
    features.recency_days * weights.recency +
    features.access_frequency * weights.access_patterns +
    features.historical_success_rate * weights.historical_success +
    (if features.file_type_match { 1.0 } else { 0.0 }) * weights.file_type_match
);
```

**Result**: Context-aware, adaptive scoring that improves over time.

---

## Configuration

### Enable/Disable Evaluation

**Python configuration**:
```python
from mnemosyne_core import OptimizerConfig

# Enable evaluation (default)
config = OptimizerConfig(
    enable_evaluation=True,
    db_path=None  # Use default path
)

# Disable evaluation
config = OptimizerConfig(
    enable_evaluation=False
)
```

**Environment variable**:
```bash
export MNEMOSYNE_DISABLE_EVALUATION=1
```

**Database path**:
```python
# Auto-detected (priority order):
# 1. .mnemosyne/project.db (if exists)
# 2. ~/.local/share/mnemosyne/mnemosyne.db (XDG default)

# Or specify explicitly
config = OptimizerConfig(
    enable_evaluation=True,
    db_path="/path/to/custom.db"
)
```

### Default Weights

Default weights before any learning:

```rust
DEFAULT_WEIGHTS = WeightSet {
    keyword_match: 0.35,      // Keyword overlap
    recency: 0.15,            // Recently created/used
    access_patterns: 0.25,    // Historical access frequency
    historical_success: 0.15, // Past success rate
    file_type_match: 0.10,    // File type match
}
```

**Rationale**: Balanced initial weights. System learns to adjust based on experience.

---

## Examples

### Example 1: Session-Level Learning

```python
# Session start
optimizer = OptimizerAgent(config=OptimizerConfig(enable_evaluation=True))

# Task 1: Implement Rust async function
task_desc = "Implement async HTTP client using tokio"
context = await optimizer.optimize_context(task_desc, current_context)

# Optimizer provides:
# - rust-async.md (relevance: 0.75, default weights)
# - tokio-runtime.md (relevance: 0.70, default weights)
# - http-client.md (relevance: 0.65, default weights)

# User accesses rust-async.md, edits code, commits
feedback_collector.record_context_accessed(eval_id_1)
feedback_collector.record_context_edited(eval_id_1)
feedback_collector.record_context_committed(eval_id_1)

# Task 2: Debug async runtime error
task_desc = "Debug tokio runtime panic in async handler"
context = await optimizer.optimize_context(task_desc, current_context)

# Optimizer now uses learned session weights:
# - rust-async.md (relevance: 0.82, boosted by session learning)
# - tokio-runtime.md (relevance: 0.78, boosted)
# - error-handling.md (relevance: 0.68, default)

# Result: Better skill selection based on recent success
```

### Example 2: Project-Level Learning

```python
# Project: Mnemosyne (Rust + Python + Database)
# Over weeks, patterns emerge:

# Week 1: Rust implementation tasks
# - rust-memory-management.md: accessed 15 times, edited 10 times
# - rust-async.md: accessed 12 times, edited 8 times
# - rust-error-handling.md: accessed 10 times, edited 6 times

# Week 2: Python orchestration tasks
# - python-asyncio.md: accessed 8 times, edited 5 times
# - python-typing.md: accessed 6 times, edited 4 times

# Week 3: Database debugging tasks
# - sql-query-optimization.md: accessed 7 times, edited 3 times
# - database-transactions.md: accessed 5 times, edited 2 times

# Project-level weights learned:
# - For Rust implementation: keyword_match=0.40, historical_success=0.25
# - For Python orchestration: keyword_match=0.35, access_patterns=0.20
# - For database debugging: keyword_match=0.30, recency=0.25

# Result: Project-specific skill preferences encoded in weights
```

### Example 3: Graceful Degradation

```python
# Evaluation disabled
config = OptimizerConfig(enable_evaluation=False)
optimizer = OptimizerAgent(config)

# Task: Implement Rust async function
task_desc = "Implement async HTTP client using tokio"
context = await optimizer.optimize_context(task_desc, current_context)

# Optimizer falls back to basic keyword matching:
# - rust-async.md (relevance: 0.75, keyword match)
# - tokio-runtime.md (relevance: 0.70, keyword match)
# - http-client.md (relevance: 0.65, keyword match)

# No evaluation data collected
# No learned weights used
# No functional loss

# Result: System works perfectly without evaluation
```

---

## Privacy Summary

### Data Flow

```
Task Description
    ↓ (hash: SHA256, 16 chars)
Task Hash (stored)
    ↓
Keywords Extraction (max 10, generic)
    ↓ (compute overlap)
Keyword Overlap Score (stored)
    ↓ (discard keywords)
Keywords (DISCARDED)
    ↓
Statistical Features (stored)
    ↓
Ground Truth: Was Useful? (stored)
    ↓
Weight Update (stored)
```

**Privacy guarantee at each step**:
1. Task hash: Cannot reverse to original description
2. Keywords: Generic only, max 10, discarded after use
3. Statistical features: No raw content, only metrics
4. Ground truth: Boolean signal, no context
5. Weights: Statistical parameters, no content

### What Can Be Inferred?

**From evaluation database**:
```
✓ Task hashes (meaningless without original task)
✓ Generic keywords (broad categories: "rust", "api", "database")
✓ Statistical features (keyword overlap: 0.75, recency: 7.2 days)
✓ Feedback signals (accessed: true, edited: true, useful: true)
✓ Learned weights (keyword_match: 0.40, historical_success: 0.25)
```

**Cannot be inferred**:
```
✗ Original task descriptions
✗ Actual code written
✗ File contents
✗ Sensitive variable names
✗ Business logic
✗ API keys or secrets
✗ Personal information
```

**Conclusion**: Evaluation data is privacy-preserving by design. No content reconstruction possible.

---

## Transparency

### Open Source

All evaluation code is open source:

**Core implementation**:
- [src/evaluation/mod.rs](../src/evaluation/mod.rs) - Module overview
- [src/evaluation/feedback_collector.rs](../src/evaluation/feedback_collector.rs) - Feedback collection
- [src/evaluation/feature_extractor.rs](../src/evaluation/feature_extractor.rs) - Feature extraction
- [src/evaluation/relevance_scorer.rs](../src/evaluation/relevance_scorer.rs) - Relevance scoring

**Python bindings**:
- [src/bindings/evaluation.rs](../src/bindings/evaluation.rs) - PyO3 bindings

**Integration**:
- [src/orchestration/agents/optimizer.py](../src/orchestration/agents/optimizer.py) - Optimizer agent

**Tests**:
- [tests/unit/test_evaluation.rs](../tests/unit/test_evaluation.rs) - Unit tests

**Audit**: Anyone can review the code to verify privacy guarantees.

### No Telemetry

**Mnemosyne never collects telemetry.** The evaluation system is the only component that stores behavioral data, and it does so:

- **Locally**: `.mnemosyne/project.db` (gitignored)
- **Privacy-preserving**: Statistical features only
- **User-controlled**: Easy enable/disable
- **Transparent**: Open source

---

## Getting Help

### Questions

- **Privacy concerns**: See [docs/PRIVACY.md](docs/PRIVACY.md)
- **Technical questions**: GitHub Discussions
- **Bug reports**: GitHub Issues

### Debugging

**Inspect evaluation data**:
```bash
# Open database
sqlite3 .mnemosyne/project.db

# List tables
.tables

# Inspect evaluations
SELECT session_id, context_type, was_accessed, was_useful
FROM context_evaluations
ORDER BY context_provided_at DESC
LIMIT 10;

# Inspect learned weights
SELECT scope, scope_id, work_phase, task_type,
       keyword_match, historical_success
FROM learned_weights
ORDER BY last_updated DESC
LIMIT 10;
```

**Check optimizer status**:
```python
status = optimizer.get_status()
print(f"Evaluation enabled: {status['evaluation_enabled']}")
print(f"Loaded skills: {status['loaded_skills']}")
```

---

## References

- **Privacy Policy**: [docs/PRIVACY.md](docs/PRIVACY.md)
- **Architecture**: [ARCHITECTURE.md](ARCHITECTURE.md)
- **Orchestration**: [ORCHESTRATION.md](ORCHESTRATION.md)
- **Optimizer Agent**: [src/orchestration/agents/optimizer.py](src/orchestration/agents/optimizer.py)

---

**Summary**: Mnemosyne's evaluation system learns context relevance over time using privacy-preserving statistical features and online learning. All data is stored locally with strong privacy guarantees. System works perfectly when disabled.
