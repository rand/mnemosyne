#!/bin/bash
# Comprehensive test runner for Mnemosyne
#
# Runs all tests including LLM tests if API key is available
# Usage:
#   ./test-all.sh              # Run all tests
#   ./test-all.sh --skip-llm   # Skip LLM tests even if API key available
#   ./test-all.sh --llm-only   # Run only LLM tests

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse arguments
SKIP_LLM=false
LLM_ONLY=false

for arg in "$@"; do
    case $arg in
        --skip-llm)
            SKIP_LLM=true
            ;;
        --llm-only)
            LLM_ONLY=true
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --skip-llm    Skip LLM tests even if API key is available"
            echo "  --llm-only    Run only LLM tests"
            echo "  --help        Show this help message"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $arg${NC}"
            echo "Run '$0 --help' for usage information"
            exit 1
            ;;
    esac
done

# Check if API key is available
check_api_key() {
    if [ -n "$ANTHROPIC_API_KEY" ]; then
        echo -e "${GREEN}✓${NC} ANTHROPIC_API_KEY found in environment"
        return 0
    fi

    # Check via mnemosyne config
    if cargo run -q -- config show-key >/dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} API key found via mnemosyne config"
        return 0
    fi

    echo -e "${YELLOW}✗${NC} No API key found"
    return 1
}

# Print section header
print_header() {
    echo ""
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
}

# Run unit tests
run_unit_tests() {
    print_header "Running Unit Tests"
    cargo test --lib
    echo -e "${GREEN}✓ Unit tests passed${NC}"
}

# Run integration tests (non-ignored)
run_integration_tests() {
    print_header "Running Integration Tests"
    cargo test --test '*'
    echo -e "${GREEN}✓ Integration tests passed${NC}"
}

# Run LLM tests (ignored tests)
run_llm_tests() {
    print_header "Running LLM Tests (requires API key)"

    if ! check_api_key; then
        echo -e "${YELLOW}Skipping LLM tests: No API key found${NC}"
        echo -e "Set API key with: ${BLUE}export ANTHROPIC_API_KEY=sk-ant-...${NC}"
        echo -e "Or run: ${BLUE}cargo run -- secrets set ANTHROPIC_API_KEY${NC}"
        return 0
    fi

    echo "Running ignored tests..."
    cargo test --lib -- --ignored
    cargo test --test llm_enrichment_test -- --ignored
    echo -e "${GREEN}✓ LLM tests passed${NC}"
}

# Main execution
main() {
    echo -e "${BLUE}Mnemosyne Test Suite${NC}"
    echo -e "Running in: ${PWD}"
    echo ""

    # Check if in correct directory
    if [ ! -f "Cargo.toml" ]; then
        echo -e "${RED}Error: Cargo.toml not found. Please run from project root.${NC}"
        exit 1
    fi

    if [ "$LLM_ONLY" = true ]; then
        run_llm_tests
    elif [ "$SKIP_LLM" = true ]; then
        run_unit_tests
        run_integration_tests
    else
        run_unit_tests
        run_integration_tests
        run_llm_tests
    fi

    echo ""
    print_header "Test Summary"
    echo -e "${GREEN}✓ All tests completed successfully${NC}"
    echo ""
}

# Run main
main

exit 0
