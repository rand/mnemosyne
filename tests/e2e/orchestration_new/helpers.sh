#!/usr/bin/env bash
#
# Helper utilities for autonomous orchestration E2E tests
#
# Provides functions for:
# - API server lifecycle management
# - SSE stream testing
# - Event verification
# - Hook script testing

# ============================================================================
# API Server Management
# ============================================================================

# Start API server and wait for health
# Usage: start_api_server BIN DB_PATH PID_FILE LOG_FILE
# Returns: 0 on success, 1 on failure
start_api_server() {
    local bin=$1
    local db_path=$2
    local pid_file=$3
    local log_file=$4
    local max_wait=${5:-15}

    # Clean up old PID file if exists
    if [ -f "$pid_file" ]; then
        local old_pid
        old_pid=$(cat "$pid_file" 2>/dev/null || echo "")
        if [ -n "$old_pid" ] && kill -0 "$old_pid" 2>/dev/null; then
            echo "Warning: Killing existing API server (PID: $old_pid)" >&2
            kill -TERM "$old_pid" 2>/dev/null || true
            sleep 2
            if kill -0 "$old_pid" 2>/dev/null; then
                kill -9 "$old_pid" 2>/dev/null || true
            fi
        fi
        rm -f "$pid_file"
    fi

    # Start API server
    DATABASE_URL="sqlite://$db_path" nohup "$bin" api-server > "$log_file" 2>&1 &
    local server_pid=$!
    echo "$server_pid" > "$pid_file"

    echo "API server started (PID: $server_pid)" >&2

    # Wait for health check
    local waited=0
    while [ $waited -lt $max_wait ]; do
        if curl -s --max-time 1 http://localhost:3000/health > /dev/null 2>&1; then
            echo "API server ready (waited ${waited}s)" >&2
            return 0
        fi

        # Check if process died
        if ! kill -0 "$server_pid" 2>/dev/null; then
            echo "Error: API server process died (check $log_file)" >&2
            return 1
        fi

        sleep 1
        waited=$((waited + 1))
    done

    echo "Error: API server health check timeout (waited ${max_wait}s)" >&2
    return 1
}

# Stop API server gracefully
# Usage: stop_api_server PID_FILE [timeout_secs]
# Returns: 0 on success, 1 on failure
stop_api_server() {
    local pid_file=$1
    local timeout=${2:-5}

    if [ ! -f "$pid_file" ]; then
        echo "Warning: PID file not found: $pid_file" >&2
        return 1
    fi

    local server_pid
    server_pid=$(cat "$pid_file" 2>/dev/null || echo "")

    if [ -z "$server_pid" ]; then
        echo "Warning: Empty PID file" >&2
        rm -f "$pid_file"
        return 1
    fi

    if ! kill -0 "$server_pid" 2>/dev/null; then
        echo "Warning: Process not running (PID: $server_pid)" >&2
        rm -f "$pid_file"
        return 1
    fi

    # Graceful shutdown (SIGTERM)
    echo "Stopping API server (PID: $server_pid)..." >&2
    kill -TERM "$server_pid" 2>/dev/null || true

    # Wait for graceful shutdown
    local waited=0
    while [ $waited -lt $timeout ]; do
        if ! kill -0 "$server_pid" 2>/dev/null; then
            echo "API server stopped gracefully (${waited}s)" >&2
            rm -f "$pid_file"
            return 0
        fi
        sleep 1
        waited=$((waited + 1))
    done

    # Force kill if timeout
    echo "Warning: Graceful shutdown timeout, force killing..." >&2
    kill -9 "$server_pid" 2>/dev/null || true
    sleep 1

    if ! kill -0 "$server_pid" 2>/dev/null; then
        rm -f "$pid_file"
        return 0
    else
        echo "Error: Failed to stop API server (PID: $server_pid)" >&2
        return 1
    fi
}

# Check if API server is running
# Usage: is_api_server_running
# Returns: 0 if running, 1 if not
is_api_server_running() {
    curl -s --max-time 1 http://localhost:3000/health > /dev/null 2>&1
}

# Get API server version
# Usage: get_api_server_version
# Outputs: version string or "unknown"
get_api_server_version() {
    if is_api_server_running; then
        curl -s http://localhost:3000/health 2>/dev/null | jq -r '.version // "unknown"'
    else
        echo "unknown"
    fi
}

# ============================================================================
# SSE Stream Testing
# ============================================================================

# Subscribe to SSE stream and capture events
# Usage: subscribe_sse OUTPUT_FILE DURATION_SECS
# Returns: 0 on success
subscribe_sse() {
    local output_file=$1
    local duration=${2:-10}

    timeout "${duration}s" curl -N -s http://localhost:3000/events/stream > "$output_file" 2>&1 &
    local sse_pid=$!

    # Wait for connection
    sleep 2

    echo "$sse_pid"
}

# Count events in SSE output
# Usage: count_sse_events OUTPUT_FILE
# Outputs: number of events
count_sse_events() {
    local output_file=$1

    if [ ! -f "$output_file" ]; then
        echo "0"
        return
    fi

    grep -c "^data:" "$output_file" 2>/dev/null || echo "0"
}

# Extract SSE event by type
# Usage: extract_sse_event OUTPUT_FILE EVENT_TYPE
# Outputs: JSON event data
extract_sse_event() {
    local output_file=$1
    local event_type=$2

    if [ ! -f "$output_file" ]; then
        return 1
    fi

    # Extract data lines
    grep "^data:" "$output_file" 2>/dev/null | sed 's/^data: //' | \
        jq -r "select(.event_type.type == \"$event_type\")" 2>/dev/null
}

# Verify SSE event received
# Usage: verify_sse_event OUTPUT_FILE EVENT_TYPE
# Returns: 0 if event found, 1 if not
verify_sse_event() {
    local output_file=$1
    local event_type=$2

    local event
    event=$(extract_sse_event "$output_file" "$event_type")

    [ -n "$event" ]
}

# ============================================================================
# Event Emission Testing
# ============================================================================

# Emit test event via POST /events/emit
# Usage: emit_test_event EVENT_JSON
# Returns: 0 on success, 1 on failure
emit_test_event() {
    local event_json=$1

    local response
    response=$(curl -s -X POST \
        -H "Content-Type: application/json" \
        -d "$event_json" \
        http://localhost:3000/events/emit 2>&1)

    local exit_code=$?

    if [ $exit_code -eq 0 ]; then
        echo "$response" | jq -r '.success // false' | grep -q "true"
    else
        return 1
    fi
}

# Wait for event to appear in log
# Usage: wait_for_event_in_log LOG_FILE PATTERN TIMEOUT_SECS
# Returns: 0 if found, 1 if timeout
wait_for_event_in_log() {
    local log_file=$1
    local pattern=$2
    local timeout=${3:-10}

    local waited=0
    while [ $waited -lt $timeout ]; do
        if grep -q "$pattern" "$log_file" 2>/dev/null; then
            return 0
        fi
        sleep 1
        waited=$((waited + 1))
    done

    return 1
}

# ============================================================================
# Hook Script Testing
# ============================================================================

# Execute session-start hook
# Usage: execute_session_start_hook HOOK_SCRIPT SESSION_ID
# Returns: 0 on success, 1 on failure
execute_session_start_hook() {
    local hook_script=$1
    local session_id=$2

    if [ ! -f "$hook_script" ]; then
        echo "Error: Hook script not found: $hook_script" >&2
        return 1
    fi

    if [ ! -x "$hook_script" ]; then
        echo "Error: Hook script not executable: $hook_script" >&2
        return 1
    fi

    # Execute hook
    export SESSION_ID="$session_id"
    "$hook_script" 2>&1
}

# Execute session-end hook
# Usage: execute_session_end_hook HOOK_SCRIPT SESSION_ID
# Returns: 0 on success, 1 on failure
execute_session_end_hook() {
    local hook_script=$1
    local session_id=$2

    if [ ! -f "$hook_script" ]; then
        echo "Error: Hook script not found: $hook_script" >&2
        return 1
    fi

    if [ ! -x "$hook_script" ]; then
        echo "Error: Hook script not executable: $hook_script" >&2
        return 1
    fi

    # Execute hook
    export SESSION_ID="$session_id"
    "$hook_script" 2>&1
}

# Validate hook script syntax
# Usage: validate_hook_script HOOK_SCRIPT
# Returns: 0 if valid, 1 if invalid
validate_hook_script() {
    local hook_script=$1

    if [ ! -f "$hook_script" ]; then
        echo "Error: Hook script not found: $hook_script" >&2
        return 1
    fi

    # Check shebang
    if ! head -1 "$hook_script" | grep -q "^#!/"; then
        echo "Warning: Hook script missing shebang: $hook_script" >&2
    fi

    # Syntax check (bash -n)
    bash -n "$hook_script" 2>&1
}

# ============================================================================
# Event Verification Helpers
# ============================================================================

# Check if event exists in database
# Usage: event_exists_in_db DB_PATH EVENT_TYPE
# Returns: 0 if found, 1 if not
event_exists_in_db() {
    local db_path=$1
    local event_type=$2

    # Check if events table exists
    local table_exists
    table_exists=$(DATABASE_URL="sqlite://$db_path" sqlite3 "$db_path" \
        "SELECT name FROM sqlite_master WHERE type='table' AND name='events'" 2>/dev/null || echo "")

    if [ -z "$table_exists" ]; then
        return 1
    fi

    # Query for event
    local event_count
    event_count=$(DATABASE_URL="sqlite://$db_path" sqlite3 "$db_path" \
        "SELECT COUNT(*) FROM events WHERE event_type LIKE '%${event_type}%'" 2>/dev/null || echo "0")

    [ "$event_count" -gt 0 ]
}

# Count events in database
# Usage: count_events_in_db DB_PATH
# Outputs: event count
count_events_in_db() {
    local db_path=$1

    # Check if events table exists
    local table_exists
    table_exists=$(DATABASE_URL="sqlite://$db_path" sqlite3 "$db_path" \
        "SELECT name FROM sqlite_master WHERE type='table' AND name='events'" 2>/dev/null || echo "")

    if [ -z "$table_exists" ]; then
        echo "0"
        return
    fi

    DATABASE_URL="sqlite://$db_path" sqlite3 "$db_path" \
        "SELECT COUNT(*) FROM events" 2>/dev/null || echo "0"
}

# ============================================================================
# Log Analysis Helpers
# ============================================================================

# Extract error messages from log
# Usage: extract_errors_from_log LOG_FILE
# Outputs: error lines
extract_errors_from_log() {
    local log_file=$1

    if [ ! -f "$log_file" ]; then
        return
    fi

    grep -E "ERROR|FATAL|Error:|error:" "$log_file" 2>/dev/null || true
}

# Count log entries by level
# Usage: count_log_level LOG_FILE LEVEL
# Outputs: count
count_log_level() {
    local log_file=$1
    local level=$2

    if [ ! -f "$log_file" ]; then
        echo "0"
        return
    fi

    grep -c "$level" "$log_file" 2>/dev/null || echo "0"
}

# Check for warning patterns
# Usage: has_warnings LOG_FILE
# Returns: 0 if warnings found, 1 if not
has_warnings() {
    local log_file=$1

    if [ ! -f "$log_file" ]; then
        return 1
    fi

    grep -qE "WARN|WARNING|Warning:" "$log_file" 2>/dev/null
}

# ============================================================================
# Exports
# ============================================================================

# Export all functions for use in test scripts
export -f start_api_server stop_api_server is_api_server_running get_api_server_version
export -f subscribe_sse count_sse_events extract_sse_event verify_sse_event
export -f emit_test_event wait_for_event_in_log
export -f execute_session_start_hook execute_session_end_hook validate_hook_script
export -f event_exists_in_db count_events_in_db
export -f extract_errors_from_log count_log_level has_warnings
