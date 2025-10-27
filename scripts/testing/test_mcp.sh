#!/bin/bash
# Test script for MCP server JSON-RPC protocol

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Testing Mnemosyne MCP Server${NC}\n"

# Function to send request and get response
test_request() {
    local name="$1"
    local request="$2"

    echo -e "${GREEN}Test: $name${NC}"
    echo "Request: $request"
    echo "$request" | cargo run --quiet -- serve 2>&1 &
    local pid=$!
    sleep 1
    kill $pid 2>/dev/null
    echo ""
}

# Test 1: Initialize
test_request "Initialize" '{"jsonrpc":"2.0","method":"initialize","id":1}'

# Test 2: List tools
test_request "List Tools" '{"jsonrpc":"2.0","method":"tools/list","id":2}'

# Test 3: Call tool (recall)
test_request "Call Recall Tool" '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"mnemosyne.recall","arguments":{"query":"test"}},"id":3}'

echo -e "${BLUE}Tests complete${NC}"
