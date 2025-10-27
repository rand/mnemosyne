---
name: Bug Report
about: Report a bug or unexpected behavior
title: '[BUG] '
labels: bug
assignees: ''
---

## Description

A clear and concise description of the bug.

## Environment

- **Mnemosyne Version**: [e.g., 1.0.0]
- **OS**: [e.g., macOS 14.0, Ubuntu 22.04]
- **Rust Version**: [e.g., 1.75.0]
- **Python Version** (if using Python bindings): [e.g., 3.11]
- **Installation Method**: [automated script / manual / cargo install]

## Steps to Reproduce

1. Run command `mnemosyne ...`
2. Expected behavior: ...
3. Actual behavior: ...

## Expected Behavior

What you expected to happen.

## Actual Behavior

What actually happened. Include error messages, stack traces, or unexpected output.

```
[Paste error messages or output here]
```

## Additional Context

### Configuration

```bash
# Output of:
mnemosyne --version
mnemosyne secrets get ANTHROPIC_API_KEY  # (redact the actual key!)
echo $MNEMOSYNE_DB_PATH
```

### Database State

```bash
# Output of:
ls -la ~/.local/share/mnemosyne/
# or your custom database path
```

### Logs

If available, include relevant log output:

```bash
# For debug logs:
RUST_LOG=debug mnemosyne [command] 2>&1 | tee /tmp/mnemosyne-debug.log
```

### MCP Integration (if applicable)

- **Claude Code Version**: [e.g., 1.2.0]
- **MCP Config**: [project-specific / global]

```json
// Contents of .claude/mcp_config.json (if relevant)
```

## Workaround

If you found a workaround, please describe it here so others can benefit.

## Possible Solution

If you have ideas about what might be causing the issue or how to fix it, please share.

---

**Before submitting:**
- [ ] I've checked [TROUBLESHOOTING.md](../../TROUBLESHOOTING.md)
- [ ] I've searched existing issues
- [ ] I've included all requested information
- [ ] I've redacted sensitive information (API keys, paths with usernames)
