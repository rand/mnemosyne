#!/bin/bash
#
# DSPy Pipeline Monitoring Cron Job
#
# Run this script regularly (hourly/daily) to:
# 1. Check orchestration run health
# 2. Monitor A/B test status
# 3. Track dataset quality trends
# 4. Send alerts via email/Slack/file
#
# Recommended crontab entry (hourly):
# 0 * * * * /path/to/cron_monitoring.sh >> /var/log/dspy_monitoring.log 2>&1
#

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASE_DIR="${SCRIPT_DIR}/.."
LOG_DIR="/var/log/dspy_optimization"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
LOG_FILE="${LOG_DIR}/monitoring_${TIMESTAMP}.log"

# Monitoring config (optional - uses defaults if not set)
MONITORING_CONFIG="${MONITORING_CONFIG:-${SCRIPT_DIR}/monitoring_config.json}"

# Ensure log directory exists
mkdir -p "${LOG_DIR}"

# Logging function
log() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] $*" | tee -a "${LOG_FILE}"
}

log "========================================="
log "DSPy Monitoring - Starting"
log "========================================="

# Change to DSPy modules directory
cd "${BASE_DIR}" || exit 1

# Check if monitoring config exists
CONFIG_ARGS=""
if [ -f "${MONITORING_CONFIG}" ]; then
    log "Using configuration from ${MONITORING_CONFIG}"
    CONFIG_ARGS="--config ${MONITORING_CONFIG}"
fi

# Run all monitoring checks
log "Running monitoring checks..."
env PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 \
    uv run python3 data_collection/monitoring.py check-all ${CONFIG_ARGS} \
    2>&1 | tee -a "${LOG_FILE}"

EXIT_CODE=$?

if [ ${EXIT_CODE} -eq 0 ]; then
    log "========================================="
    log "Monitoring - COMPLETED"
    log "========================================="
else
    log "========================================="
    log "Monitoring - FAILED (exit code: ${EXIT_CODE})"
    log "========================================="
fi

exit ${EXIT_CODE}
