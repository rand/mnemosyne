# Autonomous Session Orchestration E2E Tests

Comprehensive end-to-end tests for the autonomous session orchestration system.

## Overview

These tests validate the complete event broadcasting pipeline:

```text
Session Start
    ↓ (session-start.sh hook)
API Server Auto-Start
    ↓ (health check)
SSE Subscriber Connect
    ↓ (HTTP POST /events/emit)
CLI Commands → Events
    ↓ (SSE /events/stream)
Orchestrator Receives Events
    ↓ (session-end.sh hook)
Graceful Shutdown
```

## Test Files

### `test_autonomous_session.sh`

Main E2E test validating:

1. **Session Initialization** (Test 1)
   - API server auto-starts via session-start hook emulation
   - Health check passes
   - SessionStarted event emitted

2. **API Server Health** (Test 2)
   - `/health` endpoint returns healthy status
   - Server version reported

3. **CLI Event Emission - remember** (Test 3)
   - `mnemosyne remember` command executes
   - RememberExecuted event emitted to API server
   - Event propagates through system

4. **CLI Event Emission - recall** (Test 4)
   - `mnemosyne recall` command executes
   - RecallExecuted event emitted to API server
   - Event propagates through system

5. **CLI Event Emission - status** (Test 5)
   - `mnemosyne status` command executes
   - StatusCheckExecuted event emitted to API server
   - Event propagates through system

6. **SSE Event Stream** (Test 6)
   - `/events/stream` endpoint accepts SSE connections
   - Events are broadcast via SSE
   - SSE client receives event data

7. **Event Persistence** (Test 7)
   - Events stored in database (if enabled)
   - Event count matches emitted events

8. **Session End** (Test 8)
   - SessionEnded event emitted via session-end hook emulation
   - API server stops gracefully (SIGTERM)
   - Server fully shut down

9. **Hook Scripts Validation** (Test 9)
   - `session-start.sh` exists and is executable
   - `session-end.sh` exists and is executable

10. **Event Flow Verification** (Test 10)
    - API server logs show startup messages
    - Event emission detected in logs
    - SSE connections detected in logs

### `helpers.sh`

Test utilities providing:

- **API Server Management**:
  - `start_api_server`: Start server and wait for health
  - `stop_api_server`: Graceful shutdown with timeout
  - `is_api_server_running`: Check if server is up
  - `get_api_server_version`: Query server version

- **SSE Stream Testing**:
  - `subscribe_sse`: Connect to SSE stream and capture events
  - `count_sse_events`: Count events in SSE output
  - `extract_sse_event`: Extract specific event by type
  - `verify_sse_event`: Check if event type was received

- **Event Emission**:
  - `emit_test_event`: POST event to `/events/emit`
  - `wait_for_event_in_log`: Wait for event in log file

- **Hook Script Testing**:
  - `execute_session_start_hook`: Run session-start hook
  - `execute_session_end_hook`: Run session-end hook
  - `validate_hook_script`: Check hook script syntax

- **Event Verification**:
  - `event_exists_in_db`: Check if event is in database
  - `count_events_in_db`: Get total event count

- **Log Analysis**:
  - `extract_errors_from_log`: Find error messages
  - `count_log_level`: Count log entries by level
  - `has_warnings`: Check for warnings

## Running Tests

### Prerequisites

1. **Build mnemosyne**:
   ```bash
   cd /Users/rand/src/mnemosyne
   cargo build --release
   ```

2. **Ensure port 3000 is available**:
   ```bash
   lsof -i :3000  # Should be empty
   ```

### Run Tests

```bash
# Run autonomous session test
./tests/e2e/orchestration_new/test_autonomous_session.sh

# Run with debug output
CC_HOOK_DEBUG=1 ./tests/e2e/orchestration_new/test_autonomous_session.sh
```

### Expected Output

```
========================================
E2E Test: Autonomous Session Orchestration
========================================

[SETUP] Test environment ready
  Binary: /Users/rand/src/mnemosyne/target/release/mnemosyne
  Database: /tmp/mnemosyne_autonomous_session_1234567890.db

========================================
Test 1: Session Start (API Server Auto-Start)
========================================

[PASS] API server ready (version: 2.1.0, PID: 12345)
[PASS] SessionStarted event emitted

========================================
Test 2: API Server Health Check
========================================

[PASS] API server health check passed

========================================
Test 3: CLI Event Emission (remember)
========================================

[PASS] remember command executed successfully
[PASS] RememberExecuted event emitted to API server

... (more tests)

========================================
Test Summary
========================================
Passed: 15
Failed: 0
========================================
All tests passed!
```

## Test Coverage

### Event Types Tested

- ✅ SessionStarted
- ✅ SessionEnded
- ✅ CliCommandStarted
- ✅ CliCommandCompleted
- ✅ RememberExecuted
- ✅ RecallExecuted
- ✅ StatusCheckExecuted
- ✅ DatabaseOperation

### System Components Tested

- ✅ Session start hook (`session-start.sh`)
- ✅ Session end hook (`session-end.sh`)
- ✅ API server lifecycle (start/stop)
- ✅ API server health endpoint (`/health`)
- ✅ Event emission endpoint (`/events/emit`)
- ✅ SSE stream endpoint (`/events/stream`)
- ✅ CLI event bridge (HTTP POST)
- ✅ Event persistence (database)

### Integration Points Tested

- ✅ Hook → API server → Health check
- ✅ CLI → Event bridge → API server
- ✅ API server → SSE broadcaster → SSE client
- ✅ API server → Event persistence → Database
- ✅ Graceful shutdown on session end

## Debugging

### Common Issues

**1. API server fails to start**

Check logs:
```bash
cat .claude/server.log
```

Common causes:
- Port 3000 already in use
- Binary not found or not executable
- Database initialization failed

**2. SSE stream receives no events**

Verify event emission:
```bash
curl -X POST http://localhost:3000/events/emit \
  -H "Content-Type: application/json" \
  -d '{"event_type":{"type":"heartbeat","instance_id":"test","timestamp":"2024-01-01T00:00:00Z"}}'
```

**3. Events not in logs**

Set log level:
```bash
RUST_LOG=debug ./tests/e2e/orchestration_new/test_autonomous_session.sh
```

**4. Timeout waiting for health check**

Increase timeout:
```bash
# Edit test file, change MAX_WAIT=15 to MAX_WAIT=30
```

### Debug Mode

Enable debug output from hooks:
```bash
export CC_HOOK_DEBUG=1
./tests/e2e/orchestration_new/test_autonomous_session.sh
```

View all API server output:
```bash
# After test, check full log
cat .claude/server.log
```

## Architecture

### Event Flow

```text
┌─────────────────┐
│ CLI Command     │
│ (remember)      │
└────────┬────────┘
         │ Execute
         ↓
┌─────────────────┐
│ Event Bridge    │ event_bridge.rs
│ emit_event()    │
└────────┬────────┘
         │ HTTP POST /events/emit
         ↓
┌─────────────────┐
│ API Server      │ api/events.rs
│ EventBroadcaster│
└────┬───────┬────┘
     │       │ Broadcast (in-memory)
     │       ↓
     │  ┌─────────────┐
     │  │ SSE Stream  │ /events/stream
     │  │ Connected   │
     │  │ Clients     │
     │  └─────────────┘
     │
     │ Persist
     ↓
┌─────────────────┐
│ Event Database  │ (if enabled)
│ events table    │
└─────────────────┘
```

### SSE Subscriber

```text
┌──────────────────────┐
│ SSE Subscriber       │ orchestration/sse_subscriber.rs
│                      │
│ • Connects to        │
│   /events/stream     │
│ • Receives events    │
│ • Converts to        │
│   AgentEvent         │
│ • Sends to           │
│   Orchestrator       │
└──────────┬───────────┘
           │ CliEventReceived message
           ↓
┌──────────────────────┐
│ Orchestrator Actor   │ orchestration/actors/orchestrator.rs
│                      │
│ • Receives CLI       │
│   events             │
│ • Coordinates agents │
│ • Updates work queue │
└──────────────────────┘
```

## Future Enhancements

### Planned Tests

1. **Multi-Session Test**:
   - Multiple concurrent sessions
   - Session isolation verification
   - Event routing per session

2. **SSE Reconnection Test**:
   - API server restart during SSE connection
   - Exponential backoff validation
   - Event loss prevention

3. **Event Ordering Test**:
   - Verify event sequence preservation
   - Concurrent event emission
   - Race condition detection

4. **Error Recovery Test**:
   - API server unavailable
   - Network errors
   - Malformed events

5. **Performance Test**:
   - High-frequency event emission
   - SSE client limits
   - Event backpressure

### Integration with Existing Tests

Consider adding to:
- `tests/e2e/integration_3_hooks.sh`: Hook integration
- `tests/e2e/orchestration_1_single_agent.sh`: Orchestrator coordination
- `tests/e2e/run_all.sh`: Full test suite

## References

- **Session Hooks**: `.claude/hooks/session-start.sh`, `.claude/hooks/session-end.sh`
- **Event Bridge**: `src/cli/event_bridge.rs`
- **API Server**: `src/cli/api_server.rs`, `src/api/server.rs`
- **SSE Subscriber**: `src/orchestration/sse_subscriber.rs`
- **Event Types**: `src/api/events.rs`, `src/orchestration/events.rs`
- **Orchestrator**: `src/orchestration/actors/orchestrator.rs`

## Maintenance

### Before Committing

1. Run tests locally:
   ```bash
   ./tests/e2e/orchestration_new/test_autonomous_session.sh
   ```

2. Check for port conflicts:
   ```bash
   lsof -i :3000
   ```

3. Clean up test artifacts:
   ```bash
   rm -f .claude/server.pid .claude/server.log /tmp/mnemosyne_*.db
   ```

### CI/CD Integration

Add to CI pipeline:
```yaml
- name: Run Autonomous Session E2E Tests
  run: |
    cargo build --release
    ./tests/e2e/orchestration_new/test_autonomous_session.sh
```

Consider:
- Running in isolated network namespace
- Using ephemeral ports for parallel CI jobs
- Capturing logs as CI artifacts
