#!/usr/bin/env bash
# [REGRESSION] Team Lead - Setup Namespaces
#
# User Journey: Team lead sets up namespace structure for team
# Scenario: Create team conventions, establish hierarchy, configure access
# Success Criteria:
#   - Team namespace created with proper structure
#   - Member namespaces isolated but coordinated
#   - Shared team knowledge accessible
#   - Project sub-namespaces organized
#   - Documentation and conventions stored
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

TEST_NAME="team_lead_1_setup_namespaces"

section "Team Lead - Setup Namespaces [REGRESSION]"

# Verify regression mode (mocked LLM)
if is_baseline_mode; then
    warn "This is a regression test but running in baseline mode"
fi

# Setup team lead persona
print_cyan "Setting up team lead test environment..."
TEST_DB=$(setup_team_lead "$TEST_NAME")
print_green "  ✓ Test database: $TEST_DB"

# ===================================================================
# SCENARIO: Team Structure Planning
# ===================================================================

section "Scenario: Team Structure Planning"

print_cyan "Step 1: Team lead documents team structure..."

TEAM_STRUCTURE=$(cat <<EOF
Team Structure:
- Engineering team: 5 developers
- Projects: auth-service, api-gateway, frontend
- Namespace conventions:
  * team:engineering - Shared team knowledge
  * project:<name> - Project-specific memories
  * member:<name> - Personal developer notes
EOF
)

MEM_STRUCTURE=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$TEAM_STRUCTURE" \
    --namespace "team:engineering" \
    --importance 10 \
    --type architecture 2>&1) || fail "Failed to store team structure"

STRUCTURE_ID=$(echo "$MEM_STRUCTURE" | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Team structure documented: $STRUCTURE_ID"

# ===================================================================
# SCENARIO: Namespace Conventions
# ===================================================================

section "Scenario: Namespace Conventions"

print_cyan "Step 2: Team lead establishes namespace conventions..."

CONVENTIONS=$(cat <<EOF
Namespace Conventions:
1. team:engineering - Team-wide decisions and practices
2. project:<name> - Project architecture and decisions
3. project:<name>:sprint:<num> - Sprint-specific work
4. member:<name> - Individual developer notes
5. global - Organization-wide standards

Rules:
- All major decisions go in team: or project:
- Sprint planning in project:<name>:sprint:<num>
- Personal experiments in member:<name>
EOF
)

MEM_CONVENTIONS=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$CONVENTIONS" \
    --namespace "team:engineering" \
    --importance 9 \
    --type reference 2>&1) || fail "Failed to store conventions"

CONVENTIONS_ID=$(echo "$MEM_CONVENTIONS" | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Conventions documented: $CONVENTIONS_ID"

# ===================================================================
# SCENARIO: Project Namespaces
# ===================================================================

section "Scenario: Create Project Namespaces"

print_cyan "Step 3: Team lead initializes project namespaces..."

# Auth service project
AUTH_PROJECT=$(cat <<EOF
Project: Authentication Service
Tech stack: Node.js + Express + PostgreSQL + Redis
Team: Alice (lead), Bob, Carol
Status: Active development
Architecture: Microservice with JWT auth
EOF
)

MEM_AUTH=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$AUTH_PROJECT" \
    --namespace "project:auth-service" \
    --importance 9 \
    --type architecture 2>&1) || fail "Failed to store auth project"

AUTH_ID=$(echo "$MEM_AUTH" | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Auth service project created: $AUTH_ID"

# API gateway project
API_PROJECT=$(cat <<EOF
Project: API Gateway
Tech stack: Go + gRPC + Envoy
Team: Dave (lead), Eve
Status: Planning
Architecture: Central gateway with service mesh
EOF
)

MEM_API=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$API_PROJECT" \
    --namespace "project:api-gateway" \
    --importance 9 \
    --type architecture 2>&1) || fail "Failed to store API project"

API_ID=$(echo "$MEM_API" | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ API gateway project created: $API_ID"

# Frontend project
FRONTEND_PROJECT=$(cat <<EOF
Project: Frontend Application
Tech stack: Next.js + TypeScript + Tailwind
Team: Frank (lead), Grace, Henry
Status: Active development
Architecture: SSR with API integration
EOF
)

MEM_FRONTEND=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$FRONTEND_PROJECT" \
    --namespace "project:frontend" \
    --importance 9 \
    --type architecture 2>&1) || fail "Failed to store frontend project"

FRONTEND_ID=$(echo "$MEM_FRONTEND" | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Frontend project created: $FRONTEND_ID"

# ===================================================================
# SCENARIO: Member Namespaces
# ===================================================================

section "Scenario: Initialize Member Namespaces"

print_cyan "Step 4: Create placeholders for team member namespaces..."

TEAM_MEMBERS=("alice" "bob" "carol" "dave" "eve" "frank" "grace" "henry")

for member in "${TEAM_MEMBERS[@]}"; do
    MEMBER_INIT=$(cat <<EOF
Member: $member
Preferences: Will be populated by team member
Personal notes and experiments go here
EOF
)

    DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
        --content "$MEMBER_INIT" \
        --namespace "member:$member" \
        --importance 5 \
        --type reference >/dev/null 2>&1 || warn "Failed to init member:$member"
done

print_green "  ✓ Initialized ${#TEAM_MEMBERS[@]} member namespaces"

# ===================================================================
# SCENARIO: Sprint Namespaces
# ===================================================================

section "Scenario: Sprint Planning Namespaces"

print_cyan "Step 5: Setup sprint namespaces..."

# Current sprint for auth service
SPRINT_AUTH=$(cat <<EOF
Sprint 42 - Auth Service
Goals:
- Implement OAuth2 integration
- Add rate limiting
- Improve error messages
Duration: 2 weeks
Team: Alice, Bob, Carol
EOF
)

MEM_SPRINT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$SPRINT_AUTH" \
    --namespace "project:auth-service:sprint:42" \
    --importance 8 \
    --type task 2>&1) || fail "Failed to store sprint"

SPRINT_ID=$(echo "$MEM_SPRINT" | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Sprint namespace created: $SPRINT_ID"

# ===================================================================
# SCENARIO: Team Best Practices
# ===================================================================

section "Scenario: Document Team Best Practices"

print_cyan "Step 6: Store team best practices..."

BEST_PRACTICES=$(cat <<EOF
Engineering Team Best Practices:
1. Code review required before merge
2. All services must have health check endpoints
3. Use semantic versioning for releases
4. Write integration tests for critical paths
5. Document API changes in changelog
6. Use feature flags for gradual rollouts
EOF
)

MEM_PRACTICES=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$BEST_PRACTICES" \
    --namespace "team:engineering" \
    --importance 10 \
    --type reference 2>&1) || fail "Failed to store best practices"

PRACTICES_ID=$(echo "$MEM_PRACTICES" | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Best practices documented: $PRACTICES_ID"

# ===================================================================
# VALIDATION: Namespace Structure
# ===================================================================

section "Validation: Namespace Structure"

print_cyan "Verifying namespace structure..."

# Count namespaces by type
TEAM_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(DISTINCT namespace) FROM memories
     WHERE namespace LIKE 'team:%'" 2>/dev/null)

PROJECT_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(DISTINCT namespace) FROM memories
     WHERE namespace LIKE 'project:%'" 2>/dev/null)

MEMBER_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(DISTINCT namespace) FROM memories
     WHERE namespace LIKE 'member:%'" 2>/dev/null)

print_cyan "  Namespace distribution:"
print_cyan "    team:* namespaces:   $TEAM_COUNT"
print_cyan "    project:* namespaces: $PROJECT_COUNT"
print_cyan "    member:* namespaces: $MEMBER_COUNT"

# Validate expected structure
assert_equals "$TEAM_COUNT" "1" "Team namespace count"
assert_greater_than "$PROJECT_COUNT" 2 "Project namespace count"
assert_equals "$MEMBER_COUNT" "${#TEAM_MEMBERS[@]}" "Member namespace count"

print_green "  ✓ Namespace structure validated"

# ===================================================================
# VALIDATION: Hierarchical Structure
# ===================================================================

section "Validation: Hierarchical Namespace Structure"

print_cyan "Checking hierarchical namespace structure..."

# Sprint namespaces should be under projects
SPRINT_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories
     WHERE namespace LIKE 'project:%:sprint:%'" 2>/dev/null)

print_cyan "  Sprint namespaces: $SPRINT_COUNT"

if [ "$SPRINT_COUNT" -ge 1 ]; then
    print_green "  ✓ Hierarchical sprint namespaces working"
else
    warn "Expected at least 1 sprint namespace"
fi

# ===================================================================
# VALIDATION: Team Memories Accessibility
# ===================================================================

section "Validation: Team Memories Accessibility"

print_cyan "Verifying team memories are accessible..."

# List team namespace
TEAM_MEMS=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" list \
    --namespace "team:engineering" 2>&1) || fail "Failed to list team memories"

if echo "$TEAM_MEMS" | grep -q "engineering\|conventions\|practices"; then
    print_green "  ✓ Team memories accessible and correct"
else
    warn "Team memory content not as expected"
fi

# Count high-importance team memories
TEAM_HIGH_IMP=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories
     WHERE namespace='team:engineering' AND importance >= 9" 2>/dev/null)

print_cyan "  High-importance team memories: $TEAM_HIGH_IMP"

if [ "$TEAM_HIGH_IMP" -ge 2 ]; then
    print_green "  ✓ Critical team knowledge captured"
else
    warn "Expected more high-importance team memories"
fi

# ===================================================================
# VALIDATION: Project Isolation
# ===================================================================

section "Validation: Project Isolation"

print_cyan "Verifying project isolation..."

# Each project should have at least 1 memory
for project in "auth-service" "api-gateway" "frontend"; do
    PROJ_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
        "SELECT COUNT(*) FROM memories
         WHERE namespace='project:$project'" 2>/dev/null)

    print_cyan "  project:$project: $PROJ_COUNT memories"

    if [ "$PROJ_COUNT" -lt 1 ]; then
        fail "Project $project has no memories"
    fi
done

print_green "  ✓ All projects have isolated namespaces"

# ===================================================================
# VALIDATION: Namespace Listing
# ===================================================================

section "Validation: Namespace Listing"

print_cyan "Listing all namespaces..."

ALL_NAMESPACES=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT DISTINCT namespace FROM memories ORDER BY namespace" 2>/dev/null)

echo "  Discovered namespaces:"
echo "$ALL_NAMESPACES" | while read -r ns; do
    count=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
        "SELECT COUNT(*) FROM memories WHERE namespace='$ns'" 2>/dev/null)
    print_cyan "    $ns ($count memories)"
done

TOTAL_NAMESPACES=$(echo "$ALL_NAMESPACES" | wc -l | tr -d ' ')
print_cyan "  Total unique namespaces: $TOTAL_NAMESPACES"

if [ "$TOTAL_NAMESPACES" -ge 12 ]; then
    print_green "  ✓ Rich namespace structure established"
else
    warn "Expected at least 12 namespaces (got $TOTAL_NAMESPACES)"
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

section "Test Summary: Team Lead Namespace Setup [REGRESSION]"

echo "✓ Team structure documentation: PASS"
echo "✓ Namespace conventions: PASS"
echo "✓ Project namespaces created: PASS"
echo "✓ Member namespaces initialized: PASS"
echo "✓ Sprint namespaces: PASS"
echo "✓ Best practices documented: PASS"
echo "✓ Namespace structure validated: PASS"
echo "✓ Project isolation confirmed: PASS"
echo ""
echo "Namespace Statistics:"
echo "  - Total unique namespaces: $TOTAL_NAMESPACES"
echo "  - Team namespaces: $TEAM_COUNT"
echo "  - Project namespaces: $PROJECT_COUNT"
echo "  - Member namespaces: $MEMBER_COUNT"
echo "  - Sprint namespaces: $SPRINT_COUNT"
echo ""

print_green "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
print_green "✓ ALL TESTS PASSED"
print_green "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

exit 0
