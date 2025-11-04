#!/usr/bin/env python3
"""End-to-end integration tests for SpecFlow workflow with DSPy integration.

Tests verify the complete feature specification workflow:
1. Spec creation and parsing
2. DSPy-powered validation
3. Ambiguity detection
4. Pattern-based fallback
5. Improvement suggestions
6. JSON output format
7. Error handling

This test suite validates the integration between:
- SpecFlow slash commands (.claude/commands/feature-*.md)
- ReviewerModule DSPy signatures
- Spec validation infrastructure (specflow_integration.py)
- Artifact storage and retrieval
"""

import os
import json
import pytest
import tempfile
from pathlib import Path
from typing import Dict, Any

# Import module under test
import sys
sys.path.insert(0, str(Path(__file__).parent))

from specflow_integration import (
    validate_feature_spec,
    detect_ambiguities,
    suggest_improvements,
    parse_feature_spec,
    detect_vague_terms,
    check_scenario_completeness,
    pattern_based_validation,
    DSPY_AVAILABLE,
)

try:
    import dspy
    from reviewer_module import ReviewerModule
except ImportError:
    ReviewerModule = None


# =============================================================================
# Test Fixtures
# =============================================================================

@pytest.fixture
def temp_spec_dir():
    """Create temporary directory for test specs."""
    with tempfile.TemporaryDirectory() as tmpdir:
        yield Path(tmpdir)


@pytest.fixture
def minimal_spec(temp_spec_dir):
    """Create minimal valid feature spec."""
    spec_content = """---
type: feature_spec
id: test-feature
name: Test Feature
version: 1.0.0
status: draft
created_at: 2025-11-03T12:00:00Z
updated_at: 2025-11-03T12:00:00Z
---

# Feature: Test Feature

## Overview

A test feature for validation.

**Business Value**: Testing purposes

## User Scenarios (Prioritized)

### P0: Basic Scenario

**As a** developer
**I want** to test validation
**So that** specs work correctly

**Acceptance Criteria**:
- [ ] Criterion 1: API response time is under 200ms (p95)
- [ ] Criterion 2: Error rate below 0.1% in production
- [ ] Criterion 3: 100% test coverage for critical paths

## Requirements

### Functional
- Feature must validate specs correctly
- Feature must detect ambiguities

### Non-Functional
- **Performance**: Validation completes in under 5 seconds
- **Security**: No sensitive data in spec files

## Success Criteria

- Validation accuracy above 90%
- Zero false positives in testing
"""
    spec_path = temp_spec_dir / "test-feature.md"
    spec_path.write_text(spec_content)
    return spec_path


@pytest.fixture
def vague_spec(temp_spec_dir):
    """Create spec with vague terms and ambiguities."""
    spec_content = """---
type: feature_spec
id: vague-feature
name: Vague Feature
version: 1.0.0
---

# Feature: Vague Feature

## Overview

A fast and secure feature that is easy to use.

## User Scenarios (Prioritized)

### P0: Vague Scenario

**As a** user
**I want** quick responses
**So that** the system feels responsive

**Acceptance Criteria**:
- [ ] System is fast
- [ ] Security is good

### P2: Low Priority

**As a** admin
**I want** better performance
**So that** users are happy

**Acceptance Criteria**:
- [ ] Performance is optimized

## Requirements

### Functional
- System should be intuitive

### Non-Functional
- **Performance**: Must be scalable
- **Security**: Should be safe

## Success Criteria

- Users like it
"""
    spec_path = temp_spec_dir / "vague-feature.md"
    spec_path.write_text(spec_content)
    return spec_path


@pytest.fixture
def incomplete_spec(temp_spec_dir):
    """Create spec with missing sections and insufficient criteria."""
    spec_content = """---
type: feature_spec
id: incomplete-feature
name: Incomplete Feature
---

# Feature: Incomplete Feature

## Overview

Missing sections.

## User Scenarios (Prioritized)

### P1: Only Scenario

**As a** user
**I want** something
**So that** it works

**Acceptance Criteria**:
- [ ] One criterion only

## Requirements

TBD
"""
    spec_path = temp_spec_dir / "incomplete-feature.md"
    spec_path.write_text(spec_content)
    return spec_path


@pytest.fixture
def reviewer_module():
    """Create ReviewerModule if DSPy available."""
    if not DSPY_AVAILABLE or ReviewerModule is None:
        pytest.skip("DSPy not available - skipping integration tests")

    api_key = os.getenv("ANTHROPIC_API_KEY")
    if not api_key:
        pytest.skip("ANTHROPIC_API_KEY not set - skipping integration tests")

    # Configure DSPy with Claude Haiku for fast testing
    dspy.configure(lm=dspy.LM('anthropic/claude-haiku-4-5-20251001', api_key=api_key))

    return ReviewerModule()


# =============================================================================
# Test: Spec Parsing
# =============================================================================

class TestSpecParsing:
    """Test feature spec parsing functionality."""

    def test_parse_minimal_spec(self, minimal_spec):
        """Test parsing minimal valid spec."""
        spec = parse_feature_spec(minimal_spec)

        assert "frontmatter" in spec
        assert spec["frontmatter"]["id"] == "test-feature"
        assert spec["frontmatter"]["name"] == "Test Feature"

        assert "overview" in spec
        assert "test feature" in spec["overview"].lower()

        assert "scenarios" in spec
        assert len(spec["scenarios"]) == 1

        scenario = spec["scenarios"][0]
        assert scenario["priority"] == "P0"
        assert scenario["name"] == "Basic Scenario"
        assert scenario["actor"] == "developer"
        assert scenario["goal"] == "to test validation"
        assert scenario["benefit"] == "specs work correctly"
        assert len(scenario["acceptance_criteria"]) == 3

    def test_parse_multiple_scenarios(self, vague_spec):
        """Test parsing spec with multiple scenarios."""
        spec = parse_feature_spec(vague_spec)

        assert len(spec["scenarios"]) == 2

        # Check P0 scenario
        p0_scenario = spec["scenarios"][0]
        assert p0_scenario["priority"] == "P0"

        # Check P2 scenario
        p2_scenario = spec["scenarios"][1]
        assert p2_scenario["priority"] == "P2"

    def test_parse_missing_sections(self, incomplete_spec):
        """Test parsing spec with missing sections."""
        spec = parse_feature_spec(incomplete_spec)

        assert "frontmatter" in spec
        assert "overview" in spec
        # Some sections may be missing, should handle gracefully

    def test_parse_nonexistent_spec(self, temp_spec_dir):
        """Test parsing nonexistent spec file."""
        nonexistent = temp_spec_dir / "nonexistent.md"

        with pytest.raises(FileNotFoundError):
            parse_feature_spec(nonexistent)


# =============================================================================
# Test: Pattern-Based Validation (Fallback)
# =============================================================================

class TestPatternBasedValidation:
    """Test pattern-based validation (no DSPy required)."""

    def test_detect_vague_terms(self):
        """Test detection of vague terms without metrics."""
        # Text with vague terms
        vague_text = "The system must be fast and secure while remaining easy to use."
        vague_terms = detect_vague_terms(vague_text)

        assert "fast" in vague_terms
        assert "secure" in vague_terms
        assert "easy" in vague_terms

    def test_detect_vague_terms_with_metrics(self):
        """Test that quantified terms are not flagged as vague."""
        # Text with quantified terms
        quantified_text = "The system must be fast (response time < 200ms) and secure (AES-256 encryption)."
        vague_terms = detect_vague_terms(quantified_text)

        # Should not flag terms that are quantified
        assert len(vague_terms) == 0 or "fast" not in vague_terms

    def test_check_scenario_completeness_sufficient(self):
        """Test scenario with sufficient acceptance criteria."""
        scenarios = [{
            "priority": "P0",
            "name": "Test Scenario",
            "acceptance_criteria": [
                "Criterion 1: Response time < 200ms",
                "Criterion 2: Error rate < 0.1%",
                "Criterion 3: Test coverage > 90%"
            ]
        }]

        issues = check_scenario_completeness(scenarios)

        # Should not flag scenario with 3 criteria
        assert len(issues) == 0

    def test_check_scenario_completeness_insufficient(self):
        """Test P0 scenario with insufficient criteria."""
        scenarios = [{
            "priority": "P0",
            "name": "Test Scenario",
            "acceptance_criteria": ["Only one criterion"]
        }]

        issues = check_scenario_completeness(scenarios)

        # Should flag P0 scenario with < 3 criteria
        assert len(issues) > 0
        assert "only 1" in issues[0].lower()

    def test_pattern_validation_minimal_spec(self, minimal_spec):
        """Test pattern-based validation on minimal spec."""
        spec = parse_feature_spec(minimal_spec)
        result = pattern_based_validation(spec)

        assert result.completeness_score >= 0.7
        assert len(result.issues) == 0 or len(result.issues) == 1  # May have minor issues

    def test_pattern_validation_vague_spec(self, vague_spec):
        """Test pattern-based validation on vague spec."""
        spec = parse_feature_spec(vague_spec)
        result = pattern_based_validation(spec)

        # Should detect multiple issues
        assert not result.is_valid
        assert len(result.issues) > 0
        assert result.completeness_score < 0.8

    def test_pattern_validation_incomplete_spec(self, incomplete_spec):
        """Test pattern-based validation on incomplete spec."""
        spec = parse_feature_spec(incomplete_spec)
        result = pattern_based_validation(spec)

        # Should detect missing P0 scenarios and insufficient criteria
        assert not result.is_valid
        assert result.completeness_score < 0.7


# =============================================================================
# Test: Public API Functions
# =============================================================================

class TestPublicAPI:
    """Test public API functions."""

    def test_validate_feature_spec_valid(self, minimal_spec):
        """Test validation of valid spec."""
        result = validate_feature_spec(minimal_spec)

        assert "is_valid" in result
        assert "issues" in result
        assert "suggestions" in result
        assert "requirements" in result
        assert "ambiguities" in result
        assert "completeness_score" in result

        assert isinstance(result["completeness_score"], float)
        assert 0.0 <= result["completeness_score"] <= 1.0

    def test_validate_feature_spec_vague(self, vague_spec):
        """Test validation detects vague terms."""
        result = validate_feature_spec(vague_spec)

        assert not result["is_valid"]
        assert len(result["issues"]) > 0

        # Should detect vague terms in overview
        issues_text = " ".join(result["issues"])
        assert any(term in issues_text.lower() for term in ["fast", "secure", "easy"])

    def test_validate_feature_spec_incomplete(self, incomplete_spec):
        """Test validation detects incomplete specs."""
        result = validate_feature_spec(incomplete_spec)

        assert not result["is_valid"]
        assert result["completeness_score"] < 0.7

        # Should detect missing P0 and insufficient criteria
        issues_text = " ".join(result["issues"])
        assert "p0" in issues_text.lower() or "criteria" in issues_text.lower()

    def test_detect_ambiguities_vague_spec(self, vague_spec):
        """Test ambiguity detection on vague spec."""
        ambiguities = detect_ambiguities(vague_spec)

        # Should detect ambiguities (if DSPy available) or empty list (pattern-based)
        assert isinstance(ambiguities, list)

        if DSPY_AVAILABLE and len(ambiguities) > 0:
            # Verify ambiguity structure
            amb = ambiguities[0]
            assert "location" in amb
            assert "term" in amb
            assert "question" in amb
            assert "impact" in amb

    def test_suggest_improvements_incomplete_spec(self, incomplete_spec):
        """Test improvement suggestions."""
        suggestions = suggest_improvements(incomplete_spec)

        assert isinstance(suggestions, list)
        assert len(suggestions) > 0

    def test_validate_nonexistent_spec(self, temp_spec_dir):
        """Test validation of nonexistent spec."""
        nonexistent = temp_spec_dir / "nonexistent.md"
        result = validate_feature_spec(nonexistent)

        assert not result["is_valid"]
        assert len(result["issues"]) > 0
        assert "parse" in result["issues"][0].lower() or "not found" in result["issues"][0].lower()


# =============================================================================
# Test: DSPy Integration (requires API key)
# =============================================================================

class TestDSpyIntegration:
    """Test DSPy-powered validation (requires ANTHROPIC_API_KEY)."""

    @pytest.mark.skipif(not DSPY_AVAILABLE, reason="DSPy not available")
    def test_dspy_requirement_extraction(self, reviewer_module, minimal_spec):
        """Test requirement extraction using ReviewerModule."""
        spec = parse_feature_spec(minimal_spec)
        feature_name = spec["frontmatter"]["name"]
        overview = spec.get("overview", "")

        user_intent = f"{feature_name}. {overview}"

        result = reviewer_module.extract_requirements(
            user_intent=user_intent,
            context="Test feature specification"
        )

        assert hasattr(result, 'requirements')
        assert isinstance(result.requirements, list)
        # Should extract some requirements
        assert len(result.requirements) > 0

    @pytest.mark.skipif(not DSPY_AVAILABLE, reason="DSPy not available")
    def test_dspy_validation_quality(self, minimal_spec):
        """Test DSPy-powered validation provides detailed analysis."""
        result = validate_feature_spec(minimal_spec)

        # DSPy validation should provide extracted requirements
        if DSPY_AVAILABLE and len(result["requirements"]) > 0:
            assert isinstance(result["requirements"], list)
            # Should extract meaningful requirements
            assert any(len(req) > 10 for req in result["requirements"])


# =============================================================================
# Test: JSON Output Format
# =============================================================================

class TestJSONOutput:
    """Test JSON output format for CLI integration."""

    def test_json_serializable_output(self, minimal_spec):
        """Test that validation result is JSON-serializable."""
        result = validate_feature_spec(minimal_spec)

        # Should be JSON-serializable
        json_str = json.dumps(result, indent=2)
        assert len(json_str) > 0

        # Should round-trip correctly
        parsed = json.loads(json_str)
        assert parsed["completeness_score"] == result["completeness_score"]

    def test_json_output_structure(self, minimal_spec):
        """Test JSON output has expected structure."""
        result = validate_feature_spec(minimal_spec)

        # Required fields
        required_fields = [
            "is_valid",
            "issues",
            "suggestions",
            "requirements",
            "ambiguities",
            "completeness_score"
        ]

        for field in required_fields:
            assert field in result

        # Type checks
        assert isinstance(result["is_valid"], bool)
        assert isinstance(result["issues"], list)
        assert isinstance(result["suggestions"], list)
        assert isinstance(result["requirements"], list)
        assert isinstance(result["ambiguities"], list)
        assert isinstance(result["completeness_score"], float)


# =============================================================================
# Test: Error Handling and Edge Cases
# =============================================================================

class TestErrorHandling:
    """Test error handling and edge cases."""

    def test_malformed_yaml_frontmatter(self, temp_spec_dir):
        """Test handling of malformed YAML frontmatter."""
        malformed_spec = temp_spec_dir / "malformed.md"
        malformed_spec.write_text("""---
this is not: valid: yaml:
more: invalid
---

# Feature

Content here
""")

        # Should handle gracefully
        spec = parse_feature_spec(malformed_spec)
        # Frontmatter may be empty or partial
        assert "full_text" in spec

    def test_empty_spec_file(self, temp_spec_dir):
        """Test handling of empty spec file."""
        empty_spec = temp_spec_dir / "empty.md"
        empty_spec.write_text("")

        result = validate_feature_spec(empty_spec)

        # Should fail validation
        assert not result["is_valid"]
        assert len(result["issues"]) > 0

    def test_spec_with_no_scenarios(self, temp_spec_dir):
        """Test spec with no user scenarios."""
        no_scenarios_spec = temp_spec_dir / "no-scenarios.md"
        no_scenarios_spec.write_text("""---
type: feature_spec
id: no-scenarios
---

# Feature: No Scenarios

## Overview

Feature without scenarios.

## Requirements

Some requirements here.
""")

        result = validate_feature_spec(no_scenarios_spec)

        # Should detect missing scenarios
        assert not result["is_valid"]
        assert result["completeness_score"] < 0.5


# =============================================================================
# Test: End-to-End Workflow
# =============================================================================

class TestEndToEndWorkflow:
    """Test complete SpecFlow workflow."""

    def test_create_validate_improve_cycle(self, temp_spec_dir):
        """Test create → validate → improve cycle."""
        # Step 1: Create spec with known issues
        spec_with_issues = temp_spec_dir / "improve-me.md"
        spec_with_issues.write_text("""---
type: feature_spec
id: improve-me
name: Feature To Improve
---

# Feature: Feature To Improve

## Overview

A fast and secure feature.

## User Scenarios (Prioritized)

### P0: Main Scenario

**As a** user
**I want** quick responses
**So that** it's responsive

**Acceptance Criteria**:
- [ ] System is fast

## Requirements

### Functional
- Must be easy to use

## Success Criteria

- Users are satisfied
""")

        # Step 2: Validate and get issues
        result1 = validate_feature_spec(spec_with_issues)

        assert not result1["is_valid"]
        initial_score = result1["completeness_score"]
        assert initial_score < 0.7

        # Step 3: Get improvement suggestions
        suggestions = suggest_improvements(spec_with_issues)
        assert len(suggestions) > 0

        # Step 4: Apply improvements (simulate)
        improved_spec = temp_spec_dir / "improved.md"
        improved_spec.write_text("""---
type: feature_spec
id: improved
name: Improved Feature
---

# Feature: Improved Feature

## Overview

A feature with response time under 200ms (p95) and AES-256 encryption.

**Business Value**: Provides fast and secure user experience

## User Scenarios (Prioritized)

### P0: Main Scenario

**As a** user
**I want** API responses within 200ms (p95 latency)
**So that** the interface remains interactive

**Acceptance Criteria**:
- [ ] API response time is under 200ms at p95
- [ ] Error rate is below 0.1% in production
- [ ] All endpoints use AES-256 encryption

## Requirements

### Functional
- API must respond to requests
- All data must be encrypted at rest and in transit

### Non-Functional
- **Performance**: 200ms p95 latency under 1000 req/s load
- **Security**: AES-256 encryption, HTTPS only

## Success Criteria

- User satisfaction score above 4.5/5.0
- API latency p95 < 200ms for 30 consecutive days
- Zero security incidents in production
""")

        # Step 5: Validate improved spec
        result2 = validate_feature_spec(improved_spec)

        improved_score = result2["completeness_score"]

        # Score should improve
        assert improved_score > initial_score
        # May or may not be valid depending on detection sensitivity
        # but score should be significantly better

    def test_workflow_with_clear_spec(self, minimal_spec):
        """Test workflow with clear, well-defined spec."""
        # Validate clear spec
        result = validate_feature_spec(minimal_spec)

        # Should pass validation
        assert result["completeness_score"] >= 0.7

        # Should have few or no issues
        assert len(result["issues"]) <= 1

        # Ambiguities should be minimal
        ambiguities = detect_ambiguities(minimal_spec)
        assert len(ambiguities) <= 1


# =============================================================================
# Test: CLI Integration
# =============================================================================

class TestCLIIntegration:
    """Test CLI entry point and output formats."""

    def test_cli_json_output_format(self, minimal_spec):
        """Test that CLI JSON output is well-formed."""
        result = validate_feature_spec(minimal_spec)

        # Simulate CLI JSON output
        json_output = json.dumps(result, indent=2)

        # Should be valid JSON
        parsed = json.loads(json_output)
        assert parsed is not None

        # Should have all required fields
        assert "is_valid" in parsed
        assert "completeness_score" in parsed

    def test_ambiguities_only_mode(self, vague_spec):
        """Test ambiguities-only detection (for --ambiguities-only flag)."""
        ambiguities = detect_ambiguities(vague_spec)

        # Should return list of ambiguities
        assert isinstance(ambiguities, list)

        # If DSPy available and ambiguities detected, verify structure
        if len(ambiguities) > 0:
            for amb in ambiguities:
                assert isinstance(amb, dict)
                # Should have required fields
                if "location" in amb:
                    assert isinstance(amb["location"], str)


if __name__ == "__main__":
    # Run tests with pytest
    pytest.main([__file__, "-v", "--tb=short"])
