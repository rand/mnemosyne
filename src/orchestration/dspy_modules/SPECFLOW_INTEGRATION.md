# SpecFlow + ReviewerModule Integration

**Status**: Phase 3 Complete - Integration Layer Ready
**Date**: 2025-11-02
**Purpose**: Integrate DSPy ReviewerModule intelligence with SpecFlow slash commands for automated spec validation

---

## Overview

This integration connects the ReviewerModule's requirement extraction and validation capabilities with the SpecFlow specification workflow, enabling:

1. **Automated Spec Validation**: Verify feature specs are complete and unambiguous
2. **Intelligent Ambiguity Detection**: Use LLM to identify vague terms and missing metrics
3. **Requirement Extraction**: Extract implicit requirements from user scenarios
4. **Quality Guidance**: Generate actionable suggestions for improving specs

## Architecture

```
┌─────────────────────────┐
│  /feature-specify       │
│  (Slash Command)        │
└───────────┬─────────────┘
            │
            │ 1. Create spec
            ↓
┌─────────────────────────┐
│  Feature Spec           │
│  (.mnemosyne/artifacts/ │
│   specs/feature-id.md)  │
└───────────┬─────────────┘
            │
            │ 2. Validate
            ↓
┌─────────────────────────┐
│  specflow_integration.py│
│  (Integration Layer)    │
└───────────┬─────────────┘
            │
            │ 3. Analyze with LLM
            ↓
┌─────────────────────────┐
│  ReviewerModule         │
│  (DSPy Intelligence)    │
└───────────┬─────────────┘
            │
            │ 4. Return results
            ↓
┌─────────────────────────┐
│  Validation Results     │
│  - Issues               │
│  - Suggestions          │
│  - Ambiguities          │
└─────────────────────────┘
```

## Components

### 1. specflow_integration.py

**Location**: `src/orchestration/dspy_modules/specflow_integration.py`

**Functions**:
- `validate_feature_spec(spec_path)` - Full validation with issues and suggestions
- `detect_ambiguities(spec_path)` - Find vague terms and missing metrics
- `suggest_improvements(spec_path)` - Generate actionable guidance

**Validation Modes**:
- **DSPy-based** (when available): Uses ReviewerModule for intelligent analysis
- **Pattern-based** (fallback): Uses regex patterns for basic validation

### 2. Slash Command Integration

#### /feature-specify Enhancement

**After creating spec** (add to step 10):

```python
from orchestration.dspy_modules.specflow_integration import validate_feature_spec

# Validate spec
result = validate_feature_spec(f".mnemosyne/artifacts/specs/{feature_id}.md")

if not result["is_valid"]:
    print("\n⚠️  Spec validation found issues:")
    for issue in result["issues"]:
        print(f"  - {issue}")

    if result["suggestions"]:
        print("\n💡 Suggestions:")
        for suggestion in result["suggestions"]:
            print(f"  - {suggestion}")

    if result["ambiguities"]:
        print(f"\n🔍 {len(result['ambiguities'])} ambiguities detected")
        print("   Run: /feature-clarify {feature_id} --auto")
else:
    print(f"\n✓ Spec validation passed (score: {result['completeness_score']:.0%})")
```

#### /feature-clarify Enhancement

**Auto-detect ambiguities** (add to step 1):

```python
from orchestration.dspy_modules.specflow_integration import detect_ambiguities

# If --auto flag
if args.auto:
    ambiguities = detect_ambiguities(spec_path)

    # Present top 3 ambiguities as questions
    questions = []
    for amb in ambiguities[:3]:
        questions.append({
            "question": amb["question"],
            "context": amb["location"],
            "impact": amb["impact"],
        })
```

## Usage Examples

### CLI Validation

```bash
# Validate spec
cd src/orchestration/dspy_modules
python specflow_integration.py ../../.mnemosyne/artifacts/specs/jwt-auth.md

# JSON output
python specflow_integration.py jwt-auth.md --json

# Only detect ambiguities
python specflow_integration.py jwt-auth.md --ambiguities-only
```

### Python API

```python
from specflow_integration import validate_feature_spec

# Full validation
result = validate_feature_spec(".mnemosyne/artifacts/specs/jwt-auth.md")

print(f"Valid: {result['is_valid']}")
print(f"Completeness: {result['completeness_score']:.0%}")
print(f"Issues: {len(result['issues'])}")
print(f"Extracted {len(result['requirements'])} requirements")

# Check specific issues
for issue in result["issues"]:
    if "vague" in issue.lower():
        print(f"Vagueness issue: {issue}")
```

## Validation Checks

### Pattern-Based Checks (Always Active)

1. **Vague Terms Detection**:
   - Searches for: fast, slow, easy, secure, scalable, etc.
   - Flags terms not followed by quantitative metrics
   - Example: "The system should be fast" → Issue

2. **Scenario Completeness**:
   - P0/P1 scenarios require ≥3 acceptance criteria
   - Checks for vague terms in criteria
   - Ensures actionable, testable criteria

3. **Success Criteria**:
   - Requires measurable success criteria section
   - Checks for quantitative metrics
   - Example: "95% uptime" ✓ vs "Highly available" ✗

### DSPy-Based Checks (When Available)

1. **Requirement Extraction**:
   - Uses ReviewerModule.extract_requirements
   - Compares extracted vs. documented requirements
   - Flags missing implicit requirements

2. **Semantic Analysis**:
   - Understands context and intent
   - Detects ambiguity beyond pattern matching
   - Suggests specific metrics and thresholds

## Completeness Score

**Formula**: `score = 1.0 - (penalty_1 + penalty_2 + ...)`

**Penalties**:
- Vague terms in overview: -0.2
- Missing acceptance criteria: -0.1 per scenario (max -0.3)
- No P0 scenarios: -0.3
- Missing success criteria: -0.2
- Ambiguities: -0.05 per ambiguity (max -0.2)

**Interpretation**:
- 1.0 (100%): Perfect spec, no issues
- 0.8-0.99 (80-99%): Good, minor improvements
- 0.6-0.79 (60-79%): Acceptable, needs clarification
- <0.6 (<60%): Incomplete, requires rework

## Integration Workflow

### Current State (Phase 3)

1. ✅ Integration layer created (`specflow_integration.py`)
2. ✅ Validation API implemented
3. ✅ Ambiguity detection implemented
4. ✅ CLI tool available
5. ⏸️ Slash command integration pending

### Next Steps (Sprint 2)

1. **Update /feature-specify**:
   - Add validation call after spec creation
   - Display issues and suggestions
   - Offer to run /feature-clarify if ambiguities found

2. **Update /feature-clarify**:
   - Add --auto flag for automated ambiguity detection
   - Use detect_ambiguities() to find top 3 questions
   - Present questions with context and impact

3. **Update /feature-plan**:
   - Validate spec before generating plan
   - Ensure all requirements extracted
   - Flag incomplete specs

4. **Add /feature-validate** (new):
   - Standalone validation command
   - Run validation on existing specs
   - Generate validation report

## Testing

### Unit Tests

```bash
# Test pattern-based validation
pytest test_specflow_integration.py::test_pattern_validation

# Test DSPy validation
pytest test_specflow_integration.py::test_dspy_validation

# Test spec parsing
pytest test_specflow_integration.py::test_spec_parsing
```

### Integration Tests

```bash
# Create sample spec
/feature-specify Sample authentication feature

# Validate
python specflow_integration.py .mnemosyne/artifacts/specs/sample-auth.md

# Expected: Issues for vague terms, suggestions for improvement
```

## Performance

### Latency

- **Pattern-based**: <100ms (regex matching)
- **DSPy-based**: 1-3 seconds (LLM call)

### Optimization

- Cache parsed specs for repeated validation
- Run pattern checks first, DSPy only if needed
- Parallelize multiple spec validations

## Error Handling

### Graceful Degradation

1. **DSPy unavailable**: Falls back to pattern matching
2. **LLM error**: Uses pattern validation, logs warning
3. **Parse error**: Returns error with suggestion to fix format

### Error Messages

```python
# Parse error
{
  "is_valid": False,
  "issues": ["Failed to parse spec: YAML frontmatter invalid"],
  "suggestions": ["Verify YAML syntax between --- delimiters"],
  ...
}

# DSPy failure (falls back to patterns)
{
  "is_valid": True,
  "issues": [],
  "suggestions": ["Note: Advanced validation unavailable, used pattern matching"],
  ...
}
```

## Future Enhancements

### Phase 4: Optimization

1. **Batch Validation**: Validate multiple specs in parallel
2. **Caching**: Cache validation results, invalidate on change
3. **Progressive Enhancement**: Show pattern results immediately, DSPy async

### Phase 5: Advanced Features

1. **Spec Comparison**: Compare before/after versions
2. **Requirement Tracing**: Link requirements to implementation
3. **Quality Trends**: Track spec quality over time
4. **Auto-Fix**: Suggest specific text replacements

## References

- [ReviewerModule](./reviewer_module.py) - DSPy module for validation
- [Feature Spec Format](../../artifacts/specs/README.md) - Spec structure
- [/feature-specify](../../.claude/commands/feature-specify.md) - Spec creation
- [/feature-clarify](../../.claude/commands/feature-clarify.md) - Ambiguity resolution
- [Training Data](./training_data/README.md) - ReviewerModule training

---

## Changelog

- **2025-11-02**: Created integration layer with validation API
- **TBD**: Integrate with /feature-specify slash command
- **TBD**: Add /feature-validate standalone command
