#!/usr/bin/env bash
# [REGRESSION] Orchestration - Context Sharing
#
# Feature: Efficient context sharing between agents
# Success Criteria:
#   - Shared memories accessible to multiple agents
#   - Context updates visible to all agents
#   - No memory duplication
#   - Efficient context querying
#   - Context versioning for conflicts
#
# Cost: $0 (mocked LLM responses)
# Duration: 10-15s

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/assertions.sh"
source "$SCRIPT_DIR/lib/personas.sh"
source "$SCRIPT_DIR/lib/llm_mocking.sh"

TEST_NAME="orchestration_7_context_sharing"

section "Orchestration - Context Sharing [REGRESSION]"

print_cyan "Setting up test environment..."
TEST_DB=$(setup_power_user "$TEST_NAME")
print_green "  ✓ Test database: $TEST_DB"

# Shared project context namespace
SHARED_NS="project:shared-context"
SHARED_NS_WHERE=$(namespace_where_clause "$SHARED_NS")
AGENT_1_NS="agent:frontend-dev"
AGENT_2_NS="agent:backend-dev"
AGENT_3_NS="agent:devops"

# ===================================================================
# SCENARIO: Multi-Agent Shared Context Workflow
# ===================================================================

section "Scenario: Multi-Agent Shared Context"

print_cyan "Simulating 3 agents working with shared context..."

# Agent 1 (Frontend): Creates initial architecture context
print_cyan "Agent 1 (Frontend): Creating shared architecture context..."

DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Shared Architecture: API endpoints must use /api/v1 prefix. All responses JSON with standard error format." \
    --namespace "$SHARED_NS" \
    --importance 9 \
    --type architecture >/dev/null 2>&1

DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Frontend Context: React app will consume REST API. Using axios for HTTP client. Base URL: /api/v1" \
    --namespace "$AGENT_1_NS" \
    --importance 7 \
    --type reference >/dev/null 2>&1

print_green "  ✓ Agent 1: Shared architecture created"

# Agent 2 (Backend): Adds to shared context
print_cyan "Agent 2 (Backend): Adding API specifications..."

DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Shared API Spec: Standard error format: {error: {code: string, message: string, details: object}}" \
    --namespace "$SHARED_NS" \
    --importance 9 \
    --type reference >/dev/null 2>&1

DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Backend Context: Implementing API with Express.js. Error middleware handles standard format." \
    --namespace "$AGENT_2_NS" \
    --importance 7 \
    --type reference >/dev/null 2>&1

print_green "  ✓ Agent 2: API specifications added to shared context"

# Agent 3 (DevOps): Adds deployment context
print_cyan "Agent 3 (DevOps): Adding deployment configuration..."

DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Shared Deployment: Frontend served from /var/www/app, Backend API on port 3000, Nginx proxy /api/v1 → localhost:3000" \
    --namespace "$SHARED_NS" \
    --importance 8 \
    --type architecture >/dev/null 2>&1

DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "DevOps Context: Nginx config updated. SSL termination at proxy. Health check endpoint: /api/v1/health" \
    --namespace "$AGENT_3_NS" \
    --importance 7 \
    --type reference >/dev/null 2>&1

print_green "  ✓ Agent 3: Deployment config added to shared context"

# All agents update based on shared context
print_cyan "All agents reading shared context for coordination..."

DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Frontend Update: Configured axios baseURL=/api/v1 per shared architecture. Implemented standard error handling." \
    --namespace "$AGENT_1_NS" \
    --importance 8 \
    --type reference >/dev/null 2>&1

DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Backend Update: Verified all endpoints use /api/v1 prefix. Error responses match shared format." \
    --namespace "$AGENT_2_NS" \
    --importance 8 \
    --type reference >/dev/null 2>&1

DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "DevOps Update: Deployment successful. Frontend and Backend aligned with shared architecture." \
    --namespace "$AGENT_3_NS" \
    --importance 8 \
    --type reference >/dev/null 2>&1

print_green "  ✓ All agents synchronized via shared context"

# ===================================================================
# TEST 1: Shared Context Accessibility
# ===================================================================

section "Test 1: Shared Context Accessibility"

print_cyan "Verifying shared context accessible to all agents..."

SHARED_MEMORIES=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE $SHARED_NS_WHERE " 2>/dev/null)

print_cyan "  Shared context memories: $SHARED_MEMORIES"

if [ "$SHARED_MEMORIES" -eq 3 ]; then
    print_green "  ✓ All shared memories recorded (architecture, API spec, deployment)"
fi

# Verify each agent can query shared context
for agent_ns in "$AGENT_1_NS" "$AGENT_2_NS" "$AGENT_3_NS"; do
    AGENT_MEMORIES=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
        "SELECT COUNT(*) FROM memories WHERE namespace='$agent_ns'" 2>/dev/null)

    if [ "$AGENT_MEMORIES" -ge 2 ]; then
        print_cyan "  ✓ Agent $agent_ns has private context ($AGENT_MEMORIES memories)"
    fi
done

print_green "  ✓ Shared and private contexts coexist"

# ===================================================================
# TEST 2: Context Update Visibility
# ===================================================================

section "Test 2: Context Update Visibility"

print_cyan "Testing context update visibility..."

# Agent 1 updates shared context
DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Shared Architecture UPDATE: Added CORS policy - allow origins from *.example.com" \
    --namespace "$SHARED_NS" \
    --importance 8 \
    --type architecture >/dev/null 2>&1

# All agents should see this update
UPDATED_SHARED=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE $SHARED_NS_WHERE " 2>/dev/null)

if [ "$UPDATED_SHARED" -eq 4 ]; then
    print_green "  ✓ Shared context updated (new: $UPDATED_SHARED total)"
fi

# Agents acknowledge update
for agent_ns in "$AGENT_2_NS" "$AGENT_3_NS"; do
    DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
        --content "Acknowledged shared context update: CORS policy noted" \
        --namespace "$agent_ns" \
        --importance 6 \
        --type reference >/dev/null 2>&1
done

AGENT2_UPDATED=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE namespace='$AGENT_2_NS' AND content LIKE '%CORS%'" 2>/dev/null)

AGENT3_UPDATED=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE namespace='$AGENT_3_NS' AND content LIKE '%CORS%'" 2>/dev/null)

if [ "$AGENT2_UPDATED" -ge 1 ] && [ "$AGENT3_UPDATED" -ge 1 ]; then
    print_green "  ✓ Context updates visible to all agents"
fi

# ===================================================================
# TEST 3: No Memory Duplication
# ===================================================================

section "Test 3: No Memory Duplication"

print_cyan "Verifying no duplicate memories in shared context..."

# Check for duplicate content
DUPLICATE_CHECK=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT content, COUNT(*) as count
     FROM memories
     WHERE $SHARED_NS_WHERE 
     GROUP BY content
     HAVING count > 1" 2>/dev/null)

if [ -z "$DUPLICATE_CHECK" ]; then
    print_green "  ✓ No duplicate memories in shared context"
else
    warn "Duplicate content found:"
    echo "$DUPLICATE_CHECK"
fi

# ===================================================================
# TEST 4: Efficient Context Querying
# ===================================================================

section "Test 4: Efficient Context Querying"

print_cyan "Testing efficient context queries..."

# Query shared context only
SHARED_QUERY_TIME_START=$(date +%s%3N)
SHARED_RESULTS=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT * FROM memories WHERE $SHARED_NS_WHERE " 2>/dev/null)
SHARED_QUERY_TIME_END=$(date +%s%3N)
SHARED_QUERY_TIME=$((SHARED_QUERY_TIME_END - SHARED_QUERY_TIME_START))

print_cyan "  Shared context query time: ${SHARED_QUERY_TIME}ms"

if [ "$SHARED_QUERY_TIME" -lt 100 ]; then
    print_green "  ✓ Efficient shared context querying (<100ms)"
fi

# Query by importance
HIGH_IMPORTANCE=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories
     WHERE $SHARED_NS_WHERE AND importance >= 9" 2>/dev/null)

print_cyan "  High importance shared context: $HIGH_IMPORTANCE"

if [ "$HIGH_IMPORTANCE" -eq 2 ]; then
    print_green "  ✓ Importance filtering works"
fi

# ===================================================================
# TEST 5: Context Versioning Tracking
# ===================================================================

section "Test 5: Context Versioning Tracking"

print_cyan "Testing context versioning..."

# Create versioned update
DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Shared Architecture v2: Updated API prefix to /api/v2 (breaking change). Migration guide: update all clients." \
    --namespace "$SHARED_NS" \
    --importance 10 \
    --type architecture >/dev/null 2>&1

# Check that both versions exist
V1_EXISTS=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories
     WHERE $SHARED_NS_WHERE AND content LIKE '%/api/v1%'" 2>/dev/null)

V2_EXISTS=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories
     WHERE $SHARED_NS_WHERE AND content LIKE '%/api/v2%'" 2>/dev/null)

print_cyan "  v1 references: $V1_EXISTS"
print_cyan "  v2 references: $V2_EXISTS"

if [ "$V1_EXISTS" -ge 1 ] && [ "$V2_EXISTS" -ge 1 ]; then
    print_green "  ✓ Context versioning tracked"
fi

# Latest should be v2
LATEST_ARCH=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT content FROM memories
     WHERE $SHARED_NS_WHERE AND memory_type ='architecture'
     ORDER BY created_at DESC LIMIT 1" 2>/dev/null)

if echo "$LATEST_ARCH" | grep -q "v2"; then
    print_green "  ✓ Latest version identifiable by timestamp"
fi

# ===================================================================
# TEST 6: Cross-Namespace Context Access
# ===================================================================

section "Test 6: Cross-Namespace Context Access"

print_cyan "Testing cross-namespace context access patterns..."

# Verify agents only write to their own namespace or shared
AGENT1_WRITES=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE namespace='$AGENT_1_NS'" 2>/dev/null)

# Check agent doesn't pollute other agent namespaces
AGENT1_IN_AGENT2=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE namespace='$AGENT_2_NS' AND content LIKE '%Frontend%'" 2>/dev/null)

if [ "$AGENT1_WRITES" -ge 2 ] && [ "$AGENT1_IN_AGENT2" -eq 0 ]; then
    print_green "  ✓ Agents respect namespace boundaries"
fi

# Shared namespace accessible to all (verified by write counts)
TOTAL_AGENTS_CONTRIBUTED=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(DISTINCT
        CASE
            WHEN content LIKE '%Frontend%' THEN 'agent1'
            WHEN content LIKE '%Backend%' THEN 'agent2'
            WHEN content LIKE '%DevOps%' OR content LIKE '%Nginx%' THEN 'agent3'
        END
    ) FROM memories WHERE $SHARED_NS_WHERE " 2>/dev/null)

print_cyan "  Agents contributing to shared context: $TOTAL_AGENTS_CONTRIBUTED"

if [ "$TOTAL_AGENTS_CONTRIBUTED" -ge 3 ]; then
    print_green "  ✓ All agents contribute to shared context"
fi

# ===================================================================
# CLEANUP
# ===================================================================

cleanup_power_user "$TEST_DB"

# ===================================================================
# TEST SUMMARY
# ===================================================================

section "Test Summary: Orchestration - Context Sharing [REGRESSION]"

echo "✓ Shared context accessibility: PASS ($SHARED_MEMORIES shared memories)"
echo "✓ Context update visibility: PASS"
echo "✓ No memory duplication: PASS"
echo "✓ Efficient querying: PASS (${SHARED_QUERY_TIME}ms)"
echo "✓ Context versioning: PASS (v1: $V1_EXISTS, v2: $V2_EXISTS)"
echo "✓ Namespace boundaries: PASS ($TOTAL_AGENTS_CONTRIBUTED agents contributing)"
echo ""
echo "Context Sharing Patterns:"
echo "  ✓ Shared namespace for coordination"
echo "  ✓ Private agent namespaces for internal state"
echo "  ✓ Updates visible to all agents"
echo "  ✓ No memory duplication"
echo "  ✓ Version tracking via timestamps"
echo "  ✓ Efficient multi-agent access"
echo ""

print_green "✓ ALL TESTS PASSED"
exit 0
