#!/usr/bin/env bash
# Memory diagnostics collection script
# Collects system-wide and mnemosyne-specific memory information

set -euo pipefail

TIMESTAMP=$(date +%Y%m%d_%H%M%S)
OUTPUT_DIR="${1:-./diagnostics-${TIMESTAMP}}"

mkdir -p "$OUTPUT_DIR"

echo "Collecting memory diagnostics to: $OUTPUT_DIR"

# System-wide memory info
echo "=== System Memory Info ===" | tee "$OUTPUT_DIR/system-memory.log"
if [[ "$(uname)" == "Darwin" ]]; then
    echo "Total Memory:" | tee -a "$OUTPUT_DIR/system-memory.log"
    sysctl -n hw.memsize | awk '{print $0 / 1048576 " MB"}' | tee -a "$OUTPUT_DIR/system-memory.log"

    echo -e "\nMemory Pressure:" | tee -a "$OUTPUT_DIR/system-memory.log"
    vm_stat | tee -a "$OUTPUT_DIR/system-memory.log"

    echo -e "\nActive Processes (by memory):" | tee -a "$OUTPUT_DIR/system-memory.log"
    ps aux | sort -rk 4 | head -20 | tee -a "$OUTPUT_DIR/system-memory.log"
else
    free -h | tee -a "$OUTPUT_DIR/system-memory.log"
    ps aux | sort -rk 4 | head -20 | tee -a "$OUTPUT_DIR/system-memory.log"
fi

# Mnemosyne-specific diagnostics
echo -e "\n=== Mnemosyne Processes ===" | tee "$OUTPUT_DIR/mnemosyne-processes.log"
if pgrep -f mnemosyne > /dev/null; then
    echo "Found mnemosyne processes:" | tee -a "$OUTPUT_DIR/mnemosyne-processes.log"

    for pid in $(pgrep -f mnemosyne); do
        echo -e "\n--- Process $pid ---" | tee -a "$OUTPUT_DIR/mnemosyne-processes.log"

        # Memory usage
        if [[ "$(uname)" == "Darwin" ]]; then
            ps -p "$pid" -o pid,ppid,rss,vsz,command | tee -a "$OUTPUT_DIR/mnemosyne-processes.log"

            # Open file descriptors
            echo -e "\nOpen file descriptors:" | tee -a "$OUTPUT_DIR/mnemosyne-processes.log"
            lsof -p "$pid" 2>/dev/null | wc -l | tee -a "$OUTPUT_DIR/mnemosyne-processes.log"

            # Detailed memory regions
            echo -e "\nMemory regions:" | tee -a "$OUTPUT_DIR/mnemosyne-processes.log"
            vmmap "$pid" 2>/dev/null | grep -E "^[A-Z]|TOTAL" | tee -a "$OUTPUT_DIR/mnemosyne-processes.log"
        else
            ps -p "$pid" -o pid,ppid,rss,vsz,command | tee -a "$OUTPUT_DIR/mnemosyne-processes.log"

            echo -e "\nOpen file descriptors:" | tee -a "$OUTPUT_DIR/mnemosyne-processes.log"
            ls -l /proc/"$pid"/fd 2>/dev/null | wc -l | tee -a "$OUTPUT_DIR/mnemosyne-processes.log"

            echo -e "\nMemory map:" | tee -a "$OUTPUT_DIR/mnemosyne-processes.log"
            cat /proc/"$pid"/smaps 2>/dev/null | grep -E "^Size|^Rss|^Pss" | tee -a "$OUTPUT_DIR/mnemosyne-processes.log"
        fi
    done
else
    echo "No mnemosyne processes found" | tee -a "$OUTPUT_DIR/mnemosyne-processes.log"
fi

# Check for crash logs
echo -e "\n=== Recent Crash Logs ===" | tee "$OUTPUT_DIR/crash-logs.log"
if [[ "$(uname)" == "Darwin" ]]; then
    if ls ~/Library/Logs/DiagnosticReports/mnemosyne* 1> /dev/null 2>&1; then
        echo "Found crash logs:" | tee -a "$OUTPUT_DIR/crash-logs.log"
        ls -lt ~/Library/Logs/DiagnosticReports/mnemosyne* | head -5 | tee -a "$OUTPUT_DIR/crash-logs.log"

        # Copy most recent crash log
        LATEST_CRASH=$(ls -t ~/Library/Logs/DiagnosticReports/mnemosyne* | head -1)
        if [[ -n "$LATEST_CRASH" ]]; then
            echo -e "\n=== Latest Crash Report ===" | tee -a "$OUTPUT_DIR/crash-logs.log"
            cat "$LATEST_CRASH" | tee -a "$OUTPUT_DIR/crash-logs.log"
        fi
    else
        echo "No crash logs found" | tee -a "$OUTPUT_DIR/crash-logs.log"
    fi
else
    # Linux - check dmesg for OOM killer
    echo "Checking dmesg for OOM events:" | tee -a "$OUTPUT_DIR/crash-logs.log"
    dmesg | grep -i "out of memory\|oom\|killed" | tail -20 | tee -a "$OUTPUT_DIR/crash-logs.log"
fi

# Database size
echo -e "\n=== Database Statistics ===" | tee "$OUTPUT_DIR/database-stats.log"
DB_PATH="${HOME}/.local/share/mnemosyne/mnemosyne.db"
if [[ -f "$DB_PATH" ]]; then
    echo "Database size:" | tee -a "$OUTPUT_DIR/database-stats.log"
    du -h "$DB_PATH" | tee -a "$OUTPUT_DIR/database-stats.log"

    echo -e "\nDatabase info:" | tee -a "$OUTPUT_DIR/database-stats.log"
    sqlite3 "$DB_PATH" "
        SELECT 'Memories:', COUNT(*) FROM memories;
        SELECT 'Memory Links:', COUNT(*) FROM memory_links;
        SELECT 'Events:', COUNT(*) FROM events;
        SELECT 'Work Items:', COUNT(*) FROM work_items;
    " | tee -a "$OUTPUT_DIR/database-stats.log"
else
    echo "Database not found at $DB_PATH" | tee -a "$OUTPUT_DIR/database-stats.log"
fi

# System limits
echo -e "\n=== System Limits ===" | tee "$OUTPUT_DIR/system-limits.log"
if [[ "$(uname)" == "Darwin" ]]; then
    echo "ulimit -a:" | tee -a "$OUTPUT_DIR/system-limits.log"
    ulimit -a | tee -a "$OUTPUT_DIR/system-limits.log"
else
    ulimit -a | tee -a "$OUTPUT_DIR/system-limits.log"
fi

echo -e "\n✓ Diagnostics collected to: $OUTPUT_DIR"
echo "Files created:"
ls -lh "$OUTPUT_DIR"
