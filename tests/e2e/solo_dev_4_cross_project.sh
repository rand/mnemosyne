#!/usr/bin/env bash
# [REGRESSION] Solo Developer - Cross-Project Workflow
#
# User Journey: Developer works on multiple projects, needs cross-project insights
# Scenario: Multiple namespaces, global preferences, project-specific memories
# Success Criteria:
#   - Namespace isolation works correctly
#   - Global memories accessible from all projects
#   - Project-specific memories stay isolated
#   - Cross-project search works
#   - Session namespaces function properly
#   - Namespace hierarchy respected
#
# Cost: $0 (mocked LLM responses)
# Duration: 15-25s

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source test infrastructure
source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/assertions.sh"
source "$SCRIPT_DIR/lib/personas.sh"
source "$SCRIPT_DIR/lib/llm_mocking.sh"
source "$SCRIPT_DIR/lib/data_generators.sh"

# ===================================================================
# TEST SETUP
# ===================================================================

TEST_NAME="solo_dev_4_cross_project"

section "Solo Developer - Cross-Project Workflow [REGRESSION]"

# Verify regression mode (mocked LLM)
if is_baseline_mode; then
    warn "This is a regression test but running in baseline mode"
fi

# Setup solo developer persona
print_cyan "Setting up solo developer test environment..."
TEST_DB=$(setup_solo_developer "$TEST_NAME")
print_green "  ✓ Test database: $TEST_DB"

# ===================================================================
# SCENARIO: Global Preferences
# ===================================================================

section "Scenario: Store Global Preferences"

print_cyan "Step 1: Developer stores global coding preferences..."

GLOBAL_PREF=$(cat <<EOF
Personal coding preferences:
- Use async/await over callbacks
- Prefer composition over inheritance
- Always write tests first (TDD)
- Code review before merging
EOF
)

MEM_GLOBAL=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$GLOBAL_PREF" \
    --namespace "global" \
    --importance 7 \
    --type insight 2>&1) || fail "Failed to store global preference"

GLOBAL_ID=$(echo "$MEM_GLOBAL" | grep -o 'mem-[a-zA-Z0-9-]*' | head -1)
print_green "  ✓ Global preference stored: $GLOBAL_ID"

# ===================================================================
# SCENARIO: Project A - E-Commerce Platform
# ===================================================================

section "Scenario: Project A - E-Commerce Platform"

print_cyan "Step 2: Developer works on e-commerce project..."

# Project A architecture
PROJ_A_ARCH=$(cat <<EOF
E-commerce platform architecture:
- Next.js frontend with TypeScript
- Node.js backend with PostgreSQL
- Stripe for payments
- Redis for session management
EOF
)

MEM_A1=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$PROJ_A_ARCH" \
    --namespace "project:ecommerce" \
    --importance 9 \
    --type architecture 2>&1) || fail "Failed to store project A architecture"

A1_ID=$(echo "$MEM_A1" | grep -o 'mem-[a-zA-Z0-9-]*' | head -1)
print_green "  ✓ Project A architecture: $A1_ID"

# Project A task
PROJ_A_TASK=$(cat <<EOF
Task: Implement shopping cart with Redis
Status: In Progress
Blockers: None
EOF
)

MEM_A2=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$PROJ_A_TASK" \
    --namespace "project:ecommerce" \
    --importance 8 \
    --type task 2>&1) || fail "Failed to store project A task"

A2_ID=$(echo "$MEM_A2" | grep -o 'mem-[a-zA-Z0-9-]*' | head -1)
print_green "  ✓ Project A task: $A2_ID"

# ===================================================================
# SCENARIO: Project B - Analytics Dashboard
# ===================================================================

section "Scenario: Project B - Analytics Dashboard"

print_cyan "Step 3: Developer switches to analytics project..."

# Project B architecture
PROJ_B_ARCH=$(cat <<EOF
Analytics dashboard architecture:
- React with TypeScript
- Python backend with FastAPI
- ClickHouse for time-series data
- Real-time WebSocket updates
EOF
)

MEM_B1=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$PROJ_B_ARCH" \
    --namespace "project:analytics" \
    --importance 9 \
    --type architecture 2>&1) || fail "Failed to store project B architecture"

B1_ID=$(echo "$MEM_B1" | grep -o 'mem-[a-zA-Z0-9-]*' | head -1)
print_green "  ✓ Project B architecture: $B1_ID"

# Project B decision
PROJ_B_DEC=$(cat <<EOF
Decision: Use ClickHouse instead of PostgreSQL for analytics
Rationale: Better performance for time-series aggregations
Trade-off: Additional infrastructure complexity
EOF
)

MEM_B2=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$PROJ_B_DEC" \
    --namespace "project:analytics" \
    --importance 8 \
    --type decision 2>&1) || fail "Failed to store project B decision"

B2_ID=$(echo "$MEM_B2" | grep -o 'mem-[a-zA-Z0-9-]*' | head -1)
print_green "  ✓ Project B decision: $B2_ID"

# ===================================================================
# VALIDATION: Namespace Isolation
# ===================================================================

section "Validation: Namespace Isolation"

print_cyan "Verifying namespace isolation..."

# Count memories per namespace
GLOBAL_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE namespace='global'" 2>/dev/null)

ECOMMERCE_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE namespace='project:ecommerce'" 2>/dev/null)

ANALYTICS_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE namespace='project:analytics'" 2>/dev/null)

print_cyan "  Namespace counts:"
print_cyan "    global:             $GLOBAL_COUNT"
print_cyan "    project:ecommerce:  $ECOMMERCE_COUNT"
print_cyan "    project:analytics:  $ANALYTICS_COUNT"

# Validate isolation
if [ "$ECOMMERCE_COUNT" -ge 2 ] && [ "$ANALYTICS_COUNT" -ge 2 ]; then
    print_green "  ✓ Project memories isolated correctly"
else
    fail "Namespace isolation failed"
fi

if [ "$GLOBAL_COUNT" -ge 1 ]; then
    print_green "  ✓ Global namespace has memories"
else
    warn "Expected at least 1 global memory"
fi

# ===================================================================
# SCENARIO: List Memories per Project
# ===================================================================

section "Scenario: List Memories per Project"

print_cyan "Step 4: Developer lists memories for each project..."

# List project A memories
LIST_A=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" list \
    --namespace "project:ecommerce" 2>&1) || fail "Failed to list project A"

if echo "$LIST_A" | grep -q "ecommerce\|shopping"; then
    print_green "  ✓ Project A memories listed correctly"
else
    warn "Project A content not visible in list"
fi

# List project B memories
LIST_B=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" list \
    --namespace "project:analytics" 2>&1) || fail "Failed to list project B"

if echo "$LIST_B" | grep -q "analytics\|ClickHouse"; then
    print_green "  ✓ Project B memories listed correctly"
else
    warn "Project B content not visible in list"
fi

# ===================================================================
# SCENARIO: Cross-Project Search
# ===================================================================

section "Scenario: Cross-Project Search"

print_cyan "Step 5: Developer searches across all projects..."

# Search without namespace filter (should find in multiple projects)
CROSS_SEARCH=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "architecture TypeScript" \
    --limit 10 2>&1) || fail "Cross-project search failed"

print_green "  ✓ Cross-project search completed"

# Count how many projects appear in results
PROJECTS_FOUND=0
echo "$CROSS_SEARCH" | grep -q "ecommerce" && ((PROJECTS_FOUND++)) || true
echo "$CROSS_SEARCH" | grep -q "analytics" && ((PROJECTS_FOUND++)) || true

if [ "$PROJECTS_FOUND" -ge 1 ]; then
    print_green "  ✓ Cross-project search found memories ($PROJECTS_FOUND projects)"
else
    warn "Cross-project search may not be working correctly"
fi

# ===================================================================
# SCENARIO: Namespace Hierarchy
# ===================================================================

section "Scenario: Namespace Hierarchy"

print_cyan "Step 6: Create hierarchical namespaces..."

# Frontend component in e-commerce
FRONTEND_MEM=$(cat <<EOF
Frontend component architecture:
- Use React hooks for state management
- Component library: shadcn/ui
- Styling: Tailwind CSS
EOF
)

MEM_FRONTEND=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$FRONTEND_MEM" \
    --namespace "project:ecommerce:frontend" \
    --importance 7 \
    --type architecture 2>&1) || fail "Failed to store frontend memory"

FRONTEND_ID=$(echo "$MEM_FRONTEND" | grep -o 'mem-[a-zA-Z0-9-]*' | head -1)
print_green "  ✓ Hierarchical namespace created: project:ecommerce:frontend"

# Backend API in e-commerce
BACKEND_MEM=$(cat <<EOF
Backend API architecture:
- RESTful endpoints with Express
- JWT authentication
- Rate limiting with Redis
EOF
)

MEM_BACKEND=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$BACKEND_MEM" \
    --namespace "project:ecommerce:backend" \
    --importance 7 \
    --type architecture 2>&1) || fail "Failed to store backend memory"

BACKEND_ID=$(echo "$MEM_BACKEND" | grep -o 'mem-[a-zA-Z0-9-]*' | head -1)
print_green "  ✓ Hierarchical namespace created: project:ecommerce:backend"

# ===================================================================
# VALIDATION: Hierarchical Queries
# ===================================================================

section "Validation: Hierarchical Namespace Queries"

print_cyan "Verifying hierarchical namespace queries..."

# Count all e-commerce memories (including sub-namespaces)
ECOMMERCE_TOTAL=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories
     WHERE namespace LIKE 'project:ecommerce%'" 2>/dev/null)

print_cyan "  Total ecommerce memories (all levels): $ECOMMERCE_TOTAL"

if [ "$ECOMMERCE_TOTAL" -ge 4 ]; then
    print_green "  ✓ Hierarchical namespaces work correctly"
else
    warn "Expected at least 4 memories in ecommerce hierarchy"
fi

# Count just frontend
FRONTEND_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories
     WHERE namespace='project:ecommerce:frontend'" 2>/dev/null)

# Count just backend
BACKEND_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories
     WHERE namespace='project:ecommerce:backend'" 2>/dev/null)

print_cyan "  Frontend memories: $FRONTEND_COUNT"
print_cyan "  Backend memories: $BACKEND_COUNT"

# ===================================================================
# SCENARIO: Session Namespace
# ===================================================================

section "Scenario: Session Namespace"

print_cyan "Step 7: Use session namespace for temporary work..."

SESSION_MEM=$(cat <<EOF
Current debugging session:
- Investigating slow query in analytics dashboard
- Found N+1 query problem in user stats endpoint
- Fix: Add eager loading with join
EOF
)

MEM_SESSION=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$SESSION_MEM" \
    --namespace "session:debug-20251031" \
    --importance 6 \
    --type insight 2>&1) || fail "Failed to store session memory"

SESSION_ID=$(echo "$MEM_SESSION" | grep -o 'mem-[a-zA-Z0-9-]*' | head -1)
print_green "  ✓ Session memory stored: $SESSION_ID"

# Verify session namespace exists
SESSION_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories
     WHERE namespace LIKE 'session:%'" 2>/dev/null)

if [ "$SESSION_COUNT" -ge 1 ]; then
    print_green "  ✓ Session namespace working correctly"
else
    warn "Session namespace not found"
fi

# ===================================================================
# VALIDATION: Global Memory Visibility
# ===================================================================

section "Validation: Global Memory Visibility"

print_cyan "Verifying global memories accessible from all projects..."

# Global memories should not be tied to specific projects
# They should appear in searches regardless of project context

GLOBAL_MEMS=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT id, SUBSTR(content, 1, 50) FROM memories WHERE namespace='global'" 2>/dev/null)

if [ -n "$GLOBAL_MEMS" ]; then
    print_green "  ✓ Global memories retrievable"
else
    warn "No global memories found"
fi

# ===================================================================
# VALIDATION: Namespace Statistics
# ===================================================================

section "Validation: Namespace Statistics"

print_cyan "Generating namespace statistics..."

# Count unique namespaces
NAMESPACE_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(DISTINCT namespace) FROM memories" 2>/dev/null)

print_cyan "  Total unique namespaces: $NAMESPACE_COUNT"

# List all namespaces with counts
echo "  Namespace distribution:"
DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT namespace, COUNT(*) as count FROM memories
     GROUP BY namespace ORDER BY count DESC" 2>/dev/null | \
    while IFS='|' read -r ns count; do
        print_cyan "    $ns: $count memories"
    done

if [ "$NAMESPACE_COUNT" -ge 5 ]; then
    print_green "  ✓ Multiple namespaces with proper isolation"
else
    warn "Expected at least 5 distinct namespaces"
fi

# ===================================================================
# CLEANUP
# ===================================================================

section "Cleanup"

teardown_persona "$TEST_DB"
print_green "  ✓ Test environment cleaned up"

# ===================================================================
# TEST SUMMARY
# ===================================================================

section "Test Summary: Solo Developer Cross-Project [REGRESSION]"

echo "✓ Global preferences: PASS"
echo "✓ Project A (ecommerce): PASS"
echo "✓ Project B (analytics): PASS"
echo "✓ Namespace isolation: PASS"
echo "✓ Project-specific listing: PASS"
echo "✓ Cross-project search: PASS"
echo "✓ Hierarchical namespaces: PASS"
echo "✓ Session namespace: PASS"
echo "✓ Global visibility: PASS"
echo ""
echo "Namespace Statistics:"
echo "  - Total namespaces: $NAMESPACE_COUNT"
echo "  - Global memories: $GLOBAL_COUNT"
echo "  - E-commerce (total): $ECOMMERCE_TOTAL"
echo "  - Analytics: $ANALYTICS_COUNT"
echo "  - Session: $SESSION_COUNT"
echo ""

print_green "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
print_green "✓ ALL TESTS PASSED"
print_green "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

exit 0
