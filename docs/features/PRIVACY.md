# Privacy Policy - Mnemosyne Evaluation System

**Last Updated**: October 28, 2025

---

## Overview

Mnemosyne's evaluation system helps the Optimizer agent learn which context (skills, memories, files) is most relevant over time. **Privacy is a fundamental design constraint**, not an afterthought.

This document provides formal privacy guarantees for all data collected by the evaluation system.

---

## Core Privacy Principles

1. **Local-Only Storage**: All evaluation data is stored locally on your machine
2. **No Network Transmission**: No data is sent to external services beyond standard Anthropic API calls
3. **Privacy-Preserving Features**: Only statistical features stored, never raw content
4. **User Control**: Full control over evaluation data with simple enable/disable
5. **Graceful Degradation**: System works perfectly with evaluation disabled

---

## What Data Is Collected

### Task Metadata (Privacy-Preserving)

When the Optimizer agent provides context for a task, the following metadata is collected:

```
✓ Task hash (SHA256, first 16 chars only)
✓ Generic keywords (max 10, e.g., "rust", "api", "database")
✓ Task type classification (feature/bugfix/refactor/test/documentation)
✓ Work phase (planning/implementation/debugging/testing)
✓ File type patterns (.rs, .py, .md)
✓ Generic technology names (rust, tokio, postgres)
✓ Error context type (compilation/runtime/test_failure/none)
```

**What is NOT stored**:
```
✗ Raw task descriptions
✗ Full file contents
✗ Actual code snippets
✗ Sensitive variable names or values
✗ API keys or secrets
✗ Personal identifiable information (PII)
✗ Business logic details
```

### Feedback Signals (Behavioral)

The system tracks implicit feedback about provided context:

```
✓ Was context accessed? (boolean)
✓ Access count (integer)
✓ Time to first access (milliseconds)
✓ Was context edited? (boolean)
✓ Was context committed? (boolean)
✓ Was context cited in agent response? (boolean)
```

These signals help determine if context was useful without storing what you did with it.

### Statistical Features (Computed Metrics)

Only computed statistical features are stored:

```
✓ Keyword overlap score (0.0-1.0, Jaccard similarity)
✓ Recency (days since context created)
✓ Access frequency (accesses per day)
✓ Historical success rate (0.0-1.0)
✓ Co-occurrence score (0.0-1.0)
```

**Privacy guarantee**: Features are statistical aggregates, not raw data.

---

## Data Storage

### Location

All evaluation data is stored in a local SQLite database:

```
~/.local/share/mnemosyne/mnemosyne.db  (XDG default)
  OR
.mnemosyne/project.db                   (project-specific)
```

**No cloud storage. No remote databases. No telemetry.**

### Schema

Evaluation data is stored in two tables:

**context_evaluations**:
- Evaluation ID (UUID)
- Session ID (UUID)
- Agent role (optimizer/executor/reviewer/orchestrator)
- Context type (skill/memory/file)
- Context ID (file path or memory ID)
- Task hash (16 chars, SHA256)
- Task keywords (JSON array, max 10)
- Feedback signals (booleans, integers, timestamps)
- Task metadata (task_type, work_phase, error_context)

**relevance_features**:
- Evaluation ID (foreign key)
- Statistical features (keyword_overlap_score, recency_days, etc.)
- Ground truth label (was_useful: boolean)

### Gitignore Protection

The `.mnemosyne/` directory is automatically gitignored to prevent accidental commits:

```gitignore
# Mnemosyne local database (never commit)
.mnemosyne/
```

---

## Privacy Protections

### 1. Hashing Task Descriptions

Task descriptions are hashed using SHA256, and only the first 16 characters are stored:

```rust
// Example
task_description = "Implement authentication with JWT"
hash = sha256(task_description) = "a3f8e9d1..." (64 chars)
stored = "a3f8e9d1..." (16 chars only)
```

**Rationale**: Hash provides uniqueness for deduplication without revealing content.

**Collision risk**: With 16 hex chars (2^64 possibilities), collision probability is negligible for typical workloads (<10^6 tasks).

### 2. Limited Keyword Extraction

Generic keywords are extracted from task descriptions, but:

- **Max 10 keywords** per task
- **Generic terms only**: technology names, file types, broad categories
- **Sensitive terms filtered**: passwords, keys, secrets, tokens, credentials
- **Used transiently**: Keywords used for overlap calculation, then only scores persist

**Example**:
```
Task: "Add JWT authentication to /api/auth endpoint using bcrypt"
Keywords extracted: ["jwt", "authentication", "api", "auth", "endpoint", "bcrypt"]
Keywords stored: ["jwt", "authentication", "api", "bcrypt"] (4 keywords)
Keyword overlap score: 0.75 (stored)
Keywords themselves: [DISCARDED after feature computation]
```

### 3. Statistical Features Only

Only computed statistical features are persisted, never raw content:

**Stored**:
```json
{
  "keyword_overlap_score": 0.75,
  "recency_days": 7.2,
  "access_frequency": 0.5,
  "was_useful": true
}
```

**NOT stored**:
```json
{
  "task_content": "...",
  "file_content": "...",
  "full_keywords": [...]
}
```

### 4. Local-Only Processing

**No network calls** are made by the evaluation system:

- Feature extraction: local computation
- Weight updates: local database writes
- Relevance scoring: local weight lookups

**Anthropic API calls**: The Optimizer agent uses Claude to analyze tasks, but this happens via **existing** Anthropic API integration (same as all Mnemosyne LLM operations). No additional network calls for evaluation.

### 5. Graceful Degradation

If evaluation is disabled, the Optimizer agent works perfectly:

```python
# With evaluation enabled
config = OptimizerConfig(enable_evaluation=True)
# Uses learned weights, collects feedback

# With evaluation disabled
config = OptimizerConfig(enable_evaluation=False)
# Falls back to keyword matching, no data collection
```

**No functional loss. No errors. No warnings.**

---

## Data Retention

### Automatic Cleanup

Evaluation data is subject to automatic cleanup:

- **Session data**: Retained for 30 days
- **Project data**: Retained for 180 days
- **Global data**: Retained for 365 days
- **Unused evaluations**: Archived after 90 days of inactivity

### Manual Cleanup

You can manually delete evaluation data:

```bash
# Delete all evaluation data
rm -f ~/.local/share/mnemosyne/mnemosyne.db

# Delete project-specific evaluation data
rm -f .mnemosyne/project.db
```

**Note**: Deleting evaluation data does **not** affect Mnemosyne's core memory system. Only learned relevance weights are lost (system falls back to basic keyword matching).

---

## User Control

### Disabling Evaluation

**Option 1**: Configuration (Python)

```python
from mnemosyne_core import OptimizerConfig

config = OptimizerConfig(
    enable_evaluation=False  # Disable evaluation system
)
```

**Option 2**: Environment Variable

```bash
export MNEMOSYNE_DISABLE_EVALUATION=1
```

**Option 3**: Database Deletion

```bash
# Remove evaluation database (will be recreated if enabled)
rm -f ~/.local/share/mnemosyne/mnemosyne.db
```

### Inspecting Evaluation Data

Since evaluation data is stored in SQLite, you can inspect it directly:

```bash
# Open database
sqlite3 ~/.local/share/mnemosyne/mnemosyne.db

# List tables
.tables

# Inspect evaluations
SELECT session_id, context_type, was_accessed, was_useful
FROM context_evaluations
LIMIT 10;

# Inspect features
SELECT evaluation_id, keyword_overlap_score, recency_days, was_useful
FROM relevance_features
LIMIT 10;
```

**Full transparency**: All evaluation data is inspectable.

---

## Compliance

### GDPR (General Data Protection Regulation)

**Article 5 - Principles**:
- **Lawfulness**: User explicitly enables evaluation system
- **Purpose limitation**: Data used only for context relevance learning
- **Data minimization**: Only statistical features collected, no raw content
- **Accuracy**: Feedback signals directly reflect user behavior
- **Storage limitation**: Automatic retention policies (30-365 days)
- **Integrity**: SQLite ACID guarantees, local-only storage
- **Confidentiality**: Local storage, no transmission

**Article 17 - Right to Erasure**:
```bash
# User can delete all evaluation data at any time
rm -f ~/.local/share/mnemosyne/mnemosyne.db
rm -f .mnemosyne/project.db
```

### CCPA (California Consumer Privacy Act)

**Right to Know**: This document provides full disclosure of collected data.

**Right to Delete**: Users can delete evaluation data at any time (see above).

**Right to Opt-Out**: Users can disable evaluation system (see "User Control").

**No Sale of Data**: Evaluation data is never transmitted, shared, or sold. Period.

### Other Privacy Laws

The evaluation system's **local-only, privacy-preserving design** complies with:
- PIPEDA (Canada)
- LGPD (Brazil)
- POPIA (South Africa)
- APPI (Japan)

**Rationale**: Since no personal data is collected or transmitted, most privacy regulations do not apply.

---

## Security

### Threat Model

**Threats considered**:
1. Accidental exposure via git commits
2. Unauthorized local access to evaluation database
3. Inference attacks on statistical features
4. Network eavesdropping (non-issue: no network calls)

**Mitigations**:
1. `.mnemosyne/` automatically gitignored
2. Standard filesystem permissions (user-only read/write)
3. Statistical features provide k-anonymity (no unique identifiers)
4. No network transmission = no eavesdropping risk

### Attack Resistance

**Reconstruction attacks**: Can statistical features be used to reconstruct task descriptions?

**Answer**: No. Features are aggregate metrics (e.g., "30% keyword overlap") without original keywords or content. This is information-theoretically impossible to reverse.

**Linkage attacks**: Can evaluations be linked to specific users or tasks?

**Answer**: Unlikely. Task hashes (16 chars) and keywords (generic terms) provide minimal linkage surface. Evaluations are scoped to local machine.

---

## Transparency

### Open Source

The evaluation system is fully open source:

- **Code**: [src/evaluation/](https://github.com/rand/mnemosyne/tree/main/src/evaluation)
- **Tests**: [tests/unit/test_evaluation.rs](https://github.com/rand/mnemosyne/tree/main/tests/unit)
- **Documentation**: This file

**Audit**: Anyone can review the code to verify privacy guarantees.

### No Telemetry

**Mnemosyne never collects telemetry.** The evaluation system is the only component that stores behavioral data, and it does so:

- Locally (no transmission)
- Privacy-preserving (statistical features only)
- User-controlled (easy disable)
- Transparent (open source)

---

## Questions and Concerns

### "What if I don't trust the privacy guarantees?"

**Disable evaluation**:
```python
OptimizerConfig(enable_evaluation=False)
```

Or set environment variable:
```bash
export MNEMOSYNE_DISABLE_EVALUATION=1
```

**Verify via code audit**: All evaluation code is in `src/evaluation/`. Review it yourself.

### "Can Anthropic see my evaluation data?"

**No.** The evaluation system makes **zero** network calls. Anthropic API calls used by the Optimizer agent are the **same** calls Mnemosyne already makes for memory enrichment. No additional data is sent.

### "What about embeddings?"

Currently, the evaluation system does **not** use embeddings (semantic similarity is not yet implemented). If added in the future:

- Embeddings will be computed **locally** (no API calls)
- Or computed via Anthropic API (same as existing memory embeddings)
- Embeddings will be stored **locally** in the same database
- No additional privacy concerns (embeddings are statistical representations)

### "Can this be used for surveillance?"

**No.** Evaluation data is:
- Local-only (no remote access)
- Statistical (no raw content)
- User-controlled (easy disable)
- Scoped to one machine (no cross-machine aggregation)

An attacker with filesystem access could read the SQLite database, but they would only find:
- Task hashes (meaningless without original task)
- Generic keywords (broad categories)
- Statistical features (keyword overlap scores, access counts)

**No actionable surveillance data.**

---

## Future Privacy Considerations

### Planned Features

**Semantic similarity** (embeddings):
- Will use existing Anthropic API calls (no new network calls)
- Embeddings stored locally
- No additional privacy risk

**Co-occurrence tracking**:
- Statistical feature: "How often does skill A appear with skill B?"
- No content stored, only co-occurrence scores
- Privacy-preserving

**Historical success rate**:
- Statistical feature: "How often was context useful in past?"
- Aggregated across evaluations
- No individual task reconstruction possible

### Privacy-Preserving Enhancements

**Potential future improvements**:
1. **Differential privacy**: Add noise to statistical features
2. **Federated learning**: Learn weights without central aggregation (already achieved via local-only design)
3. **Encrypted database**: SQLite with SQLCipher extension
4. **Ephemeral mode**: Disable all persistence (in-memory only)

---

## Contact

Privacy concerns or questions?

- **GitHub Issues**: [github.com/rand/mnemosyne/issues](https://github.com/rand/mnemosyne/issues)
- **Email**: rand.arete@gmail.com
- **Response time**: 48 hours

---

## Changelog

### 2025-10-28
- Initial privacy policy for evaluation system

---

**Summary**: Mnemosyne's evaluation system is designed with privacy as a core constraint. All data is local, statistical, and user-controlled. No telemetry. No transmission. No surprises.
