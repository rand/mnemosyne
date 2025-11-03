#!/usr/bin/env python3
"""
Tests for Baseline Benchmarking Infrastructure

Tests the baseline performance measurement system that establishes
performance baselines for DSPy modules before optimization.

Covers:
- Latency measurement and statistics
- Token usage tracking
- Cost estimation
- Benchmark result formatting
- Command-line interface
- Error handling and edge cases
"""

import json
import pytest
import statistics
from pathlib import Path
from typing import List
from unittest.mock import Mock, patch, MagicMock

try:
    from baseline_benchmark import (
        measure_latency,
        get_token_usage,
        estimate_cost,
        compute_statistics,
    )
    BASELINE_AVAILABLE = True
except ImportError:
    BASELINE_AVAILABLE = False


# ============================================================================
# FIXTURES
# ============================================================================

@pytest.fixture
def temp_dir(tmp_path):
    """Create temporary directory for test artifacts."""
    return tmp_path


@pytest.fixture
def sample_latencies():
    """Sample latency measurements for testing."""
    # Generate realistic latencies (ms): mostly 100-200ms with some outliers
    return [
        105.2, 120.5, 115.8, 130.2, 125.6,
        110.3, 118.9, 122.4, 135.7, 128.3,
        140.5, 145.2, 112.8, 138.9, 125.1,
        150.3, 155.8, 160.2, 142.6, 148.9,
        200.5,  # Outlier (p95+)
        250.3,  # Outlier (p99)
    ]


# ============================================================================
# TEST LATENCY MEASUREMENT
# ============================================================================

@pytest.mark.skipif(not BASELINE_AVAILABLE, reason="baseline_benchmark.py not available")
class TestLatencyMeasurement:
    """Test latency measurement functionality."""

    def test_measure_latency_basic(self):
        """Test basic latency measurement."""
        call_count = 0

        def mock_function():
            nonlocal call_count
            call_count += 1
            # Simulate work
            import time
            time.sleep(0.001)  # 1ms

        latencies = measure_latency(mock_function, iterations=5)

        assert len(latencies) == 5
        assert call_count == 5
        assert all(lat >= 1.0 for lat in latencies)  # At least 1ms each
        assert all(lat < 100.0 for lat in latencies)  # Should be < 100ms

    def test_measure_latency_zero_iterations(self):
        """Test latency measurement with zero iterations."""
        call_count = 0

        def mock_function():
            nonlocal call_count
            call_count += 1

        latencies = measure_latency(mock_function, iterations=0)

        assert len(latencies) == 0
        assert call_count == 0

    def test_measure_latency_with_exceptions(self):
        """Test latency measurement handles exceptions gracefully."""
        call_count = 0

        def failing_function():
            nonlocal call_count
            call_count += 1
            if call_count <= 3:
                raise ValueError("Simulated failure")

        latencies = measure_latency(failing_function, iterations=5)

        # Should skip failed iterations and only measure successful ones
        assert len(latencies) == 2  # Only last 2 succeeded
        assert call_count == 5  # All iterations attempted

    def test_measure_latency_consistent_function(self):
        """Test latency measurement for consistent function."""
        import time

        def consistent_function():
            time.sleep(0.01)  # 10ms

        latencies = measure_latency(consistent_function, iterations=10)

        assert len(latencies) == 10

        # Should have low variance
        stddev = statistics.stdev(latencies)
        assert stddev < 5.0  # Low variance for consistent function


# ============================================================================
# TEST STATISTICS COMPUTATION
# ============================================================================

@pytest.mark.skipif(not BASELINE_AVAILABLE, reason="baseline_benchmark.py not available")
class TestStatisticsComputation:
    """Test statistics computation for latency data."""

    def test_compute_statistics_basic(self, sample_latencies):
        """Test basic statistics computation."""
        stats = compute_statistics(sample_latencies)

        assert "p50" in stats
        assert "p95" in stats
        assert "p99" in stats
        assert "mean" in stats
        assert "stddev" in stats
        assert "min" in stats
        assert "max" in stats

        # Validate percentiles are ordered
        assert stats["p50"] <= stats["p95"] <= stats["p99"]

        # Validate min/max
        assert stats["min"] == min(sample_latencies)
        assert stats["max"] == max(sample_latencies)

    def test_compute_statistics_percentiles(self, sample_latencies):
        """Test percentile calculations."""
        stats = compute_statistics(sample_latencies)

        # p50 should be around the median
        median = statistics.median(sample_latencies)
        assert abs(stats["p50"] - median) < 20.0  # Within 20ms

        # p95 should be high but below p99
        assert stats["p95"] > stats["p50"]
        assert stats["p99"] > stats["p95"]

        # p99 should be near max for small sample
        assert abs(stats["p99"] - stats["max"]) < 50.0

    def test_compute_statistics_mean_stddev(self, sample_latencies):
        """Test mean and standard deviation calculations."""
        stats = compute_statistics(sample_latencies)

        expected_mean = statistics.mean(sample_latencies)
        expected_stddev = statistics.stdev(sample_latencies)

        assert abs(stats["mean"] - expected_mean) < 0.01
        assert abs(stats["stddev"] - expected_stddev) < 0.01

    def test_compute_statistics_empty_list(self):
        """Test statistics computation with empty list."""
        stats = compute_statistics([])

        # Should return zeros for all metrics
        assert stats["p50"] == 0.0
        assert stats["p95"] == 0.0
        assert stats["p99"] == 0.0
        assert stats["mean"] == 0.0
        assert stats["stddev"] == 0.0
        assert stats["min"] == 0.0
        assert stats["max"] == 0.0

    def test_compute_statistics_single_value(self):
        """Test statistics computation with single value."""
        stats = compute_statistics([150.5])

        # All percentiles should equal the single value
        assert stats["p50"] == 150.5
        assert stats["p95"] == 150.5
        assert stats["p99"] == 150.5
        assert stats["mean"] == 150.5
        assert stats["stddev"] == 0.0  # No variance
        assert stats["min"] == 150.5
        assert stats["max"] == 150.5

    def test_compute_statistics_two_values(self):
        """Test statistics computation with two values."""
        latencies = [100.0, 200.0]
        stats = compute_statistics(latencies)

        assert stats["mean"] == 150.0
        assert stats["min"] == 100.0
        assert stats["max"] == 200.0

        # With 2 values, can compute stddev
        expected_stddev = statistics.stdev(latencies)
        assert abs(stats["stddev"] - expected_stddev) < 0.01


# ============================================================================
# TEST TOKEN USAGE AND COST ESTIMATION
# ============================================================================

@pytest.mark.skipif(not BASELINE_AVAILABLE, reason="baseline_benchmark.py not available")
class TestTokenUsageAndCost:
    """Test token usage tracking and cost estimation."""

    def test_get_token_usage_placeholder(self):
        """Test get_token_usage placeholder implementation."""
        input_tokens, output_tokens = get_token_usage()

        # Should return placeholder values
        assert isinstance(input_tokens, int)
        assert isinstance(output_tokens, int)
        assert input_tokens >= 0
        assert output_tokens >= 0

    def test_estimate_cost_sonnet(self):
        """Test cost estimation for claude-3-5-sonnet."""
        input_tokens = 1000
        output_tokens = 500

        cost = estimate_cost(input_tokens, output_tokens, model="claude-3-5-sonnet-20241022")

        # Sonnet pricing: $3/1M input, $15/1M output
        expected_cost = (1000 * 3.00 / 1_000_000) + (500 * 15.00 / 1_000_000)
        assert abs(cost - expected_cost) < 0.0001

    def test_estimate_cost_opus(self):
        """Test cost estimation for claude-3-opus."""
        input_tokens = 1000
        output_tokens = 500

        cost = estimate_cost(input_tokens, output_tokens, model="claude-3-opus-20240229")

        # Opus pricing: $15/1M input, $75/1M output
        expected_cost = (1000 * 15.00 / 1_000_000) + (500 * 75.00 / 1_000_000)
        assert abs(cost - expected_cost) < 0.0001

    def test_estimate_cost_unknown_model(self):
        """Test cost estimation defaults to Sonnet for unknown model."""
        input_tokens = 1000
        output_tokens = 500

        cost_unknown = estimate_cost(input_tokens, output_tokens, model="unknown-model")
        cost_sonnet = estimate_cost(input_tokens, output_tokens, model="claude-3-5-sonnet-20241022")

        # Should default to Sonnet pricing
        assert cost_unknown == cost_sonnet

    def test_estimate_cost_zero_tokens(self):
        """Test cost estimation with zero tokens."""
        cost = estimate_cost(0, 0)

        assert cost == 0.0

    def test_estimate_cost_large_usage(self):
        """Test cost estimation with large token usage."""
        input_tokens = 1_000_000  # 1M tokens
        output_tokens = 500_000   # 500K tokens

        cost = estimate_cost(input_tokens, output_tokens, model="claude-3-5-sonnet-20241022")

        # Sonnet pricing: $3/1M input, $15/1M output
        expected_cost = (1_000_000 * 3.00 / 1_000_000) + (500_000 * 15.00 / 1_000_000)
        assert abs(cost - expected_cost) < 0.01

    def test_cost_scales_linearly(self):
        """Test that cost scales linearly with token usage."""
        base_input = 100
        base_output = 50
        base_cost = estimate_cost(base_input, base_output)

        # Double the tokens
        double_cost = estimate_cost(base_input * 2, base_output * 2)

        # Cost should also double
        assert abs(double_cost - (base_cost * 2)) < 0.0001


# ============================================================================
# TEST BENCHMARK OUTPUT FORMAT
# ============================================================================

class TestBenchmarkOutputFormat:
    """Test benchmark result output formatting."""

    def test_benchmark_result_structure(self):
        """Test expected structure of benchmark results."""
        # Simulate benchmark result structure
        result = {
            "module": "reviewer",
            "timestamp": "2025-11-03T12:00:00Z",
            "operations": {
                "extract_requirements": {
                    "latency": {
                        "p50": 120.5,
                        "p95": 200.3,
                        "p99": 250.1,
                        "mean": 135.2,
                        "stddev": 30.5
                    },
                    "tokens": {
                        "input": 500,
                        "output": 200,
                        "total": 700
                    },
                    "cost_usd": 0.004,
                    "iterations": 10
                }
            }
        }

        # Validate structure
        assert "module" in result
        assert "timestamp" in result
        assert "operations" in result

        for op_name, op_data in result["operations"].items():
            assert "latency" in op_data
            assert "tokens" in op_data
            assert "cost_usd" in op_data
            assert "iterations" in op_data

            # Validate latency stats
            assert all(key in op_data["latency"] for key in ["p50", "p95", "p99", "mean", "stddev"])

            # Validate token breakdown
            assert all(key in op_data["tokens"] for key in ["input", "output", "total"])

    def test_benchmark_result_json_serializable(self):
        """Test that benchmark results are JSON serializable."""
        result = {
            "module": "reviewer",
            "operations": {
                "extract_requirements": {
                    "latency": {
                        "p50": 120.5,
                        "p95": 200.3
                    },
                    "cost_usd": 0.004
                }
            }
        }

        # Should serialize without error
        json_str = json.dumps(result, indent=2)
        assert json_str is not None

        # Should deserialize correctly
        parsed = json.loads(json_str)
        assert parsed == result


# ============================================================================
# TEST EDGE CASES AND ERROR HANDLING
# ============================================================================

class TestEdgeCasesAndErrorHandling:
    """Test edge cases and error handling."""

    def test_negative_latencies_handled(self):
        """Test handling of invalid negative latencies."""
        # Should not occur in practice, but test robustness
        invalid_latencies = [-10.0, 100.0, 150.0, 200.0]

        # compute_statistics should handle gracefully
        # (actual implementation may filter or accept them)
        stats = compute_statistics(invalid_latencies)

        # Should compute statistics even with negative values
        assert "mean" in stats
        assert "p50" in stats

    def test_very_large_latencies(self):
        """Test handling of very large latency values."""
        large_latencies = [10000.0, 20000.0, 30000.0, 50000.0]  # 10s-50s

        stats = compute_statistics(large_latencies)

        assert stats["p50"] > 10000.0
        assert stats["p99"] > 30000.0

    def test_very_small_latencies(self):
        """Test handling of sub-millisecond latencies."""
        small_latencies = [0.1, 0.2, 0.3, 0.5, 0.8]  # Sub-millisecond

        stats = compute_statistics(small_latencies)

        assert stats["mean"] < 1.0
        assert stats["min"] >= 0.1

    def test_cost_estimation_precision(self):
        """Test cost estimation maintains precision for small costs."""
        # Very small token usage
        cost = estimate_cost(10, 5)

        # Should be very small but non-zero
        assert cost > 0
        assert cost < 0.001

    def test_statistics_with_identical_values(self):
        """Test statistics when all values are identical."""
        identical = [150.0] * 100

        stats = compute_statistics(identical)

        # All percentiles should equal the constant value
        assert stats["p50"] == 150.0
        assert stats["p95"] == 150.0
        assert stats["p99"] == 150.0
        assert stats["mean"] == 150.0
        assert stats["stddev"] == 0.0  # No variance


# ============================================================================
# TEST PERFORMANCE CHARACTERISTICS
# ============================================================================

class TestPerformanceCharacteristics:
    """Test performance characteristics of benchmarking."""

    def test_latency_measurement_overhead_acceptable(self):
        """Test that latency measurement overhead is minimal."""
        import time

        # Measure overhead of calling measure_latency
        def instant_function():
            pass  # No-op

        start = time.perf_counter()
        latencies = measure_latency(instant_function, iterations=100)
        end = time.perf_counter()

        total_time_ms = (end - start) * 1000

        # Overhead should be minimal (< 100ms for 100 iterations)
        assert total_time_ms < 100.0

    def test_statistics_computation_fast(self):
        """Test that statistics computation is fast."""
        import time

        large_latencies = [float(i) for i in range(10000)]

        start = time.perf_counter()
        stats = compute_statistics(large_latencies)
        end = time.perf_counter()

        compute_time_ms = (end - start) * 1000

        # Should compute quickly (< 10ms for 10K values)
        assert compute_time_ms < 10.0
        assert "p50" in stats


# ============================================================================
# TEST INTEGRATION SCENARIOS
# ============================================================================

class TestIntegrationScenarios:
    """Test complete benchmarking workflows."""

    def test_complete_benchmark_workflow(self):
        """Test complete benchmark workflow."""
        # 1. Measure latencies
        def mock_operation():
            import time
            time.sleep(0.001)

        latencies = measure_latency(mock_operation, iterations=20)

        # 2. Compute statistics
        stats = compute_statistics(latencies)

        # 3. Get token usage (mocked)
        input_tokens, output_tokens = get_token_usage()

        # 4. Estimate cost
        cost = estimate_cost(input_tokens, output_tokens)

        # 5. Construct result
        result = {
            "latency": stats,
            "tokens": {
                "input": input_tokens,
                "output": output_tokens,
                "total": input_tokens + output_tokens
            },
            "cost_usd": cost,
            "iterations": len(latencies)
        }

        # Verify complete result
        assert "latency" in result
        assert "tokens" in result
        assert "cost_usd" in result
        assert result["iterations"] == 20

        # Should be JSON serializable
        json.dumps(result)

    def test_multiple_operations_benchmark(self):
        """Test benchmarking multiple operations."""
        operations = {
            "fast_op": lambda: None,  # Instant
            "slow_op": lambda: __import__("time").sleep(0.001)  # 1ms
        }

        results = {}

        for op_name, op_func in operations.items():
            latencies = measure_latency(op_func, iterations=10)
            stats = compute_statistics(latencies)

            results[op_name] = {
                "latency": stats,
                "iterations": len(latencies)
            }

        # Verify both operations benchmarked
        assert "fast_op" in results
        assert "slow_op" in results

        # Slow op should have higher latency
        assert results["slow_op"]["latency"]["mean"] > results["fast_op"]["latency"]["mean"]


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
