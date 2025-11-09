# Running Autonomous Session E2E Tests

## Quick Commands

```bash
# Basic run
./tests/e2e/orchestration_new/test_autonomous_session.sh

# With debug output
CC_HOOK_DEBUG=1 RUST_LOG=debug ./tests/e2e/orchestration_new/test_autonomous_session.sh

# Clean up after tests
rm -f .claude/server.{pid,log} /tmp/mnemosyne_*.db
```

## Test Execution Flow

```
1. Build Check
   └─ Ensure mnemosyne binary exists at target/release/mnemosyne

2. Environment Setup
   ├─ Create temporary test database
   ├─ Set DATABASE_URL
   └─ Initialize database schema

3. Test 1: Session Start
   ├─ Start API server manually (simulates session-start.sh)
   ├─ Wait for health check (max 15s)
   └─ Emit SessionStarted event

4. Test 2: Health Check
   └─ Verify /health endpoint returns healthy status

5. Tests 3-5: CLI Command Events
   ├─ Execute remember command → Verify RememberExecuted event
   ├─ Execute recall command → Verify RecallExecuted event
   └─ Execute status command → Verify StatusCheckExecuted event

6. Test 6: SSE Stream
   ├─ Connect to /events/stream endpoint
   ├─ Generate event via CLI command
   └─ Verify events received via SSE

7. Test 7: Event Persistence
   └─ Query events table for stored events

8. Test 8: Session End
   ├─ Emit SessionEnded event
   ├─ Stop API server gracefully (SIGTERM)
   └─ Verify server fully stopped

9. Test 9: Hook Validation
   ├─ Check session-start.sh exists and is executable
   └─ Check session-end.sh exists and is executable

10. Test 10: Log Analysis
    ├─ Verify startup messages in logs
    ├─ Verify event emission in logs
    └─ Verify SSE connections in logs

11. Cleanup
    ├─ Stop API server if still running
    ├─ Remove PID and log files
    └─ Remove test database
```

## Expected Results

### All Tests Pass (18 assertions)

```
Passed: 18
Failed: 0
```

### Acceptable Warnings (4 possible)

These warnings are OK and don't indicate failures:

1. **"RememberExecuted event not found in logs"**
   - Reason: Event logging may be at debug level
   - Impact: None (event emission still verified)

2. **"SSE stream connected but no events received"**
   - Reason: Timing issue or events not logged
   - Impact: Low (SSE connection still verified)

3. **"Events table does not exist"**
   - Reason: Event persistence is optional
   - Impact: None (feature may not be enabled)

4. **"SSE stream connections not detected in logs"**
   - Reason: SSE logging may be disabled
   - Impact: None (SSE functionality still verified)

## Performance Benchmarks

| Operation | Expected Time | Acceptable Range |
|-----------|--------------|------------------|
| API Server Start | 2-5s | < 15s |
| Health Check | < 1s | < 3s |
| CLI Command | < 1s | < 2s |
| SSE Connection | < 2s | < 5s |
| Graceful Shutdown | 1-2s | < 5s |
| **Total Test Time** | **30-45s** | **< 60s** |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All tests passed |
| 1 | One or more tests failed |

## Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `CC_HOOK_DEBUG` | Enable hook debug output | unset |
| `RUST_LOG` | Rust logging level | unset |
| `MNEMOSYNE_DISABLE_AUTO_START_API` | Disable API auto-start | unset |
| `MNEMOSYNE_DISABLE_EVENTS` | Disable event emission | unset |
| `SKIP_API_KEY_CHECK` | Skip API key validation | 1 (set by test) |

## File Artifacts

### Created During Test

- `.claude/server.pid` - API server process ID
- `.claude/server.log` - API server output
- `.claude/memory-state.json` - Session state (if created by hook)
- `/tmp/mnemosyne_autonomous_session_*.db` - Test database

### Cleaned Up After Test

All artifacts are removed by the cleanup trap on test exit.

## Logs Location

- **API Server Log**: `.claude/server.log`
- **Test Output**: stdout/stderr
- **SSE Output**: `/tmp/mnemosyne_sse_test_*.log` (temporary)

## Debugging Failed Tests

### Step 1: Identify Failed Test

```bash
# Look for [FAIL] lines in output
./tests/e2e/orchestration_new/test_autonomous_session.sh | grep FAIL
```

### Step 2: Check Logs

```bash
# API server log
cat .claude/server.log

# Last 50 lines
tail -50 .claude/server.log
```

### Step 3: Verify Prerequisites

```bash
# Binary exists
ls -la target/release/mnemosyne

# Port available
lsof -i :3000

# Dependencies installed
which curl jq sqlite3
```

### Step 4: Run in Debug Mode

```bash
CC_HOOK_DEBUG=1 RUST_LOG=debug ./tests/e2e/orchestration_new/test_autonomous_session.sh
```

### Step 5: Manual Testing

```bash
# Start API server manually
DATABASE_URL="sqlite:///tmp/test.db" ./target/release/mnemosyne api-server &

# Wait for health
sleep 3

# Check health
curl http://localhost:3000/health | jq

# Test event emission
./target/release/mnemosyne internal session-started --instance-id test

# Check SSE
curl -N http://localhost:3000/events/stream

# Cleanup
pkill -9 mnemosyne
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: E2E Tests - Autonomous Session

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Install Dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y curl jq sqlite3

      - name: Build Mnemosyne
        run: cargo build --release

      - name: Run E2E Tests
        run: ./tests/e2e/orchestration_new/test_autonomous_session.sh

      - name: Upload Logs on Failure
        if: failure()
        uses: actions/upload-artifact@v3
        with:
          name: test-logs
          path: |
            .claude/server.log
            /tmp/mnemosyne_*.db
```

### GitLab CI Example

```yaml
e2e_autonomous_session:
  stage: test
  script:
    - cargo build --release
    - ./tests/e2e/orchestration_new/test_autonomous_session.sh
  artifacts:
    when: on_failure
    paths:
      - .claude/server.log
    expire_in: 1 week
```

## Test Isolation

Each test run:

- ✅ Uses unique test database (`/tmp/mnemosyne_autonomous_session_<timestamp>.db`)
- ✅ Generates unique session ID (UUID)
- ✅ Cleans up all artifacts on exit (via trap)
- ✅ Independent of other test runs
- ✅ Can run in parallel (if using different ports)

## Parallel Execution

To run multiple test instances in parallel:

```bash
# Terminal 1
./tests/e2e/orchestration_new/test_autonomous_session.sh

# Terminal 2 (will fail - port conflict)
./tests/e2e/orchestration_new/test_autonomous_session.sh
```

**Note**: Parallel execution currently not supported due to hardcoded port 3000. Future enhancement: Use ephemeral ports.

## Test Maintenance

### Weekly

- Run tests to ensure no regressions
- Check for new warnings
- Update documentation if behavior changes

### Monthly

- Review test coverage
- Add new test cases for new features
- Refactor tests for clarity

### Quarterly

- Benchmark performance
- Update CI/CD integration
- Add missing coverage

## Related Tests

- `test_interactive_mode.sh` - Interactive mode orchestration
- `tests/e2e/orchestration_1_single_agent.sh` - Single agent orchestration
- `tests/e2e/integration_3_hooks.sh` - Hook integration tests

## References

- **Main Documentation**: `README.md`
- **Coverage Analysis**: `TEST_COVERAGE.md`
- **Quick Start**: `QUICKSTART.md`
- **Session Hooks**: `.claude/hooks/session-start.sh`, `.claude/hooks/session-end.sh`
- **Implementation**: `src/cli/event_bridge.rs`, `src/orchestration/sse_subscriber.rs`
