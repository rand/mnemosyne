# Quick Start: Autonomous Session E2E Tests

## TL;DR

```bash
# Build mnemosyne
cd /Users/rand/src/mnemosyne
./scripts/rebuild-and-update-install.sh

# Run tests
./tests/e2e/orchestration_new/test_autonomous_session.sh
```

## Prerequisites

1. **Mnemosyne built**:
   ```bash
   cargo build --release
   # Binary at: target/release/mnemosyne
   ```

2. **Port 3000 available**:
   ```bash
   # Check if port is in use
   lsof -i :3000

   # If in use, kill the process
   kill -9 $(lsof -t -i:3000)
   ```

3. **Dependencies installed**:
   - `curl` (for HTTP requests)
   - `jq` (for JSON parsing)
   - `sqlite3` (for database queries)

## Running Tests

### Basic Run

```bash
./tests/e2e/orchestration_new/test_autonomous_session.sh
```

### Debug Mode

```bash
# Enable hook debug output
CC_HOOK_DEBUG=1 ./tests/e2e/orchestration_new/test_autonomous_session.sh

# Enable Rust debug logging
RUST_LOG=debug ./tests/e2e/orchestration_new/test_autonomous_session.sh

# Enable both
CC_HOOK_DEBUG=1 RUST_LOG=debug ./tests/e2e/orchestration_new/test_autonomous_session.sh
```

### Verbose Mode

```bash
# Show all output (no suppression)
set -x
./tests/e2e/orchestration_new/test_autonomous_session.sh
```

## Expected Output

### Success (All Tests Pass)

```
========================================
E2E Test: Autonomous Session Orchestration
========================================

[SETUP] Test environment ready
  Binary: /Users/rand/src/mnemosyne/target/release/mnemosyne
  Database: /tmp/mnemosyne_autonomous_session_1699999999.db

========================================
Test 1: Session Start (API Server Auto-Start)
========================================

API server started (PID: 12345)
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

========================================
Test 4: CLI Event Emission (recall)
========================================

[PASS] recall command executed successfully
[PASS] RecallExecuted event emitted to API server

========================================
Test 5: CLI Event Emission (status)
========================================

[PASS] status command executed successfully
[PASS] StatusCheckExecuted event emitted to API server

========================================
Test 6: SSE Event Stream Endpoint
========================================

[PASS] SSE stream endpoint working (received events)
  Events received via SSE: 3

========================================
Test 7: Event Persistence
========================================

[PASS] Events persisted in database (count: 5)

========================================
Test 8: Session End (Graceful Shutdown)
========================================

[PASS] SessionEnded event emitted
Stopping API server gracefully (simulating session-end.sh)...
[PASS] API server stopped gracefully
[PASS] API server fully stopped

========================================
Test 9: Hook Scripts Validation
========================================

[PASS] session-start.sh exists and is executable
[PASS] session-end.sh exists and is executable

========================================
Test 10: Event Flow Verification
========================================

[PASS] API server startup logged
[PASS] Event emission detected in logs
[PASS] SSE stream connections detected in logs
  Total log lines: 127

========================================
Cleanup
========================================

[PASS] Test artifacts cleaned up

========================================
Test Summary
========================================
Passed: 18
Failed: 0
========================================
All tests passed!
```

### Partial Success (Some Warnings)

```
...
========================================
Test 6: SSE Event Stream Endpoint
========================================

[WARN] SSE stream connected but no events received

========================================
Test 7: Event Persistence
========================================

[WARN] Events table does not exist (event persistence may not be enabled)

...

========================================
Test Summary
========================================
Passed: 15
Failed: 0
========================================
All tests passed!
```

**Note**: Warnings are acceptable - they indicate optional features not enabled.

### Failure

```
...
========================================
Test 1: Session Start (API Server Auto-Start)
========================================

[FAIL] API server failed to start (check .claude/server.log)

Last 20 lines of log:
thread 'main' panicked at 'Failed to bind address'
...

========================================
Test Summary
========================================
Passed: 0
Failed: 1
========================================
Some tests failed
```

## Troubleshooting

### Problem: Port 3000 already in use

```bash
# Symptom
[FAIL] API server failed to start
Error: Address already in use (os error 48)

# Solution
lsof -i :3000
kill -9 $(lsof -t -i:3000)

# Re-run tests
./tests/e2e/orchestration_new/test_autonomous_session.sh
```

### Problem: Binary not found

```bash
# Symptom
[ERROR] Binary not found after build: /path/to/mnemosyne

# Solution
cd /Users/rand/src/mnemosyne
cargo build --release

# Verify binary exists
ls -la target/release/mnemosyne

# Re-run tests
./tests/e2e/orchestration_new/test_autonomous_session.sh
```

### Problem: Timeout waiting for API server

```bash
# Symptom
[FAIL] API server startup timeout (waited 15s)

# Check logs
cat .claude/server.log

# Common causes:
# 1. Port conflict (see above)
# 2. Database initialization failed
# 3. Binary crashed on startup

# Solution: Check logs and address root cause
```

### Problem: SSE stream receives no events

```bash
# Symptom
[WARN] SSE stream connected but no events received

# Verify event emission works
curl -X POST http://localhost:3000/events/emit \
  -H "Content-Type: application/json" \
  -d '{
    "event_type": {
      "type": "heartbeat",
      "instance_id": "test",
      "timestamp": "2024-01-01T00:00:00Z"
    }
  }'

# Check SSE stream
curl -N http://localhost:3000/events/stream &
# Should see: data: {...}

# If no output, check API server logs
cat .claude/server.log
```

### Problem: Tests hang

```bash
# Symptom
Test appears frozen, no output

# Solutions:
# 1. Check for orphaned processes
ps aux | grep mnemosyne

# 2. Check for lock files
ls -la .claude/*.pid

# 3. Kill all related processes
pkill -9 mnemosyne
rm -f .claude/server.pid .claude/server.log

# 4. Re-run tests
./tests/e2e/orchestration_new/test_autonomous_session.sh
```

## Clean Up

### Manual Cleanup

```bash
# Stop API server
if [ -f .claude/server.pid ]; then
    kill -9 $(cat .claude/server.pid)
    rm -f .claude/server.pid
fi

# Remove logs
rm -f .claude/server.log .claude/memory-state.json

# Remove test databases
rm -f /tmp/mnemosyne_*.db

# Verify cleanup
lsof -i :3000  # Should be empty
ps aux | grep mnemosyne  # Should show no running processes
```

### Full Reset

```bash
# Nuclear option: Clean everything
pkill -9 mnemosyne
rm -f .claude/server.pid .claude/server.log .claude/memory-state.json
rm -f /tmp/mnemosyne_*.db
lsof -ti :3000 | xargs kill -9 2>/dev/null || true

# Rebuild
cargo clean
cargo build --release

# Re-run tests
./tests/e2e/orchestration_new/test_autonomous_session.sh
```

## Test Files

```
tests/e2e/orchestration_new/
├── test_autonomous_session.sh    # Main E2E test (10 test cases)
├── helpers.sh                     # Test utilities
├── README.md                      # Detailed documentation
├── TEST_COVERAGE.md               # Coverage analysis
└── QUICKSTART.md                  # This file
```

## Next Steps

1. **Run tests**: Validate your environment
2. **Check output**: Ensure all tests pass
3. **Review logs**: Understand event flow
4. **Iterate**: Modify and re-test as needed

## Getting Help

- **Documentation**: See `README.md` for detailed docs
- **Coverage**: See `TEST_COVERAGE.md` for test coverage
- **Implementation**: See `.claude/hooks/session-*.sh` for hook scripts
- **Source**: See `src/cli/event_bridge.rs`, `src/orchestration/sse_subscriber.rs`

## Common Commands

```bash
# Build
cargo build --release

# Run tests
./tests/e2e/orchestration_new/test_autonomous_session.sh

# Debug
CC_HOOK_DEBUG=1 RUST_LOG=debug ./tests/e2e/orchestration_new/test_autonomous_session.sh

# Clean
pkill -9 mnemosyne; rm -f .claude/*.pid .claude/*.log /tmp/mnemosyne_*.db

# Check port
lsof -i :3000

# View logs
cat .claude/server.log

# Test SSE
curl -N http://localhost:3000/events/stream

# Health check
curl http://localhost:3000/health | jq
```

## Success Criteria

Tests should:
- ✅ Pass all 10 test cases
- ✅ Complete in < 60 seconds
- ✅ Clean up all resources
- ✅ Show no errors in logs (warnings OK)
- ✅ Leave no orphaned processes
