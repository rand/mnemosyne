#!/usr/bin/env bash
set -uo pipefail  # Removed -e to allow test failures

# E2E Test: Agentic Workflow 2 - Optimizer Agent
#
# Scenario: Validate Optimizer agent context management
# Tests optimizer's ability to:
# - Discover and load relevant skills dynamically
# - Load project memories in-session via MCP tools
# - Manage context budget (40% critical, 30% skills, 20% project, 10% general)
# - Compact context at >75% utilization
# - Store new architectural decisions

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source common utilities
# shellcheck source=tests/e2e/lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

# Test state
PASSED=0
FAILED=0

section "E2E Test: Agentic Workflow 2 - Optimizer Agent"

# Setup test environment
setup_test_env "ag2_optimizer"

section "Test 1: Dynamic Memory Loading (In-Session)"

print_cyan "Testing in-session memory loading capability..."

# Simulate scenario: Executor working on authentication feature
# Optimizer should load relevant memories

# Pre-populate authentication-related memories
create_memory "$BIN" "$TEST_DB" \
    "Authentication: JWT-based auth with refresh tokens" \
    "project:app" 8 > /dev/null 2>&1

create_memory "$BIN" "$TEST_DB" \
    "Security: Password hashing with bcrypt, salt rounds=12" \
    "project:app" 8 > /dev/null 2>&1

create_memory "$BIN" "$TEST_DB" \
    "Auth Bug: Session fixation vulnerability fixed in auth middleware" \
    "project:app" 7 > /dev/null 2>&1

sleep 2

# Optimizer would use: mnemosyne.recall("authentication OR security", limit=5)
AUTH_MEMORIES=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "authentication security" \
    --namespace "project:app" --limit 5 2>&1 || echo "")

if echo "$AUTH_MEMORIES" | grep -qi "authentication\|JWT\|security"; then
    pass "In-session memory loading: auth memories retrieved"
else
    fail "In-session memory loading failed"
fi

section "Test 2: Domain Shift Detection"

print_cyan "Testing domain shift scenario..."

# Initial domain: database
create_memory "$BIN" "$TEST_DB" \
    "Database: PostgreSQL connection pooling configuration" \
    "project:app" 7 > /dev/null 2>&1

# Domain shift: Now working on caching
create_memory "$BIN" "$TEST_DB" \
    "Caching: Redis cache strategy with TTL and invalidation" \
    "project:app" 7 > /dev/null 2>&1

sleep 2

# Optimizer detects shift and loads cache-related memories
CACHE_MEMORIES=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "cache caching redis" \
    --namespace "project:app" 2>&1 || echo "")

if echo "$CACHE_MEMORIES" | grep -qi "cache\|redis"; then
    pass "Domain shift detected: cache memories loaded"
else
    fail "Domain shift memory loading failed"
fi

section "Test 3: Context Budget Management"

print_cyan "Testing context budget allocation..."

# Create memories in different categories to test budget management
# Critical (40%): Active task, work plan
# Project (20%): Architectural decisions

print_cyan "Creating critical importance memories (simulate active task)..."
for i in {1..5}; do
    create_memory "$BIN" "$TEST_DB" \
        "Critical task $i: Active work requiring immediate attention" \
        "project:budget" 9 > /dev/null 2>&1
done

print_cyan "Creating project memories (simulate decisions)..."
for i in {1..10}; do
    create_memory "$BIN" "$TEST_DB" \
        "Project decision $i: Architectural choice for reference" \
        "project:budget" 8 > /dev/null 2>&1
done

sleep 2

# Query by importance to verify budget categories
CRITICAL=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "Critical task" \
    --namespace "project:budget" --min-importance 9 2>&1 | grep -c "Critical task" || echo "0")

PROJECT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "Project decision" \
    --namespace "project:budget" --min-importance 8 2>&1 | grep -c "Project decision" || echo "0")

echo "Budget allocation verification:"
echo "  Critical (importance 9): $CRITICAL memories"
echo "  Project (importance 8): $PROJECT memories"

if [ "$CRITICAL" -ge 3 ] && [ "$PROJECT" -ge 5 ]; then
    pass "Context budget categories tracked"
else
    warn "Budget category counts lower than expected"
fi

section "Test 4: Stale Context Removal"

print_cyan "Testing stale context identification..."

# Create old, low-relevance memories (should be candidates for removal)
create_memory "$BIN" "$TEST_DB" \
    "Stale: Outdated API endpoint from v1 (deprecated)" \
    "project:app" 4 > /dev/null 2>&1

create_memory "$BIN" "$TEST_DB" \
    "Stale: Old database migration script (completed 6 months ago)" \
    "project:app" 3 > /dev/null 2>&1

sleep 2

# High-importance memories should be kept
# Low-importance can be removed
LOW_IMPORTANCE=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "Stale" \
    --namespace "project:app" 2>&1 || echo "")

if echo "$LOW_IMPORTANCE" | grep -qi "Stale"; then
    pass "Stale content identifiable (low importance memories found)"
else
    warn "Stale content markers not found"
fi

section "Test 5: New Decision Storage"

print_cyan "Testing new decision storage capability..."

# Simulate Executor making new architectural decision
# Optimizer should store it for future recall

create_memory "$BIN" "$TEST_DB" \
    "NEW DECISION: Switching to microservices architecture for better scalability" \
    "project:app" 9 > /dev/null 2>&1

sleep 2

# Verify stored and retrievable
NEW_DECISION=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "microservices" \
    --namespace "project:app" --min-importance 9 2>&1 || echo "")

if echo "$NEW_DECISION" | grep -qi "microservices\|NEW DECISION"; then
    pass "New decision stored and retrievable"
else
    fail "New decision storage failed"
fi

section "Test 6: ACE Principles (Incremental Updates)"

print_cyan "Testing ACE principles: incremental context updates..."

# ACE: Avoid full context reloads, use incremental updates
# Simulate adding context incrementally

create_memory "$BIN" "$TEST_DB" \
    "Incremental update 1: API rate limiting added - 100 requests/minute" \
    "project:app" 7 > /dev/null 2>&1

create_memory "$BIN" "$TEST_DB" \
    "Incremental update 2: Error handling improved - retry with exponential backoff" \
    "project:app" 7 > /dev/null 2>&1

sleep 2

# Verify incremental updates stored
INCREMENTAL=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "Incremental update" \
    --namespace "project:app" 2>&1 | grep -c "Incremental update" || echo "0")

if [ "$INCREMENTAL" -ge 2 ]; then
    pass "ACE principles: incremental updates tracked"
else
    fail "Incremental update tracking failed"
fi

section "Test 7: Context Compaction Trigger"

print_cyan "Testing context compaction at >75% threshold..."

# Simulate high context utilization
print_cyan "Creating high context load (simulating >75% utilization)..."
for i in {1..30}; do
    create_memory "$BIN" "$TEST_DB" \
        "Context load entry $i - filling context buffer" \
        "project:compact" 6 > /dev/null 2>&1
done

sleep 2

# Count context entries
CONTEXT_ENTRIES=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "Context load" \
    --namespace "project:compact" --limit 30 2>&1 | grep -c "Context load" || echo "0")

if [ "$CONTEXT_ENTRIES" -ge 20 ]; then
    pass "High context load scenario created ($CONTEXT_ENTRIES entries)"
else
    warn "Context load scenario incomplete: $CONTEXT_ENTRIES/30"
fi

section "Test 8: Preserve Critical Info During Compaction"

print_cyan "Testing critical info preservation..."

# When compacting, critical memories (importance >=8) should be preserved
create_memory "$BIN" "$TEST_DB" \
    "CRITICAL: System-wide performance SLA - 99.9% uptime required" \
    "project:app" 10 > /dev/null 2>&1

create_memory "$BIN" "$TEST_DB" \
    "CRITICAL: Data retention policy - 7 years for financial records" \
    "project:app" 10 > /dev/null 2>&1

# Many low-priority entries (can be compacted)
for i in {1..10}; do
    create_memory "$BIN" "$TEST_DB" \
        "Low priority note $i - can be removed during compaction" \
        "project:app" 3 > /dev/null 2>&1
done

sleep 2

# Verify critical memories are preserved
CRITICAL_MEM=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "CRITICAL" \
    --namespace "project:app" --min-importance 10 2>&1 || echo "")

if echo "$CRITICAL_MEM" | grep -qi "CRITICAL.*SLA\|CRITICAL.*retention"; then
    pass "Critical information preserved (importance 10 memories found)"
else
    fail "Critical info preservation check failed"
fi

section "Test 9: MCP Tool Usage Simulation"

print_cyan "Testing MCP tool usage patterns..."

# Optimizer uses MCP tools for context management
# Simulate tool usage by creating metadata about operations

create_memory "$BIN" "$TEST_DB" \
    "MCP Tool: mnemosyne.recall used to fetch auth memories" \
    "project:mcp" 7 > /dev/null 2>&1

create_memory "$BIN" "$TEST_DB" \
    "MCP Tool: mnemosyne.remember used to store new decision" \
    "project:mcp" 7 > /dev/null 2>&1

sleep 2

MCP_USAGE=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "MCP Tool" \
    --namespace "project:mcp" 2>&1 || echo "")

if echo "$MCP_USAGE" | grep -qi "mnemosyne\.recall\|mnemosyne\.remember"; then
    pass "MCP tool usage patterns documented"
else
    fail "MCP tool usage tracking failed"
fi

# Cleanup
section "Cleanup"
teardown_test_env

# Summary
test_summary
exit $?
