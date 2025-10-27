#!/bin/bash
# Run comprehensive API-based tests

set -e

# Get API key from keychain
export ANTHROPIC_API_KEY=$(security find-generic-password -s "mnemosyne-memory-system" -a "anthropic-api-key" -w 2>/dev/null)

# Activate virtual environment
source .venv/bin/activate

echo "=========================================="
echo "Running Comprehensive Test Suite"
echo "=========================================="
echo ""

# Run Part 1: Work Plan Protocol
echo "Part 1: Work Plan Protocol Tests"
echo "-----------------------------------"
python -m pytest tests/orchestration/test_work_plan_protocol.py -v --tb=short

echo ""
echo "Part 2: Agent Coordination Tests"
echo "-----------------------------------"
python -m pytest tests/orchestration/test_agent_coordination.py -v --tb=short -m integration

echo ""
echo "Part 4: Anti-Pattern Tests"
echo "-----------------------------------"
python -m pytest tests/orchestration/test_anti_patterns.py -v --tb=short

echo ""
echo "=========================================="
echo "Test Suite Complete"
echo "=========================================="
