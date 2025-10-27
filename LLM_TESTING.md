# LLM Testing with API Keys

## Problem

The Anthropic Messages API (`https://api.anthropic.com/v1/messages`) requires regular API keys (`sk-ant-api03-...`) and does **not** accept OAuth tokens (`sk-ant-oat01-...`).

Claude Code stores OAuth credentials in the macOS keychain at:
- Service: `Claude Code-credentials`
- Format: `{"claudeAiOauth":{"accessToken":"sk-ant-oat01-..."}}`

However, these OAuth tokens cannot be used directly with the Messages API.

## Current Status

**Failing Tests (5)**:
- `test_enrich_memory_architecture_decision`
- `test_enrich_memory_bug_fix`
- `test_link_generation`
- `test_consolidation_decision_merge`
- `test_consolidation_decision_keep_both`

**Error**: `401 Unauthorized: {"type":"authentication_error","message":"invalid x-api-key"}`

## Solutions

### Option 1: Use Regular API Key (Recommended for Development)

Get a regular API key from https://console.anthropic.com/settings/keys and set it:

```bash
# Via environment variable
export ANTHROPIC_API_KEY=sk-ant-api03-...

# Or via mnemosyne CLI
./target/release/mnemosyne config set-key

# Or directly in keychain
security add-generic-password -U -s "mnemosyne-memory-system" -a "anthropic-api-key" -w "sk-ant-api03-..."
```

Then run tests:
```bash
cargo test --test llm_enrichment_test -- --ignored --test-threads=1
```

### Option 2: Mock LLM Responses (For CI/CD)

For automated testing without API keys, implement mock responses in test fixtures.

### Option 3: OAuth Proxy (Future Enhancement)

Create a proxy service that:
1. Accepts OAuth tokens from Claude Code
2. Exchanges them for API access tokens
3. Forwards requests to Messages API

This would require Anthropic OAuth API support or a custom authentication bridge.

## Current Workaround

The LLM integration tests are marked with `#[ignore]` and require manual API key configuration. They are not run in normal `cargo test` execution.

All other tests (55 total) pass without API keys:
- 30 library tests ✓
- 16 integration tests (hybrid search, namespace isolation) ✓
- 25 Python unit tests ✓

## Next Steps

1. Obtain regular API key for development/testing
2. Run LLM tests to verify enrichment, linking, and consolidation workflows
3. Consider implementing mock fixtures for CI/CD pipelines
