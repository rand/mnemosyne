#!/bin/bash
#
# Monthly DSPy Optimization Cron Job
#
# Run this script monthly to:
# 1. Collect new training data (git mining + synthetic generation)
# 2. Validate and filter through quality gates
# 3. Version datasets with provenance tracking
# 4. Run MIPROv2 optimization
# 5. Evaluate results and make deployment decisions
#
# Recommended crontab entry (1st of month at 2 AM):
# 0 2 1 * * /path/to/cron_monthly_optimization.sh >> /var/log/dspy_optimization.log 2>&1
#

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASE_DIR="${SCRIPT_DIR}/.."
LOG_DIR="/var/log/dspy_optimization"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
LOG_FILE="${LOG_DIR}/monthly_optimization_${TIMESTAMP}.log"

# Ensure log directory exists
mkdir -p "${LOG_DIR}"

# Logging function
log() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] $*" | tee -a "${LOG_FILE}"
}

# Error handler
error_exit() {
    log "ERROR: $1"
    exit 1
}

# Start
log "========================================="
log "Monthly DSPy Optimization - Starting"
log "========================================="
log "Base directory: ${BASE_DIR}"
log "Log file: ${LOG_FILE}"

# Check dependencies
log "Checking dependencies..."
command -v python3 >/dev/null 2>&1 || error_exit "python3 not found"
command -v uv >/dev/null 2>&1 || error_exit "uv not found"

# Check for required environment variables
if [ -z "${ANTHROPIC_API_KEY:-}" ]; then
    log "WARNING: ANTHROPIC_API_KEY not set, attempting to load from mnemosyne config..."

    # Try to get API key from mnemosyne
    API_KEY=$(cd "${BASE_DIR}/../.." && \
              env PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 \
              cargo run --quiet --bin mnemosyne -- secrets get anthropic_api_key 2>/dev/null || echo "")

    if [ -n "${API_KEY}" ]; then
        export ANTHROPIC_API_KEY="${API_KEY}"
        log "Loaded API key from mnemosyne secrets"
    else
        error_exit "ANTHROPIC_API_KEY not available"
    fi
fi

# Change to DSPy modules directory
cd "${BASE_DIR}" || error_exit "Failed to change to ${BASE_DIR}"

# Configuration defaults (can be overridden via environment variables)
GIT_MINING_TARGET="${GIT_MINING_TARGET:-30}"
SYNTHETIC_TARGET="${SYNTHETIC_TARGET:-20}"
MIPRO_TRIALS="${MIPRO_TRIALS:-50}"
OUTPUT_DIR="${OUTPUT_DIR:-/tmp/optimization_runs}"

log "Configuration:"
log "  Git mining target: ${GIT_MINING_TARGET} examples"
log "  Synthetic target: ${SYNTHETIC_TARGET} examples"
log "  MIPROv2 trials: ${MIPRO_TRIALS}"
log "  Output directory: ${OUTPUT_DIR}"

# Run optimization orchestrator
log "Starting optimization orchestrator..."
env PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 \
    ANTHROPIC_API_KEY="${ANTHROPIC_API_KEY}" \
    uv run python3 data_collection/optimization_orchestrator.py \
    --git-target "${GIT_MINING_TARGET}" \
    --synthetic-target "${SYNTHETIC_TARGET}" \
    --trials "${MIPRO_TRIALS}" \
    --output-dir "${OUTPUT_DIR}" \
    2>&1 | tee -a "${LOG_FILE}"

EXIT_CODE=$?

if [ ${EXIT_CODE} -eq 0 ]; then
    log "========================================="
    log "Monthly Optimization - COMPLETED"
    log "========================================="

    # Find and display the summary
    SUMMARY_FILE=$(find "${OUTPUT_DIR}" -name "orchestration_summary_*.json" -type f | sort -r | head -1)
    if [ -n "${SUMMARY_FILE}" ]; then
        log "Summary file: ${SUMMARY_FILE}"
        log "Results:"
        python3 -m json.tool "${SUMMARY_FILE}" 2>/dev/null | grep -E '"success"|"results"' | head -20 | tee -a "${LOG_FILE}"
    fi

    exit 0
else
    log "========================================="
    log "Monthly Optimization - FAILED (exit code: ${EXIT_CODE})"
    log "========================================="
    error_exit "Optimization orchestrator failed"
fi
