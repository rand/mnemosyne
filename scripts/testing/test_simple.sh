#!/bin/bash
# Simple test that sends one request and exits

echo '{"jsonrpc":"2.0","method":"initialize","id":1}' | cargo run --quiet -- serve
