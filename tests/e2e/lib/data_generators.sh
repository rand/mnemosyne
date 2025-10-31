#!/usr/bin/env bash
# Test Data Generators
#
# Provides functions to generate realistic test data for memories,
# work items, namespaces, and other Mnemosyne entities.

# Source common utilities
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=tests/e2e/lib/common.sh
source "$SCRIPT_DIR/common.sh"

# ===================================================================
# MEMORY GENERATORS
# ===================================================================

# Generate realistic memory content for different types
# Args: memory_type
generate_memory_content() {
    local type="${1:-insight}"

    case "$type" in
        insight)
            cat <<EOF
User prefers async/await patterns over callbacks for better readability.
Observed during code review of authentication service.
This reduces callback hell and makes error handling more straightforward.
EOF
            ;;
        architecture)
            cat <<EOF
Architecture decision: Use microservices architecture with event-driven communication.
Rationale: Better scalability and independent deployment of services.
Trade-offs: Increased complexity in distributed system debugging and monitoring.
Alternatives considered: Monolithic architecture (simpler but less scalable).
EOF
            ;;
        decision)
            cat <<EOF
Decision: Adopt PostgreSQL as primary database instead of MongoDB.
Context: Need for strong ACID guarantees and complex relational queries.
Participants: Engineering team, CTO
Date: 2025-10-31
Outcome: Migration plan approved, timeline Q1 2026
EOF
            ;;
        task)
            cat <<EOF
Task: Implement user authentication with JWT tokens
Dependencies: Database schema migration, API endpoint design
Estimated effort: 2-3 days
Assignee: Backend team
Priority: High
Status: In Progress
EOF
            ;;
        reference)
            cat <<EOF
Reference: PostgreSQL Performance Tuning Guide
URL: https://www.postgresql.org/docs/current/performance-tips.html
Summary: Comprehensive guide covering indexing, query optimization, and configuration.
Relevance: Critical for database performance optimization work.
Tags: database, performance, postgresql
EOF
            ;;
        *)
            echo "Generic memory content for type: $type"
            ;;
    esac
}

# Generate batch of memories with varying importance
# Args: count, namespace, db_path
generate_memory_batch() {
    local count="${1:-10}"
    local namespace="${2:-project:test}"
    local db="$3"

    print_cyan "Generating $count memories in namespace '$namespace'..."

    local types=("insight" "architecture" "decision" "task" "reference")
    local created=0

    for i in $(seq 1 "$count"); do
        local type="${types[$((i % 5))]}"
        local importance=$((5 + (i % 6)))  # 5-10
        local content=$(generate_memory_content "$type")

        if DATABASE_URL="sqlite://$db" "$BIN" remember \
            --content "$content (ID: $i)" \
            --namespace "$namespace" \
            --importance "$importance" \
            --type "$type" >/dev/null 2>&1; then
            ((created++))
        fi
    done

    print_green "  ✓ Created $created/$count memories"
    echo "$created"
}

# Generate memories with specific keywords (for search testing)
# Args: keywords[], namespace, db_path
generate_keyword_memories() {
    local keywords=("$1")
    local namespace="${2:-project:test}"
    local db="$3"

    shift 3

    print_cyan "Generating memories with keywords..."

    for keyword in "${keywords[@]}"; do
        local content="This memory is about $keyword and related concepts. "
        content+="It demonstrates $keyword usage in production systems."

        DATABASE_URL="sqlite://$db" "$BIN" remember \
            --content "$content" \
            --namespace "$namespace" \
            --importance 7 >/dev/null 2>&1 || true
    done

    print_green "  ✓ Created ${#keywords[@]} keyword-targeted memories"
}

# Generate duplicate/similar memories (for consolidation testing)
# Args: base_content, variation_count, namespace, db_path
generate_duplicate_memories() {
    local base="$1"
    local count="${2:-3}"
    local namespace="${3:-project:test}"
    local db="$4"

    print_cyan "Generating $count similar memories..."

    local variations=(
        "$base"
        "$base (with minor rephrasing)"
        "Similar to: $base"
        "$base - updated version"
        "Duplicate: $base"
    )

    for i in $(seq 0 $((count - 1))); do
        local content="${variations[$i]}"
        DATABASE_URL="sqlite://$db" "$BIN" remember \
            --content "$content" \
            --namespace "$namespace" \
            --importance 7 >/dev/null 2>&1 || true
    done

    print_green "  ✓ Created $count similar memories for consolidation"
}

# ===================================================================
# NAMESPACE GENERATORS
# ===================================================================

# Generate hierarchical namespace structure
# Args: base_namespace, depth, db_path
generate_namespace_hierarchy() {
    local base="${1:-project}"
    local depth="${2:-3}"
    local db="$3"

    print_cyan "Generating namespace hierarchy (depth: $depth)..."

    local namespaces=(
        "global"
        "$base:myproject"
        "$base:myproject:frontend"
        "$base:myproject:backend"
        "$base:myproject:frontend:components"
        "$base:myproject:backend:api"
        "session:$base:myproject"
        "team:engineering"
        "agent:orchestrator"
    )

    for ns in "${namespaces[@]:0:$depth}"; do
        DATABASE_URL="sqlite://$db" "$BIN" remember \
            --content "Namespace marker for $ns" \
            --namespace "$ns" \
            --importance 5 >/dev/null 2>&1 || true
    done

    print_green "  ✓ Created namespace hierarchy"
}

# ===================================================================
# LINK GENERATORS
# ===================================================================

# Generate memory links (for graph testing)
# Note: This requires memories to already exist and uses recall + manual linking
# Args: db_path, link_count
generate_memory_links() {
    local db="$1"
    local count="${2:-5}"

    print_cyan "Generating $count memory links..."

    # Get list of memory IDs
    local mem_ids=$(DATABASE_URL="sqlite://$db" sqlite3 "$db" \
        "SELECT id FROM memories LIMIT 10" 2>/dev/null || echo "")

    if [ -z "$mem_ids" ]; then
        warn "No memories found to link"
        return 1
    fi

    # Convert to array
    local ids=($mem_ids)
    local created=0

    for i in $(seq 0 $((count - 1))); do
        local source="${ids[$i]}"
        local target="${ids[$(((i + 1) % ${#ids[@]}))]}"

        if [ -n "$source" ] && [ -n "$target" ]; then
            DATABASE_URL="sqlite://$db" sqlite3 "$db" \
                "INSERT OR IGNORE INTO memory_links (source_id, target_id, strength, link_type) \
                VALUES ('$source', '$target', 0.8, 'relates_to')" 2>/dev/null && ((created++))
        fi
    done

    print_green "  ✓ Created $created memory links"
    echo "$created"
}

# ===================================================================
# WORK ITEM GENERATORS
# ===================================================================

# Generate work items for orchestration testing
# Args: count, db_path
generate_work_items() {
    local count="${1:-5}"
    local db="$2"

    print_cyan "Generating $count work items..."

    local states=("Ready" "Active" "PendingReview" "Complete")
    local priorities=(1 2 3 5 8)

    for i in $(seq 1 "$count"); do
        local description="Work item $i: Implement feature XYZ"
        local state="${states[$((i % 4))]}"
        local priority="${priorities[$((i % 5))]}"

        # Note: Work items are created via orchestration system, not CLI
        # This is a simplified version for testing
        DATABASE_URL="sqlite://$db" sqlite3 "$db" \
            "INSERT INTO work_items (id, description, state, priority, created_at) \
            VALUES ('work-$i', '$description', '$state', $priority, datetime('now'))" \
            2>/dev/null || true
    done

    print_green "  ✓ Created $count work items"
}

# ===================================================================
# TIME-BASED GENERATORS
# ===================================================================

# Generate memories with specific timestamps (for temporal queries)
# Args: db_path, days_back_max
generate_temporal_memories() {
    local db="$1"
    local days="${2:-30}"

    print_cyan "Generating temporal memories (last $days days)..."

    for day in $(seq 0 "$days" | shuf | head -10); do
        local timestamp=$(date -u -v-${day}d +"%Y-%m-%d %H:%M:%S" 2>/dev/null || \
                         date -u -d "$day days ago" +"%Y-%m-%d %H:%M:%S" 2>/dev/null)

        local content="Memory from $day days ago: Important decision or insight"

        # Insert directly with custom timestamp
        local mem_id="mem-$(date +%s)-$day"
        DATABASE_URL="sqlite://$db" sqlite3 "$db" \
            "INSERT INTO memories (id, content, namespace, importance, created_at) \
            VALUES ('$mem_id', '$content', 'project:test', 7, '$timestamp')" \
            2>/dev/null || true
    done

    print_green "  ✓ Created temporal memories"
}

# ===================================================================
# REALISTIC DATA GENERATORS
# ===================================================================

# Generate realistic project setup (combined memories, namespaces, links)
# Args: project_name, db_path
generate_realistic_project() {
    local project="${1:-testproject}"
    local db="$2"

    print_cyan "Generating realistic project: $project..."

    # 1. Architecture decisions
    DATABASE_URL="sqlite://$db" "$BIN" remember \
        --content "$(generate_memory_content architecture)" \
        --namespace "project:$project" \
        --importance 9 \
        --type architecture >/dev/null 2>&1

    # 2. Team decisions
    DATABASE_URL="sqlite://$db" "$BIN" remember \
        --content "$(generate_memory_content decision)" \
        --namespace "team:$project" \
        --importance 8 \
        --type decision >/dev/null 2>&1

    # 3. Active tasks
    for i in {1..3}; do
        DATABASE_URL="sqlite://$db" "$BIN" remember \
            --content "$(generate_memory_content task)" \
            --namespace "project:$project" \
            --importance 8 \
            --type task >/dev/null 2>&1
    done

    # 4. Insights from development
    for i in {1..5}; do
        DATABASE_URL="sqlite://$db" "$BIN" remember \
            --content "$(generate_memory_content insight)" \
            --namespace "project:$project" \
            --importance 7 \
            --type insight >/dev/null 2>&1
    done

    # 5. Reference materials
    DATABASE_URL="sqlite://$db" "$BIN" remember \
        --content "$(generate_memory_content reference)" \
        --namespace "project:$project" \
        --importance 6 \
        --type reference >/dev/null 2>&1

    print_green "  ✓ Realistic project '$project' generated"
}

# Generate stress test data (large volume)
# Args: db_path, size (small/medium/large)
generate_stress_data() {
    local db="$1"
    local size="${2:-medium}"

    local count=100
    case "$size" in
        small) count=100 ;;
        medium) count=1000 ;;
        large) count=10000 ;;
    esac

    print_cyan "Generating stress test data ($size: $count memories)..."

    for i in $(seq 1 "$count"); do
        local ns="project:stress:shard$((i % 10))"
        local content="Stress test memory $i with some content to simulate real usage"

        DATABASE_URL="sqlite://$db" sqlite3 "$db" \
            "INSERT INTO memories (id, content, namespace, importance, created_at) \
            VALUES ('stress-$i', '$content', '$ns', $((5 + (i % 6))), datetime('now'))" \
            2>/dev/null || true

        # Progress indicator
        if [ $((i % 100)) -eq 0 ]; then
            echo -n "."
        fi
    done

    echo ""
    print_green "  ✓ Created $count stress test memories"
}

# ===================================================================
# HELPER FUNCTIONS
# ===================================================================

# Get random memory ID from database
# Args: db_path
get_random_memory_id() {
    local db="$1"

    DATABASE_URL="sqlite://$db" sqlite3 "$db" \
        "SELECT id FROM memories ORDER BY RANDOM() LIMIT 1" 2>/dev/null || echo ""
}

# Count memories in namespace
# Args: db_path, namespace
count_memories_in_namespace() {
    local db="$1"
    local ns="$2"

    DATABASE_URL="sqlite://$db" sqlite3 "$db" \
        "SELECT COUNT(*) FROM memories WHERE namespace='$ns'" 2>/dev/null || echo "0"
}

# Export all generator functions
export -f generate_memory_content
export -f generate_memory_batch
export -f generate_keyword_memories
export -f generate_duplicate_memories
export -f generate_namespace_hierarchy
export -f generate_memory_links
export -f generate_work_items
export -f generate_temporal_memories
export -f generate_realistic_project
export -f generate_stress_data
export -f get_random_memory_id
export -f count_memories_in_namespace
