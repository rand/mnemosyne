#!/usr/bin/env bash
# Advanced Test Assertions
#
# Provides rich assertion helpers for e2e tests beyond basic pass/fail.
# Supports JSON validation, pattern matching, threshold checking, etc.

# Source common utilities
_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=tests/e2e/lib/common.sh
source "$_LIB_DIR/common.sh"

# ===================================================================
# JSON ASSERTIONS
# ===================================================================

# Assert JSON is valid
# Args: json_string
assert_valid_json() {
    local json="$1"

    if ! echo "$json" | jq empty 2>/dev/null; then
        fail "Invalid JSON: $json"
        return 1
    fi

    return 0
}

# Assert JSON field exists
# Args: json_string, field_path
assert_json_field_exists() {
    local json="$1"
    local field="$2"

    if ! echo "$json" | jq -e "$field" >/dev/null 2>&1; then
        fail "JSON field '$field' does not exist"
        return 1
    fi

    return 0
}

# Assert JSON field equals value
# Args: json_string, field_path, expected_value
assert_json_field_equals() {
    local json="$1"
    local field="$2"
    local expected="$3"

    local actual=$(echo "$json" | jq -r "$field")

    if [ "$actual" != "$expected" ]; then
        fail "JSON field '$field' mismatch: expected '$expected', got '$actual'"
        return 1
    fi

    return 0
}

# Assert JSON array length
# Args: json_string, array_path, expected_length
assert_json_array_length() {
    local json="$1"
    local array="$2"
    local expected="$3"

    local actual=$(echo "$json" | jq "$array | length")

    if [ "$actual" != "$expected" ]; then
        fail "JSON array '$array' length mismatch: expected $expected, got $actual"
        return 1
    fi

    return 0
}

# Assert JSON array not empty
# Args: json_string, array_path
assert_json_array_not_empty() {
    local json="$1"
    local array="$2"

    local length=$(echo "$json" | jq "$array | length")

    if [ "$length" -eq 0 ]; then
        fail "JSON array '$array' is empty"
        return 1
    fi

    return 0
}

# ===================================================================
# STRING ASSERTIONS
# ===================================================================

# Assert string contains substring
# Args: haystack, needle
assert_contains() {
    local haystack="$1"
    local needle="$2"

    if ! echo "$haystack" | grep -qF "$needle"; then
        fail "String does not contain '$needle'"
        fail "  Haystack: ${haystack:0:100}..."
        return 1
    fi

    return 0
}

# Assert string does not contain substring
# Args: haystack, needle
assert_not_contains() {
    local haystack="$1"
    local needle="$2"

    if echo "$haystack" | grep -qF "$needle"; then
        fail "String should not contain '$needle' but does"
        fail "  Haystack: ${haystack:0:100}..."
        return 1
    fi

    return 0
}

# Assert string matches regex
# Args: string, regex
assert_matches() {
    local string="$1"
    local regex="$2"

    if ! echo "$string" | grep -qE "$regex"; then
        fail "String does not match regex: $regex"
        fail "  String: ${string:0:100}..."
        return 1
    fi

    return 0
}

# Assert string equals (exact match)
# Args: actual, expected
assert_equals() {
    local actual="$1"
    local expected="$2"

    if [ "$actual" != "$expected" ]; then
        fail "String mismatch:"
        fail "  Expected: $expected"
        fail "  Actual:   $actual"
        return 1
    fi

    return 0
}

# Assert string not empty
# Args: string, description
assert_not_empty() {
    local string="$1"
    local desc="${2:-string}"

    if [ -z "$string" ]; then
        fail "$desc is empty"
        return 1
    fi

    return 0
}

# ===================================================================
# NUMERIC ASSERTIONS
# ===================================================================

# Assert number greater than threshold
# Args: value, threshold
assert_greater_than() {
    local value="$1"
    local threshold="$2"

    if ! (( $(echo "$value > $threshold" | bc -l) )); then
        fail "Value $value is not greater than $threshold"
        return 1
    fi

    return 0
}

# Assert number less than threshold
# Args: value, threshold
assert_less_than() {
    local value="$1"
    local threshold="$2"

    if ! (( $(echo "$value < $threshold" | bc -l) )); then
        fail "Value $value is not less than $threshold"
        return 1
    fi

    return 0
}

# Assert number within range
# Args: value, min, max
assert_in_range() {
    local value="$1"
    local min="$2"
    local max="$3"

    if ! (( $(echo "$value >= $min && $value <= $max" | bc -l) )); then
        fail "Value $value is not in range [$min, $max]"
        return 1
    fi

    return 0
}

# Assert number equals (with tolerance)
# Args: actual, expected, tolerance (default 0.01)
assert_numeric_equals() {
    local actual="$1"
    local expected="$2"
    local tolerance="${3:-0.01}"

    local diff=$(echo "scale=6; if ($actual > $expected) $actual - $expected else $expected - $actual" | bc)

    if ! (( $(echo "$diff <= $tolerance" | bc -l) )); then
        fail "Numeric mismatch: expected $expected ± $tolerance, got $actual (diff: $diff)"
        return 1
    fi

    return 0
}

# ===================================================================
# FILE ASSERTIONS
# ===================================================================

# Assert file exists
# Args: filepath
assert_file_exists() {
    local filepath="$1"

    if [ ! -f "$filepath" ]; then
        fail "File does not exist: $filepath"
        return 1
    fi

    return 0
}

# Assert directory exists
# Args: dirpath
assert_dir_exists() {
    local dirpath="$1"

    if [ ! -d "$dirpath" ]; then
        fail "Directory does not exist: $dirpath"
        return 1
    fi

    return 0
}

# Assert file contains text
# Args: filepath, search_text
assert_file_contains() {
    local filepath="$1"
    local text="$2"

    if [ ! -f "$filepath" ]; then
        fail "File does not exist: $filepath"
        return 1
    fi

    if ! grep -qF "$text" "$filepath"; then
        fail "File $filepath does not contain: $text"
        return 1
    fi

    return 0
}

# Assert file is empty
# Args: filepath
assert_file_empty() {
    local filepath="$1"

    if [ ! -f "$filepath" ]; then
        fail "File does not exist: $filepath"
        return 1
    fi

    if [ -s "$filepath" ]; then
        fail "File is not empty: $filepath"
        return 1
    fi

    return 0
}

# Assert file size within range
# Args: filepath, min_bytes, max_bytes
assert_file_size() {
    local filepath="$1"
    local min="$2"
    local max="$3"

    if [ ! -f "$filepath" ]; then
        fail "File does not exist: $filepath"
        return 1
    fi

    local size=$(stat -f%z "$filepath" 2>/dev/null || stat -c%s "$filepath" 2>/dev/null)

    if [ "$size" -lt "$min" ] || [ "$size" -gt "$max" ]; then
        fail "File size $size bytes not in range [$min, $max]"
        return 1
    fi

    return 0
}

# ===================================================================
# COMMAND ASSERTIONS
# ===================================================================

# Assert command succeeds (exit code 0)
# Args: command, args...
assert_command_succeeds() {
    local output_file=$(mktemp)

    if ! "$@" >"$output_file" 2>&1; then
        fail "Command failed: $*"
        fail "Output: $(cat "$output_file")"
        rm -f "$output_file"
        return 1
    fi

    rm -f "$output_file"
    return 0
}

# Assert command fails (exit code non-zero)
# Args: command, args...
assert_command_fails() {
    local output_file=$(mktemp)

    if "$@" >"$output_file" 2>&1; then
        fail "Command should have failed but succeeded: $*"
        fail "Output: $(cat "$output_file")"
        rm -f "$output_file"
        return 1
    fi

    rm -f "$output_file"
    return 0
}

# Assert command output contains text
# Args: text_to_find, command, args...
assert_output_contains() {
    local text="$1"
    shift

    local output=$("$@" 2>&1)

    if ! echo "$output" | grep -qF "$text"; then
        fail "Command output does not contain: $text"
        fail "Command: $*"
        fail "Output: ${output:0:200}..."
        return 1
    fi

    return 0
}

# Assert command completes within timeout
# Args: timeout_seconds, command, args...
assert_completes_within() {
    local timeout="$1"
    shift

    local start=$(date +%s)

    if ! timeout "${timeout}s" "$@" >/dev/null 2>&1; then
        fail "Command did not complete within ${timeout}s: $*"
        return 1
    fi

    local end=$(date +%s)
    local duration=$((end - start))

    print_green "  ✓ Completed in ${duration}s (timeout: ${timeout}s)"
    return 0
}

# ===================================================================
# DATABASE ASSERTIONS
# ===================================================================

# Assert memory exists in database
# Args: database_path, memory_id
assert_memory_exists() {
    local db="$1"
    local mem_id="$2"

    local count=$(sqlite3 "$db" "SELECT COUNT(*) FROM memories WHERE id='$mem_id'" 2>/dev/null || echo "0")

    if [ "$count" -eq 0 ]; then
        fail "Memory $mem_id does not exist in database"
        return 1
    fi

    return 0
}

# Assert memory count in namespace
# Args: database_path, namespace, expected_count
assert_memory_count() {
    local db="$1"
    local namespace="$2"
    local expected="$3"

    local actual=$(sqlite3 "$db" "SELECT COUNT(*) FROM memories WHERE namespace='$namespace'" 2>/dev/null || echo "0")

    if [ "$actual" != "$expected" ]; then
        fail "Memory count mismatch in namespace '$namespace': expected $expected, got $actual"
        return 1
    fi

    return 0
}

# Assert link exists between memories
# Args: database_path, source_id, target_id
assert_link_exists() {
    local db="$1"
    local source="$2"
    local target="$3"

    local count=$(sqlite3 "$db" "SELECT COUNT(*) FROM memory_links WHERE source_id='$source' AND target_id='$target'" 2>/dev/null || echo "0")

    if [ "$count" -eq 0 ]; then
        fail "Link does not exist: $source -> $target"
        return 1
    fi

    return 0
}

# ===================================================================
# TIMING ASSERTIONS
# ===================================================================

# Assert operation completes quickly (< threshold)
# Usage: time_operation <threshold_ms> <command> <args...>
# Returns: elapsed time in milliseconds
time_operation() {
    local threshold="$1"
    shift

    local start=$(date +%s%3N)  # milliseconds
    "$@" >/dev/null 2>&1
    local end=$(date +%s%3N)

    local elapsed=$((end - start))

    if [ "$elapsed" -gt "$threshold" ]; then
        warn "Operation took ${elapsed}ms (threshold: ${threshold}ms)"
    else
        print_green "  ✓ Operation completed in ${elapsed}ms"
    fi

    echo "$elapsed"
}

# ===================================================================
# AGGREGATE ASSERTIONS
# ===================================================================

# Assert all items in list satisfy predicate
# Args: predicate_function, items...
assert_all() {
    local predicate="$1"
    shift

    for item in "$@"; do
        if ! "$predicate" "$item"; then
            fail "Predicate failed for item: $item"
            return 1
        fi
    done

    return 0
}

# Assert any item in list satisfies predicate
# Args: predicate_function, items...
assert_any() {
    local predicate="$1"
    shift

    for item in "$@"; do
        if "$predicate" "$item" 2>/dev/null; then
            return 0
        fi
    done

    fail "No item satisfied predicate"
    return 1
}

# Export all assertion functions
export -f assert_valid_json
export -f assert_json_field_exists
export -f assert_json_field_equals
export -f assert_json_array_length
export -f assert_json_array_not_empty
export -f assert_contains
export -f assert_not_contains
export -f assert_matches
export -f assert_equals
export -f assert_not_empty
export -f assert_greater_than
export -f assert_less_than
export -f assert_in_range
export -f assert_numeric_equals
export -f assert_file_exists
export -f assert_dir_exists
export -f assert_file_contains
export -f assert_file_empty
export -f assert_file_size
export -f assert_command_succeeds
export -f assert_command_fails
export -f assert_output_contains
export -f assert_completes_within
export -f assert_memory_exists
export -f assert_memory_count
export -f assert_link_exists
export -f time_operation
export -f assert_all
export -f assert_any
