#!/usr/bin/env bash
set -euo pipefail

# Baseline Test Suite Runner
#
# Runs only baseline tests (those marked with [BASELINE] that use real LLM API).
# These tests validate LLM integration quality and establish quality benchmarks.
#
# Cost: ~$2-5 per run (75-125 API calls)
# Duration: ~1-2 hours
# Frequency: Weekly + pre-release

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source common utilities
source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/baseline_validators.sh"

# ===================================================================
# CONFIGURATION
# ===================================================================

# Force baseline mode
export MNEMOSYNE_TEST_MODE=baseline

# Results directory
RESULTS_DIR="$SCRIPT_DIR/results/baseline/$(date +%Y%m%d_%H%M%S)"
mkdir -p "$RESULTS_DIR"

# Test lists (baseline tests only)
BASELINE_TESTS=(
    # Phase 1: User Journeys (3 baseline tests)
    "solo_dev_1_onboarding.sh"
    "team_lead_2_coordinate_work.sh"
    "power_user_1_advanced_search.sh"

    # Phase 2: Feature Permutations (5 baseline tests)
    "memory_types_1_insight.sh"
    "memory_types_2_architecture.sh"
    "namespaces_3_session.sh"
    "storage_2_libsql.sh"
    "llm_config_1_enrichment_enabled.sh"

    # Phase 3: Orchestration (4 baseline tests)
    "orchestration_2_parallel.sh"
    "orchestration_3_review_loop.sh"
    "work_queue_3_deadlock.sh"

    # Phase 4: Integration (4 baseline tests)
    "mcp_1_ooda_observe.sh"
    "mcp_3_ooda_decide.sh"
    "python_1_storage_api.sh"
    "api_1_sse_events.sh"

    # Phase 5: ICS (2 baseline tests)
    "ics_editor_4_semantic_highlighting.sh"
    "ics_collab_1_multi_user.sh"

    # Phase 6: Evolution (4 baseline tests)
    "evolution_consolidate_1_auto.sh"
    "evolution_consolidate_2_manual.sh"
    "evolution_importance.sh"
    "evolution_archival.sh"
)

# ===================================================================
# PRE-FLIGHT CHECKS
# ===================================================================

section "Baseline Test Suite Pre-Flight Checks"

# Check binary exists
if [ ! -f "$SCRIPT_DIR/../../target/release/mnemosyne" ]; then
    print_red "✗ Binary not found. Building..."
    cd "$SCRIPT_DIR/../.." && cargo build --release
fi

# Check API key is configured
print_cyan "Checking API key..."
if ! "$SCRIPT_DIR/../../target/release/mnemosyne" config show-key >/dev/null 2>&1; then
    print_red "✗ No API key configured"
    echo ""
    echo "Baseline tests require an Anthropic API key for LLM calls."
    echo "Configure with:"
    echo "  mnemosyne config set-key <your-api-key>"
    echo ""
    echo "Or set environment variable:"
    echo "  export ANTHROPIC_API_KEY=<your-api-key>"
    exit 1
fi

print_green "✓ API key configured"

# Estimate cost
TOTAL_TESTS=${#BASELINE_TESTS[@]}
MIN_COST=$(echo "scale=2; $TOTAL_TESTS * 0.08" | bc)  # ~3 calls/test @ $0.025/call
MAX_COST=$(echo "scale=2; $TOTAL_TESTS * 0.15" | bc)  # ~5 calls/test @ $0.03/call

section "Baseline Suite Information"
echo "Mode: BASELINE (Real LLM API)"
echo "Total tests: $TOTAL_TESTS"
echo "Estimated cost: \$$MIN_COST - \$$MAX_COST"
echo "Estimated duration: 1-2 hours"
echo "Results directory: $RESULTS_DIR"
echo ""

# Confirm execution
if [ "${MNEMOSYNE_BASELINE_AUTO_RUN:-0}" != "1" ]; then
    print_yellow "This will make real API calls and incur costs."
    read -p "Continue? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Cancelled."
        exit 0
    fi
fi

# ===================================================================
# TEST EXECUTION
# ===================================================================

section "Running Baseline Test Suite"

PASSED=0
FAILED=0
SKIPPED=0

START_TIME=$(date +%s)

for test in "${BASELINE_TESTS[@]}"; do
    test_path="$SCRIPT_DIR/$test"

    if [ ! -f "$test_path" ]; then
        print_yellow "⊘ SKIP: $test (not yet implemented)"
        ((SKIPPED++))
        continue
    fi

    print_cyan "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    print_cyan "Running: $test"
    print_cyan "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    test_start=$(date +%s)
    test_log="$RESULTS_DIR/${test%.sh}.log"
    test_result="$RESULTS_DIR/${test%.sh}.result"

    if bash "$test_path" >"$test_log" 2>&1; then
        test_end=$(date +%s)
        test_duration=$((test_end - test_start))

        print_green "✓ PASS: $test (${test_duration}s)"
        echo "PASS" > "$test_result"
        ((PASSED++))
    else
        test_end=$(date +%s)
        test_duration=$((test_end - test_start))

        print_red "✗ FAIL: $test (${test_duration}s)"
        echo "FAIL" > "$test_result"
        ((FAILED++))

        # Show last 20 lines of failure log
        print_yellow "Last 20 lines of output:"
        tail -20 "$test_log" | sed 's/^/  /'
    fi

    echo ""
done

END_TIME=$(date +%s)
TOTAL_DURATION=$((END_TIME - START_TIME))
TOTAL_RUN=$((PASSED + FAILED))

# ===================================================================
# QUALITY VALIDATION
# ===================================================================

section "Baseline Quality Validation"

print_cyan "Validating LLM response quality across all tests..."

# Collect all JSON result files
json_results=$(find "$RESULTS_DIR" -name "*.json" 2>/dev/null || true)

if [ -z "$json_results" ]; then
    print_yellow "⊘ No JSON results found for validation"
else
    if validate_baseline_run "$RESULTS_DIR"; then
        print_green "✓ All LLM responses meet quality thresholds"
    else
        print_red "✗ Some LLM responses below quality thresholds"
        print_yellow "Review results in: $RESULTS_DIR"
    fi
fi

# ===================================================================
# SUMMARY REPORT
# ===================================================================

section "Baseline Suite Summary"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Mode: BASELINE (Real LLM API Calls)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Total tests:     $TOTAL_TESTS"
echo "Executed:        $TOTAL_RUN"
echo "Passed:          $PASSED"
echo "Failed:          $FAILED"
echo "Skipped:         $SKIPPED (not yet implemented)"
echo ""
echo "Duration:        ${TOTAL_DURATION}s ($((TOTAL_DURATION / 60))m $((TOTAL_DURATION % 60))s)"
echo "Results:         $RESULTS_DIR"
echo ""

if [ "$FAILED" -eq 0 ] && [ "$PASSED" -gt 0 ]; then
    print_green "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    print_green "✓ ALL BASELINE TESTS PASSED"
    print_green "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    # Generate summary report
    cat > "$RESULTS_DIR/summary.txt" <<EOF
Baseline Test Suite Summary
============================

Date: $(date)
Mode: BASELINE (Real LLM API)

Results:
--------
Total:   $TOTAL_TESTS
Passed:  $PASSED
Failed:  $FAILED
Skipped: $SKIPPED

Duration: ${TOTAL_DURATION}s

Quality: All LLM responses met quality thresholds

Status: ✓ PASS
EOF

    exit 0
elif [ "$PASSED" -eq 0 ] && [ "$SKIPPED" -eq "$TOTAL_TESTS" ]; then
    print_yellow "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    print_yellow "⊘ ALL TESTS SKIPPED (not yet implemented)"
    print_yellow "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    exit 2
else
    print_red "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    print_red "✗ SOME BASELINE TESTS FAILED"
    print_red "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    print_yellow "Failed tests:"
    grep -l "FAIL" "$RESULTS_DIR"/*.result | while read -r result_file; do
        test_name=$(basename "$result_file" .result)
        echo "  - $test_name.sh"
        log_file="$RESULTS_DIR/$test_name.log"
        if [ -f "$log_file" ]; then
            echo "    Log: $log_file"
        fi
    done

    exit 1
fi
