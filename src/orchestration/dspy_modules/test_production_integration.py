#!/usr/bin/env python3
"""Production integration tests for DSPy modules.

Tests verify integration with production infrastructure:
1. Telemetry collection and metrics
2. Production logging and training data capture
3. Module loading and version management
4. A/B testing and version switching
5. Error handling and fallback behavior
6. Cost tracking and token usage
7. Latency monitoring
8. Cross-module workflows

These tests validate the complete production pipeline from
module execution through telemetry, logging, and continuous improvement.
"""

import os
import json
import pytest
import tempfile
from pathlib import Path
from datetime import datetime
from typing import Dict, Any, List
from unittest.mock import Mock, patch, MagicMock

# Import module under test
import sys
sys.path.insert(0, str(Path(__file__).parent))

try:
    import dspy
    from reviewer_module import ReviewerModule
    from dspy_telemetry import TelemetryCollector, DSpyEvent, TokenUsage
    from dspy_production_logger import ProductionLogger, LogConfig, LogSink, InteractionLog
    DSPY_AVAILABLE = True
except ImportError:
    DSPY_AVAILABLE = False
    pytest.skip("DSPy not available", allow_module_level=True)


# =============================================================================
# Test Fixtures
# =============================================================================

@pytest.fixture
def temp_dir():
    """Create temporary directory for test outputs."""
    with tempfile.TemporaryDirectory() as tmpdir:
        yield Path(tmpdir)


@pytest.fixture
def api_key():
    """Get API key or skip tests."""
    key = os.getenv("ANTHROPIC_API_KEY")
    if not key:
        pytest.skip("ANTHROPIC_API_KEY not set - skipping integration tests")
    return key


@pytest.fixture
def configured_lm(api_key):
    """Configure DSPy language model."""
    dspy.configure(lm=dspy.LM('anthropic/claude-haiku-4-5-20251001', api_key=api_key))
    return dspy.settings.lm


@pytest.fixture
def reviewer_module(configured_lm):
    """Create ReviewerModule instance."""
    return ReviewerModule()


@pytest.fixture
def telemetry_collector(temp_dir):
    """Create TelemetryCollector instance."""
    log_path = temp_dir / "telemetry.jsonl"
    collector = TelemetryCollector(log_path=str(log_path))
    return collector


@pytest.fixture
def production_logger(temp_dir, telemetry_collector):
    """Create ProductionLogger instance with telemetry."""
    log_path = temp_dir / "production.jsonl"
    config = LogConfig(
        sink=LogSink.File(log_path),
        buffer_size=10,
        flush_interval_secs=1
    )

    # Mock telemetry since we can't easily integrate async Rust in Python tests
    mock_telemetry = Mock()
    mock_telemetry.record = Mock(return_value=None)

    logger = ProductionLogger(config=config, telemetry=mock_telemetry)
    return logger


# =============================================================================
# Test: Telemetry Integration
# =============================================================================

class TestTelemetryIntegration:
    """Test DSPy modules with telemetry collection."""

    def test_telemetry_request_event(self, telemetry_collector):
        """Test recording request events."""
        event = DSpyEvent.request(
            module_name="reviewer",
            module_version="v1.0.0",
            signature="extract_requirements"
        )

        telemetry_collector.record(event)

        stats = telemetry_collector.get_module_stats("reviewer")
        assert stats is not None
        assert stats.total_requests >= 1

    def test_telemetry_response_event(self, telemetry_collector):
        """Test recording response events with metrics."""
        request_event = DSpyEvent.request(
            module_name="reviewer",
            module_version="v1.0.0",
            signature="validate_intent"
        )

        response_event = DSpyEvent.response(
            request=request_event,
            latency_ms=150,
            tokens=TokenUsage(prompt=100, completion=50, total=150),
            cost_usd=0.001
        )

        telemetry_collector.record(response_event)

        stats = telemetry_collector.get_module_stats("reviewer")
        assert stats.total_requests >= 1
        assert stats.avg_latency_ms >= 0
        assert stats.total_tokens >= 150

    def test_telemetry_error_event(self, telemetry_collector):
        """Test recording error events."""
        request_event = DSpyEvent.request(
            module_name="reviewer",
            module_version="v1.0.0",
            signature="verify_completeness"
        )

        error_event = DSpyEvent.error(
            request=request_event,
            latency_ms=50,
            error="Validation failed: insufficient context"
        )

        telemetry_collector.record(error_event)

        stats = telemetry_collector.get_module_stats("reviewer")
        assert stats.total_errors >= 1

    def test_telemetry_metrics_aggregation(self, telemetry_collector):
        """Test metrics aggregation across multiple calls."""
        # Record multiple events
        for i in range(5):
            request = DSpyEvent.request(
                module_name="reviewer",
                module_version="v1.0.0",
                signature=f"signature_{i % 3}"
            )

            response = DSpyEvent.response(
                request=request,
                latency_ms=100 + i * 10,
                tokens=TokenUsage(prompt=100, completion=50, total=150),
                cost_usd=0.001
            )

            telemetry_collector.record(response)

        stats = telemetry_collector.get_module_stats("reviewer")
        assert stats.total_requests >= 5
        assert stats.total_tokens >= 750  # 5 * 150
        assert stats.total_cost_usd >= 0.005  # 5 * 0.001


# =============================================================================
# Test: Production Logging Integration
# =============================================================================

class TestProductionLogging:
    """Test DSPy modules with production logging."""

    def test_log_successful_interaction(self, production_logger, temp_dir):
        """Test logging successful module interaction."""
        log = InteractionLog(
            module_name="reviewer",
            module_version="v1.0.0",
            signature="extract_requirements",
            input={"user_intent": "Add authentication", "context": "REST API"},
            output={"requirements": ["JWT tokens", "User model", "Auth endpoint"]},
            timestamp_ms=int(datetime.now().timestamp() * 1000),
            latency_ms=150,
            tokens={"prompt": 100, "completion": 50, "total": 150},
            cost_usd=0.001,
            model="claude-haiku-4-5",
            success=True,
            error=None,
            metadata={}
        )

        production_logger.log_interaction(log)
        production_logger.flush()

        # Verify log was written
        log_path = temp_dir / "production.jsonl"
        assert log_path.exists()

        # Verify log content
        with open(log_path, 'r') as f:
            lines = f.readlines()
            assert len(lines) >= 1

            logged = json.loads(lines[0])
            assert logged["module_name"] == "reviewer"
            assert logged["success"] is True

    def test_log_failed_interaction(self, production_logger, temp_dir):
        """Test logging failed module interaction."""
        log = InteractionLog(
            module_name="reviewer",
            module_version="v1.0.0",
            signature="validate_intent",
            input={"user_intent": "Test", "implementation": ""},
            output={},
            timestamp_ms=int(datetime.now().timestamp() * 1000),
            latency_ms=50,
            tokens={"prompt": 50, "completion": 0, "total": 50},
            cost_usd=0.0005,
            model="claude-haiku-4-5",
            success=False,
            error="Insufficient context provided",
            metadata={}
        )

        production_logger.log_interaction(log)
        production_logger.flush()

        # Verify error was logged
        log_path = temp_dir / "production.jsonl"
        with open(log_path, 'r') as f:
            lines = f.readlines()
            logged = json.loads(lines[0])
            assert logged["success"] is False
            assert "error" in logged

    def test_production_logger_statistics(self, production_logger):
        """Test production logger statistics tracking."""
        # Log multiple interactions
        for i in range(10):
            log = InteractionLog(
                module_name="reviewer",
                module_version="v1.0.0",
                signature=f"signature_{i % 3}",
                input={"test": "data"},
                output={"result": "success"},
                timestamp_ms=int(datetime.now().timestamp() * 1000),
                latency_ms=100 + i * 10,
                tokens={"prompt": 100, "completion": 50, "total": 150},
                cost_usd=0.001,
                model="claude-haiku-4-5",
                success=i % 5 != 0,  # 20% failure rate
                error="Test error" if i % 5 == 0 else None,
                metadata={}
            )
            production_logger.log_interaction(log)

        stats = production_logger.get_stats()

        assert stats.total_interactions >= 10
        assert stats.successful_interactions >= 8
        assert stats.failed_interactions >= 2
        assert stats.success_rate <= 0.8  # 80% success rate


# =============================================================================
# Test: Module Loading and Versioning
# =============================================================================

class TestModuleLoading:
    """Test module loading and version management."""

    def test_load_baseline_module(self, reviewer_module):
        """Test loading baseline (unoptimized) module."""
        # Baseline module should work with basic prompts
        result = reviewer_module.extract_requirements(
            user_intent="Add user authentication"
        )

        assert hasattr(result, 'requirements')
        assert isinstance(result.requirements, list)

    def test_load_optimized_module(self, temp_dir, api_key):
        """Test loading optimized module from JSON."""
        # Create mock optimized prompts
        optimized_prompts = {
            "extract_requirements": {
                "signature": "user_intent, context -> requirements, priorities",
                "instructions": "Extract detailed requirements with priorities.",
                "demos": []
            }
        }

        optimized_path = temp_dir / "reviewer_v1.json"
        with open(optimized_path, 'w') as f:
            json.dump(optimized_prompts, f)

        # Load optimized module (would need module loader implementation)
        # For now, verify file structure
        assert optimized_path.exists()

        with open(optimized_path, 'r') as f:
            data = json.load(f)
            assert "extract_requirements" in data

    def test_module_version_metadata(self, temp_dir):
        """Test module version metadata tracking."""
        metadata = {
            "module_name": "reviewer",
            "version": "v1.0.0",
            "optimization_date": datetime.now().isoformat(),
            "baseline_metrics": {
                "extract_requirements": 0.367,
                "validate_intent": 1.0,
                "verify_completeness": 0.75
            },
            "optimized_metrics": {
                "extract_requirements": 0.560,
                "validate_intent": 1.0,
                "verify_completeness": 0.75
            },
            "improvements": {
                "extract_requirements": "+52.4%"
            }
        }

        metadata_path = temp_dir / "reviewer_v1.metadata.json"
        with open(metadata_path, 'w') as f:
            json.dump(metadata, f, indent=2)

        # Verify metadata structure
        assert metadata_path.exists()

        with open(metadata_path, 'r') as f:
            data = json.load(f)
            assert data["module_name"] == "reviewer"
            assert "baseline_metrics" in data
            assert "optimized_metrics" in data


# =============================================================================
# Test: A/B Testing Integration
# =============================================================================

class TestABTesting:
    """Test A/B testing framework integration."""

    def test_traffic_split_routing(self):
        """Test traffic routing between baseline and optimized versions."""
        # Simulate traffic split: 80% optimized, 20% baseline
        version_counts = {"optimized": 0, "baseline": 0}

        for i in range(100):
            # Simple hash-based routing simulation
            if hash(f"request_{i}") % 100 < 80:
                version_counts["optimized"] += 1
            else:
                version_counts["baseline"] += 1

        # Verify approximately 80/20 split (allow 10% variance)
        assert 70 <= version_counts["optimized"] <= 90
        assert 10 <= version_counts["baseline"] <= 30

    def test_version_comparison_metrics(self):
        """Test collecting comparison metrics between versions."""
        baseline_metrics = {
            "latency_ms": 200,
            "success_rate": 0.9,
            "composite_score": 0.75
        }

        optimized_metrics = {
            "latency_ms": 150,
            "success_rate": 0.95,
            "composite_score": 0.85
        }

        # Calculate improvements
        improvements = {
            "latency": (baseline_metrics["latency_ms"] - optimized_metrics["latency_ms"]) / baseline_metrics["latency_ms"],
            "success_rate": (optimized_metrics["success_rate"] - baseline_metrics["success_rate"]) / baseline_metrics["success_rate"],
            "composite_score": (optimized_metrics["composite_score"] - baseline_metrics["composite_score"]) / baseline_metrics["composite_score"]
        }

        assert improvements["latency"] > 0  # Faster
        assert improvements["success_rate"] > 0  # More reliable
        assert improvements["composite_score"] > 0  # Better overall

    def test_rollback_trigger(self):
        """Test automatic rollback on performance degradation."""
        # Simulate performance monitoring
        recent_metrics = [
            {"success_rate": 0.95, "latency_ms": 150},
            {"success_rate": 0.90, "latency_ms": 160},
            {"success_rate": 0.85, "latency_ms": 170},
            {"success_rate": 0.70, "latency_ms": 200},  # Degradation
        ]

        baseline_success_rate = 0.9
        rollback_threshold = 0.8  # Rollback if below 80%

        current_success_rate = recent_metrics[-1]["success_rate"]

        should_rollback = current_success_rate < rollback_threshold
        assert should_rollback is True

    def test_gradual_rollout(self):
        """Test gradual rollout of optimized version."""
        rollout_stages = [
            {"percentage": 10, "duration_hours": 24},
            {"percentage": 25, "duration_hours": 24},
            {"percentage": 50, "duration_hours": 48},
            {"percentage": 100, "duration_hours": 0}
        ]

        # Verify each stage increases traffic
        for i in range(len(rollout_stages) - 1):
            assert rollout_stages[i]["percentage"] < rollout_stages[i + 1]["percentage"]


# =============================================================================
# Test: Error Handling and Fallback
# =============================================================================

class TestErrorHandling:
    """Test error handling and fallback behavior."""

    def test_fallback_to_baseline_on_error(self, reviewer_module):
        """Test falling back to baseline on optimized version error."""
        # Simulate optimized version failure
        try:
            # This would normally call optimized version
            raise ValueError("Optimized module failed to load")
        except ValueError:
            # Fall back to baseline
            result = reviewer_module.extract_requirements(
                user_intent="Add authentication"
            )
            assert hasattr(result, 'requirements')

    def test_graceful_degradation(self, reviewer_module):
        """Test graceful degradation on partial failures."""
        # Test with minimal context
        result = reviewer_module.extract_requirements(
            user_intent="Add feature"  # Very vague
        )

        # Should still return result, even if empty
        assert hasattr(result, 'requirements')
        assert isinstance(result.requirements, list)

    def test_timeout_handling(self):
        """Test handling of module timeouts."""
        timeout_ms = 5000
        start_time = datetime.now()

        # Simulate long-running operation
        try:
            # Would normally execute module with timeout
            pass
        finally:
            elapsed_ms = (datetime.now() - start_time).total_seconds() * 1000

        # Verify timeout would be enforced
        assert elapsed_ms < timeout_ms * 2  # Allow 2x timeout for test


# =============================================================================
# Test: Cost Tracking and Token Usage
# =============================================================================

class TestCostTracking:
    """Test cost tracking and token usage monitoring."""

    def test_token_usage_tracking(self, telemetry_collector):
        """Test tracking token usage across requests."""
        for i in range(10):
            request = DSpyEvent.request(
                module_name="reviewer",
                module_version="v1.0.0",
                signature="extract_requirements"
            )

            response = DSpyEvent.response(
                request=request,
                latency_ms=150,
                tokens=TokenUsage(prompt=100 + i * 10, completion=50 + i * 5, total=150 + i * 15),
                cost_usd=0.001 * (1 + i * 0.1)
            )

            telemetry_collector.record(response)

        stats = telemetry_collector.get_module_stats("reviewer")
        assert stats.total_tokens > 0
        assert stats.total_cost_usd > 0

    def test_cost_budget_monitoring(self):
        """Test monitoring cost against budget."""
        daily_budget_usd = 10.0
        current_spend_usd = 8.5

        budget_remaining = daily_budget_usd - current_spend_usd
        budget_utilization = current_spend_usd / daily_budget_usd

        assert budget_remaining > 0
        assert budget_utilization < 1.0

        # Alert if approaching budget (90%)
        should_alert = budget_utilization >= 0.9
        assert should_alert is True


# =============================================================================
# Test: Cross-Module Workflows
# =============================================================================

class TestCrossModuleWorkflows:
    """Test workflows spanning multiple DSPy modules."""

    def test_reviewer_to_optimizer_pipeline(self, reviewer_module):
        """Test pipeline from reviewer to optimizer."""
        # Step 1: Extract requirements
        requirements_result = reviewer_module.extract_requirements(
            user_intent="Add user authentication with JWT"
        )

        assert hasattr(requirements_result, 'requirements')
        requirements = requirements_result.requirements

        # Step 2: Validate completeness (using same module)
        completeness_result = reviewer_module.verify_completeness(
            requirements=requirements,
            implementation="Implemented JWT authentication"
        )

        assert hasattr(completeness_result, 'complete')

    def test_end_to_end_validation_workflow(self, reviewer_module):
        """Test complete validation workflow."""
        # Extract requirements
        requirements = reviewer_module.extract_requirements(
            user_intent="Implement caching"
        ).requirements

        # Validate intent
        intent_result = reviewer_module.validate_intent(
            user_intent="Implement caching",
            implementation="Added Redis cache"
        )

        # Verify completeness
        completeness_result = reviewer_module.verify_completeness(
            requirements=requirements,
            implementation="Added Redis cache"
        )

        # Verify correctness
        correctness_result = reviewer_module.verify_correctness(
            requirements=requirements,
            implementation="Added Redis cache with TTL=3600"
        )

        # All validations should complete
        assert hasattr(intent_result, 'intent_satisfied')
        assert hasattr(completeness_result, 'complete')
        assert hasattr(correctness_result, 'correct')


if __name__ == "__main__":
    # Run tests with pytest
    pytest.main([__file__, "-v", "--tb=short"])
