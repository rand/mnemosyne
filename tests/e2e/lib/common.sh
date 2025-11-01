#!/usr/bin/env bash
#
# Common utilities for E2E tests
#
# This library provides shared functionality for all E2E test scripts:
# - Color output
# - Test assertions
# - Database setup/teardown
# - Performance measurement
# - Mock data generation
# - API key validation

# ============================================================================
# Mode Detection
# ============================================================================

# Check if running in baseline mode (real API calls)
is_baseline_mode() {
    [ "${MNEMOSYNE_TEST_MODE:-regression}" = "baseline" ]
}

# ============================================================================
# Color Output
# ============================================================================

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Print colored message
# Usage: print_color COLOR "message"
print_color() {
    local color=$1
    shift
    echo -e "${color}$*${NC}"
}

print_red() { print_color "$RED" "$@"; }
print_green() { print_color "$GREEN" "$@"; }
print_yellow() { print_color "$YELLOW" "$@"; }
print_blue() { print_color "$BLUE" "$@"; }
print_cyan() { print_color "$CYAN" "$@"; }

# Section headers
section() {
    echo ""
    echo "========================================"
    echo "$@"
    echo "========================================"
}

# ============================================================================
# Test Result Tracking
# ============================================================================

# Global test counters (must be initialized by test script)
# PASSED=0
# FAILED=0

# Mark test as passed
pass() {
    local test_name=$1
    print_green "[PASS] $test_name"
    : $((PASSED=${PASSED:-0}+1))
}

# Mark test as failed with optional message
fail() {
    local test_name=$1
    shift
    print_red "[FAIL] $test_name"
    if [ $# -gt 0 ]; then
        echo "  $*"
    fi
    : $((FAILED=${FAILED:-0}+1))
}

# Mark test as skipped
skip() {
    local test_name=$1
    shift
    print_yellow "[SKIP] $test_name"
    if [ $# -gt 0 ]; then
        echo "  $*"
    fi
}

# Mark test as warning (passed but with issues)
warn() {
    local test_name=$1
    shift
    print_yellow "[WARN] $test_name"
    if [ $# -gt 0 ]; then
        echo "  $*"
    fi
}

# Print test summary
test_summary() {
    section "Test Summary"
    echo -e "Passed: ${GREEN}$PASSED${NC}"
    echo -e "Failed: ${RED}$FAILED${NC}"
    echo "========================================"

    if [ "$FAILED" -eq 0 ]; then
        print_green "All tests passed!"
        return 0
    else
        print_red "Some tests failed"
        return 1
    fi
}

# ============================================================================
# Assertions
# ============================================================================

# Assert string contains substring
# Usage: assert_contains "haystack" "needle" "test name"
assert_contains() {
    local haystack=$1
    local needle=$2
    local test_name=$3

    if echo "$haystack" | grep -qi "$needle"; then
        pass "$test_name"
    else
        fail "$test_name" "Expected to find '$needle' in output"
        echo "Actual output: $haystack"
    fi
    return 0  # Always return 0 to not exit script with set -e
}

# Assert string does NOT contain substring
# Usage: assert_not_contains "haystack" "needle" "test name"
assert_not_contains() {
    local haystack=$1
    local needle=$2
    local test_name=$3

    if echo "$haystack" | grep -qi "$needle"; then
        fail "$test_name" "Did not expect to find '$needle' in output"
        echo "Actual output: $haystack"
    else
        pass "$test_name"
    fi
    return 0  # Always return 0 to not exit script with set -e
}

# Assert command succeeds (exit code 0)
# Usage: assert_success "command" "test name"
assert_success() {
    local test_name=$1
    local exit_code=$2

    if [ "$exit_code" -eq 0 ]; then
        pass "$test_name"
    else
        fail "$test_name" "Command exited with code $exit_code (expected 0)"
    fi
    return 0  # Always return 0 to not exit script with set -e
}

# Assert command fails (non-zero exit code)
# Usage: assert_failure "test name" exit_code
assert_failure() {
    local test_name=$1
    local exit_code=$2

    if [ "$exit_code" -ne 0 ]; then
        pass "$test_name"
    else
        fail "$test_name" "Command succeeded (expected failure)"
    fi
    return 0  # Always return 0 to not exit script with set -e
}

# Assert number comparison
# Usage: assert_lt value threshold "test name"
assert_lt() {
    local value=$1
    local threshold=$2
    local test_name=$3

    if [ "$value" -lt "$threshold" ]; then
        pass "$test_name (${value} < ${threshold})"
    else
        fail "$test_name" "Value $value not less than threshold $threshold"
    fi
    return 0  # Always return 0 to not exit script with set -e
}

assert_gt() {
    local value=$1
    local threshold=$2
    local test_name=$3

    if [ "$value" -gt "$threshold" ]; then
        pass "$test_name (${value} > ${threshold})"
    else
        fail "$test_name" "Value $value not greater than threshold $threshold"
    fi
    return 0  # Always return 0 to not exit script with set -e
}

assert_eq() {
    local value=$1
    local expected=$2
    local test_name=$3

    if [ "$value" -eq "$expected" ]; then
        pass "$test_name (${value} == ${expected})"
    else
        fail "$test_name" "Value $value does not equal expected $expected"
    fi
    return 0  # Always return 0 to not exit script with set -e
}

# Assert file exists
# Usage: assert_file_exists "/path/to/file" "test name"
assert_file_exists() {
    local file_path=$1
    local test_name=$2

    if [ -f "$file_path" ]; then
        pass "$test_name"
    else
        fail "$test_name" "File not found: $file_path"
    fi
    return 0  # Always return 0 to not exit script with set -e
}

# Assert file does NOT exist
assert_file_not_exists() {
    local file_path=$1
    local test_name=$2

    if [ ! -f "$file_path" ]; then
        pass "$test_name"
    else
        fail "$test_name" "File exists but shouldn't: $file_path"
    fi
    return 0  # Always return 0 to not exit script with set -e
}

# ============================================================================
# Binary and Path Detection
# ============================================================================

# Get project root directory
get_project_root() {
    local script_dir
    script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    echo "$(cd "$script_dir/../../.." && pwd)"
}

# Get mnemosyne binary path (release build)
get_binary_path() {
    local project_root
    project_root=$(get_project_root)
    echo "$project_root/target/release/mnemosyne"
}

# Build mnemosyne if not already built
ensure_binary() {
    local binary
    binary=$(get_binary_path)
    local project_root
    project_root=$(get_project_root)

    if [ ! -f "$binary" ]; then
        print_yellow "[SETUP] Building Mnemosyne (release mode)..."
        cd "$project_root" || exit 1
        cargo build --release > /dev/null 2>&1

        if [ ! -f "$binary" ]; then
            print_red "[ERROR] Binary not found after build: $binary"
            exit 1
        fi
        print_green "[SETUP] Build complete"
    fi

    echo "$binary"
}

# ============================================================================
# Database Setup/Teardown
# ============================================================================

# Create temporary test database
# Returns the database path
create_test_db() {
    local test_name=${1:-test}
    local timestamp
    timestamp=$(date +%s)
    local db_path="/tmp/mnemosyne_${test_name}_${timestamp}.db"

    echo "$db_path"
}

# Clean up test database and related files
cleanup_test_db() {
    local db_path=$1

    if [ -n "$db_path" ]; then
        rm -f "$db_path" "${db_path}-wal" "${db_path}-shm"
        print_cyan "[CLEANUP] Removed test database: $db_path"
    fi
}

# Initialize database with schema
init_test_db() {
    local db_path=$1
    local binary=$2

    # Just storing a dummy memory will initialize the schema
    DATABASE_URL="sqlite://$db_path" "$binary" remember "Test initialization" \
        --namespace "test:init" --importance 1 > /dev/null 2>&1 || true
}

# ============================================================================
# API Key Validation
# ============================================================================

# Check if API key is configured
check_api_key() {
    local binary=$1

    if "$binary" config show-key > /dev/null 2>&1; then
        print_green "[SETUP] API key configured"
        return 0
    else
        print_red "[ERROR] No API key configured"
        echo "Run: $binary secrets init"
        echo "Or set: export ANTHROPIC_API_KEY=sk-ant-api03-..."
        return 1
    fi
}

# Check if API key is available (either env or configured)
has_api_key() {
    if [ -n "$ANTHROPIC_API_KEY" ]; then
        return 0
    fi

    local binary
    binary=$(get_binary_path)
    "$binary" config show-key > /dev/null 2>&1
}

# ============================================================================
# Performance Measurement
# ============================================================================

# Measure command execution time in milliseconds
# Usage: measure_time "command" args...
# Returns: elapsed_ms (sets global variable ELAPSED_MS)
measure_time() {
    local start_ns end_ns

    start_ns=$(date +%s%N)
    "$@" > /dev/null 2>&1
    end_ns=$(date +%s%N)

    ELAPSED_MS=$(( (end_ns - start_ns) / 1000000 ))
    echo "$ELAPSED_MS"
}

# Measure and print execution time
measure_and_print() {
    local description=$1
    shift

    local elapsed
    elapsed=$(measure_time "$@")

    echo "$description: ${elapsed}ms"
    echo "$elapsed"
}

# Measure average time over multiple iterations
# Usage: measure_average 5 "command" args...
measure_average() {
    local iterations=$1
    shift
    local total_ms=0

    print_cyan "Running $iterations iterations..."

    for i in $(seq 1 "$iterations"); do
        local elapsed
        elapsed=$(measure_time "$@")
        total_ms=$((total_ms + elapsed))
        echo "  Iteration $i: ${elapsed}ms"
    done

    local avg_ms=$((total_ms / iterations))
    echo "Average: ${avg_ms}ms"
    echo "$avg_ms"
}

# ============================================================================
# Mock Data Generation
# ============================================================================

# Generate random UUID (for test data)
generate_uuid() {
    cat /proc/sys/kernel/random/uuid 2>/dev/null || uuidgen 2>/dev/null || \
        echo "$(date +%s)-$(( RANDOM % 10000 ))-$(( RANDOM % 10000 ))-$(( RANDOM % 10000 ))-$(( RANDOM % 100000000 ))"
}

# Create sample memory with specified parameters
# Usage: create_memory binary db_path content namespace importance
create_memory() {
    local binary=$1
    local db_path=$2
    local content=$3
    local namespace=$4
    local importance=$5

    DATABASE_URL="sqlite://$db_path" "$binary" remember --content "$content" \
        --namespace "$namespace" --importance "$importance" 2>&1
}

# Create batch of sample memories
# Usage: create_sample_memories binary db_path count namespace
create_sample_memories() {
    local binary=$1
    local db_path=$2
    local count=$3
    local namespace=$4

    print_cyan "Creating $count sample memories..."

    for i in $(seq 1 "$count"); do
        local content="Sample memory $i - Generated for testing purposes"
        local importance=$(( (i % 10) + 1 ))

        create_memory "$binary" "$db_path" "$content" "$namespace" "$importance" > /dev/null 2>&1
    done

    print_green "Created $count memories"
}

# Create memories with specific importance distribution
# Usage: create_tiered_memories binary db_path critical_count important_count low_count namespace
create_tiered_memories() {
    local binary=$1
    local db_path=$2
    local critical_count=$3   # importance >= 8
    local important_count=$4  # importance == 7
    local low_count=$5        # importance < 7
    local namespace=$6

    print_cyan "Creating tiered memories (critical: $critical_count, important: $important_count, low: $low_count)..."

    # Critical memories (importance >= 8)
    for i in $(seq 1 "$critical_count"); do
        local content="Critical decision $i - High importance architectural choice"
        create_memory "$binary" "$db_path" "$content" "$namespace" 8 > /dev/null 2>&1
    done

    # Important memories (importance == 7)
    for i in $(seq 1 "$important_count"); do
        local content="Important pattern $i - Useful coding pattern or insight"
        create_memory "$binary" "$db_path" "$content" "$namespace" 7 > /dev/null 2>&1
    done

    # Low-importance memories
    for i in $(seq 1 "$low_count"); do
        local importance=$(( (i % 6) + 1 ))  # 1-6
        local content="Low priority note $i - Reference information"
        create_memory "$binary" "$db_path" "$content" "$namespace" "$importance" > /dev/null 2>&1
    done

    local total=$((critical_count + important_count + low_count))
    print_green "Created $total tiered memories"
}

# Create memories with specific keywords for search testing
# Usage: create_keyword_memories binary db_path
create_keyword_memories() {
    local binary=$1
    local db_path=$2
    local namespace=${3:-project:test}

    print_cyan "Creating keyword-rich memories for search testing..."

    # Database-related memories
    create_memory "$binary" "$db_path" \
        "Chose PostgreSQL for main database - ACID guarantees, JSON support, excellent performance" \
        "$namespace" 8 > /dev/null 2>&1

    create_memory "$binary" "$db_path" \
        "Using Redis for session caching and rate limiting - fast in-memory operations, TTL support" \
        "$namespace" 7 > /dev/null 2>&1

    # API-related memories
    create_memory "$binary" "$db_path" \
        "REST API with versioning (/v1/), pagination via cursor, standard HTTP codes" \
        "$namespace" 7 > /dev/null 2>&1

    # Performance memories
    create_memory "$binary" "$db_path" \
        "Optimized product search with FTS5 full-text indexing - reduced query time from 800ms to 50ms" \
        "$namespace" 7 > /dev/null 2>&1

    # Bug fix memories
    create_memory "$binary" "$db_path" \
        "Fixed race condition in cart checkout - wrapped HashMap in Arc<RwLock>, added transaction isolation" \
        "$namespace" 6 > /dev/null 2>&1

    print_green "Created 5 keyword-rich memories"
}

# ============================================================================
# Wait and Retry Utilities
# ============================================================================

# Wait for condition to be true
# Usage: wait_for "condition command" timeout_seconds
wait_for() {
    local condition=$1
    local timeout=${2:-10}
    local elapsed=0

    while ! eval "$condition" > /dev/null 2>&1; do
        if [ $elapsed -ge $timeout ]; then
            print_red "Timeout waiting for: $condition"
            return 1
        fi
        sleep 1
        elapsed=$((elapsed + 1))
    done

    return 0
}

# Retry command until success
# Usage: retry 5 "command" args...
retry() {
    local max_attempts=$1
    shift
    local attempt=1

    while [ $attempt -le $max_attempts ]; do
        if "$@"; then
            return 0
        fi

        print_yellow "Attempt $attempt/$max_attempts failed, retrying..."
        attempt=$((attempt + 1))
        sleep 1
    done

    print_red "All $max_attempts attempts failed"
    return 1
}

# ============================================================================
# Test Environment Setup
# ============================================================================

# Set up common test environment
# Usage: setup_test_env "test_name"
# Sets: BIN, TEST_DB, DATABASE_URL
setup_test_env() {
    local test_name=$1

    # Detect project root
    export PROJECT_ROOT
    PROJECT_ROOT=$(get_project_root)

    # Ensure binary exists
    export BIN
    BIN=$(ensure_binary)

    # Create test database
    export TEST_DB
    TEST_DB=$(create_test_db "$test_name")
    export DATABASE_URL="sqlite://$TEST_DB"

    # Validate API key if needed
    if [ "${SKIP_API_KEY_CHECK:-0}" -eq 0 ]; then
        if ! check_api_key "$BIN"; then
            exit 1
        fi
    fi

    print_cyan "[SETUP] Test environment ready"
    echo "  Binary: $BIN"
    echo "  Database: $TEST_DB"
}

# Tear down test environment
teardown_test_env() {
    if [ -n "$TEST_DB" ]; then
        cleanup_test_db "$TEST_DB"
    fi
}

# ============================================================================
# Namespace Query Helpers
# ============================================================================

# Generate SQL WHERE clause for namespace matching
# Handles JSON-serialized namespace format in database
# Usage: namespace_where_clause "project:myproject"
# Returns: SQL WHERE clause fragment
namespace_where_clause() {
    local namespace_str="$1"

    if [ "$namespace_str" = "global" ]; then
        echo "json_extract(namespace, '\$.type') = 'global'"
    elif echo "$namespace_str" | grep -q '^project:'; then
        local project_name=$(echo "$namespace_str" | sed 's/^project://')
        echo "json_extract(namespace, '\$.type') = 'project' AND json_extract(namespace, '\$.name') = '$project_name'"
    elif echo "$namespace_str" | grep -q '^session:.*\/'; then
        local project_name=$(echo "$namespace_str" | sed 's/^session:\([^/]*\)\/.*$/\1/')
        local session_id=$(echo "$namespace_str" | sed 's/^session:[^/]*\/\(.*\)$/\1/')
        echo "json_extract(namespace, '\$.type') = 'session' AND json_extract(namespace, '\$.project') = '$project_name' AND json_extract(namespace, '\$.session_id') = '$session_id'"
    else
        # Unknown format - fall back to Global
        echo "json_extract(namespace, '\$.type') = 'global'"
    fi
}

# Delete memories by namespace
# SQLite doesn't allow json_extract in DELETE with FTS triggers
# So we select IDs first, then delete by ID
delete_by_namespace() {
    local db_path="$1"
    local namespace_str="$2"
    local where_clause=$(namespace_where_clause "$namespace_str")

    # Get IDs to delete
    local ids=$(DATABASE_URL="sqlite://$db_path" sqlite3 "$db_path" \
        "SELECT id FROM memories WHERE $where_clause" 2>/dev/null) || true

    # Delete each ID (if any)
    if [ -n "$ids" ]; then
        while IFS= read -r id; do
            if [ -n "$id" ]; then
                DATABASE_URL="sqlite://$db_path" sqlite3 "$db_path" \
                    "DELETE FROM memories WHERE id='$id'" 2>/dev/null || true
            fi
        done <<< "$ids"
    fi

    return 0
}

# ============================================================================
# Exports
# ============================================================================

# Export all functions for use in test scripts
export -f print_color print_red print_green print_yellow print_blue print_cyan
export -f section
export -f pass fail skip warn test_summary
export -f assert_contains assert_not_contains assert_success assert_failure
export -f assert_lt assert_gt assert_eq
export -f assert_file_exists assert_file_not_exists
export -f get_project_root get_binary_path ensure_binary
export -f create_test_db cleanup_test_db init_test_db
export -f check_api_key has_api_key
export -f measure_time measure_and_print measure_average
export -f generate_uuid create_memory create_sample_memories create_tiered_memories create_keyword_memories
export -f wait_for retry
export -f setup_test_env teardown_test_env
export -f namespace_where_clause delete_by_namespace
