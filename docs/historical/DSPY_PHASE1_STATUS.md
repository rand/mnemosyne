# DSPy/DSRs Phase 1: Feasibility Assessment - Status Report

## Overview

**Goal**: Assess whether DSPy (via DSRs Rust port) can improve Mnemosyne's LLM interactions through systematic prompt optimization rather than manual tuning.

**Current Phase**: Phase 1.2 - Verify Anthropic API Integration

**Branch**: `feature/dspy-llm-optimization`

**Status**: IN PROGRESS

---

## Phase 1.1: Setup ✅ COMPLETE

### Completed Tasks

1. **Dependency Added** (`Cargo.toml`)
   - Added `dspy-rs = "0.7.1"`
   - Successfully compiled with 98 new transitive dependencies
   - No conflicts with existing dependencies

2. **Module Structure Created** (`src/services/dspy_llm.rs`)
   - Created skeleton `DspyLlmService` struct
   - Created `DspyConfig` with Anthropic API key support
   - Exported from `services` module
   - Basic tests passing

3. **Verified Compilation**
   - Clean compilation after adding DSPy imports
   - Successfully imported `dspy_rs::core::lm::LM`
   - Successfully imported `dspy_rs::sign` macro

### Git Commit

```
88c7ef2 Add DSPy-rs dependency and initial module structure
```

---

## Phase 1.2: Verify Anthropic API Integration 🔄 IN PROGRESS

### Findings So Far

#### 1. Anthropic Support in DSRs

**Confirmed**: DSRs supports Anthropic API natively

**Evidence from source code review**:
- File: `crates/dspy-rs/src/core/lm/client_registry.rs`
- Uses `ANTHROPIC_API_KEY` environment variable
- Model string format: `"anthropic:claude-3-5-haiku-20241022"`
- Client builder: `anthropic::ClientBuilder::new(&key).build()`
- Completion model: `anthropic::completion::CompletionModel::new(client, model_id)`

#### 2. DSRs API Structure

**Module Organization**:
- `dspy_rs::core::lm::LM` - Language model configuration
- `dspy_rs::sign!` - Macro for defining signatures (input/output schema)
- `dspy_rs::core::Signature` - Trait for type-safe LLM interactions
- `dspy_rs::predictors::Predictor` - Prediction execution
- `dspy_rs::optimizer` - Prompt optimization (COPRO, MIPROv2, GEPA)

**Initialization Patterns** (from source):
1. Provider via model string: `"provider:model"` format
2. OpenAI-compatible with auth: Requires `base_url` + `api_key`
3. Local server: Requires only `base_url` (for vLLM, etc.)

**Configuration Parameters**:
- `model`: Provider and model specification
- `temperature`: Sampling randomness
- `max_tokens`: Response length limit
- `base_url`: Optional endpoint override
- `api_key`: Optional authentication (or env var)

#### 3. Examples Available

DSRs provides 10 examples (all using OpenAI by default):
1. `01-simple.rs` - Basic QA with rating
2. `02-module-iteration-and-updation.rs` - Module updates
3. `03-evaluate-hotpotqa.rs` - Evaluation
4. `04-optimize-hotpotqa.rs` - Optimization
5. `05-heterogenous-examples.rs` - Mixed examples
6. `06-other-providers-batch.rs` - Batch processing
7. `07-inspect-history.rs` - History inspection
8. `08-optimize-mipro.rs` - MIPROv2 optimizer
9. `09-gepa-sentiment.rs` - GEPA sentiment analysis
10. `10-gepa-llm-judge.rs` - GEPA LLM judging

**Note**: No explicit Anthropic examples found, but source code confirms support.

### Next Steps for Phase 1.2

1. **Create minimal Anthropic test**
   - Initialize LM with `"anthropic:claude-3-5-haiku-20241022"`
   - Verify API key is read from environment
   - Make simple completion request
   - Confirm response parsing works

2. **Test signature definition**
   - Use `sign!` macro to define enrichment signature
   - Define input fields: `content`, `context`
   - Define output fields: `summary`, `keywords`, `tags`, `memory_type`, `importance`

3. **Compare with manual approach**
   - Call same API with DSPy vs manual `reqwest`
   - Measure latency difference
   - Check response parsing reliability

### Blockers / Risks

1. **Documentation Gap**: DSRs documentation is sparse
   - Examples use OpenAI, not Anthropic
   - API patterns must be inferred from source code
   - May require trial-and-error

2. **API Compatibility**: Need to verify
   - Anthropic API message format matches expectations
   - Response parsing handles Claude-specific formats
   - Error handling works correctly

3. **Version Stability**: DSRs is v0.7.1
   - Pre-1.0 software may have breaking changes
   - API may evolve rapidly
   - Community/support may be limited

---

## Current Mnemosyne LLM Integration

### Existing Prompts (Manual)

#### 1. Memory Enrichment (`llm.rs:99-191`)

**Input**: Raw content + context
**Output**: `SUMMARY:`, `KEYWORDS:`, `TAGS:`, `TYPE:`, `IMPORTANCE:`
**Parsing**: `extract_field()` with prefix matching
**Pain Points**:
- Brittle string parsing
- No validation loop
- Fixed format requirements
- Manual temperature/token tuning

#### 2. Link Generation (`llm.rs:194-294`)

**Input**: New memory + candidate memories
**Output**: `LINK: index, type, strength, reason`
**Parsing**: Line-by-line prefix matching
**Pain Points**:
- Manual candidate formatting
- No few-shot examples
- Fixed link type enum
- No optimization

#### 3. Consolidation Decision (`llm.rs:297-393`)

**Input**: Two memories
**Output**: `DECISION:`, `REASON:`, `SUPERSEDING_ID:`
**Parsing**: Field extraction
**Pain Points**:
- Pairwise only (no clusters)
- No semantic understanding tuning
- Fixed decision logic

#### 4. Cluster Consolidation (NEW - `consolidation.rs:318-435`)

**Input**: Memory cluster
**Output**: JSON with `action`, `primary_memory_id`, `secondary_memory_ids`, `rationale`
**Parsing**: `serde_json` deserialization
**Pain Points**:
- Manual JSON prompt engineering
- No optimization for accuracy
- No cost controls

### Opportunity for DSPy

**What DSPy Could Improve**:
1. **Systematic Optimization**: Replace manual tuning with metrics-driven approach
2. **Few-Shot Learning**: Automatically find best examples for each task
3. **Structured Outputs**: Type-safe signatures instead of string parsing
4. **Evaluation**: Built-in metrics and A/B testing
5. **Cost Optimization**: Find cheaper prompts that work as well
6. **Adaptability**: Re-optimize as data/requirements change

---

## Success Criteria for Phase 1

### Must Have ✅
- [x] DSRs dependency compiles
- [x] Module structure created
- [ ] Anthropic API connectivity verified
- [ ] Simple completion request works
- [ ] Response parsing functional

### Nice to Have
- [ ] Signature macro working
- [ ] Basic enrichment prototype
- [ ] Latency benchmarks
- [ ] Error handling tested

### Decision Point

**GO to Phase 1.3** if:
- ✅ Anthropic API works
- ✅ Latency is acceptable (< 2x manual)
- ✅ Code complexity manageable
- ✅ No critical bugs

**NO-GO** if:
- ❌ API integration broken
- ❌ Performance unacceptable
- ❌ Too complex for benefit
- ❌ Fundamental DSRs limitations

---

## Time Spent

- **Phase 1.1 Setup**: 1.5 hours
- **Phase 1.2 Research**: 1 hour (so far)
- **Total**: 2.5 hours / 4-6 hour budget

---

## Next Session Plan

1. Create minimal Anthropic API test
2. Implement simple completion request
3. Verify response parsing
4. Measure latency
5. Document findings
6. Make go/no-go decision for Phase 1.3

---

**Last Updated**: 2025-10-28
**Author**: Claude Code
**Status**: Active Development
