# Crash Recovery Documentation

**Created**: 2025-11-05
**Incident**: Multiple crashes with iTerm2 terminal corruption

## Root Cause Analysis

### Primary Issues Identified

1. **Stale PID File Causing Port Conflicts**
   - `.claude/server.pid` contained PID 74955 (dead process)
   - Actual mnemosyne server was PID 75180 on port 3000
   - Test server script repeatedly failed trying to start on occupied port
   - No validation that PID in file is still running

2. **Terminal Corruption Mechanism**
   - Background processes writing to stderr while shell is active
   - iTerm2 cannot distinguish prompt output from background process output
   - Multiple sources writing ANSI codes/control sequences simultaneously
   - This corrupts terminal state and can crash iTerm2 itself

3. **Hook Output Contributing to Corruption**
   - Hook scripts were writing status messages to stderr
   - During testing, multiple hooks firing simultaneously
   - Combined with test script output created stderr flood
   - Solution: Gate all hook stderr behind `CC_HOOK_DEBUG=1` flag

4. **Test Infrastructure Issues**
   - `timeout` command not available on macOS (GNU coreutils package)
   - Tests failing with "command not found"
   - No proper cleanup between test runs
   - Background processes not properly detached

### How Crashes Occurred

**Crash Sequence**:
1. Test server script runs in background
2. Finds port 3000 occupied (due to stale PID tracking)
3. Attempts to kill and restart repeatedly
4. Each attempt writes to stderr
5. Hooks also writing to stderr (if CC_HOOK_DEBUG was set)
6. Multiple background processes flooding terminal with stderr
7. iTerm2's terminal emulator state corrupts from competing ANSI sequences
8. Eventually crashes iTerm2 itself, requiring restart

**Evidence Found**:
- Process list showing mnemosyne PID 75180 on port 3000
- Stale PID file with 74955 (dead)
- Hook modifications in git diff (CC_HOOK_DEBUG gates)
- Test logs showing "Port 3000 already in use" errors
- No crash reports (indicates terminal corruption, not program crash)

## Recovery Steps

### Immediate Cleanup

```bash
# 1. Kill all mnemosyne processes safely
pkill -9 mnemosyne

# 2. Verify port is free
lsof -i :3000 || echo "Port 3000 is free"

# 3. Remove stale PID files
rm -f .claude/server.pid

# 4. Check for background test processes
ps aux | grep -E "(test-server|mnemosyne)" | grep -v grep

# 5. Kill any stale test processes
pkill -f "test-server"
```

### Verify System State

```bash
# Check no mnemosyne processes running
ps aux | grep mnemosyne | grep -v grep

# Check no orphaned background jobs
jobs -l

# Verify port availability
lsof -i :3000
```

### If Terminal is Corrupted

```bash
# Reset terminal state
reset

# Or restart shell
exec $SHELL

# If iTerm2 is unresponsive: Force quit and restart
```

## Prevention Strategies

### 1. Hook Silence by Default

All hooks now gate stderr output behind `CC_HOOK_DEBUG`:

```bash
# In all hook scripts
if [ "${CC_HOOK_DEBUG:-0}" = "1" ]; then
  echo "Debug message" >&2
fi
```

**Usage**:
- Production: `CC_HOOK_DEBUG=0` (default) - silent
- Debug: `export CC_HOOK_DEBUG=1` - verbose

### 2. Improved PID File Handling

Test server script should:
```bash
# Validate PID before using
if [ -f "$PID_FILE" ]; then
  OLD_PID=$(cat "$PID_FILE")
  if kill -0 "$OLD_PID" 2>/dev/null; then
    # Process exists, safe to kill
    kill "$OLD_PID"
  else
    # Stale PID, just remove file
    rm -f "$PID_FILE"
  fi
fi
```

### 3. Proper Background Process Detachment

All background processes should:
```bash
# Full fd detachment
nohup command \
  </dev/null \
  >>"$LOG_FILE" 2>&1 \
  & echo $! > "$PID_FILE"

# NOT just: command &
# NOT: command > log 2>&1 &  (still attached to terminal)
```

### 4. Test Infrastructure Fixes

**macOS Timeout Workaround**:

The GNU `timeout` command is not available on macOS by default. Solutions:

```bash
# Option 1: Install GNU coreutils (provides gtimeout)
brew install coreutils
gtimeout 10s ./test-script.sh

# Option 2: Use macOS native timeout (if available)
timeout 10s ./test-script.sh

# Option 3: Implement bash-based timeout
( cmdpid=$BASHPID; (sleep 10; kill $cmdpid) & exec ./test-script.sh )

# Option 4: Check availability before using
if command -v timeout >/dev/null 2>&1; then
  timeout 10s ./test-script.sh
elif command -v gtimeout >/dev/null 2>&1; then
  gtimeout 10s ./test-script.sh
else
  # Run without timeout or use bash alternative
  ./test-script.sh
fi
```

**Current Status**: Project scripts don't use `timeout` command. If adding timeout to future test scripts, use one of the above approaches.

### 5. Cleanup Script

**Script Location**: `scripts/cleanup-processes.sh`

A comprehensive cleanup script is provided:
```bash
# Basic cleanup (processes and PID files)
./scripts/cleanup-processes.sh

# Cleanup including log files
./scripts/cleanup-processes.sh --clean-logs
```

**What it does**:
- Gracefully terminates all mnemosyne and test-server processes (SIGTERM)
- Force kills if graceful termination fails (SIGKILL)
- Removes stale PID files
- Checks port 3000 status
- Optionally cleans log files with --clean-logs flag
- Provides clear status feedback throughout

## Safe Testing Protocol

### Before Running Tests

```bash
# 1. Clean state
./scripts/cleanup-processes.sh

# 2. Verify clean
ps aux | grep mnemosyne | grep -v grep || echo "Clean"

# 3. Ensure hooks are silent
unset CC_HOOK_DEBUG  # or export CC_HOOK_DEBUG=0
```

### During Testing

```bash
# Run tests with proper logging
./test-script.sh 2>&1 | tee test-output.log

# NOT: ./test-script.sh &  (can corrupt terminal)
```

### After Testing

```bash
# Always cleanup
./scripts/cleanup-processes.sh
```

## Diagnostic Commands

### Check System State

```bash
# All mnemosyne processes
ps aux | grep mnemosyne | grep -v grep

# Port usage
lsof -i :3000

# PID file contents
cat .claude/server.pid 2>/dev/null || echo "No PID file"

# Validate PID is running
PID=$(cat .claude/server.pid 2>/dev/null)
kill -0 "$PID" 2>/dev/null && echo "Running" || echo "Dead/Invalid"

# Hook debug status
echo "CC_HOOK_DEBUG=${CC_HOOK_DEBUG:-0}"

# Background jobs
jobs -l
```

### Check for Terminal Issues

```bash
# If terminal is slow/corrupted
reset

# Check for processes writing to terminal
lsof | grep "$(tty)"

# Kill processes attached to current TTY
pkill -t "$(basename $(tty))"
```

## Architecture Decisions

### Why Gate Hook Output?

**Problem**: Hooks fire on every tool use, creating stderr noise that:
- Distracts user from actual work
- Can corrupt terminal when multiple hooks fire
- Makes debugging harder (signal-to-noise ratio)

**Solution**: Silent by default, verbose on demand
- Production: Clean, minimal output
- Development: Export `CC_HOOK_DEBUG=1` for visibility

### Why nohup + Full FD Redirection?

**Problem**: Background processes without proper detachment:
- Remain attached to controlling terminal
- Write to parent shell's stderr
- Can receive SIGHUP when terminal closes
- Compete with shell prompt for output

**Solution**: Triple protection
1. `nohup` - ignore SIGHUP signal
2. `</dev/null` - disconnect stdin
3. `>>"$LOG" 2>&1` - redirect all output to log

### Why PID Validation?

**Problem**: PID files can become stale if:
- Process crashes without cleanup
- System reboot
- Manual kill
- PID reused by different process

**Solution**: Always validate with `kill -0 $PID`
- Returns 0 if process exists
- Returns non-zero if dead/invalid
- No actual signal sent (safe check)

## Future Improvements

1. **PID Lock Files**: Use flock for atomic PID management
2. **Health Monitoring**: Periodic health checks with automatic cleanup
3. **Structured Logging**: JSON logs instead of text to stderr
4. **Process Supervision**: Consider using supervise/systemd for server
5. **Test Isolation**: Run tests in separate process groups
6. **Terminal Safety**: Detect terminal type and disable ANSI codes if needed

## Resolution

**Issue Resolved**: 2025-11-05 (Commit 87b7a33)

After implementing the terminal corruption fixes (eec1a33, 048f26d), a new issue emerged:

**Problem**: File descriptor leak causing EIO error (errno -5) on fd 17 during hook execution.

**Root Cause**: Subprocess calls in hooks (uuidgen, date, mnemosyne, jq) were inheriting file descriptors from parent process without proper stdin protection.

**Solution**: Added explicit stdin protection to all subprocess invocations:
- Commands not reading stdin: Added `< /dev/null`
- jq reading files: Changed to `jq ... < "$FILE"` pattern
- jq in pipes: Left unchanged (pipe is stdin)

**Files Fixed**:
- `.claude/hooks/session-start.sh` (4 changes)
- `.claude/hooks/post-tool-use.sh` (5 changes)
- `.claude/hooks/on-stop.sh` (2 changes)

**Validation**: Comprehensive test cycle with 689 passing unit tests and 12 specialized FD safety tests confirmed complete resolution. See [FD_LEAK_FIX_TEST_RESULTS.md](FD_LEAK_FIX_TEST_RESULTS.md) for full test report.

**System Status**: ✅ **Stable** - All issues resolved, no regressions detected.

---

## References

- [iTerm2 Terminal Corruption Issues](https://github.com/zsh-users/zsh-autosuggestions/issues/107)
- [Bash Background Process Best Practices](https://stackoverflow.com/questions/48446853/preventing-background-process-from-writing-to-console)
- [Proper Process Daemonization](https://technology.amis.nl/tech/linux-background-process-and-redirecting-the-standard-input-output-and-error-stream/)
- [FD Leak Fix Test Results](FD_LEAK_FIX_TEST_RESULTS.md) - Comprehensive validation report

---

**Last Updated**: 2025-11-05
**Status**: ✅ All issues resolved and validated
