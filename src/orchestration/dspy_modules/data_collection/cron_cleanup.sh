#!/bin/bash
#
# DSPy Data Collection Cleanup Cron Job
#
# Run this script weekly to:
# 1. Clean up old temporary files from optimization runs
# 2. Archive old dataset versions
# 3. Rotate logs
# 4. Remove failed experiments
#
# Recommended crontab entry (Sunday at 3 AM):
# 0 3 * * 0 /path/to/cron_cleanup.sh >> /var/log/dspy_cleanup.log 2>&1
#

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASE_DIR="${SCRIPT_DIR}/.."
LOG_DIR="/var/log/dspy_optimization"
OUTPUT_DIR="${OUTPUT_DIR:-/tmp/optimization_runs}"
AB_EXPERIMENTS_DIR="${AB_EXPERIMENTS_DIR:-.ab_experiments}"

# Retention periods (days)
TEMP_FILES_RETENTION=7       # Keep temp files for 1 week
DATASET_VERSION_RETENTION=90 # Keep old dataset versions for 3 months
LOG_RETENTION=30             # Keep logs for 1 month
AB_EXPERIMENT_RETENTION=30   # Keep completed A/B experiments for 1 month

# Logging
log() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] $*"
}

log "========================================="
log "DSPy Cleanup - Starting"
log "========================================="

# 1. Clean up old temporary optimization files
log "Cleaning up old optimization run files (>${TEMP_FILES_RETENTION} days)..."
if [ -d "${OUTPUT_DIR}" ]; then
    BEFORE=$(du -sh "${OUTPUT_DIR}" 2>/dev/null | cut -f1 || echo "0")
    find "${OUTPUT_DIR}" -type f -mtime +${TEMP_FILES_RETENTION} -delete 2>/dev/null || true
    find "${OUTPUT_DIR}" -type d -empty -delete 2>/dev/null || true
    AFTER=$(du -sh "${OUTPUT_DIR}" 2>/dev/null | cut -f1 || echo "0")
    log "  Optimization runs: ${BEFORE} -> ${AFTER}"
else
    log "  No optimization run directory found"
fi

# 2. Archive old dataset versions (keep latest 3 versions per signature)
log "Archiving old dataset versions..."
cd "${BASE_DIR}" || exit 1

if [ -d "training_data" ]; then
    for sig in extract_requirements validate_intent validate_completeness validate_correctness generate_guidance; do
        SIG_DIR="training_data/${sig}"
        if [ -d "${SIG_DIR}" ]; then
            # Count versions
            VERSION_COUNT=$(find "${SIG_DIR}" -maxdepth 1 -type d -name 'v*' | wc -l | tr -d ' ')

            if [ "${VERSION_COUNT}" -gt 3 ]; then
                # Keep latest 3, archive the rest
                ARCHIVE_COUNT=$((VERSION_COUNT - 3))
                log "  ${sig}: ${VERSION_COUNT} versions, archiving ${ARCHIVE_COUNT} old versions"

                # Find versions to archive (excluding latest 3)
                find "${SIG_DIR}" -maxdepth 1 -type d -name 'v*' -printf '%T@ %p\n' | \
                    sort -n | \
                    head -n ${ARCHIVE_COUNT} | \
                    cut -d' ' -f2- | \
                    while read -r old_version; do
                        # Check if it's older than retention period
                        MTIME=$(stat -f "%m" "${old_version}" 2>/dev/null || stat -c "%Y" "${old_version}" 2>/dev/null || echo "0")
                        NOW=$(date +%s)
                        AGE_DAYS=$(( (NOW - MTIME) / 86400 ))

                        if [ ${AGE_DAYS} -gt ${DATASET_VERSION_RETENTION} ]; then
                            VERSION_NAME=$(basename "${old_version}")
                            log "    Archiving ${sig}/${VERSION_NAME} (${AGE_DAYS} days old)"

                            # Create archive
                            ARCHIVE_NAME="${SIG_DIR}/${VERSION_NAME}.tar.gz"
                            tar -czf "${ARCHIVE_NAME}" -C "${SIG_DIR}" "${VERSION_NAME}" 2>/dev/null && \
                                rm -rf "${old_version}"
                        fi
                    done
            else
                log "  ${sig}: ${VERSION_COUNT} versions (keeping all)"
            fi
        fi
    done
else
    log "  No training_data directory found"
fi

# 3. Rotate logs
log "Rotating logs (>${LOG_RETENTION} days)..."
if [ -d "${LOG_DIR}" ]; then
    BEFORE=$(du -sh "${LOG_DIR}" 2>/dev/null | cut -f1 || echo "0")

    # Compress old logs
    find "${LOG_DIR}" -type f -name "*.log" -mtime +7 -not -name "*.gz" -exec gzip {} \; 2>/dev/null || true

    # Delete very old logs
    find "${LOG_DIR}" -type f -name "*.log.gz" -mtime +${LOG_RETENTION} -delete 2>/dev/null || true
    find "${LOG_DIR}" -type f -name "*.log" -mtime +${LOG_RETENTION} -delete 2>/dev/null || true

    AFTER=$(du -sh "${LOG_DIR}" 2>/dev/null | cut -f1 || echo "0")
    log "  Logs: ${BEFORE} -> ${AFTER}"
else
    log "  No log directory found"
fi

# 4. Clean up old A/B experiment states
log "Cleaning up old A/B experiments (>${AB_EXPERIMENT_RETENTION} days)..."
if [ -d "${AB_EXPERIMENTS_DIR}" ]; then
    BEFORE=$(find "${AB_EXPERIMENTS_DIR}" -name "*.json" | wc -l | tr -d ' ')

    # Remove completed/failed experiments older than retention period
    find "${AB_EXPERIMENTS_DIR}" -name "*.json" -type f -mtime +${AB_EXPERIMENT_RETENTION} | \
        while read -r exp_file; do
            # Check if experiment is in terminal state
            if grep -q '"final_status": "completed"\|"final_status": "rolled_back"\|"final_status": "failed"' "${exp_file}" 2>/dev/null; then
                rm -f "${exp_file}"
                log "  Removed old experiment: $(basename "${exp_file}")"
            fi
        done

    AFTER=$(find "${AB_EXPERIMENTS_DIR}" -name "*.json" | wc -l | tr -d ' ')
    log "  A/B experiments: ${BEFORE} -> ${AFTER}"
else
    log "  No A/B experiments directory found"
fi

# 5. Summary
log "========================================="
log "Cleanup - COMPLETED"
log "========================================="

# Generate disk usage report
log "Disk usage summary:"
log "  Optimization runs: $(du -sh "${OUTPUT_DIR}" 2>/dev/null | cut -f1 || echo "N/A")"
log "  Training data: $(du -sh "training_data" 2>/dev/null | cut -f1 || echo "N/A")"
log "  Logs: $(du -sh "${LOG_DIR}" 2>/dev/null | cut -f1 || echo "N/A")"
log "  A/B experiments: $(du -sh "${AB_EXPERIMENTS_DIR}" 2>/dev/null | cut -f1 || echo "N/A")"
