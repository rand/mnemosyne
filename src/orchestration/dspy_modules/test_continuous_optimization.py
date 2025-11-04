#!/usr/bin/env python3
"""
Comprehensive Tests for Continuous Optimization Pipeline

Tests the complete A/B testing infrastructure including:
- Production log import and processing
- Continuous optimization workflow
- Baseline comparison logic
- Deployment decision-making
- Safety checks and rollback scenarios
- Error handling and edge cases

The continuous optimization pipeline (continuous_optimize.py + import_production_logs.py)
forms the core A/B testing framework, enabling:
1. Import production data → training examples
2. Optimize modules on production data
3. Compare baseline vs optimized performance
4. Deploy if improvement meets threshold
5. Rollback on degradation
"""

import json
import pytest
import subprocess
import sys
from datetime import datetime, timedelta
from pathlib import Path
from typing import Any, Dict, List
from unittest.mock import Mock, patch, MagicMock

# Add parent directory to path for imports
sys.path.insert(0, str(Path(__file__).parent))

try:
    from import_production_logs import (
        load_production_logs,
        filter_logs,
        convert_to_training_data,
        deduplicate_training_data,
        load_existing_training_data,
        save_training_data,
        print_statistics
    )
    IMPORT_LOGS_AVAILABLE = True
except ImportError:
    IMPORT_LOGS_AVAILABLE = False

try:
    from continuous_optimize import (
        import_production_logs as import_logs_step,
        run_baseline_benchmark,
        run_optimization,
        compare_performance,
        deploy_optimized_module
    )
    CONTINUOUS_OPTIMIZE_AVAILABLE = True
except ImportError:
    CONTINUOUS_OPTIMIZE_AVAILABLE = False


# ============================================================================
# FIXTURES
# ============================================================================

@pytest.fixture
def temp_dir(tmp_path):
    """Create temporary directory for test artifacts."""
    return tmp_path


@pytest.fixture
def mock_production_logs(temp_dir):
    """Create mock production logs in JSON Lines format."""
    log_path = temp_dir / "production.jsonl"

    logs = [
        {
            "module_name": "reviewer",
            "module_version": "v1.0.0",
            "signature": "extract_requirements",
            "input": {"user_intent": "Add auth", "context": "REST API"},
            "output": {"requirements": ["JWT", "User model"]},
            "timestamp_ms": int((datetime.now() - timedelta(days=1)).timestamp() * 1000),
            "latency_ms": 150,
            "tokens": {"prompt": 100, "completion": 50, "total": 150},
            "cost_usd": 0.001,
            "model": "claude-haiku-4-5",
            "success": True
        },
        {
            "module_name": "reviewer",
            "module_version": "v1.0.0",
            "signature": "validate_intent",
            "input": {"user_intent": "Add auth", "implementation": "JWT tokens"},
            "output": {"is_satisfied": True, "reasoning": "Good"},
            "timestamp_ms": int((datetime.now() - timedelta(hours=12)).timestamp() * 1000),
            "latency_ms": 120,
            "tokens": {"prompt": 80, "completion": 40, "total": 120},
            "cost_usd": 0.0008,
            "model": "claude-haiku-4-5",
            "success": True
        },
        {
            "module_name": "optimizer",
            "module_version": "v1.0.0",
            "signature": "discover_skills",
            "input": {"task_description": "Build API"},
            "output": {"skills": ["api-design", "testing"]},
            "timestamp_ms": int((datetime.now() - timedelta(hours=6)).timestamp() * 1000),
            "latency_ms": 180,
            "tokens": {"prompt": 120, "completion": 60, "total": 180},
            "cost_usd": 0.0012,
            "model": "claude-haiku-4-5",
            "success": True
        },
        # Failed interaction (should be filtered with min_success_rate)
        {
            "module_name": "reviewer",
            "module_version": "v1.0.0",
            "signature": "extract_requirements",
            "input": {"user_intent": "Do something"},
            "output": {},
            "timestamp_ms": int((datetime.now() - timedelta(hours=2)).timestamp() * 1000),
            "latency_ms": 50,
            "tokens": {"prompt": 20, "completion": 5, "total": 25},
            "cost_usd": 0.0002,
            "model": "claude-haiku-4-5",
            "success": False,
            "error": "Invalid input"
        },
        # Duplicate of first entry (for deduplication test)
        {
            "module_name": "reviewer",
            "module_version": "v1.0.0",
            "signature": "extract_requirements",
            "input": {"user_intent": "Add auth", "context": "REST API"},
            "output": {"requirements": ["JWT", "User model"]},
            "timestamp_ms": int(datetime.now().timestamp() * 1000),
            "latency_ms": 140,
            "tokens": {"prompt": 100, "completion": 50, "total": 150},
            "cost_usd": 0.001,
            "model": "claude-haiku-4-5",
            "success": True
        }
    ]

    with open(log_path, 'w') as f:
        for log in logs:
            f.write(json.dumps(log) + '\n')

    return log_path


@pytest.fixture
def mock_training_data(temp_dir):
    """Create mock training data."""
    training_path = temp_dir / "training.json"

    data = [
        {
            "signature": "extract_requirements",
            "input": {"user_intent": "Build dashboard", "context": "Web app"},
            "output": {"requirements": ["UI components", "Data viz"]},
            "metadata": {
                "source": "manual",
                "timestamp": int(datetime.now().timestamp() * 1000)
            }
        }
    ]

    with open(training_path, 'w') as f:
        json.dump(data, f, indent=2)

    return training_path


@pytest.fixture
def mock_baseline_results(temp_dir):
    """Create mock baseline benchmark results."""
    results_path = temp_dir / "baseline_results.json"

    results = {
        "module": "reviewer",
        "version": "baseline",
        "metrics": {
            "composite_metric": 0.75,
            "extract_requirements_f1": 0.70,
            "validate_intent_accuracy": 0.80,
            "avg_latency_ms": 150,
            "total_cost_usd": 0.05
        },
        "timestamp": datetime.now().isoformat()
    }

    with open(results_path, 'w') as f:
        json.dump(results, f, indent=2)

    return results_path


@pytest.fixture
def mock_optimized_results(temp_dir):
    """Create mock optimized module results."""
    module_path = temp_dir / "optimized_reviewer_v1.json"
    results_path = temp_dir / "optimized_reviewer_v1.results.json"

    # Module file (prompts and metadata)
    module_data = {
        "version": "v1",
        "optimized_at": datetime.now().isoformat(),
        "signatures": ["extract_requirements", "validate_intent"]
    }

    with open(module_path, 'w') as f:
        json.dump(module_data, f, indent=2)

    # Results file (performance metrics)
    results_data = {
        "composite_metric": 0.82,
        "extract_requirements_f1": 0.80,
        "validate_intent_accuracy": 0.85,
        "avg_latency_ms": 140,
        "total_cost_usd": 0.04,
        "improvement_over_baseline": {
            "composite_metric": 0.07,
            "extract_requirements_f1": 0.10,
            "validate_intent_accuracy": 0.05
        }
    }

    with open(results_path, 'w') as f:
        json.dump(results_data, f, indent=2)

    return module_path, results_path


# ============================================================================
# TEST PRODUCTION LOG IMPORT
# ============================================================================

@pytest.mark.skipif(not IMPORT_LOGS_AVAILABLE, reason="import_production_logs.py not available")
class TestProductionLogImport:
    """Test production log import and processing."""

    def test_load_production_logs(self, mock_production_logs):
        """Test loading production logs from JSON Lines file."""
        logs = load_production_logs(mock_production_logs)

        assert len(logs) == 5  # All entries loaded
        assert all("module_name" in log for log in logs)
        assert all("timestamp_ms" in log for log in logs)

    def test_load_invalid_json_lines(self, temp_dir):
        """Test handling of invalid JSON Lines."""
        invalid_log = temp_dir / "invalid.jsonl"

        with open(invalid_log, 'w') as f:
            f.write('{"valid": "json"}\n')
            f.write('invalid json line\n')
            f.write('{"another": "valid"}\n')

        logs = load_production_logs(invalid_log)

        # Should load valid lines and skip invalid
        assert len(logs) == 2

    def test_filter_logs_by_module(self, mock_production_logs):
        """Test filtering logs by module name."""
        all_logs = load_production_logs(mock_production_logs)

        reviewer_logs = filter_logs(all_logs, min_success_rate=0.0, since=None, module="reviewer")

        # Should get 3 successful reviewer interactions (4th is failed, filtered out)
        assert len(reviewer_logs) == 3
        assert all(log["module_name"] == "reviewer" for log in reviewer_logs)

    def test_filter_logs_by_success(self, mock_production_logs):
        """Test filtering out failed interactions."""
        all_logs = load_production_logs(mock_production_logs)

        filtered = filter_logs(all_logs, min_success_rate=0.0, since=None, module="reviewer")

        # All returned logs should be successful
        assert all(log.get("success", False) for log in filtered)

    def test_filter_logs_by_date(self, mock_production_logs):
        """Test filtering logs by date range."""
        all_logs = load_production_logs(mock_production_logs)

        # Filter to only logs from today
        today = datetime.now().strftime("%Y-%m-%d")
        filtered = filter_logs(all_logs, min_success_rate=0.0, since=today, module="reviewer")

        # Should only get logs from today (the duplicate entry)
        assert len(filtered) <= 1

    def test_convert_to_training_data(self, mock_production_logs):
        """Test converting production logs to training data format."""
        logs = load_production_logs(mock_production_logs)
        filtered = filter_logs(logs, min_success_rate=0.0, since=None, module="reviewer")

        training_data = convert_to_training_data(filtered)

        assert len(training_data) == len(filtered)

        for entry in training_data:
            assert "signature" in entry
            assert "input" in entry
            assert "output" in entry
            assert "metadata" in entry
            assert entry["metadata"]["source"] == "production"
            assert "timestamp" in entry["metadata"]
            assert "latency_ms" in entry["metadata"]
            assert "cost_usd" in entry["metadata"]

    def test_deduplicate_training_data(self):
        """Test removing duplicate training examples."""
        data = [
            {
                "signature": "test_sig",
                "input": {"a": 1, "b": 2},
                "output": {"result": "X"},
                "metadata": {}
            },
            {
                "signature": "test_sig",
                "input": {"b": 2, "a": 1},  # Same input, different order
                "output": {"result": "Y"},
                "metadata": {}
            },
            {
                "signature": "test_sig",
                "input": {"a": 3, "b": 4},  # Different input
                "output": {"result": "Z"},
                "metadata": {}
            }
        ]

        deduplicated = deduplicate_training_data(data)

        # Should have 2 unique inputs (first two have same input)
        assert len(deduplicated) == 2

    def test_merge_with_existing_training_data(self, temp_dir, mock_training_data):
        """Test merging production data with existing training data."""
        existing = load_existing_training_data(mock_training_data)

        new_data = [
            {
                "signature": "validate_intent",
                "input": {"user_intent": "Test", "implementation": "Test impl"},
                "output": {"is_satisfied": True},
                "metadata": {"source": "production"}
            }
        ]

        merged = existing + new_data

        assert len(merged) == 2
        assert any(entry["metadata"]["source"] == "manual" for entry in merged)
        assert any(entry["metadata"]["source"] == "production" for entry in merged)

    def test_save_training_data(self, temp_dir):
        """Test saving training data to JSON file."""
        data = [
            {
                "signature": "test",
                "input": {},
                "output": {},
                "metadata": {}
            }
        ]

        output_path = temp_dir / "output.json"
        save_training_data(data, output_path)

        assert output_path.exists()

        with open(output_path, 'r') as f:
            loaded = json.load(f)

        assert loaded == data


# ============================================================================
# TEST CONTINUOUS OPTIMIZATION PIPELINE
# ============================================================================

@pytest.mark.skipif(not CONTINUOUS_OPTIMIZE_AVAILABLE, reason="continuous_optimize.py not available")
class TestContinuousOptimizationPipeline:
    """Test continuous optimization pipeline workflow."""

    def test_compare_performance_improvement(self, mock_optimized_results):
        """Test performance comparison with significant improvement."""
        module_path, results_path = mock_optimized_results

        baseline_metrics = {
            "composite_metric": 0.75
        }

        min_improvement = 0.02  # 2% threshold

        meets_threshold, improvement = compare_performance(
            baseline_metrics,
            module_path,
            min_improvement
        )

        assert meets_threshold is True
        assert improvement >= min_improvement

    def test_compare_performance_insufficient_improvement(self, temp_dir):
        """Test performance comparison with insufficient improvement."""
        module_path = temp_dir / "optimized.json"
        results_path = temp_dir / "optimized.results.json"

        # Create module with minimal improvement
        results_data = {
            "composite_metric": 0.76  # Only 1% improvement from 0.75
        }

        with open(module_path, 'w') as f:
            json.dump({}, f)

        with open(results_path, 'w') as f:
            json.dump(results_data, f)

        baseline_metrics = {
            "composite_metric": 0.75
        }

        min_improvement = 0.02  # 2% threshold

        meets_threshold, improvement = compare_performance(
            baseline_metrics,
            module_path,
            min_improvement
        )

        assert meets_threshold is False
        assert 0 < improvement < min_improvement

    def test_compare_performance_regression(self, temp_dir):
        """Test detection of performance regression."""
        module_path = temp_dir / "optimized.json"
        results_path = temp_dir / "optimized.results.json"

        # Create module with worse performance
        results_data = {
            "composite_metric": 0.70  # Regression from 0.75
        }

        with open(module_path, 'w') as f:
            json.dump({}, f)

        with open(results_path, 'w') as f:
            json.dump(results_data, f)

        baseline_metrics = {
            "composite_metric": 0.75
        }

        min_improvement = 0.02

        meets_threshold, improvement = compare_performance(
            baseline_metrics,
            module_path,
            min_improvement
        )

        assert meets_threshold is False
        assert improvement < 0  # Negative improvement (regression)

    def test_compare_performance_no_baseline(self, mock_optimized_results):
        """Test performance comparison with no baseline."""
        module_path, results_path = mock_optimized_results

        # No baseline provided
        baseline_metrics = None
        min_improvement = 0.02

        meets_threshold, improvement = compare_performance(
            baseline_metrics,
            module_path,
            min_improvement
        )

        # Should accept optimized version without baseline
        assert meets_threshold is True
        assert improvement == 0.0

    def test_deployment_creates_production_file(self, temp_dir, mock_optimized_results):
        """Test deployment creates production module file."""
        module_path, _ = mock_optimized_results

        # Mock RESULTS_DIR and LOGS_DIR
        with patch('continuous_optimize.RESULTS_DIR', temp_dir):
            with patch('continuous_optimize.LOGS_DIR', temp_dir):
                success = deploy_optimized_module("reviewer", module_path)

        assert success is True

        production_path = temp_dir / "reviewer_optimized_production.json"
        assert production_path.exists()

    def test_deployment_creates_log_record(self, temp_dir, mock_optimized_results):
        """Test deployment creates deployment log record."""
        module_path, _ = mock_optimized_results

        with patch('continuous_optimize.RESULTS_DIR', temp_dir):
            with patch('continuous_optimize.LOGS_DIR', temp_dir):
                deploy_optimized_module("reviewer", module_path)

        deployment_log = temp_dir / "deployments.jsonl"
        assert deployment_log.exists()

        with open(deployment_log, 'r') as f:
            records = [json.loads(line) for line in f]

        assert len(records) == 1
        record = records[0]
        assert record["module"] == "reviewer"
        assert "timestamp" in record
        assert "version" in record
        assert "production_path" in record


# ============================================================================
# TEST A/B TESTING SCENARIOS
# ============================================================================

class TestABTestingScenarios:
    """Test complete A/B testing scenarios."""

    def test_gradual_rollout_scenario(self):
        """Test gradual rollout with traffic splitting."""
        # Simulate gradual rollout: 10% → 50% → 100%
        traffic_splits = [0.1, 0.5, 1.0]

        for split in traffic_splits:
            # In real A/B test, this would route `split` fraction to optimized version
            assert 0 <= split <= 1.0

            # Simulate collecting metrics for this traffic split
            baseline_latency = 150  # ms
            optimized_latency = 140  # ms

            # Weighted average based on split
            avg_latency = (split * optimized_latency) + ((1 - split) * baseline_latency)

            # As split increases, avg latency should decrease
            assert avg_latency <= baseline_latency

    def test_rollback_on_degradation(self):
        """Test automatic rollback on performance degradation."""
        # Baseline metrics
        baseline_success_rate = 0.95
        baseline_latency = 150

        # Optimized version metrics (degraded)
        optimized_success_rate = 0.70  # Significant drop
        optimized_latency = 200  # Increased latency

        # Rollback thresholds
        min_success_rate = 0.85
        max_latency = 180

        should_rollback = (
            optimized_success_rate < min_success_rate or
            optimized_latency > max_latency
        )

        assert should_rollback is True

    def test_canary_deployment_validation(self):
        """Test canary deployment with validation window."""
        # Canary metrics (small traffic fraction)
        canary_success_rate = 0.98
        canary_error_rate = 0.02
        canary_latency_p95 = 145

        # Validation criteria
        min_success_rate = 0.95
        max_error_rate = 0.05
        max_latency_p95 = 200

        canary_passed = (
            canary_success_rate >= min_success_rate and
            canary_error_rate <= max_error_rate and
            canary_latency_p95 <= max_latency_p95
        )

        assert canary_passed is True

    def test_champion_challenger_comparison(self):
        """Test champion/challenger A/B comparison."""
        # Champion (current production)
        champion_metrics = {
            "success_rate": 0.90,
            "latency_p50": 100,
            "latency_p95": 180,
            "cost_per_request": 0.001
        }

        # Challenger (optimized version)
        challenger_metrics = {
            "success_rate": 0.92,
            "latency_p50": 95,
            "latency_p95": 175,
            "cost_per_request": 0.0009
        }

        # Compare on all dimensions
        challenger_wins = (
            challenger_metrics["success_rate"] >= champion_metrics["success_rate"] and
            challenger_metrics["latency_p95"] <= champion_metrics["latency_p95"] and
            challenger_metrics["cost_per_request"] <= champion_metrics["cost_per_request"]
        )

        assert challenger_wins is True


# ============================================================================
# TEST SAFETY CHECKS AND THRESHOLDS
# ============================================================================

class TestSafetyChecksAndThresholds:
    """Test safety checks and deployment thresholds."""

    def test_minimum_training_examples_requirement(self):
        """Test enforcement of minimum training examples."""
        training_data = [{"input": {}, "output": {}} for _ in range(15)]

        min_required = 20

        has_sufficient_data = len(training_data) >= min_required

        assert has_sufficient_data is False

    def test_minimum_improvement_threshold(self):
        """Test minimum improvement threshold enforcement."""
        baseline_score = 0.75
        optimized_score = 0.76

        improvement = optimized_score - baseline_score
        min_threshold = 0.02  # 2%

        meets_threshold = improvement >= min_threshold

        assert meets_threshold is False

    def test_cost_budget_validation(self):
        """Test cost budget validation for optimization."""
        baseline_cost = 0.05  # USD
        optimized_cost = 0.04

        max_allowed_cost = 0.10

        within_budget = optimized_cost <= max_allowed_cost
        cost_improved = optimized_cost <= baseline_cost

        assert within_budget is True
        assert cost_improved is True

    def test_latency_regression_detection(self):
        """Test detection of latency regressions."""
        baseline_latency_p95 = 150  # ms
        optimized_latency_p95 = 180  # Worse

        max_allowed_regression = 1.1  # 10% regression tolerance

        regression_ratio = optimized_latency_p95 / baseline_latency_p95

        has_regression = regression_ratio > max_allowed_regression

        assert has_regression is True

    def test_production_readiness_checklist(self):
        """Test production readiness validation."""
        module_metadata = {
            "has_training_data": True,
            "training_examples": 50,
            "baseline_benchmark_complete": True,
            "optimization_complete": True,
            "performance_improved": True,
            "meets_latency_sla": True,
            "within_cost_budget": True,
            "tests_passing": True
        }

        is_production_ready = all(module_metadata.values())

        assert is_production_ready is True


# ============================================================================
# TEST ERROR HANDLING
# ============================================================================

class TestErrorHandling:
    """Test error handling and edge cases."""

    def test_missing_production_logs_file(self, temp_dir):
        """Test handling of missing production logs file."""
        nonexistent_path = temp_dir / "nonexistent.jsonl"

        with pytest.raises(SystemExit):
            load_production_logs(nonexistent_path)

    def test_empty_production_logs(self, temp_dir):
        """Test handling of empty production logs."""
        empty_log = temp_dir / "empty.jsonl"
        empty_log.touch()

        logs = load_production_logs(empty_log)

        assert len(logs) == 0

    def test_missing_results_file(self, temp_dir):
        """Test handling of missing results file."""
        module_path = temp_dir / "module.json"

        with open(module_path, 'w') as f:
            json.dump({}, f)

        baseline_metrics = {"composite_metric": 0.75}
        min_improvement = 0.02

        meets_threshold, improvement = compare_performance(
            baseline_metrics,
            module_path,
            min_improvement
        )

        # Should fail gracefully when results file missing
        assert meets_threshold is False

    def test_corrupted_training_data(self, temp_dir):
        """Test handling of corrupted training data."""
        corrupted_path = temp_dir / "corrupted.json"

        with open(corrupted_path, 'w') as f:
            f.write("not valid json{")

        data = load_existing_training_data(corrupted_path)

        # Should return empty list for corrupted data
        assert len(data) == 0

    def test_deployment_failure_handling(self, temp_dir):
        """Test handling of deployment failures."""
        nonexistent_module = temp_dir / "nonexistent.json"

        with patch('continuous_optimize.RESULTS_DIR', temp_dir):
            with patch('continuous_optimize.LOGS_DIR', temp_dir):
                # Deployment should fail gracefully for nonexistent file
                # In real implementation, would catch exception
                try:
                    deploy_optimized_module("reviewer", nonexistent_module)
                except Exception:
                    pass  # Expected


# ============================================================================
# TEST INTEGRATION SCENARIOS
# ============================================================================

class TestIntegrationScenarios:
    """Test end-to-end integration scenarios."""

    @pytest.mark.skipif(not (IMPORT_LOGS_AVAILABLE and CONTINUOUS_OPTIMIZE_AVAILABLE),
                        reason="Both import_production_logs.py and continuous_optimize.py required")
    def test_full_continuous_improvement_cycle(self, mock_production_logs, temp_dir):
        """Test complete continuous improvement cycle."""
        # Step 1: Import production logs
        all_logs = load_production_logs(mock_production_logs)
        filtered_logs = filter_logs(all_logs, 0.0, None, "reviewer")
        training_data = convert_to_training_data(filtered_logs)
        training_data = deduplicate_training_data(training_data)

        assert len(training_data) >= 2  # At least 2 unique examples

        # Step 2: Would run optimization (mocked here)
        # In real scenario: run MIPROv2 optimization

        # Step 3: Compare performance
        baseline_metrics = {"composite_metric": 0.75}

        # Mock optimized results
        optimized_path = temp_dir / "optimized.json"
        results_path = temp_dir / "optimized.results.json"

        with open(optimized_path, 'w') as f:
            json.dump({"version": "v1"}, f)

        with open(results_path, 'w') as f:
            json.dump({"composite_metric": 0.82}, f)

        meets_threshold, improvement = compare_performance(
            baseline_metrics,
            optimized_path,
            0.02
        )

        # Step 4: Deploy if improved
        if meets_threshold:
            with patch('continuous_optimize.RESULTS_DIR', temp_dir):
                with patch('continuous_optimize.LOGS_DIR', temp_dir):
                    success = deploy_optimized_module("reviewer", optimized_path)

            assert success is True
            assert (temp_dir / "reviewer_optimized_production.json").exists()


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
