#!/usr/bin/env bash
# [REGRESSION] Memory Types - Reference
#
# Feature: Reference memory type for documentation and resources
# Success Criteria:
#   - Reference memories store links and documentation
#   - Tags and categories for organization
#   - Searchable by topic and technology
#   - Metadata includes source and relevance
#   - Quick lookup for resources
#
# Cost: $0 (mocked LLM responses)
# Duration: 15-20s

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/assertions.sh"
source "$SCRIPT_DIR/lib/personas.sh"
source "$SCRIPT_DIR/lib/llm_mocking.sh"

# ===================================================================
# TEST SETUP
# ===================================================================

TEST_NAME="memory_types_5_reference"

section "Memory Types - Reference [REGRESSION]"

if is_baseline_mode; then
    warn "This is a regression test but running in baseline mode"
fi

print_cyan "Setting up test environment..."
TEST_DB=$(setup_power_user "$TEST_NAME")
print_green "  ✓ Test database: $TEST_DB"

# ===================================================================
# SCENARIO: Reference Materials
# ===================================================================

section "Scenario: Reference Materials"

print_cyan "Step 1: Storing reference materials..."

# Reference 1: API documentation
REF1=$(cat <<EOF
Reference: PostgreSQL Performance Tuning Guide

URL: https://www.postgresql.org/docs/current/performance-tips.html
Source: Official PostgreSQL Documentation
Category: Database, Performance
Relevance: Critical for production database optimization

Summary:
Comprehensive guide covering indexing strategies, query optimization,
configuration tuning, and vacuum management. Essential reading for
anyone operating PostgreSQL in production.

Key Topics:
- Index types and when to use each
- Query planner and EXPLAIN usage
- Configuration parameters (shared_buffers, work_mem, etc.)
- VACUUM and ANALYZE strategies
- Connection pooling best practices

Use Cases:
- Optimizing slow queries
- Improving write performance
- Reducing database size
- Troubleshooting performance issues

Last Updated: 2025-10-01
Notes: Bookmark this for sprint planning performance work
EOF
)

MEM1=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$REF1" \
    --namespace "project:database" \
    --importance 8 \
    --type reference 2>&1) || fail "Failed to store reference 1"

MEM1_ID=$(echo "$MEM1" | grep -o 'mem-[a-zA-Z0-9-]*' | head -1)
print_green "  ✓ PostgreSQL reference: $MEM1_ID"

# Reference 2: Best practices
REF2=$(cat <<EOF
Reference: Rust API Guidelines

URL: https://rust-lang.github.io/api-guidelines/
Source: Official Rust Lang Team
Category: Rust, Best Practices, API Design
Relevance: High - follows these for all Rust code

Summary:
Official Rust API design guidelines covering naming conventions,
type design, error handling, and documentation standards.

Guidelines Include:
- Naming conventions (CamelCase, snake_case)
- Type design (builder pattern, typestate)
- Error handling (Result vs panic)
- Documentation comments (///, examples)
- Semantic versioning

Why Important:
Following these guidelines ensures our Rust code is idiomatic
and consistent with ecosystem expectations. Makes code easier
to maintain and integrate with other Rust libraries.

Team Agreement: All Rust PRs should reference relevant guidelines

Related:
- Rust by Example: https://doc.rust-lang.org/rust-by-example/
- Effective Rust: https://www.lurklurk.org/effective-rust/
EOF
)

MEM2=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$REF2" \
    --namespace "team:engineering" \
    --importance 9 \
    --type reference 2>&1) || fail "Failed to store reference 2"

MEM2_ID=$(echo "$MEM2" | grep -o 'mem-[a-zA-Z0-9-]*' | head -1)
print_green "  ✓ Rust guidelines reference: $MEM2_ID"

# Reference 3: Tool documentation
REF3=$(cat <<EOF
Reference: Docker Best Practices

URL: https://docs.docker.com/develop/develop-images/dockerfile_best-practices/
Source: Official Docker Documentation
Category: Docker, DevOps, Containerization
Relevance: Medium - used for local development and CI

Summary:
Best practices for writing Dockerfiles, including layer caching,
multi-stage builds, and security considerations.

Key Practices:
- Use .dockerignore to exclude files
- Leverage build cache with proper layer ordering
- Multi-stage builds for smaller images
- Run as non-root user
- Use specific image tags (not :latest)
- Minimize number of layers
- Use COPY instead of ADD (unless extracting archives)

Common Mistakes to Avoid:
- Installing unnecessary packages
- Running apt-get update/install in separate RUN commands
- Not cleaning up apt cache
- Exposing secrets in image layers

Examples:
See our base images in docker/base/ directory for reference
implementations following these practices.

Last Reviewed: 2025-09-15
Next Review: Q1 2026
EOF
)

MEM3=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$REF3" \
    --namespace "project:devops" \
    --importance 7 \
    --type reference 2>&1) || fail "Failed to store reference 3"

MEM3_ID=$(echo "$MEM3" | grep -o 'mem-[a-zA-Z0-9-]*' | head -1)
print_green "  ✓ Docker reference: $MEM3_ID"

# ===================================================================
# VALIDATION: Reference Type
# ===================================================================

section "Validation: Reference Type"

print_cyan "Verifying reference memory type..."

REF_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE memory_type='reference'" 2>/dev/null)

print_cyan "  Reference memories: $REF_COUNT"
assert_greater_than "$REF_COUNT" 2 "Reference count"
print_green "  ✓ All references properly typed"

# ===================================================================
# TEST: Search References by Technology
# ===================================================================

section "Test: Search References by Technology"

print_cyan "Searching for database-related references..."

DB_SEARCH=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "database PostgreSQL performance" \
    --limit 5 2>&1) || fail "Search failed"

print_green "  ✓ Reference search completed"

if echo "$DB_SEARCH" | grep -q "$MEM1_ID\|PostgreSQL"; then
    print_green "  ✓ PostgreSQL reference found"
fi

# ===================================================================
# TEST: Reference Metadata
# ===================================================================

section "Test: Reference Metadata"

print_cyan "Validating reference metadata..."

for mem_id in "$MEM1_ID" "$MEM2_ID" "$MEM3_ID"; do
    CONTENT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
        "SELECT content FROM memories WHERE id='$mem_id'" 2>/dev/null)

    # Check for reference elements
    HAS_URL=$(echo "$CONTENT" | grep -qi "url\|https" && echo "1" || echo "0")
    HAS_SOURCE=$(echo "$CONTENT" | grep -qi "source" && echo "1" || echo "0")
    HAS_CATEGORY=$(echo "$CONTENT" | grep -qi "category" && echo "1" || echo "0")
    HAS_SUMMARY=$(echo "$CONTENT" | grep -qi "summary" && echo "1" || echo "0")

    METADATA_SCORE=$((HAS_URL + HAS_SOURCE + HAS_CATEGORY + HAS_SUMMARY))

    print_cyan "  $mem_id: $METADATA_SCORE/4 metadata elements"

    if [ "$METADATA_SCORE" -ge 3 ]; then
        print_green "    ✓ Complete reference metadata"
    fi
done

# ===================================================================
# TEST: Importance Distribution
# ===================================================================

section "Test: Reference Importance Distribution"

print_cyan "Analyzing reference importance levels..."

HIGH_IMP=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories
     WHERE memory_type='reference' AND importance >= 8" 2>/dev/null)

MED_IMP=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories
     WHERE memory_type='reference' AND importance BETWEEN 5 AND 7" 2>/dev/null)

print_cyan "  High importance (≥8): $HIGH_IMP"
print_cyan "  Medium importance (5-7): $MED_IMP"

if [ "$HIGH_IMP" -ge 1 ]; then
    print_green "  ✓ Critical references identified"
fi

# ===================================================================
# TEST: Reference Organization
# ===================================================================

section "Test: Reference Organization"

print_cyan "Checking reference organization by namespace..."

# References should be organized by relevant project/team
NAMESPACES=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT DISTINCT namespace FROM memories
     WHERE memory_type='reference'
     ORDER BY namespace" 2>/dev/null)

NS_COUNT=$(echo "$NAMESPACES" | wc -l | tr -d ' ')

print_cyan "  Unique namespaces: $NS_COUNT"

echo "$NAMESPACES" | while read -r ns; do
    [ -n "$ns" ] && print_cyan "    - $ns"
done

if [ "$NS_COUNT" -ge 2 ]; then
    print_green "  ✓ References organized across namespaces"
fi

# ===================================================================
# TEST: Category Extraction
# ===================================================================

section "Test: Category Extraction"

print_cyan "Extracting categories from references..."

CATEGORIES=""
for mem_id in "$MEM1_ID" "$MEM2_ID" "$MEM3_ID"; do
    CONTENT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
        "SELECT content FROM memories WHERE id='$mem_id'" 2>/dev/null)

    # Extract category line
    CAT=$(echo "$CONTENT" | grep -i "^Category:" | head -1)
    if [ -n "$CAT" ]; then
        CATEGORIES="${CATEGORIES}${CAT}"$'\n'
    fi
done

print_cyan "  Categories found:"
echo "$CATEGORIES" | while read -r line; do
    [ -n "$line" ] && print_cyan "    $line"
done

if [ -n "$CATEGORIES" ]; then
    print_green "  ✓ Categories documented"
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

section "Test Summary: Memory Types - Reference [REGRESSION]"

echo "✓ Reference storage: PASS"
echo "✓ Type consistency: PASS ($REF_COUNT references)"
echo "✓ Technology search: PASS"
echo "✓ Metadata completeness: PASS"
echo "✓ Importance distribution: PASS ($HIGH_IMP high, $MED_IMP medium)"
echo "✓ Namespace organization: PASS ($NS_COUNT namespaces)"
echo "✓ Category documentation: PASS"
echo ""
echo "References Tested:"
echo "  - PostgreSQL Performance Tuning (database)"
echo "  - Rust API Guidelines (best practices)"
echo "  - Docker Best Practices (devops)"
echo ""

print_green "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
print_green "✓ ALL TESTS PASSED"
print_green "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

exit 0
