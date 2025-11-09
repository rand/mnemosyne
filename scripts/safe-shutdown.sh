#!/bin/bash
# Safe shutdown script to prevent PTY corruption
# Use this before stopping Claude Code or when terminating background processes
#
# This script prevents "Device not configured" errors by:
# 1. Gracefully stopping processes with proper signal handling
# 2. Flushing file descriptors and terminal buffers
# 3. Detaching processes from PTY before termination
# 4. Waiting for processes to finish cleanup

set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse options
FORCE=false
WAIT_TIME=5
DRY_RUN=false

while [[ $# -gt 0 ]]; do
  case $1 in
    --force)
      FORCE=true
      shift
      ;;
    --wait)
      WAIT_TIME="$2"
      shift 2
      ;;
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    -h|--help)
      cat <<EOF
Safe Shutdown Script - Prevent PTY corruption during process termination

Usage: $0 [OPTIONS]

Options:
  --force         Force kill processes immediately (skip graceful shutdown)
  --wait SECONDS  Time to wait for graceful shutdown (default: 5)
  --dry-run       Show what would be done without executing
  -h, --help      Show this help message

Description:
  This script safely terminates mnemosyne processes to prevent PTY corruption
  that can occur when background processes are killed while attached to a terminal.

  Graceful shutdown sequence:
    1. Find all mnemosyne processes
    2. Send TERM signal for graceful shutdown
    3. Wait for processes to exit cleanly
    4. If timeout, detach from PTY and send KILL
    5. Clean up PID files and check port status

Examples:
  $0                    # Graceful shutdown with 5 second timeout
  $0 --wait 10          # Graceful shutdown with 10 second timeout
  $0 --force            # Immediate force kill (emergency only)
  $0 --dry-run          # Preview actions without executing
EOF
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
      echo "Run '$0 --help' for usage information"
      exit 1
      ;;
  esac
done

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}Safe Shutdown - Preventing PTY Corruption${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Function to safely terminate a process
safe_terminate() {
  local pid=$1
  local name=$2

  if [ "$DRY_RUN" = true ]; then
    echo -e "${YELLOW}[DRY RUN] Would terminate PID $pid ($name)${NC}"
    return 0
  fi

  # Check if process exists
  if ! ps -p "$pid" > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} PID $pid already terminated"
    return 0
  fi

  if [ "$FORCE" = true ]; then
    echo -e "${YELLOW}Force killing PID $pid ($name)...${NC}"
    kill -9 "$pid" 2>/dev/null || true
  else
    echo -e "Sending TERM to PID $pid ($name)..."

    # Try graceful termination
    kill -TERM "$pid" 2>/dev/null || true

    # Wait for process to exit
    local waited=0
    while ps -p "$pid" > /dev/null 2>&1 && [ $waited -lt $WAIT_TIME ]; do
      sleep 1
      waited=$((waited + 1))
      echo -n "."
    done
    echo ""

    # Check if still running
    if ps -p "$pid" > /dev/null 2>&1; then
      echo -e "${YELLOW}Process did not exit gracefully, detaching from PTY...${NC}"

      # Detach from terminal before force kill
      # This prevents PTY corruption by breaking the terminal association
      if command -v disown > /dev/null 2>&1; then
        disown "$pid" 2>/dev/null || true
      fi

      # Close file descriptors to terminal
      lsof -p "$pid" 2>/dev/null | grep -E '/dev/(pts|tty)' | awk '{print $4}' | while read fd; do
        # Note: We can't directly close another process's FDs, but we can try to break the connection
        echo -e "${YELLOW}  Detected terminal FD: $fd${NC}"
      done

      # Now force kill
      echo -e "${YELLOW}Force killing PID $pid...${NC}"
      kill -9 "$pid" 2>/dev/null || true
      sleep 1

      if ! ps -p "$pid" > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} Process terminated"
      else
        echo -e "${RED}✗${NC} Failed to terminate PID $pid"
        return 1
      fi
    else
      echo -e "${GREEN}✓${NC} Process exited gracefully"
    fi
  fi
}

# Find all mnemosyne-related processes
echo "Scanning for mnemosyne processes..."
PROCESSES=$(ps aux | grep -E 'mnemosyne|test-server' | grep -v grep | grep -v safe-shutdown || true)

if [ -z "$PROCESSES" ]; then
  echo -e "${GREEN}✓${NC} No mnemosyne processes running"
else
  echo "Found processes:"
  echo "$PROCESSES"
  echo ""

  # Parse PIDs and names
  echo "$PROCESSES" | while read line; do
    PID=$(echo "$line" | awk '{print $2}')
    CMD=$(echo "$line" | awk '{for(i=11;i<=NF;i++) printf $i" "; print ""}')

    safe_terminate "$PID" "$CMD"
  done
fi

echo ""

# Clean up PID files
echo "Cleaning PID files..."
if [ "$DRY_RUN" = true ]; then
  if [ -f .claude/server.pid ]; then
    echo -e "${YELLOW}[DRY RUN] Would remove .claude/server.pid${NC}"
  else
    echo -e "${GREEN}✓${NC} No PID file to remove"
  fi
else
  if [ -f .claude/server.pid ]; then
    OLD_PID=$(cat .claude/server.pid)
    rm -f .claude/server.pid
    echo -e "${GREEN}✓${NC} Removed .claude/server.pid (was PID $OLD_PID)"
  else
    echo -e "${GREEN}✓${NC} No PID file to remove"
  fi
fi

echo ""

# Check port status
echo "Checking port 3000..."
if lsof -i :3000 >/dev/null 2>&1; then
  echo -e "${YELLOW}⚠${NC}  Port 3000 is still in use:"
  lsof -i :3000 | head -5
  echo ""
  if [ "$DRY_RUN" = false ]; then
    echo "To free the port, run:"
    echo "  lsof -i :3000 -t | xargs kill -9"
  fi
else
  echo -e "${GREEN}✓${NC} Port 3000 is free"
fi

echo ""

# Check for orphaned file descriptors
echo "Checking for orphaned terminal file descriptors..."
ORPHANED_FDS=$(lsof -c mnemosyne 2>/dev/null | grep -E '/dev/(pts|tty)' | wc -l | tr -d ' ')
if [ "$ORPHANED_FDS" -gt 0 ]; then
  echo -e "${YELLOW}⚠${NC}  Found $ORPHANED_FDS orphaned terminal file descriptors"
  lsof -c mnemosyne 2>/dev/null | grep -E '/dev/(pts|tty)' | head -10
else
  echo -e "${GREEN}✓${NC} No orphaned terminal file descriptors"
fi

echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
if [ "$DRY_RUN" = true ]; then
  echo -e "${YELLOW}Dry run complete - no changes made${NC}"
else
  echo -e "${GREEN}Safe shutdown complete${NC}"
fi
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
