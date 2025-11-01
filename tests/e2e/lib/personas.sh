#!/usr/bin/env bash
# User Persona Test Helpers
#
# Provides setup/teardown and behavior patterns for different user types:
# - Solo Developer
# - Team Lead
# - AI Agent (Single)
# - Multi-Agent System
# - Python Developer
# - Dashboard Observer
# - API Consumer
# - ICS Power User

# Source common utilities
_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=tests/e2e/lib/common.sh
source "$_LIB_DIR/common.sh"

# Ensure BIN is available for all persona functions
if [ -z "${BIN:-}" ]; then
    export BIN
    BIN=$(ensure_binary)
fi

# ===================================================================
# SOLO DEVELOPER PERSONA
# ===================================================================

setup_solo_developer() {
    local test_name="$1"
    local test_db="/tmp/mnemosyne_solo_${test_name}_$(date +%s).db"

    print_cyan "[PERSONA] Setting up Solo Developer environment..." >&2

    # Create isolated database
    export DATABASE_URL="sqlite://$test_db"
    export MNEMOSYNE_NAMESPACE="project:myproject"

    # Store persona preferences
    "$BIN" remember --content "I prefer concise code reviews without fluff" \
        --namespace "global" --importance 7 >/dev/null 2>&1 || true

    "$BIN" remember --content "Use TypeScript for new projects, Rust for performance-critical code" \
        --namespace "global" --importance 8 >/dev/null 2>&1 || true

    print_green "  ✓ Solo developer environment ready" >&2
    echo "$test_db"
}

cleanup_solo_developer() {
    local test_db="$1"

    print_cyan "[PERSONA] Cleaning up Solo Developer environment..."
    rm -f "$test_db" "${test_db}-wal" "${test_db}-shm"
    unset DATABASE_URL MNEMOSYNE_NAMESPACE
}

# ===================================================================
# TEAM LEAD PERSONA
# ===================================================================

setup_team_lead() {
    local test_name="$1"
    local test_db="/tmp/mnemosyne_team_${test_name}_$(date +%s).db"

    print_cyan "[PERSONA] Setting up Team Lead environment..." >&2

    export DATABASE_URL="sqlite://$test_db"

    # Create team structure with namespaces
    local namespaces=(
        "team:engineering"
        "project:frontend"
        "project:backend"
        "feature:auth"
        "feature:dashboard"
    )

    for ns in "${namespaces[@]}"; do
        "$BIN" remember --content "Namespace setup for $ns" \
            --namespace "$ns" --importance 5 >/dev/null 2>&1 || true
    done

    # Store team preferences
    "$BIN" remember --content "Team uses conventional commits: feat/fix/docs/refactor" \
        --namespace "team:engineering" --importance 9 >/dev/null 2>&1 || true

    "$BIN" remember --content "All PRs require 2 approvals and passing CI" \
        --namespace "team:engineering" --importance 9 >/dev/null 2>&1 || true

    print_green "  ✓ Team lead environment ready (5 namespaces)" >&2
    echo "$test_db"
}

cleanup_team_lead() {
    local test_db="$1"

    print_cyan "[PERSONA] Cleaning up Team Lead environment..."
    rm -f "$test_db" "${test_db}-wal" "${test_db}-shm"
    unset DATABASE_URL
}

# ===================================================================
# POWER USER PERSONA
# ===================================================================

setup_power_user() {
    local test_name="$1"
    local test_db="/tmp/mnemosyne_power_${test_name}_$(date +%s).db"

    print_cyan "[PERSONA] Setting up Power User environment..." >&2

    export DATABASE_URL="sqlite://$test_db"
    export MNEMOSYNE_NAMESPACE="advanced:workflows"

    # Store advanced preferences
    "$BIN" remember --content "Use advanced search with filters and importance thresholds" \
        --namespace "global" --importance 8 >/dev/null 2>&1 || true

    "$BIN" remember --content "Enable all LLM features: enrichment, consolidation, discovery" \
        --namespace "global" --importance 9 >/dev/null 2>&1 || true

    print_green "  ✓ Power user environment ready" >&2
    echo "$test_db"
}

cleanup_power_user() {
    local test_db="$1"

    print_cyan "[PERSONA] Cleaning up Power User environment..."
    rm -f "$test_db" "${test_db}-wal" "${test_db}-shm"
    unset DATABASE_URL MNEMOSYNE_NAMESPACE
}

# ===================================================================
# AI AGENT (SINGLE) PERSONA
# ===================================================================

setup_ai_agent() {
    local test_name="$1"
    local test_db="/tmp/mnemosyne_agent_${test_name}_$(date +%s).db"

    print_cyan "[PERSONA] Setting up AI Agent environment..." >&2

    export DATABASE_URL="sqlite://$test_db"
    export MNEMOSYNE_SESSION_ID="session_$(date +%s)"
    export MNEMOSYNE_NAMESPACE="session:project:myproject"

    # Simulate agent storing context at phase boundaries
    "$BIN" remember --content "Phase 1 (Prompt→Spec): User wants authentication system with JWT" \
        --namespace "$MNEMOSYNE_NAMESPACE" --importance 9 >/dev/null 2>&1 || true

    "$BIN" remember --content "Typed hole: Implement JWT token generation and validation" \
        --namespace "$MNEMOSYNE_NAMESPACE" --importance 8 >/dev/null 2>&1 || true

    print_green "  ✓ AI agent environment ready (session namespace)" >&2
    echo "$test_db"
}

cleanup_ai_agent() {
    local test_db="$1"

    print_cyan "[PERSONA] Cleaning up AI Agent environment..."
    rm -f "$test_db" "${test_db}-wal" "${test_db}-shm"
    unset DATABASE_URL MNEMOSYNE_SESSION_ID MNEMOSYNE_NAMESPACE
}

# ===================================================================
# MULTI-AGENT SYSTEM PERSONA
# ===================================================================

setup_multi_agent() {
    local test_name="$1"
    local test_db="/tmp/mnemosyne_multiagent_${test_name}_$(date +%s).db"

    print_cyan "[PERSONA] Setting up Multi-Agent System environment..." >&2

    export DATABASE_URL="sqlite://$test_db"

    # Simulate 4 agents creating memories
    local agents=("orchestrator" "optimizer" "reviewer" "executor")

    for agent in "${agents[@]}"; do
        "$BIN" remember --content "Agent $agent initialized and ready" \
            --namespace "agent:$agent" --importance 7 >/dev/null 2>&1 || true
    done

    # Create shared work context
    "$BIN" remember --content "Work plan: Implement user authentication (3 tasks)" \
        --namespace "project:myproject" --importance 9 >/dev/null 2>&1 || true

    print_green "  ✓ Multi-agent environment ready (4 agents + shared context)" >&2
    echo "$test_db"
}

cleanup_multi_agent() {
    local test_db="$1"

    print_cyan "[PERSONA] Cleaning up Multi-Agent System environment..."
    rm -f "$test_db" "${test_db}-wal" "${test_db}-shm"
    unset DATABASE_URL
}

# ===================================================================
# PYTHON DEVELOPER PERSONA
# ===================================================================

setup_python_developer() {
    local test_name="$1"
    local test_db="/tmp/mnemosyne_python_${test_name}_$(date +%s).db"

    print_cyan "[PERSONA] Setting up Python Developer environment..." >&2

    export DATABASE_URL="sqlite://$test_db"
    export MNEMOSYNE_PYTHON_BINDINGS="1"

    # Python developer preferences
    "$BIN" remember --content "Use type hints (PEP 484) for all function signatures" \
        --namespace "global" --importance 8 >/dev/null 2>&1 || true

    "$BIN" remember --content "Prefer pytest over unittest for new tests" \
        --namespace "global" --importance 7 >/dev/null 2>&1 || true

    print_green "  ✓ Python developer environment ready" >&2
    echo "$test_db"
}

cleanup_python_developer() {
    local test_db="$1"

    print_cyan "[PERSONA] Cleaning up Python Developer environment..."
    rm -f "$test_db" "${test_db}-wal" "${test_db}-shm"
    unset DATABASE_URL MNEMOSYNE_PYTHON_BINDINGS
}

# ===================================================================
# API CONSUMER PERSONA
# ===================================================================

setup_api_consumer() {
    local test_name="$1"
    local test_db="/tmp/mnemosyne_api_${test_name}_$(date +%s).db"
    local api_port="${MNEMOSYNE_API_PORT:-3000}"

    print_cyan "[PERSONA] Setting up API Consumer environment..." >&2

    export DATABASE_URL="sqlite://$test_db"
    export MNEMOSYNE_API_URL="http://localhost:$api_port"

    # Start API server in background (if not already running)
    if ! curl -s "$MNEMOSYNE_API_URL/health" >/dev/null 2>&1; then
        print_cyan "  Starting API server on port $api_port..."
        "$BIN" serve --with-api --api-addr "0.0.0.0:$api_port" >/dev/null 2>&1 &
        local api_pid=$!
        export MNEMOSYNE_API_PID=$api_pid

        # Wait for server to start
        for i in {1..10}; do
            if curl -s "$MNEMOSYNE_API_URL/health" >/dev/null 2>&1; then
                break
            fi
            sleep 0.5
        done
    fi

    print_green "  ✓ API consumer environment ready" >&2
    echo "$test_db"
}

cleanup_api_consumer() {
    local test_db="$1"

    print_cyan "[PERSONA] Cleaning up API Consumer environment..."

    # Stop API server if we started it
    if [ -n "${MNEMOSYNE_API_PID:-}" ]; then
        kill "$MNEMOSYNE_API_PID" 2>/dev/null || true
        unset MNEMOSYNE_API_PID
    fi

    rm -f "$test_db" "${test_db}-wal" "${test_db}-shm"
    unset DATABASE_URL MNEMOSYNE_API_URL
}

# ===================================================================
# ICS POWER USER PERSONA
# ===================================================================

setup_ics_user() {
    local test_name="$1"
    local test_db="/tmp/mnemosyne_ics_${test_name}_$(date +%s).db"

    print_cyan "[PERSONA] Setting up ICS Power User environment..." >&2

    export DATABASE_URL="sqlite://$test_db"

    # Create sample context files for ICS
    local context_dir="/tmp/ics_context_${test_name}"
    mkdir -p "$context_dir"

    # Sample Rust file for editing
    cat > "$context_dir/main.rs" <<'EOF'
fn main() {
    println!("Hello, world!");
}

// TODO: Add error handling
fn process_data(data: &str) -> Result<(), String> {
    Ok(())
}
EOF

    # Store ICS preferences
    "$BIN" remember --content "Enable semantic highlighting Tier 3 for deep analysis" \
        --namespace "global" --importance 7 >/dev/null 2>&1 || true

    "$BIN" remember --content "Use vim mode with custom keybindings: jk for escape" \
        --namespace "global" --importance 6 >/dev/null 2>&1 || true

    export MNEMOSYNE_ICS_CONTEXT_DIR="$context_dir"

    print_green "  ✓ ICS power user environment ready" >&2
    echo "$test_db:$context_dir"
}

cleanup_ics_user() {
    local test_data="$1"
    local test_db="${test_data%:*}"
    local context_dir="${test_data#*:}"

    print_cyan "[PERSONA] Cleaning up ICS Power User environment..."

    rm -rf "$context_dir"
    rm -f "$test_db" "${test_db}-wal" "${test_db}-shm"
    unset DATABASE_URL MNEMOSYNE_ICS_CONTEXT_DIR
}

# ===================================================================
# DASHBOARD OBSERVER PERSONA
# ===================================================================

setup_dashboard_observer() {
    local test_name="$1"
    local test_db="/tmp/mnemosyne_dash_${test_name}_$(date +%s).db"

    print_cyan "[PERSONA] Setting up Dashboard Observer environment..." >&2

    export DATABASE_URL="sqlite://$test_db"

    # Populate with some agent activity
    for i in {1..5}; do
        "$BIN" remember --content "Agent activity $i: Processing task" \
            --namespace "agent:activity" --importance 5 >/dev/null 2>&1 || true
    done

    print_green "  ✓ Dashboard observer environment ready" >&2
    echo "$test_db"
}

cleanup_dashboard_observer() {
    local test_db="$1"

    print_cyan "[PERSONA] Cleaning up Dashboard Observer environment..."
    rm -f "$test_db" "${test_db}-wal" "${test_db}-shm"
    unset DATABASE_URL
}

# ===================================================================
# GENERIC PERSONA HELPERS
# ===================================================================

# Setup any persona by name
# Args: persona_name, test_name
setup_persona() {
    local persona="$1"
    local test_name="$2"

    case "$persona" in
        solo_developer|solo)
            setup_solo_developer "$test_name"
            ;;
        team_lead|team)
            setup_team_lead "$test_name"
            ;;
        power_user|power)
            setup_power_user "$test_name"
            ;;
        ai_agent|agent)
            setup_ai_agent "$test_name"
            ;;
        multi_agent|multiagent)
            setup_multi_agent "$test_name"
            ;;
        python_developer|python)
            setup_python_developer "$test_name"
            ;;
        api_consumer|api)
            setup_api_consumer "$test_name"
            ;;
        ics_user|ics)
            setup_ics_user "$test_name"
            ;;
        dashboard_observer|dashboard)
            setup_dashboard_observer "$test_name"
            ;;
        *)
            fail "Unknown persona: $persona"
            return 1
            ;;
    esac
}

# Cleanup any persona by name
# Args: persona_name, test_data (optional)
cleanup_persona() {
    local persona="$1"
    local test_data="${2:-}"

    # Skip cleanup if no test data provided
    if [ -z "$test_data" ]; then
        warn "No test data provided for cleanup of $persona"
        return 0
    fi

    case "$persona" in
        solo_developer|solo)
            cleanup_solo_developer "$test_data"
            ;;
        team_lead|team)
            cleanup_team_lead "$test_data"
            ;;
        power_user|power)
            cleanup_power_user "$test_data"
            ;;
        ai_agent|agent)
            cleanup_ai_agent "$test_data"
            ;;
        multi_agent|multiagent)
            cleanup_multi_agent "$test_data"
            ;;
        python_developer|python)
            cleanup_python_developer "$test_data"
            ;;
        api_consumer|api)
            cleanup_api_consumer "$test_data"
            ;;
        ics_user|ics)
            cleanup_ics_user "$test_data"
            ;;
        dashboard_observer|dashboard)
            cleanup_dashboard_observer "$test_data"
            ;;
        *)
            warn "Unknown persona for cleanup: $persona"
            ;;
    esac
}

# Alias for backward compatibility with tests
teardown_persona() {
    cleanup_persona "$@"
}

# Export persona functions
export -f setup_solo_developer cleanup_solo_developer
export -f setup_team_lead cleanup_team_lead
export -f setup_power_user cleanup_power_user
export -f setup_ai_agent cleanup_ai_agent
export -f setup_multi_agent cleanup_multi_agent
export -f setup_python_developer cleanup_python_developer
export -f setup_api_consumer cleanup_api_consumer
export -f setup_ics_user cleanup_ics_user
export -f setup_dashboard_observer cleanup_dashboard_observer
export -f setup_persona cleanup_persona teardown_persona
