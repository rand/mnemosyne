#!/bin/bash
# Start mnemosyne server with robust process management
# Prevents terminal corruption via proper fd detachment

set -e

PID_FILE=".claude/server.pid"
LOG_FILE=".claude/server.log"

# Create .claude directory if it doesn't exist
mkdir -p .claude

# Kill any existing server (with PID validation)
if [ -f "$PID_FILE" ]; then
  OLD_PID=$(cat "$PID_FILE")
  # Validate PID is actually running before attempting to kill
  if kill -0 "$OLD_PID" 2>/dev/null; then
    echo "Stopping existing server (PID $OLD_PID)..."
    kill -TERM "$OLD_PID" 2>/dev/null || true
    sleep 1
    # If still alive, force kill
    if kill -0 "$OLD_PID" 2>/dev/null; then
      kill -9 "$OLD_PID" 2>/dev/null || true
      sleep 0.5
    fi
  else
    echo "Removing stale PID file (process $OLD_PID not running)"
  fi
  rm -f "$PID_FILE"
fi

# Check if port is already in use and clean it up
if lsof -i :3000 -t >/dev/null 2>&1; then
  echo "Port 3000 already in use. Attempting to free..."
  PORT_PID=$(lsof -i :3000 -t)
  kill -TERM "$PORT_PID" 2>/dev/null || true
  sleep 1
  # Force kill if still occupied
  if lsof -i :3000 -t >/dev/null 2>&1; then
    echo "Force killing process on port 3000..."
    lsof -i :3000 -t | xargs kill -9 2>/dev/null || true
    sleep 0.5
  fi
fi

# Final verification that port is free
if lsof -i :3000 -t >/dev/null 2>&1; then
  echo "ERROR: Port 3000 still in use after cleanup. Cannot start server."
  lsof -i :3000
  exit 1
fi

# Start server with full fd detachment to prevent terminal corruption
# - stdin redirected from /dev/null (prevents "read" errors)
# - stdout redirected to log file (prevents terminal writes)
# - stderr redirected to log file (prevents terminal corruption)
# - nohup ensures detachment from controlling terminal
# - Background & allows script to continue

echo "Starting server..."
echo "=== Server started at $(date) ===" >> "$LOG_FILE"

nohup ./target/debug/mnemosyne serve \
  </dev/null \
  >>"$LOG_FILE" 2>&1 \
  & echo $! > "$PID_FILE"

SERVER_PID=$(cat "$PID_FILE")
echo "Server started (PID $SERVER_PID)"

# Validate PID is actually running
if ! kill -0 "$SERVER_PID" 2>/dev/null; then
  echo "ERROR: Server process (PID $SERVER_PID) is not running"
  exit 1
fi

# Health check with timeout (20 attempts, 0.5s each = 10s total)
echo "Waiting for server health check..."
for i in {1..20}; do
  if curl -sf http://localhost:3000/health >/dev/null 2>&1; then
    echo "✅ Server ready (http://localhost:3000)"
    echo "📋 Logs: tail -f $LOG_FILE"
    exit 0
  fi
  sleep 0.5
done

# Health check failed
echo "❌ Server failed health check after 10s"
echo "Recent logs:"
tail -20 "$LOG_FILE"
echo ""
echo "Server may still be starting. Check: tail -f $LOG_FILE"
exit 1
