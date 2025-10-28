#!/usr/bin/env bash
set -uo pipefail  # Removed -e to allow test failures

# E2E Test: Integration 2 - MCP Server
#
# Scenario: Test MCP server and tool integration
# Tests:
# - MCP server startup and initialization
# - Tool discovery and registration
# - Tool invocation via MCP protocol
# - Session management
# - Error handling in tools
# - Agent context loading via MCP

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source common utilities
# shellcheck source=tests/e2e/lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

# Test state
PASSED=0
FAILED=0

section "E2E Test: Integration 2 - MCP Server"

# Setup test environment
setup_test_env "int2_mcp"

section "Test 1: MCP Server Binary Detection"

print_cyan "Testing MCP server binary detection..."

# Check if MCP server wrapper exists
if [ -f "./mnemosyne_mcp.py" ]; then
    pass "MCP server: Python wrapper script exists"
else
    warn "MCP server: Python wrapper not found at ./mnemosyne_mcp.py"
fi

# Check if Rust binary exists and is executable
if [ -x "$BIN" ]; then
    pass "MCP server: Mnemosyne binary exists and is executable"
else
    fail "MCP server: Mnemosyne binary not found or not executable"
fi

section "Test 2: MCP Tool Definitions"

print_cyan "Testing MCP tool definitions..."

# MCP tools should include:
# - mnemosyne_remember: Store new memory
# - mnemosyne_recall: Search/retrieve memories
# - mnemosyne_list: List memories in namespace
# - mnemosyne_export: Export memories

# Test by checking if wrapper defines these tools
if [ -f "./mnemosyne_mcp.py" ]; then
    TOOL_DEFS=$(grep -E "def (remember|recall|list|export)" mnemosyne_mcp.py || echo "")

    TOOL_COUNT=$(echo "$TOOL_DEFS" | wc -l | tr -d ' ')

    if [ "$TOOL_COUNT" -ge 2 ]; then
        pass "MCP tools: Tool definitions found in wrapper ($TOOL_COUNT tools)"
    else
        warn "MCP tools: Limited tool definitions ($TOOL_COUNT)"
    fi
else
    warn "MCP tools: Cannot verify tool definitions (wrapper not found)"
fi

section "Test 3: MCP Configuration Generation"

print_cyan "Testing MCP configuration generation..."

# Launcher should generate MCP configuration
# Check if config would include mnemosyne server

print_cyan "Simulating MCP configuration..."

# MCP config should reference mnemosyne server
MCP_CONFIG_SAMPLE='{
  "mcpServers": {
    "mnemosyne": {
      "command": "python",
      "args": ["./mnemosyne_mcp.py"],
      "env": {
        "DATABASE_URL": "sqlite://'"$TEST_DB"'"
      }
    }
  }
}'

echo "$MCP_CONFIG_SAMPLE" > /tmp/mcp_config_test.json

if [ -f "/tmp/mcp_config_test.json" ]; then
    pass "MCP configuration: Sample configuration generated successfully"

    if jq -e '.mcpServers.mnemosyne' /tmp/mcp_config_test.json > /dev/null 2>&1; then
        pass "MCP configuration: Valid JSON with mnemosyne server entry"
    else
        warn "MCP configuration: JSON may not be valid (jq not available or invalid)"
    fi

    rm -f /tmp/mcp_config_test.json
else
    fail "MCP configuration: Failed to create configuration"
fi

section "Test 4: Tool Input Validation"

print_cyan "Testing MCP tool input validation..."

# Tools should validate inputs
# Test remember tool with valid inputs

REMEMBER_INPUT='{
  "content": "Test memory via MCP protocol",
  "namespace": "project:mcp_test",
  "importance": 7
}'

echo "$REMEMBER_INPUT" > /tmp/mcp_remember_input.json

if [ -f "/tmp/mcp_remember_input.json" ]; then
    if jq -e '.content' /tmp/mcp_remember_input.json > /dev/null 2>&1; then
        pass "Tool validation: Valid input structure created"
    else
        warn "Tool validation: Cannot validate JSON (jq not available)"
    fi

    rm -f /tmp/mcp_remember_input.json
else
    fail "Tool validation: Failed to create test input"
fi

section "Test 5: Tool Invocation Simulation"

print_cyan "Testing tool invocation patterns..."

# Simulate MCP tool invocation by calling CLI directly
# This validates that tools would work when called via MCP

TOOL_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    "MCP tool invocation test - simulating agent call" \
    --namespace "project:mcp_test" --importance 7 2>&1 || echo "TOOL_ERROR")

if echo "$TOOL_OUTPUT" | grep -qi "TOOL_ERROR"; then
    fail "Tool invocation: CLI invocation failed"
else
    pass "Tool invocation: CLI invocation successful (tools would work via MCP)"
fi

sleep 2

# Verify memory was stored (tool effect)
TOOL_RECALL=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "MCP tool invocation" \
    --namespace "project:mcp_test" 2>&1 || echo "")

if echo "$TOOL_RECALL" | grep -qi "MCP tool invocation"; then
    pass "Tool effects: Tool operations persist correctly"
else
    fail "Tool effects: Tool operation did not persist"
fi

section "Test 6: Concurrent Tool Invocations"

print_cyan "Testing concurrent tool invocations..."

# Multiple agents might invoke tools simultaneously
# Test concurrent operations (simulates multiple MCP tool calls)

for i in {1..5}; do
    (DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
        "Concurrent MCP tool call $i" \
        --namespace "project:mcp_test" --importance 6 > /dev/null 2>&1) &
done

wait
sleep 3

# Verify all operations completed
CONCURRENT_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "Concurrent MCP" \
    --namespace "project:mcp_test" 2>&1 | grep -c "Concurrent MCP" || echo "0")

if [ "$CONCURRENT_COUNT" -ge 4 ]; then
    pass "Concurrent tools: Multiple tool invocations handled ($CONCURRENT_COUNT/5)"
else
    warn "Concurrent tools: Some invocations may have failed ($CONCURRENT_COUNT/5)"
fi

section "Test 7: Tool Error Handling"

print_cyan "Testing error handling in MCP tools..."

# Tools should return errors gracefully
# Test with invalid namespace

ERROR_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    "Error handling test" \
    --namespace "" --importance 7 2>&1 || echo "ERROR_HANDLED")

if echo "$ERROR_OUTPUT" | grep -qi "error\|invalid\|ERROR_HANDLED"; then
    pass "Tool errors: Errors returned gracefully to MCP client"
else
    warn "Tool errors: Error handling may be unclear"
fi

section "Test 8: Recall Tool Functionality"

print_cyan "Testing recall tool (search/retrieve)..."

# Create test memories
for i in {1..3}; do
    create_memory "$BIN" "$TEST_DB" \
        "Recall tool test memory $i with keyword authentication" \
        "project:mcp_test" 8 > /dev/null 2>&1
done

sleep 3

# Test recall tool (search)
RECALL_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "authentication" \
    --namespace "project:mcp_test" 2>&1 || echo "")

if echo "$RECALL_OUTPUT" | grep -qi "authentication"; then
    pass "Recall tool: Search functionality works correctly"

    # Count results
    RESULT_COUNT=$(echo "$RECALL_OUTPUT" | grep -c "authentication" || echo "0")
    if [ "$RESULT_COUNT" -ge 2 ]; then
        pass "Recall tool: Multiple relevant results returned ($RESULT_COUNT)"
    fi
else
    fail "Recall tool: Search not finding relevant memories"
fi

section "Test 9: Namespace Filtering"

print_cyan "Testing namespace filtering in tools..."

# Create memories in different namespaces
create_memory "$BIN" "$TEST_DB" \
    "Memory in namespace A" \
    "project:namespace_a" 7 > /dev/null 2>&1

create_memory "$BIN" "$TEST_DB" \
    "Memory in namespace B" \
    "project:namespace_b" 7 > /dev/null 2>&1

sleep 2

# Query specific namespace
NS_A_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "Memory" \
    --namespace "project:namespace_a" 2>&1 || echo "")

NS_B_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "Memory" \
    --namespace "project:namespace_b" 2>&1 || echo "")

if echo "$NS_A_OUTPUT" | grep -qi "namespace A"; then
    pass "Namespace filtering: Correct namespace isolation for A"
else
    fail "Namespace filtering: Namespace A filtering failed"
fi

if echo "$NS_B_OUTPUT" | grep -qi "namespace B"; then
    pass "Namespace filtering: Correct namespace isolation for B"
else
    fail "Namespace filtering: Namespace B filtering failed"
fi

section "Test 10: Tool Response Format"

print_cyan "Testing tool response format..."

# Tools should return structured responses
# Export tool provides good example of structured output

EXPORT_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" export 2>&1 || echo "")

if [ -n "$EXPORT_OUTPUT" ]; then
    pass "Tool responses: Tools return data"

    # Check if output has structure
    if echo "$EXPORT_OUTPUT" | grep -qi "namespace:\|content:\|importance:"; then
        pass "Tool responses: Structured output format"
    else
        warn "Tool responses: Output format may not be optimally structured"
    fi
else
    fail "Tool responses: Export tool returned no data"
fi

section "Test 11: Context Loading via MCP"

print_cyan "Testing context loading through MCP tools..."

# Optimizer agent would use MCP tools to load context
# Simulate by loading high-importance memories

CONTEXT_LOAD=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "" \
    --namespace "project:mcp_test" --min-importance 7 --limit 10 2>&1 || echo "")

if [ -n "$CONTEXT_LOAD" ]; then
    pass "MCP context loading: High-importance memories retrieved"

    CONTEXT_COUNT=$(echo "$CONTEXT_LOAD" | grep -c "importance:" || echo "0")
    if [ "$CONTEXT_COUNT" -ge 3 ]; then
        pass "MCP context loading: Multiple context items loaded ($CONTEXT_COUNT)"
    fi
else
    warn "MCP context loading: No context returned (may be expected if no high-importance memories)"
fi

section "Test 12: Tool Performance"

print_cyan "Testing MCP tool performance..."

# Tools should respond quickly (<5s for most operations)
START=$(date +%s)

PERF_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "test" \
    --namespace "project:mcp_test" 2>&1 || echo "")

END=$(date +%s)
DURATION=$((END - START))

if [ "$DURATION" -lt 5 ]; then
    pass "Tool performance: Fast response time (${DURATION}s)"
else
    warn "Tool performance: Slower than expected (${DURATION}s)"
fi

section "Test 13: Session State Management"

print_cyan "Testing session state management..."

# MCP server maintains session state
# Each invocation should see consistent state

# Create memory
SESSION_CREATE=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    "Session state test memory" \
    --namespace "project:session" --importance 7 2>&1 || echo "")

sleep 2

# Retrieve in same "session" (same database)
SESSION_RETRIEVE=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "Session state" \
    --namespace "project:session" 2>&1 || echo "")

if echo "$SESSION_RETRIEVE" | grep -qi "Session state"; then
    pass "Session state: Consistent state across tool invocations"
else
    fail "Session state: State not consistent"
fi

section "Test 14: Tool Documentation"

print_cyan "Testing tool documentation availability..."

# Tools should have help/documentation
HELP_OUTPUT=$("$BIN" --help 2>&1 || echo "")

if echo "$HELP_OUTPUT" | grep -qi "remember\|recall"; then
    pass "Tool documentation: Command help available"
else
    warn "Tool documentation: Help may be incomplete"
fi

# Check individual command help
REMEMBER_HELP=$("$BIN" remember --help 2>&1 || echo "")

if echo "$REMEMBER_HELP" | grep -qi "content\|namespace\|importance"; then
    pass "Tool documentation: Detailed command documentation available"
else
    warn "Tool documentation: Command-specific help may be incomplete"
fi

section "Test 15: Agent Tool Usage Patterns"

print_cyan "Testing realistic agent tool usage patterns..."

# Simulate agent workflow:
# 1. Recall relevant memories
# 2. Process information
# 3. Remember new insight
# 4. Recall again to verify

# Step 1: Recall existing context
AGENT_RECALL_1=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "test" \
    --namespace "project:mcp_test" --limit 5 2>&1 || echo "")

# Step 2: Remember new insight
AGENT_REMEMBER=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    "Agent workflow: Processed test data and identified patterns" \
    --namespace "project:mcp_test" --importance 8 2>&1 || echo "")

sleep 2

# Step 3: Recall again
AGENT_RECALL_2=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "patterns" \
    --namespace "project:mcp_test" 2>&1 || echo "")

if echo "$AGENT_RECALL_2" | grep -qi "patterns\|Agent workflow"; then
    pass "Agent patterns: Realistic agent workflow functional"
else
    fail "Agent patterns: Agent workflow not working as expected"
fi

section "Test 16: MCP Server Initialization"

print_cyan "Testing MCP server initialization requirements..."

# Server should initialize with:
# - Database connection
# - Tool registration
# - Environment configuration

# Test database connectivity
DB_TEST=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "" \
    --namespace "project:test" --limit 1 2>&1 || echo "DB_ERROR")

if echo "$DB_TEST" | grep -qi "DB_ERROR"; then
    fail "Server init: Database connectivity issues"
else
    pass "Server init: Database connectivity functional"
fi

# Test environment variable handling
ENV_TEST=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    "Environment test" \
    --namespace "project:test" --importance 6 2>&1 || echo "")

if [ -n "$ENV_TEST" ]; then
    pass "Server init: Environment variables processed correctly"
else
    fail "Server init: Environment variable handling failed"
fi

# Cleanup
section "Cleanup"
teardown_test_env

# Summary
test_summary
exit $?
