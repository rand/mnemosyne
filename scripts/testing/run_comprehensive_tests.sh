#!/bin/bash
# Run comprehensive API-based tests

set -e

# Check if API key is available (uses secure system: env → age file → keychain)
# The mnemosyne binary will automatically find the key, but we verify it's accessible
if [ -z "$ANTHROPIC_API_KEY" ]; then
    # Check if mnemosyne config can access the key
    if ! cargo run -q -- config show-key >/dev/null 2>&1; then
        echo "Error: No API key found"
        echo "Set key with: mnemosyne config set-key"
        echo "Or: export ANTHROPIC_API_KEY=sk-ant-..."
        exit 1
    fi
    # Don't export the key - let mnemosyne access it securely
    echo "✓ API key accessible via mnemosyne secure system"
else
    echo "✓ ANTHROPIC_API_KEY found in environment"
fi

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
