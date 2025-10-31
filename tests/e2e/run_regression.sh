#!/usr/bin/env bash
set -euo pipefail

# Regression Test Suite Runner
#
# Runs only regression tests (those that use mocked LLM responses).
# These tests provide fast feedback for development and CI/CD.
#
# Cost: $0 (no API calls)
# Duration: <30 minutes
# Frequency: Every commit, PR validation

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source common utilities
source "$SCRIPT_DIR/lib/common.sh"

# ===================================================================
# CONFIGURATION
# ===================================================================

# Force regression mode
export MNEMOSYNE_TEST_MODE=regression

# Results directory
RESULTS_DIR="$SCRIPT_DIR/results/regression/$(date +%Y%m%d_%H%M%S)"
mkdir -p "$RESULTS_DIR"

# Parse arguments
PARALLEL=0
FAST=0
CATEGORY=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --parallel)
            PARALLEL=1
            shift
            ;;
        --fast)
            FAST=1
            shift
            ;;
        --category)
            CATEGORY="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--parallel] [--fast] [--category <name>]"
            exit 1
            ;;
    esac
done

# Test lists (regression tests only - excluding baseline)
USER_JOURNEY_TESTS=(
    "solo_dev_2_daily_workflow.sh"
    "solo_dev_3_project_evolution.sh"
    "solo_dev_4_cross_project.sh"
    "team_lead_1_setup_namespaces.sh"
    "team_lead_3_consolidate_team_knowledge.sh"
    "team_lead_4_generate_reports.sh"
    "power_user_2_bulk_operations.sh"
    "power_user_3_custom_workflows.sh"
    "power_user_4_performance_optimization.sh"
)

FEATURE_TESTS=(
    "memory_types_3_decision.sh"
    "memory_types_4_task.sh"
    "memory_types_5_reference.sh"
    "namespaces_1_global.sh"
    "namespaces_2_project.sh"
    "namespaces_4_hierarchical.sh"
    "namespaces_5_isolation.sh"
    "storage_1_local_sqlite.sh"
    "storage_3_turso_cloud.sh"
    "llm_config_2_enrichment_disabled.sh"
    "llm_config_3_partial_features.sh"
)

ORCHESTRATION_TESTS=(
    "orchestration_1_sequential.sh"
    "orchestration_4_context_optimization.sh"
    "work_queue_1_dependencies.sh"
    "work_queue_2_priorities.sh"
    "work_queue_4_branch_isolation.sh"
)

INTEGRATION_TESTS=(
    "mcp_2_ooda_orient.sh"
    "mcp_4_ooda_act.sh"
    "python_2_evaluation_api.sh"
    "python_3_async_operations.sh"
    "api_2_rest_endpoints.sh"
    "api_3_dashboard_integration.sh"
)

ICS_TESTS=(
    "ics_editor_1_crdt_basic.sh"
    "ics_editor_2_vim_mode.sh"
    "ics_editor_3_syntax_highlighting.sh"
    "ics_collab_2_conflict_resolution.sh"
    "ics_panels_1_memory_browser.sh"
    "ics_panels_2_agent_status.sh"
)

EVOLUTION_TESTS=(
    "evolution_decay.sh"
    "evolution_supersede.sh"
)

# Select tests based on category
if [ -n "$CATEGORY" ]; then
    case "$CATEGORY" in
        user_journey|user)
            REGRESSION_TESTS=("${USER_JOURNEY_TESTS[@]}")
            ;;
        feature|features)
            REGRESSION_TESTS=("${FEATURE_TESTS[@]}")
            ;;
        orchestration|orch)
            REGRESSION_TESTS=("${ORCHESTRATION_TESTS[@]}")
            ;;
        integration|int)
            REGRESSION_TESTS=("${INTEGRATION_TESTS[@]}")
            ;;
        ics|editor)
            REGRESSION_TESTS=("${ICS_TESTS[@]}")
            ;;
        evolution|evo)
            REGRESSION_TESTS=("${EVOLUTION_TESTS[@]}")
            ;;
        *)
            echo "Unknown category: $CATEGORY"
            echo "Available: user_journey, feature, orchestration, integration, ics, evolution"
            exit 1
            ;;
    esac
else
    # All regression tests
    REGRESSION_TESTS=(
        "${USER_JOURNEY_TESTS[@]}"
        "${FEATURE_TESTS[@]}"
        "${ORCHESTRATION_TESTS[@]}"
        "${INTEGRATION_TESTS[@]}"
        "${ICS_TESTS[@]}"
        "${EVOLUTION_TESTS[@]}"
    )
fi

# ===================================================================
# PRE-FLIGHT CHECKS
# ===================================================================

section "Regression Test Suite Pre-Flight Checks"

# Check binary exists
if [ ! -f "$SCRIPT_DIR/../../target/release/mnemosyne" ]; then
    print_red "✗ Binary not found. Building..."
    cd "$SCRIPT_DIR/../.." && cargo build --release
fi

print_green "✓ Binary ready"

# Verify regression mode
if [ "$MNEMOSYNE_TEST_MODE" != "regression" ]; then
    print_red "✗ MNEMOSYNE_TEST_MODE should be 'regression'"
    exit 1
fi

print_green "✓ Regression mode enabled (mocked LLM)"

section "Regression Suite Information"
echo "Mode: REGRESSION (Mocked LLM, no API calls)"
echo "Total tests: ${#REGRESSION_TESTS[@]}"
echo "Cost: \$0"
echo "Estimated duration: 10-30 minutes"
echo "Results directory: $RESULTS_DIR"
[ "$PARALLEL" -eq 1 ] && echo "Parallel execution: ENABLED"
[ "$FAST" -eq 1 ] && echo "Fast mode: ENABLED (reduced test depth)"
[ -n "$CATEGORY" ] && echo "Category filter: $CATEGORY"
echo ""

# ===================================================================
# TEST EXECUTION
# ===================================================================

section "Running Regression Test Suite"

PASSED=0
FAILED=0
SKIPPED=0

START_TIME=$(date +%s)

# Function to run a single test
run_test() {
    local test="$1"
    local test_path="$SCRIPT_DIR/$test"

    if [ ! -f "$test_path" ]; then
        print_yellow "⊘ SKIP: $test (not yet implemented)"
        return 2  # Skipped
    fi

    local test_start=$(date +%s)
    local test_log="$RESULTS_DIR/${test%.sh}.log"
    local test_result="$RESULTS_DIR/${test%.sh}.result"

    if bash "$test_path" >"$test_log" 2>&1; then
        local test_end=$(date +%s)
        local test_duration=$((test_end - test_start))
        print_green "✓ PASS: $test (${test_duration}s)"
        echo "PASS" > "$test_result"
        return 0  # Passed
    else
        local test_end=$(date +%s)
        local test_duration=$((test_end - test_start))
        print_red "✗ FAIL: $test (${test_duration}s)"
        echo "FAIL" > "$test_result"

        # Show last 10 lines of failure log (brief for regression)
        print_yellow "Last 10 lines:"
        tail -10 "$test_log" | sed 's/^/  /'
        return 1  # Failed
    fi
}

if [ "$PARALLEL" -eq 1 ]; then
    # Parallel execution (experimental)
    print_cyan "Running tests in parallel..."

    pids=()
    for test in "${REGRESSION_TESTS[@]}"; do
        run_test "$test" &
        pids+=($!)
    done

    # Wait for all tests
    for pid in "${pids[@]}"; do
        wait "$pid"
        case $? in
            0) ((PASSED++)) ;;
            1) ((FAILED++)) ;;
            2) ((SKIPPED++)) ;;
        esac
    done
else
    # Sequential execution (default)
    for test in "${REGRESSION_TESTS[@]}"; do
        print_cyan "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        print_cyan "Running: $test"
        print_cyan "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

        run_test "$test"
        case $? in
            0) ((PASSED++)) ;;
            1) ((FAILED++)) ;;
            2) ((SKIPPED++)) ;;
        esac

        echo ""
    done
fi

END_TIME=$(date +%s)
TOTAL_DURATION=$((END_TIME - START_TIME))
TOTAL_RUN=$((PASSED + FAILED))

# ===================================================================
# SUMMARY REPORT
# ===================================================================

section "Regression Suite Summary"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Mode: REGRESSION (Mocked LLM, No API Calls)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Total tests:     ${#REGRESSION_TESTS[@]}"
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
    print_green "✓ ALL REGRESSION TESTS PASSED"
    print_green "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    # Generate summary report
    cat > "$RESULTS_DIR/summary.txt" <<EOF
Regression Test Suite Summary
==============================

Date: $(date)
Mode: REGRESSION (Mocked LLM)

Results:
--------
Total:   ${#REGRESSION_TESTS[@]}
Passed:  $PASSED
Failed:  $FAILED
Skipped: $SKIPPED

Duration: ${TOTAL_DURATION}s

Cost: \$0 (no API calls)

Status: ✓ PASS
EOF

    exit 0
elif [ "$PASSED" -eq 0 ] && [ "$SKIPPED" -eq "${#REGRESSION_TESTS[@]}" ]; then
    print_yellow "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    print_yellow "⊘ ALL TESTS SKIPPED (not yet implemented)"
    print_yellow "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    exit 2
else
    print_red "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    print_red "✗ SOME REGRESSION TESTS FAILED"
    print_red "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    print_yellow "Failed tests:"
    grep -l "FAIL" "$RESULTS_DIR"/*.result 2>/dev/null | while read -r result_file; do
        test_name=$(basename "$result_file" .result)
        echo "  - $test_name.sh"
        log_file="$RESULTS_DIR/$test_name.log"
        if [ -f "$log_file" ]; then
            echo "    Log: $log_file"
        fi
    done

    exit 1
fi
