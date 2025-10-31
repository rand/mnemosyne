#!/usr/bin/env bash
# Baseline Quality Validators
#
# Validates real LLM responses in baseline mode to ensure quality standards.
# These validators establish quality thresholds for:
# - Enrichment (summary, keywords, embeddings)
# - Consolidation recommendations
# - Requirement extraction
# - Anti-pattern detection
# - Importance scoring

# Source common utilities
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=tests/e2e/lib/common.sh
source "$SCRIPT_DIR/common.sh"

# ===================================================================
# ENRICHMENT QUALITY VALIDATION
# ===================================================================

# Validate memory enrichment quality
# Args: enrichment_json
# Returns: 0 if valid, 1 if invalid
validate_enrichment_quality() {
    local response="$1"

    print_cyan "[BASELINE] Validating enrichment quality..."

    # Check JSON structure
    if ! echo "$response" | jq empty 2>/dev/null; then
        fail "Invalid JSON response"
        return 1
    fi

    # Extract fields
    local summary=$(echo "$response" | jq -r '.summary // empty')
    local keywords=$(echo "$response" | jq -r '.keywords // empty')
    local confidence=$(echo "$response" | jq -r '.confidence // 0')
    local embedding=$(echo "$response" | jq -r '.embedding // empty')

    # Validate summary
    if [ -z "$summary" ]; then
        fail "Missing summary field"
        return 1
    fi

    local summary_len=${#summary}
    if [ "$summary_len" -lt 20 ]; then
        fail "Summary too short: $summary_len chars (expected ≥20)"
        return 1
    fi

    if [ "$summary_len" -gt 500 ]; then
        warn "Summary very long: $summary_len chars (expected <500)"
    fi

    print_green "  ✓ Summary length: $summary_len chars"

    # Validate keywords
    if [ -z "$keywords" ]; then
        warn "Missing keywords field (non-fatal)"
    else
        local keyword_count=$(echo "$response" | jq '.keywords | length')
        if [ "$keyword_count" -lt 2 ]; then
            warn "Few keywords: $keyword_count (expected 3-10)"
        elif [ "$keyword_count" -gt 15 ]; then
            warn "Many keywords: $keyword_count (expected 3-10)"
        fi
        print_green "  ✓ Keywords count: $keyword_count"
    fi

    # Validate confidence
    if (( $(echo "$confidence < 0.0" | bc -l) )) || (( $(echo "$confidence > 1.0" | bc -l) )); then
        fail "Confidence out of range: $confidence (expected 0.0-1.0)"
        return 1
    fi

    if (( $(echo "$confidence < 0.7" | bc -l) )); then
        warn "Low confidence: $confidence (expected ≥0.7)"
    fi

    print_green "  ✓ Confidence: $confidence"

    # Validate embedding (optional but recommended)
    if [ -n "$embedding" ]; then
        local embedding_dim=$(echo "$response" | jq '.embedding | length')
        if [ "$embedding_dim" -ne 1536 ] && [ "$embedding_dim" -ne 0 ]; then
            warn "Unexpected embedding dimension: $embedding_dim (expected 1536 or empty)"
        else
            print_green "  ✓ Embedding dimension: $embedding_dim"
        fi
    fi

    pass "Enrichment quality validated"
    return 0
}

# ===================================================================
# CONSOLIDATION QUALITY VALIDATION
# ===================================================================

# Validate consolidation recommendation quality
# Args: consolidation_json
# Returns: 0 if valid, 1 if invalid
validate_consolidation_quality() {
    local response="$1"

    print_cyan "[BASELINE] Validating consolidation quality..."

    # Check JSON structure
    if ! echo "$response" | jq empty 2>/dev/null; then
        fail "Invalid JSON response"
        return 1
    fi

    # Extract fields
    local should_consolidate=$(echo "$response" | jq -r '.should_consolidate // empty')
    local confidence=$(echo "$response" | jq -r '.confidence // 0')
    local rationale=$(echo "$response" | jq -r '.rationale // empty')
    local consolidated_content=$(echo "$response" | jq -r '.consolidated_content // empty')

    # Validate decision field
    if [ -z "$should_consolidate" ]; then
        fail "Missing should_consolidate field"
        return 1
    fi

    if [ "$should_consolidate" != "true" ] && [ "$should_consolidate" != "false" ]; then
        fail "Invalid should_consolidate value: $should_consolidate (expected true/false)"
        return 1
    fi

    print_green "  ✓ Decision: $should_consolidate"

    # Validate confidence
    if (( $(echo "$confidence < 0.0" | bc -l) )) || (( $(echo "$confidence > 1.0" | bc -l) )); then
        fail "Confidence out of range: $confidence (expected 0.0-1.0)"
        return 1
    fi

    if (( $(echo "$confidence < 0.6" | bc -l) )); then
        warn "Low confidence: $confidence (expected ≥0.6)"
    fi

    print_green "  ✓ Confidence: $confidence"

    # Validate rationale (must be substantive)
    if [ -z "$rationale" ]; then
        fail "Missing rationale field"
        return 1
    fi

    local rationale_len=${#rationale}
    if [ "$rationale_len" -lt 30 ]; then
        fail "Rationale too short: $rationale_len chars (expected ≥30)"
        return 1
    fi

    print_green "  ✓ Rationale length: $rationale_len chars"

    # If consolidation recommended, validate consolidated content
    if [ "$should_consolidate" = "true" ]; then
        if [ -z "$consolidated_content" ]; then
            fail "Missing consolidated_content when consolidation recommended"
            return 1
        fi

        local content_len=${#consolidated_content}
        if [ "$content_len" -lt 20 ]; then
            fail "Consolidated content too short: $content_len chars (expected ≥20)"
            return 1
        fi

        print_green "  ✓ Consolidated content length: $content_len chars"
    fi

    pass "Consolidation quality validated"
    return 0
}

# ===================================================================
# REQUIREMENT EXTRACTION QUALITY VALIDATION
# ===================================================================

# Validate requirement extraction quality
# Args: requirements_json
# Returns: 0 if valid, 1 if invalid
validate_requirements_quality() {
    local response="$1"

    print_cyan "[BASELINE] Validating requirement extraction quality..."

    # Check JSON structure
    if ! echo "$response" | jq empty 2>/dev/null; then
        fail "Invalid JSON response"
        return 1
    fi

    # Extract requirements array
    local requirements=$(echo "$response" | jq -r '.requirements // empty')
    if [ -z "$requirements" ]; then
        fail "Missing requirements field"
        return 1
    fi

    local req_count=$(echo "$response" | jq '.requirements | length')
    if [ "$req_count" -eq 0 ]; then
        warn "No requirements extracted (may be valid for simple tasks)"
    else
        print_green "  ✓ Requirements extracted: $req_count"

        # Validate each requirement is non-empty
        local empty_reqs=$(echo "$response" | jq '[.requirements[] | select(length == 0)] | length')
        if [ "$empty_reqs" -gt 0 ]; then
            fail "Found $empty_reqs empty requirements"
            return 1
        fi

        # Check for reasonable requirement length (at least 10 chars each)
        local short_reqs=$(echo "$response" | jq '[.requirements[] | select(length < 10)] | length')
        if [ "$short_reqs" -gt 0 ]; then
            warn "$short_reqs requirements are very short (<10 chars)"
        fi
    fi

    # Validate confidence if present
    local confidence=$(echo "$response" | jq -r '.confidence // empty')
    if [ -n "$confidence" ]; then
        if (( $(echo "$confidence < 0.0" | bc -l) )) || (( $(echo "$confidence > 1.0" | bc -l) )); then
            fail "Confidence out of range: $confidence (expected 0.0-1.0)"
            return 1
        fi
        print_green "  ✓ Confidence: $confidence"
    fi

    pass "Requirement extraction quality validated"
    return 0
}

# ===================================================================
# ANTI-PATTERN DETECTION QUALITY VALIDATION
# ===================================================================

# Validate anti-pattern detection quality
# Args: antipatterns_json
# Returns: 0 if valid, 1 if invalid
validate_antipattern_quality() {
    local response="$1"

    print_cyan "[BASELINE] Validating anti-pattern detection quality..."

    # Check JSON structure
    if ! echo "$response" | jq empty 2>/dev/null; then
        fail "Invalid JSON response"
        return 1
    fi

    # Extract fields
    local found=$(echo "$response" | jq -r '.anti_patterns_found // empty')
    local issues=$(echo "$response" | jq -r '.issues // empty')

    if [ -z "$found" ]; then
        fail "Missing anti_patterns_found field"
        return 1
    fi

    if [ "$found" != "true" ] && [ "$found" != "false" ]; then
        fail "Invalid anti_patterns_found value: $found (expected true/false)"
        return 1
    fi

    print_green "  ✓ Anti-patterns found: $found"

    # Validate issues array
    if [ -z "$issues" ]; then
        fail "Missing issues field"
        return 1
    fi

    local issue_count=$(echo "$response" | jq '.issues | length')

    # If anti-patterns found, should have issues
    if [ "$found" = "true" ] && [ "$issue_count" -eq 0 ]; then
        fail "Anti-patterns reported but no issues listed"
        return 1
    fi

    # If no anti-patterns, should have empty issues
    if [ "$found" = "false" ] && [ "$issue_count" -gt 0 ]; then
        warn "No anti-patterns reported but $issue_count issues listed"
    fi

    if [ "$issue_count" -gt 0 ]; then
        print_green "  ✓ Issues detected: $issue_count"

        # Validate each issue has type and severity
        local invalid_issues=$(echo "$response" | jq '[.issues[] | select(.type == null or .severity == null)] | length')
        if [ "$invalid_issues" -gt 0 ]; then
            fail "$invalid_issues issues missing type or severity"
            return 1
        fi
    fi

    pass "Anti-pattern detection quality validated"
    return 0
}

# ===================================================================
# IMPORTANCE SCORING QUALITY VALIDATION
# ===================================================================

# Validate importance recalibration quality
# Args: importance_json
# Returns: 0 if valid, 1 if invalid
validate_importance_quality() {
    local response="$1"

    print_cyan "[BASELINE] Validating importance scoring quality..."

    # Check JSON structure
    if ! echo "$response" | jq empty 2>/dev/null; then
        fail "Invalid JSON response"
        return 1
    fi

    # Extract fields
    local memory_id=$(echo "$response" | jq -r '.memory_id // empty')
    local old_importance=$(echo "$response" | jq -r '.old_importance // empty')
    local new_importance=$(echo "$response" | jq -r '.new_importance // empty')

    if [ -z "$memory_id" ]; then
        fail "Missing memory_id field"
        return 1
    fi

    # Validate old importance
    if [ -z "$old_importance" ]; then
        fail "Missing old_importance field"
        return 1
    fi

    if (( $(echo "$old_importance < 1" | bc -l) )) || (( $(echo "$old_importance > 10" | bc -l) )); then
        fail "Old importance out of range: $old_importance (expected 1-10)"
        return 1
    fi

    # Validate new importance
    if [ -z "$new_importance" ]; then
        fail "Missing new_importance field"
        return 1
    fi

    if (( $(echo "$new_importance < 1" | bc -l) )) || (( $(echo "$new_importance > 10" | bc -l) )); then
        fail "New importance out of range: $new_importance (expected 1-10)"
        return 1
    fi

    print_green "  ✓ Importance recalibrated: $old_importance → $new_importance"

    # Validate factors if present
    local factors=$(echo "$response" | jq -r '.factors // empty')
    if [ -n "$factors" ]; then
        local factor_count=$(echo "$response" | jq '.factors | keys | length')
        print_green "  ✓ Factors considered: $factor_count"
    fi

    pass "Importance scoring quality validated"
    return 0
}

# ===================================================================
# SEMANTIC HIGHLIGHTING QUALITY VALIDATION
# ===================================================================

# Validate semantic highlighting (Tier 3) quality
# Args: highlighting_json
# Returns: 0 if valid, 1 if invalid
validate_semantic_highlighting_quality() {
    local response="$1"

    print_cyan "[BASELINE] Validating semantic highlighting quality..."

    # Check JSON structure
    if ! echo "$response" | jq empty 2>/dev/null; then
        fail "Invalid JSON response"
        return 1
    fi

    # Extract tier
    local tier=$(echo "$response" | jq -r '.tier // empty')
    if [ -z "$tier" ]; then
        fail "Missing tier field"
        return 1
    fi

    if [ "$tier" -ne 3 ]; then
        fail "Expected tier 3 (analytical), got tier $tier"
        return 1
    fi

    print_green "  ✓ Tier: $tier (analytical)"

    # Validate features object
    local features=$(echo "$response" | jq -r '.features // empty')
    if [ -z "$features" ]; then
        fail "Missing features field"
        return 1
    fi

    # Check for expected feature categories
    local has_discourse=$(echo "$response" | jq 'has("features.discourse_markers")')
    local has_pragmatics=$(echo "$response" | jq 'has("features.pragmatics")')
    local has_contradictions=$(echo "$response" | jq 'has("features.contradictions")')

    if [ "$has_discourse" != "true" ]; then
        warn "Missing discourse_markers in features"
    fi

    print_green "  ✓ Feature categories present"

    # Validate processing time
    local time=$(echo "$response" | jq -r '.processing_time_ms // 0')
    if [ "$time" -eq 0 ]; then
        warn "Missing or zero processing_time_ms"
    elif [ "$time" -gt 10000 ]; then
        warn "Very long processing time: ${time}ms (expected <10000ms)"
    fi

    pass "Semantic highlighting quality validated"
    return 0
}

# ===================================================================
# AGGREGATE VALIDATION
# ===================================================================

# Run all baseline validators on a complete test run
# Args: test_results_dir
# Returns: 0 if all valid, 1 if any failed
validate_baseline_run() {
    local results_dir="$1"

    print_cyan "=== BASELINE RUN VALIDATION ==="
    print_cyan "Validating all LLM responses in: $results_dir"
    echo ""

    local total=0
    local passed=0
    local failed=0

    # Find all JSON result files
    for result_file in "$results_dir"/*.json; do
        [ -e "$result_file" ] || continue

        ((total++))
        local filename=$(basename "$result_file")
        print_cyan "Validating: $filename"

        # Determine validation type from filename
        case "$filename" in
            *enrichment*)
                if validate_enrichment_quality "$(cat "$result_file")"; then
                    ((passed++))
                else
                    ((failed++))
                fi
                ;;
            *consolidation*)
                if validate_consolidation_quality "$(cat "$result_file")"; then
                    ((passed++))
                else
                    ((failed++))
                fi
                ;;
            *requirements*)
                if validate_requirements_quality "$(cat "$result_file")"; then
                    ((passed++))
                else
                    ((failed++))
                fi
                ;;
            *antipatterns*)
                if validate_antipattern_quality "$(cat "$result_file")"; then
                    ((passed++))
                else
                    ((failed++))
                fi
                ;;
            *importance*)
                if validate_importance_quality "$(cat "$result_file")"; then
                    ((passed++))
                else
                    ((failed++))
                fi
                ;;
            *highlighting*)
                if validate_semantic_highlighting_quality "$(cat "$result_file")"; then
                    ((passed++))
                else
                    ((failed++))
                fi
                ;;
            *)
                warn "Unknown result type: $filename (skipped)"
                ((total--))
                ;;
        esac

        echo ""
    done

    # Summary
    print_cyan "=== VALIDATION SUMMARY ==="
    echo "Total responses: $total"
    print_green "Passed: $passed"
    if [ "$failed" -gt 0 ]; then
        print_red "Failed: $failed"
        return 1
    else
        print_green "All validations passed!"
        return 0
    fi
}

# Export validation functions
export -f validate_enrichment_quality
export -f validate_consolidation_quality
export -f validate_requirements_quality
export -f validate_antipattern_quality
export -f validate_importance_quality
export -f validate_semantic_highlighting_quality
export -f validate_baseline_run
