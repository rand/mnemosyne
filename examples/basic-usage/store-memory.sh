#!/usr/bin/env bash
#
# Example: Store a Memory
#
# This example demonstrates how to store a memory with automatic LLM enrichment.
# The LLM will generate a summary, extract keywords, classify the type, and create tags.
#
# Usage:
#   ./store-memory.sh

set -e

echo "📝 Storing a memory with LLM enrichment..."
echo ""

# Store a memory about an architecture decision
mnemosyne remember \
  --content "Decided to use Redis for session storage instead of in-memory sessions.

             Rationale:
             - Need session persistence across server restarts
             - Plan to scale horizontally with multiple app servers
             - Redis provides fast access (< 1ms) and automatic expiration

             Trade-offs:
             - Added dependency (Redis server required)
             - Slightly slower than in-memory (negligible in practice)

             Configuration:
             - TTL: 24 hours
             - Connection pool: 10 connections
             - Fallback: Reject requests if Redis unavailable" \
  --importance 8 \
  --namespace "global" \
  --format json

echo ""
echo "✅ Memory stored successfully!"
echo ""
echo "The LLM has automatically:"
echo "  - Generated a concise summary"
echo "  - Extracted relevant keywords"
echo "  - Classified the memory type (decision/pattern/bug/context)"
echo "  - Created searchable tags"
echo "  - Identified semantic links to related memories"
echo ""
echo "Try searching for it:"
echo "  mnemosyne recall --query \"Redis session\" --format json"
