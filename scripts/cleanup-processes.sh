#!/bin/bash
# Safe cleanup of all mnemosyne processes and state files
# Use before/after testing or when processes are stuck

set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Mnemosyne Process Cleanup"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Check for running processes
RUNNING_COUNT=$(ps aux | grep -E "mnemosyne|test-server" | grep -v grep | wc -l | tr -d ' ')

if [ "$RUNNING_COUNT" -gt 0 ]; then
  echo "Found $RUNNING_COUNT mnemosyne/test-server processes:"
  ps aux | grep -E "mnemosyne|test-server" | grep -v grep || true
  echo ""

  echo "Sending TERM signal..."
  pkill -TERM mnemosyne 2>/dev/null || true
  pkill -TERM test-server 2>/dev/null || true
  sleep 2

  # Check if any survived
  REMAINING=$(ps aux | grep -E "mnemosyne|test-server" | grep -v grep | wc -l | tr -d ' ')
  if [ "$REMAINING" -gt 0 ]; then
    echo "Force killing remaining processes..."
    pkill -9 mnemosyne 2>/dev/null || true
    pkill -9 test-server 2>/dev/null || true
    sleep 1
  fi

  # Final check
  FINAL=$(ps aux | grep -E "mnemosyne|test-server" | grep -v grep | wc -l | tr -d ' ')
  if [ "$FINAL" -eq 0 ]; then
    echo "✅ All processes stopped"
  else
    echo "⚠️  Warning: Some processes may still be running:"
    ps aux | grep -E "mnemosyne|test-server" | grep -v grep || true
  fi
else
  echo "✅ No mnemosyne/test-server processes running"
fi

echo ""

# Clean up PID files
echo "Cleaning PID files..."
if [ -f .claude/server.pid ]; then
  OLD_PID=$(cat .claude/server.pid)
  rm -f .claude/server.pid
  echo "✅ Removed .claude/server.pid (was PID $OLD_PID)"
else
  echo "✅ No PID file to remove"
fi

echo ""

# Check port status
echo "Checking port 3000..."
if lsof -i :3000 >/dev/null 2>&1; then
  echo "⚠️  Port 3000 is still in use:"
  lsof -i :3000
  echo ""
  echo "To free the port, run:"
  echo "  lsof -i :3000 -t | xargs kill -9"
else
  echo "✅ Port 3000 is free"
fi

echo ""

# Optional: Clean log files
if [ "$1" = "--clean-logs" ]; then
  echo "Cleaning log files..."
  if [ -f .claude/server.log ]; then
    rm -f .claude/server.log
    echo "✅ Removed .claude/server.log"
  fi
  if ls /tmp/test*.log >/dev/null 2>&1; then
    rm -f /tmp/test*.log
    echo "✅ Removed test logs from /tmp"
  fi
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Cleanup complete"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Usage:"
echo "  ./scripts/cleanup-processes.sh              # Clean processes and PID files"
echo "  ./scripts/cleanup-processes.sh --clean-logs # Also clean log files"
echo ""
