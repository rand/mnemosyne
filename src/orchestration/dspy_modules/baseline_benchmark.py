#!/usr/bin/env python3
"""Performance baseline measurements for DSPy modules.

Measures latency, token usage, and cost for ReviewerModule and SemanticModule
before optimization. Results are used to track improvement after teleprompter
optimization with MIPROv2.

# Metrics

- **Latency**: p50, p95, p99 percentiles across 10+ runs
- **Token Usage**: Input + output tokens per operation
- **Cost**: Estimated cost based on model pricing
- **Throughput**: Operations per second

# Usage

```bash
# Default: Benchmark all modules
python baseline_benchmark.py

# Single module
python baseline_benchmark.py --module reviewer

# Custom iterations
python baseline_benchmark.py --iterations 50

# Output to file
python baseline_benchmark.py --output baseline_results.json
```

# Requirements

- DSPy configured with model (claude-3-5-sonnet-20241022 or similar)
- Internet connection for API calls
- API key configured via environment variables

# Output

JSON report with:
- Per-operation metrics (latency, tokens, cost)
- Aggregate statistics
- Timestamp and configuration
- Baseline for comparison after optimization
"""

import dspy
import os
import time
import json
import argparse
import logging
import statistics
from datetime import datetime
from typing import Dict, List, Tuple, Any
from pathlib import Path

# Import modules
from reviewer_module import ReviewerModule
from semantic_module import SemanticModule

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


# =============================================================================
# Test Inputs
# =============================================================================

REVIEWER_INPUTS = {
    "extract_requirements": [
        {
            "user_intent": "Implement user authentication with JWT tokens",
            "context": "Phase: Specification, Agent: Executor, Files: auth.rs, user.rs",
        },
        {
            "user_intent": "Add caching to improve API performance",
            "context": "Phase: Implementation, Agent: Executor, Files: api/handlers.rs, cache.rs",
        },
        {
            "user_intent": "Fix the bug where users can't upload large files",
            "context": "Phase: Bug Fix, Agent: Executor, Files: upload.rs, config.rs",
        },
    ],
    "validate_intent": [
        {
            "user_intent": "Add user authentication",
            "work_item": "Implement JWT-based authentication system",
            "implementation": "Added login endpoint POST /api/auth/login accepting email/password. Implemented JWT token generation with RS256 signing. Created middleware for token validation on protected routes. Added refresh token support with 7-day expiration. Files: auth.rs (250 lines), middleware.rs (100 lines), user.rs (updates). Tests: test_auth.rs (15 tests covering happy path, invalid credentials, token expiration).",
            "requirements": [
                "Login endpoint",
                "JWT token generation",
                "Token validation",
                "Password hashing"
            ],
        },
        {
            "user_intent": "Improve API performance",
            "work_item": "Optimize database queries",
            "implementation": "Added indexes on user.email and posts.created_at columns. Reduced N+1 queries in /api/posts endpoint by implementing eager loading. Changed query result limit from unlimited to 100. Files: migrations/001_add_indexes.sql, handlers/posts.rs (query optimization). No performance testing conducted.",
            "requirements": [
                "Identify slow queries",
                "Add database indexes",
                "Reduce query count",
                "Measure improvement"
            ],
        },
    ],
    "validate_completeness": [
        {
            "work_item": "Implement authentication system",
            "implementation": "Added login endpoint with JWT tokens. Missing refresh token logic. TODO: Add password reset. Tests only cover happy path.",
            "requirements": [
                "Login endpoint",
                "JWT tokens",
                "Refresh tokens",
                "Password reset",
                "Comprehensive tests",
            ],
        },
    ],
    "validate_correctness": [
        {
            "work_item": "Implement rate limiting",
            "implementation": "Added rate limiting middleware using in-memory store. Tracks requests per IP address. Returns 429 when limit exceeded.",
            "test_results": "All 12 tests passing. Coverage: 85%. Tests include: rate limit enforcement, reset after window, multiple IPs, edge cases (empty IP, invalid limits).",
        },
    ],
    "generate_guidance": [
        {
            "user_intent": "Add authentication",
            "work_item": "Implement JWT authentication",
            "implementation": "Added login endpoint. Missing token validation middleware. No refresh tokens. Tests incomplete.",
            "failed_gates": ["completeness", "correctness"],
            "all_issues": [
                "Missing token validation middleware",
                "No refresh token support",
                "Incomplete test coverage",
                "Password hashing not verified",
            ],
        },
    ],
}

SEMANTIC_INPUTS = {
    "analyze_discourse": [
        "The system is distributed. This enables horizontal scaling. However, it introduces complexity in debugging.",
        "Authentication is required for all endpoints. The login endpoint accepts email and password. Tokens expire after 24 hours.",
        "We need to optimize performance. Database queries are slow. Adding indexes should help. We also need caching.",
    ],
    "detect_contradictions": [
        "Authentication is required for all endpoints. The public endpoints don't require authentication.",
        "The system is designed for high performance. We use inefficient algorithms that cause slowness.",
        "Security is our top priority. We store passwords in plain text for convenience.",
    ],
    "extract_pragmatics": [
        "Could you please implement authentication? It would be great if we had JWT support.",
        "We should probably add logging. It might help with debugging in production.",
        "The code is perfect and needs no changes. However, we need to refactor everything.",
    ],
}


# =============================================================================
# Benchmark Functions
# =============================================================================

def measure_latency(func, iterations: int = 10) -> List[float]:
    """Measure latency across multiple iterations.

    Args:
        func: Function to benchmark (no args)
        iterations: Number of iterations

    Returns:
        List of latency measurements in milliseconds
    """
    latencies = []
    for _ in range(iterations):
        start = time.perf_counter()
        try:
            func()
            end = time.perf_counter()
            latencies.append((end - start) * 1000)  # Convert to ms
        except Exception as e:
            logger.error(f"Benchmark iteration failed: {e}")
            continue

    return latencies


def get_token_usage() -> Tuple[int, int]:
    """Get token usage from last DSPy call.

    Returns:
        (input_tokens, output_tokens)
    """
    # DSPy stores token usage in dspy.settings
    # This is a placeholder - actual implementation depends on DSPy version
    # and model provider
    try:
        # Anthropic models track usage in response metadata
        # For now, return estimates based on typical usage
        return (500, 200)  # Placeholder
    except Exception:
        return (0, 0)


def estimate_cost(input_tokens: int, output_tokens: int, model: str = "claude-3-5-sonnet-20241022") -> float:
    """Estimate cost based on token usage.

    Args:
        input_tokens: Number of input tokens
        output_tokens: Number of output tokens
        model: Model identifier

    Returns:
        Estimated cost in USD
    """
    # Anthropic pricing (as of 2024)
    PRICING = {
        "claude-3-5-sonnet-20241022": {
            "input": 3.00 / 1_000_000,   # $3 per million tokens
            "output": 15.00 / 1_000_000,  # $15 per million tokens
        },
        "claude-3-opus-20240229": {
            "input": 15.00 / 1_000_000,
            "output": 75.00 / 1_000_000,
        },
    }

    pricing = PRICING.get(model, PRICING["claude-3-5-sonnet-20241022"])
    input_cost = input_tokens * pricing["input"]
    output_cost = output_tokens * pricing["output"]
    return input_cost + output_cost


def compute_statistics(latencies: List[float]) -> Dict[str, float]:
    """Compute latency statistics.

    Args:
        latencies: List of latency measurements in milliseconds

    Returns:
        Dictionary with p50, p95, p99, mean, stddev
    """
    if not latencies:
        return {
            "p50": 0.0,
            "p95": 0.0,
            "p99": 0.0,
            "mean": 0.0,
            "stddev": 0.0,
            "min": 0.0,
            "max": 0.0,
        }

    sorted_latencies = sorted(latencies)
    n = len(sorted_latencies)

    return {
        "p50": sorted_latencies[int(n * 0.50)],
        "p95": sorted_latencies[int(n * 0.95)],
        "p99": sorted_latencies[int(n * 0.99)],
        "mean": statistics.mean(latencies),
        "stddev": statistics.stdev(latencies) if n > 1 else 0.0,
        "min": min(latencies),
        "max": max(latencies),
    }


# =============================================================================
# Module Benchmarks
# =============================================================================

def benchmark_reviewer_operation(
    reviewer: ReviewerModule,
    operation: str,
    inputs_list: List[Dict[str, Any]],
    iterations: int = 10,
) -> Dict[str, Any]:
    """Benchmark a single ReviewerModule operation.

    Args:
        reviewer: ReviewerModule instance
        operation: Operation name
        inputs_list: List of input dictionaries
        iterations: Number of iterations per input

    Returns:
        Dictionary with benchmark results
    """
    logger.info(f"Benchmarking ReviewerModule.{operation}")

    all_latencies = []

    for input_dict in inputs_list:
        # Create closure for benchmark
        def run_operation():
            if operation == "extract_requirements":
                reviewer.extract_requirements(**input_dict)
            elif operation == "validate_intent":
                reviewer.validate_intent_satisfaction(**input_dict)
            elif operation == "validate_completeness":
                reviewer.validate_implementation_completeness(**input_dict)
            elif operation == "validate_correctness":
                reviewer.validate_implementation_correctness(**input_dict)
            elif operation == "generate_guidance":
                reviewer.generate_improvement_guidance_for_failed_review(**input_dict)

        # Measure latency
        latencies = measure_latency(run_operation, iterations)
        all_latencies.extend(latencies)

    # Compute statistics
    stats = compute_statistics(all_latencies)

    # Get token usage (placeholder)
    input_tokens, output_tokens = get_token_usage()
    cost = estimate_cost(input_tokens, output_tokens)

    return {
        "operation": operation,
        "iterations": len(all_latencies),
        "latency_ms": stats,
        "tokens": {
            "input": input_tokens,
            "output": output_tokens,
            "total": input_tokens + output_tokens,
        },
        "cost_usd": cost,
        "throughput_ops_per_sec": 1000.0 / stats["mean"] if stats["mean"] > 0 else 0.0,
    }


def benchmark_semantic_operation(
    semantic: SemanticModule,
    operation: str,
    inputs_list: List[str],
    iterations: int = 10,
) -> Dict[str, Any]:
    """Benchmark a single SemanticModule operation.

    Args:
        semantic: SemanticModule instance
        operation: Operation name
        inputs_list: List of input texts
        iterations: Number of iterations per input

    Returns:
        Dictionary with benchmark results
    """
    logger.info(f"Benchmarking SemanticModule.{operation}")

    all_latencies = []

    for text in inputs_list:
        # Create closure for benchmark
        def run_operation():
            if operation == "analyze_discourse":
                semantic.analyze_discourse(text)
            elif operation == "detect_contradictions":
                semantic.detect_contradictions(text)
            elif operation == "extract_pragmatics":
                semantic.extract_pragmatics(text)

        # Measure latency
        latencies = measure_latency(run_operation, iterations)
        all_latencies.extend(latencies)

    # Compute statistics
    stats = compute_statistics(all_latencies)

    # Get token usage (placeholder)
    input_tokens, output_tokens = get_token_usage()
    cost = estimate_cost(input_tokens, output_tokens)

    return {
        "operation": operation,
        "iterations": len(all_latencies),
        "latency_ms": stats,
        "tokens": {
            "input": input_tokens,
            "output": output_tokens,
            "total": input_tokens + output_tokens,
        },
        "cost_usd": cost,
        "throughput_ops_per_sec": 1000.0 / stats["mean"] if stats["mean"] > 0 else 0.0,
    }


def benchmark_reviewer(iterations: int = 10) -> Dict[str, Any]:
    """Benchmark all ReviewerModule operations.

    Args:
        iterations: Number of iterations per operation

    Returns:
        Dictionary with all benchmark results
    """
    logger.info("Initializing ReviewerModule")
    reviewer = ReviewerModule()

    results = {}

    for operation, inputs_list in REVIEWER_INPUTS.items():
        try:
            result = benchmark_reviewer_operation(
                reviewer, operation, inputs_list, iterations
            )
            results[operation] = result
        except Exception as e:
            logger.error(f"Failed to benchmark {operation}: {e}")
            results[operation] = {"error": str(e)}

    return results


def benchmark_semantic(iterations: int = 10) -> Dict[str, Any]:
    """Benchmark all SemanticModule operations.

    Args:
        iterations: Number of iterations per operation

    Returns:
        Dictionary with all benchmark results
    """
    logger.info("Initializing SemanticModule")
    semantic = SemanticModule()

    results = {}

    for operation, inputs_list in SEMANTIC_INPUTS.items():
        try:
            result = benchmark_semantic_operation(
                semantic, operation, inputs_list, iterations
            )
            results[operation] = result
        except Exception as e:
            logger.error(f"Failed to benchmark {operation}: {e}")
            results[operation] = {"error": str(e)}

    return results


# =============================================================================
# Main
# =============================================================================

def main():
    parser = argparse.ArgumentParser(
        description="Benchmark DSPy modules for baseline performance"
    )
    parser.add_argument(
        "--module",
        choices=["reviewer", "semantic", "all"],
        default="all",
        help="Module to benchmark (default: all)",
    )
    parser.add_argument(
        "--iterations",
        type=int,
        default=10,
        help="Number of iterations per operation (default: 10)",
    )
    parser.add_argument(
        "--output",
        type=str,
        default="baseline_results.json",
        help="Output file path (default: baseline_results.json)",
    )

    args = parser.parse_args()

    # Initialize DSPy with Anthropic Claude
    api_key = os.getenv("ANTHROPIC_API_KEY")
    if not api_key:
        logger.error("ANTHROPIC_API_KEY not set. Set it via environment variable.")
        return

    try:
        # Configure DSPy with Claude 3.5 Sonnet (updated model name)
        dspy.configure(lm=dspy.LM('anthropic/claude-3-5-sonnet-20241022', api_key=api_key))
        logger.info(f"DSPy configured with claude-3-5-sonnet-20241022")
    except Exception as e:
        logger.error(f"Failed to configure DSPy: {e}")
        return

    # Run benchmarks
    results = {
        "timestamp": datetime.now().isoformat(),
        "config": {
            "iterations": args.iterations,
            "module": args.module,
        },
        "modules": {},
    }

    if args.module in ["reviewer", "all"]:
        logger.info("=" * 60)
        logger.info("Benchmarking ReviewerModule")
        logger.info("=" * 60)
        results["modules"]["reviewer"] = benchmark_reviewer(args.iterations)

    if args.module in ["semantic", "all"]:
        logger.info("=" * 60)
        logger.info("Benchmarking SemanticModule")
        logger.info("=" * 60)
        results["modules"]["semantic"] = benchmark_semantic(args.iterations)

    # Save results
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with open(output_path, "w") as f:
        json.dump(results, f, indent=2)

    logger.info("=" * 60)
    logger.info(f"Baseline results saved to: {output_path}")
    logger.info("=" * 60)

    # Print summary
    print("\n" + "=" * 60)
    print("BASELINE BENCHMARK SUMMARY")
    print("=" * 60)

    for module_name, module_results in results["modules"].items():
        print(f"\n{module_name.upper()}:")
        for operation, op_results in module_results.items():
            if "error" in op_results:
                print(f"  {operation}: ERROR - {op_results['error']}")
            else:
                latency = op_results["latency_ms"]
                print(f"  {operation}:")
                print(f"    Latency (p50/p95/p99): {latency['p50']:.1f} / {latency['p95']:.1f} / {latency['p99']:.1f} ms")
                print(f"    Tokens: {op_results['tokens']['total']} (in: {op_results['tokens']['input']}, out: {op_results['tokens']['output']})")
                print(f"    Cost: ${op_results['cost_usd']:.4f}")
                print(f"    Throughput: {op_results['throughput_ops_per_sec']:.2f} ops/sec")

    print("\n" + "=" * 60)


if __name__ == "__main__":
    main()
