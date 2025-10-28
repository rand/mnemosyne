#!/usr/bin/env bash
set -eo pipefail

# E2E Test Runner - Run all tests with organized output
#
# Usage:
#   ./run_all.sh                  # Run all tests sequentially
#   ./run_all.sh --parallel       # Run tests in parallel (faster but uses more resources)
#   ./run_all.sh --category human # Run specific category only
#   ./run_all.sh --quick          # Run quick tests only (skip stress tests)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
PARALLEL_MODE=false
QUICK_MODE=false
CATEGORY=""
OUTPUT_DIR="/tmp/e2e_test_results"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --parallel)
            PARALLEL_MODE=true
            shift
            ;;
        --quick)
            QUICK_MODE=true
            shift
            ;;
        --category)
            CATEGORY="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --parallel         Run tests in parallel"
            echo "  --quick            Skip long-running stress tests"
            echo "  --category NAME    Run specific category (human|agentic|failure|recovery|integration|performance)"
            echo "  --help             Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Print banner
echo ""
echo "========================================="
echo "  Mnemosyne E2E Test Suite"
echo "========================================="
echo ""
echo "Mode: $([ "$PARALLEL_MODE" = true ] && echo "Parallel" || echo "Sequential")"
echo "Quick mode: $([ "$QUICK_MODE" = true ] && echo "Enabled" || echo "Disabled")"
echo "Output: $OUTPUT_DIR"
echo ""

# Define test categories
declare -A TESTS

# Human workflows
TESTS[human]="
human_workflow_4_context_loading.sh
"

# Agentic workflows
TESTS[agentic]="
agentic_workflow_1_orchestrator.sh
agentic_workflow_2_optimizer.sh
agentic_workflow_3_reviewer.sh
agentic_workflow_4_executor.sh
"

# Failure scenarios
TESTS[failure]="
failure_1_storage_errors.sh
failure_2_llm_failures.sh
failure_3_timeout_scenarios.sh
failure_4_invalid_inputs.sh
"

# Recovery scenarios
TESTS[recovery]="
recovery_1_graceful_degradation.sh
recovery_2_fallback_modes.sh
"

# Integration tests
TESTS[integration]="
integration_1_launcher.sh
integration_2_mcp_server.sh
integration_3_hooks.sh
"

# Performance tests
TESTS[performance]="
performance_1_benchmarks.sh
"

# Stress tests (skipped in quick mode)
if [ "$QUICK_MODE" != true ]; then
    TESTS[performance]="${TESTS[performance]}
performance_2_stress_tests.sh
"
fi

# Determine which tests to run
if [ -n "$CATEGORY" ]; then
    if [ -n "${TESTS[$CATEGORY]}" ]; then
        SELECTED_TESTS=${TESTS[$CATEGORY]}
        echo "Running category: $CATEGORY"
    else
        echo -e "${RED}Error: Unknown category '$CATEGORY'${NC}"
        echo "Valid categories: human, agentic, failure, recovery, integration, performance"
        exit 1
    fi
else
    # Run all tests
    SELECTED_TESTS="${TESTS[human]} ${TESTS[agentic]} ${TESTS[failure]} ${TESTS[recovery]} ${TESTS[integration]} ${TESTS[performance]}"
    echo "Running all tests"
fi

echo ""

# Function to run a single test
run_test() {
    local test_file=$1
    local test_path="$SCRIPT_DIR/$test_file"
    local log_file="$OUTPUT_DIR/${test_file%.sh}.log"

    if [ ! -f "$test_path" ]; then
        echo -e "${YELLOW}SKIP${NC} $test_file (not found)"
        return
    fi

    if [ ! -x "$test_path" ]; then
        echo -e "${YELLOW}SKIP${NC} $test_file (not executable)"
        return
    fi

    echo -e "${CYAN}RUN${NC}  $test_file"

    if bash "$test_path" > "$log_file" 2>&1; then
        echo -e "${GREEN}PASS${NC} $test_file"
        return 0
    else
        echo -e "${RED}FAIL${NC} $test_file"
        return 1
    fi
}

# Run tests
START_TIME=$(date +%s)
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
PIDS=()

for test_file in $SELECTED_TESTS; do
    # Skip empty lines
    [ -z "$test_file" ] && continue

    ((TOTAL_TESTS++))

    if [ "$PARALLEL_MODE" = true ]; then
        # Run in background
        run_test "$test_file" &
        PIDS+=($!)
    else
        # Run sequentially
        if run_test "$test_file"; then
            ((PASSED_TESTS++))
        else
            ((FAILED_TESTS++))
        fi
    fi
done

# Wait for parallel tests to complete
if [ "$PARALLEL_MODE" = true ]; then
    echo ""
    echo "Waiting for parallel tests to complete..."

    for pid in "${PIDS[@]}"; do
        if wait $pid; then
            ((PASSED_TESTS++))
        else
            ((FAILED_TESTS++))
        fi
    done
fi

END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

# Print summary
echo ""
echo "========================================="
echo "  Test Summary"
echo "========================================="
echo ""
echo "Total tests:  $TOTAL_TESTS"
echo -e "Passed:       ${GREEN}$PASSED_TESTS${NC}"
echo -e "Failed:       ${RED}$FAILED_TESTS${NC}"
echo "Duration:     ${DURATION}s"
echo ""

# Show failed test details
if [ "$FAILED_TESTS" -gt 0 ]; then
    echo "========================================="
    echo "  Failed Tests"
    echo "========================================="
    echo ""

    for test_file in $SELECTED_TESTS; do
        [ -z "$test_file" ] && continue

        log_file="$OUTPUT_DIR/${test_file%.sh}.log"

        if [ -f "$log_file" ]; then
            # Check if test failed (look for failure indicators)
            if grep -q "Some tests failed\|Test Summary.*Failed.*[1-9]" "$log_file"; then
                echo "--- $test_file ---"
                # Show last 30 lines of failed test
                tail -30 "$log_file"
                echo ""
            fi
        fi
    done
fi

# Show summary of each test
echo "========================================="
echo "  Detailed Results"
echo "========================================="
echo ""

for test_file in $SELECTED_TESTS; do
    [ -z "$test_file" ] && continue

    log_file="$OUTPUT_DIR/${test_file%.sh}.log"

    if [ -f "$log_file" ]; then
        # Extract summary from log
        if grep -q "Test Summary" "$log_file"; then
            echo "--- $test_file ---"
            grep -A 5 "Test Summary" "$log_file" | head -6
            echo ""
        fi
    fi
done

echo "Full logs available in: $OUTPUT_DIR"
echo ""

# Generate summary report
REPORT_FILE="$OUTPUT_DIR/summary_$(date +%Y%m%d_%H%M%S).txt"

cat > "$REPORT_FILE" <<EOF
Mnemosyne E2E Test Suite - Summary Report
Generated: $(date)

Configuration:
- Mode: $([ "$PARALLEL_MODE" = true ] && echo "Parallel" || echo "Sequential")
- Quick mode: $([ "$QUICK_MODE" = true ] && echo "Enabled" || echo "Disabled")
- Category: $([ -n "$CATEGORY" ] && echo "$CATEGORY" || echo "All")

Results:
- Total tests: $TOTAL_TESTS
- Passed: $PASSED_TESTS
- Failed: $FAILED_TESTS
- Duration: ${DURATION}s

Test Categories:
EOF

# Add category summaries
for category in human agentic failure recovery integration performance; do
    if [ -n "$CATEGORY" ] && [ "$CATEGORY" != "$category" ]; then
        continue
    fi

    cat_passed=0
    cat_failed=0
    cat_total=0

    for test_file in ${TESTS[$category]}; do
        [ -z "$test_file" ] && continue

        log_file="$OUTPUT_DIR/${test_file%.sh}.log"

        if [ -f "$log_file" ]; then
            ((cat_total++))

            if grep -q "All tests passed" "$log_file"; then
                ((cat_passed++))
            elif grep -q "Some tests failed" "$log_file"; then
                ((cat_failed++))
            fi
        fi
    done

    if [ "$cat_total" -gt 0 ]; then
        echo "- $category: $cat_passed/$cat_total passed" >> "$REPORT_FILE"
    fi
done

echo "" >> "$REPORT_FILE"
echo "Detailed logs: $OUTPUT_DIR" >> "$REPORT_FILE"

echo "Summary report: $REPORT_FILE"
echo ""

# Exit with appropriate code
if [ "$FAILED_TESTS" -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed${NC}"
    exit 1
fi
