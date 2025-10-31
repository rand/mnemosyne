#!/usr/bin/env bash
#
# Phase 2.1: Manual MCP Integration Validation Script
#
# This script provides a structured workflow for manually validating
# the complete event integration in a real MCP + Claude Code environment.
#
# Prerequisites:
# 1. Mnemosyne installed (via scripts/install/install.sh)
# 2. Claude Code installed
# 3. mnemosyne-dash installed (cargo install --path crates/mnemosyne-dash)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}ℹ  ${1}${NC}"
}

log_success() {
    echo -e "${GREEN}✓  ${1}${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠  ${1}${NC}"
}

log_error() {
    echo -e "${RED}✗  ${1}${NC}"
}

section() {
    echo ""
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}  ${1}${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

check_prerequisites() {
    section "Checking Prerequisites"

    # Check mnemosyne binary
    if command -v mnemosyne &> /dev/null; then
        MNEMOSYNE_VERSION=$(mnemosyne --version 2>&1 | head -1)
        log_success "mnemosyne found: $MNEMOSYNE_VERSION"
    else
        log_error "mnemosyne not found. Install with: $PROJECT_ROOT/scripts/install/install.sh"
        exit 1
    fi

    # Check dashboard
    if command -v mnemosyne-dash &> /dev/null; then
        log_success "mnemosyne-dash found"
    else
        log_warning "mnemosyne-dash not found. Install with: cargo install --path crates/mnemosyne-dash"
        log_info "Dashboard is optional but recommended for visual validation"
    fi

    # Check Claude Code
    if command -v claude &> /dev/null; then
        log_success "claude (Claude Code) found"
    else
        log_error "Claude Code not found. Install from: https://claude.com/code"
        exit 1
    fi
}

start_api_server() {
    section "Starting API Server"

    local port=8321
    log_info "Starting Mnemosyne API server on port $port..."

    # Kill any existing server
    pkill -f "mnemosyne api-server" || true
    sleep 1

    # Start server in background
    mnemosyne api-server --port $port > /tmp/mnemosyne_api_server.log 2>&1 &
    local pid=$!
    echo $pid > /tmp/mnemosyne_api_server.pid

    log_info "API server PID: $pid"
    sleep 2

    # Check if server started
    if ! ps -p $pid > /dev/null; then
        log_error "API server failed to start. Check logs: /tmp/mnemosyne_api_server.log"
        cat /tmp/mnemosyne_api_server.log
        exit 1
    fi

    # Test health endpoint
    if curl -s http://localhost:$port/health > /dev/null; then
        log_success "API server running at http://localhost:$port"
    else
        log_error "API server not responding to health check"
        exit 1
    fi
}

start_dashboard() {
    section "Starting Dashboard (Optional)"

    if ! command -v mnemosyne-dash &> /dev/null; then
        log_warning "Skipping dashboard (not installed)"
        return
    fi

    log_info "Starting dashboard..."

    # Kill any existing dashboard
    pkill -f "mnemosyne-dash" || true
    sleep 1

    # Start dashboard in background
    mnemosyne-dash --api http://localhost:8321 > /tmp/mnemosyne_dash.log 2>&1 &
    local pid=$!
    echo $pid > /tmp/mnemosyne_dash.pid

    log_info "Dashboard PID: $pid"
    sleep 2

    if ps -p $pid > /dev/null; then
        log_success "Dashboard running. Open http://localhost:3000 in your browser"
        log_info "You should see real-time event updates in the dashboard"
    else
        log_warning "Dashboard failed to start (this is optional)"
    fi
}

test_mcp_integration() {
    section "Testing MCP Integration"

    log_info "Testing that MCP server can be reached..."

    # Check if MCP config exists
    local mcp_config="$HOME/.claude/mcp_config.json"
    if [[ -f "$mcp_config" ]]; then
        log_success "MCP config found at $mcp_config"

        # Check if mnemosyne is configured
        if grep -q "mnemosyne" "$mcp_config"; then
            log_success "Mnemosyne MCP server configured"
        else
            log_warning "Mnemosyne not found in MCP config"
            log_info "Add Mnemosyne to MCP config:"
            log_info '  "mnemosyne": {"command": "mnemosyne", "args": ["mcp"]}'
        fi
    else
        log_warning "MCP config not found. Claude Code may not be configured"
    fi
}

run_manual_validation() {
    section "Manual Validation Steps"

    cat <<EOF

${GREEN}Ready for manual validation!${NC}

Follow these steps in Claude Code:

${YELLOW}1. Memory Operations Test${NC}
   - Run: /skills
   - Run: /memory-store "Test memory for Phase 2 validation" --importance 8
   - Run: /memory-search "Phase 2"
   ${BLUE}Expected:${NC} Dashboard should show MemoryStored and MemoryRecalled events

${YELLOW}2. ICS Context Test${NC}
   - Run: mnemosyne-ics test.md
   - Edit and save the file (Ctrl+S)
   ${BLUE}Expected:${NC} Dashboard should show ContextModified event

${YELLOW}3. Orchestration Test (if available)${NC}
   - Create a simple plan file
   - Run: mnemosyne orchestrate --plan "simple task"
   ${BLUE}Expected:${NC} Dashboard should show AgentStarted/Completed events

${YELLOW}4. SSE Stream Test${NC}
   - Open terminal: curl -N http://localhost:8321/events
   - Perform any Mnemosyne operation
   ${BLUE}Expected:${NC} See real-time SSE event stream

${YELLOW}5. Multiple Subscribers Test${NC}
   - Open 2-3 terminals with: curl -N http://localhost:8321/events
   - Perform operations in Claude Code
   ${BLUE}Expected:${NC} All terminals receive the same events

${GREEN}Validation Checklist:${NC}
  [ ] API server starts and responds to health checks
  [ ] Dashboard connects and shows "Connected" status
  [ ] Memory operations emit events visible in dashboard
  [ ] ICS save operations emit ContextModified events
  [ ] Multiple SSE connections receive events concurrently
  [ ] Events appear in correct order
  [ ] No errors in API server logs (/tmp/mnemosyne_api_server.log)

${BLUE}Press Enter when validation is complete...${NC}
EOF

    read -r
}

cleanup() {
    section "Cleanup"

    log_info "Stopping services..."

    if [[ -f /tmp/mnemosyne_api_server.pid ]]; then
        kill $(cat /tmp/mnemosyne_api_server.pid) 2>/dev/null || true
        rm /tmp/mnemosyne_api_server.pid
        log_success "API server stopped"
    fi

    if [[ -f /tmp/mnemosyne_dash.pid ]]; then
        kill $(cat /tmp/mnemosyne_dash.pid) 2>/dev/null || true
        rm /tmp/mnemosyne_dash.pid
        log_success "Dashboard stopped"
    fi

    log_info "Logs preserved:"
    log_info "  API server: /tmp/mnemosyne_api_server.log"
    log_info "  Dashboard: /tmp/mnemosyne_dash.log"
}

main() {
    echo ""
    echo -e "${BLUE}╔════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║                                                                ║${NC}"
    echo -e "${BLUE}║   Phase 2.1: Manual MCP Integration Validation                ║${NC}"
    echo -e "${BLUE}║                                                                ║${NC}"
    echo -e "${BLUE}╚════════════════════════════════════════════════════════════════╝${NC}"

    trap cleanup EXIT

    check_prerequisites
    start_api_server
    start_dashboard
    test_mcp_integration
    run_manual_validation

    section "Validation Complete"
    log_success "Manual validation workflow complete"
    log_info "Review dashboard and SSE streams to confirm event delivery"
}

main "$@"
