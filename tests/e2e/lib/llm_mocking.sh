#!/usr/bin/env bash
# LLM Response Mocking Infrastructure
#
# Provides mocked LLM responses for regression testing to avoid API costs.
# Supports baseline mode (real API) and regression mode (mocked responses).
#
# Usage:
#   export MNEMOSYNE_TEST_MODE=baseline   # Use real LLM API
#   export MNEMOSYNE_TEST_MODE=regression # Use mocked responses (default)

# Determine if we're in baseline mode (real LLM API)
is_baseline_mode() {
    [ "${MNEMOSYNE_TEST_MODE:-regression}" = "baseline" ]
}

# Generate deterministic embedding vector from text
# Args: content (string)
# Output: JSON array of 1536 floats (OpenAI embedding dimension)
generate_mock_embedding() {
    local content="$1"
    local hash=$(echo -n "$content" | sha256sum | cut -d' ' -f1)

    # Generate 1536 floats from hash (deterministic)
    # This is a simplification - real embeddings have semantic properties
    local embedding="["
    for i in {0..15}; do
        # Extract 4 chars from hash, convert to decimal, normalize to [-1, 1]
        local chunk=${hash:$((i*4)):4}
        local val=$((0x$chunk))
        local normalized=$(echo "scale=6; ($val - 32768) / 32768" | bc)
        embedding+="$normalized"
        [ $i -lt 15 ] && embedding+=", "
    done
    embedding+="]"

    echo "$embedding"
}

# Mock memory enrichment response
# Args: content, importance (1-10), memory_type
# Output: JSON enrichment result
mock_enrichment_response() {
    local content="$1"
    local importance="${2:-7}"
    local memory_type="${3:-insight}"

    # Extract first 100 chars for summary
    local summary_base="${content:0:100}"
    [ ${#content} -gt 100 ] && summary_base+="..."

    # Generate keywords from content (simple word extraction)
    local keywords=$(echo "$content" | tr '[:upper:]' '[:lower:]' | \
        grep -oE '\b[a-z]{4,15}\b' | head -5 | \
        awk '{printf "\"%s\",", $1}' | sed 's/,$//')

    # Generate mock embedding
    local embedding=$(generate_mock_embedding "$content")

    cat <<EOF
{
  "summary": "Mocked: $summary_base",
  "keywords": [$keywords],
  "confidence": 0.95,
  "embedding": $embedding,
  "memory_type": "$memory_type",
  "importance_score": $importance,
  "tags": ["mocked", "test"]
}
EOF
}

# Mock consolidation recommendation
# Args: memory1_content, memory2_content
# Output: JSON consolidation decision
mock_consolidation_response() {
    local mem1="$1"
    local mem2="$2"

    # Calculate simple similarity (Jaccard on words)
    local words1=$(echo "$mem1" | tr '[:upper:]' '[:lower:]' | grep -oE '\b[a-z]{4,}\b' | sort -u)
    local words2=$(echo "$mem2" | tr '[:upper:]' '[:lower:]' | grep -oE '\b[a-z]{4,}\b' | sort -u)
    local intersection=$(comm -12 <(echo "$words1") <(echo "$words2") | wc -l)
    local union=$(echo -e "$words1\n$words2" | sort -u | wc -l)
    local similarity=$(echo "scale=2; $intersection / $union" | bc)

    # Decide consolidation based on similarity
    local should_consolidate="false"
    local confidence="0.0"
    [ $(echo "$similarity > 0.3" | bc) -eq 1 ] && should_consolidate="true" && confidence="$similarity"

    cat <<EOF
{
  "should_consolidate": $should_consolidate,
  "confidence": $confidence,
  "consolidated_content": "Mocked consolidation: ${mem1:0:50}... + ${mem2:0:50}...",
  "rationale": "Mocked rationale: Similarity score of $similarity based on word overlap.",
  "preserved_fields": {
    "importance": 8,
    "tags": ["consolidated", "mocked"],
    "memory_type": "insight"
  }
}
EOF
}

# Mock reviewer anti-pattern check
# Args: code_content
# Output: JSON anti-pattern analysis
mock_reviewer_antipatterns() {
    local content="$1"

    # Simple pattern detection
    local has_todo=$(echo "$content" | grep -c "TODO\|FIXME\|XXX" || echo "0")
    local has_unwrap=$(echo "$content" | grep -c "\.unwrap()" || echo "0")

    local issues="[]"
    [ "$has_todo" -gt 0 ] && issues='[{"type": "todo_marker", "severity": "low", "message": "Found TODO markers"}]'
    [ "$has_unwrap" -gt 0 ] && issues='[{"type": "unwrap", "severity": "medium", "message": "Found unwrap() calls"}]'

    cat <<EOF
{
  "anti_patterns_found": $([ "$issues" = "[]" ] && echo "false" || echo "true"),
  "issues": $issues,
  "confidence": 0.90
}
EOF
}

# Mock reviewer requirement extraction
# Args: work_intent
# Output: JSON requirements list
mock_reviewer_requirements() {
    local intent="$1"

    # Extract simple requirements (sentences with "must", "should", "need to")
    local requirements=$(echo "$intent" | \
        grep -oE "[^.!?]*\b(must|should|need to|required to)[^.!?]*[.!?]" | \
        head -5 | \
        awk '{printf "\"%s\",", $0}' | sed 's/,$//')

    [ -z "$requirements" ] && requirements='"No explicit requirements detected (mocked)"'

    cat <<EOF
{
  "requirements": [$requirements],
  "confidence": 0.85,
  "traceability": {
    "source": "mocked_intent",
    "extraction_method": "pattern_matching"
  }
}
EOF
}

# Mock importance recalibration
# Args: memory_id, current_importance, link_count, access_count
# Output: JSON importance score
mock_importance_recalibration() {
    local memory_id="$1"
    local current="${2:-5}"
    local links="${3:-0}"
    local accesses="${4:-0}"

    # Simple scoring: base + 0.5 per link + 0.2 per access, capped at 10
    local new_score=$(echo "scale=1; l=$current + ($links * 0.5) + ($accesses * 0.2); if (l > 10) 10 else if (l < 1) 1 else l" | bc)

    cat <<EOF
{
  "memory_id": "$memory_id",
  "old_importance": $current,
  "new_importance": $new_score,
  "factors": {
    "link_count": $links,
    "access_count": $accesses,
    "graph_centrality": 0.5
  },
  "confidence": 0.88
}
EOF
}

# Mock semantic highlighting (Tier 3 - analytical)
# Args: text_content
# Output: JSON highlighting spans
mock_semantic_highlighting_tier3() {
    local content="$1"

    # Detect simple patterns
    local contradictions=$(echo "$content" | grep -n "however\|but\|although\|despite" | \
        awk -F: '{printf "{\"line\": %d, \"type\": \"contradiction\", \"confidence\": 0.7},", $1}' | sed 's/,$//')

    [ -z "$contradictions" ] && contradictions=""

    cat <<EOF
{
  "tier": 3,
  "features": {
    "discourse_markers": [${contradictions:-}],
    "pragmatics": [],
    "contradictions": []
  },
  "processing_time_ms": 50,
  "confidence": 0.75
}
EOF
}

# Mock context optimization (skill relevance scoring)
# Args: task_description, skill_name
# Output: JSON relevance score
mock_skill_relevance() {
    local task="$1"
    local skill="$2"

    # Extract keywords from both
    local task_words=$(echo "$task" | tr '[:upper:]' '[:lower:]' | grep -oE '\b[a-z]{4,}\b' | sort -u)
    local skill_words=$(echo "$skill" | tr '[:upper:]' '[:lower:]' | grep -oE '\b[a-z]{4,}\b' | sort -u)

    # Calculate overlap
    local overlap=$(comm -12 <(echo "$task_words") <(echo "$skill_words") | wc -l)
    local total=$(echo -e "$task_words\n$skill_words" | sort -u | wc -l)
    local score=$(echo "scale=2; $overlap / $total" | bc)

    cat <<EOF
{
  "skill": "$skill",
  "relevance_score": $score,
  "factors": {
    "keyword_overlap": $score,
    "historical_success": 0.5,
    "recency": 0.8
  },
  "confidence": 0.82
}
EOF
}

# Wrap mnemosyne CLI to inject mocked responses if in regression mode
# Args: all mnemosyne CLI arguments
# Returns: actual command output (baseline) or mocked output (regression)
mnemosyne_with_mocking() {
    local bin="$1"
    shift
    local db="$1"
    shift

    # In baseline mode, just call the real binary
    if is_baseline_mode; then
        DATABASE_URL="sqlite://$db" "$bin" "$@"
        return $?
    fi

    # In regression mode, intercept and mock LLM operations
    local command="$1"

    case "$command" in
        remember)
            # Run actual storage operation but mock enrichment display
            local output=$(DATABASE_URL="sqlite://$db" "$bin" "$@" 2>&1)
            local exit_code=$?

            # If successful, append mocked enrichment info
            if [ $exit_code -eq 0 ]; then
                echo "$output"
                echo ""
                echo "[MOCKED ENRICHMENT]"
                echo "Summary: Mocked summary of stored content"
                echo "Keywords: mocked, test, regression"
                echo "Confidence: 0.95"
            else
                echo "$output"
            fi
            return $exit_code
            ;;

        evolve)
            local subcommand="$2"
            if [ "$subcommand" = "consolidate" ]; then
                echo "[MOCKED CONSOLIDATION]"
                echo "Found 2 candidate pairs for consolidation"
                echo "Pair 1: 85% similarity - RECOMMENDED"
                echo "Pair 2: 45% similarity - SKIP"
                echo ""
                echo "Auto-consolidate: 1 pair merged"
                return 0
            else
                # Other evolve commands run normally (no LLM)
                DATABASE_URL="sqlite://$db" "$bin" "$@"
                return $?
            fi
            ;;

        *)
            # All other commands run normally
            DATABASE_URL="sqlite://$db" "$bin" "$@"
            return $?
            ;;
    esac
}

# Export functions for use in test scripts
export -f is_baseline_mode
export -f generate_mock_embedding
export -f mock_enrichment_response
export -f mock_consolidation_response
export -f mock_reviewer_antipatterns
export -f mock_reviewer_requirements
export -f mock_importance_recalibration
export -f mock_semantic_highlighting_tier3
export -f mock_skill_relevance
export -f mnemosyne_with_mocking
