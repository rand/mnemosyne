# Mnemosyne Examples

This directory contains practical examples demonstrating how to use Mnemosyne.

## Directory Structure

```
examples/
├── basic-usage/          # Simple command-line usage
│   ├── store-memory.sh
│   ├── search-memories.sh
│   └── export-markdown.sh
├── workflows/            # Real-world workflow examples
│   ├── daily-standup.sh
│   └── bug-tracking.sh
└── mcp-integration/      # Claude Code integration examples
    ├── claude-code-setup.md
    └── slash-commands.md
```

## Prerequisites

All examples assume you have:
- Mnemosyne installed and in PATH
- API key configured (`mnemosyne config show-key` succeeds)
- Database initialized

If not, see [INSTALL.md](../INSTALL.md).

## Running Examples

All shell scripts are executable:

```bash
# Make executable (if needed)
chmod +x examples/basic-usage/*.sh

# Run any example
./examples/basic-usage/store-memory.sh
```

## Example Categories

### Basic Usage

Simple command-line operations:
- Store a memory with enrichment
- Search memories by query
- Export memories to Markdown

**Start here if new to Mnemosyne.**

### Workflows

Complete workflow demonstrations:
- Daily standup preparation
- Bug tracking and resolution
- Team knowledge sharing

**Use these as templates for your own workflows.**

### MCP Integration

Claude Code integration:
- Setup instructions
- Slash command reference
- Programmatic tool usage

**Read these to understand Claude Code integration.**

## Learning Path

1. **Start**: [basic-usage/store-memory.sh](basic-usage/store-memory.sh)
2. **Search**: [basic-usage/search-memories.sh](basic-usage/search-memories.sh)
3. **Export**: [basic-usage/export-markdown.sh](basic-usage/export-markdown.sh)
4. **Workflow**: [workflows/bug-tracking.sh](workflows/bug-tracking.sh)
5. **Integration**: [mcp-integration/claude-code-setup.md](mcp-integration/claude-code-setup.md)

## Contributing Examples

Have a useful example? Contributions welcome!

1. Create example script in appropriate directory
2. Add documentation at the top of the script
3. Test the example works
4. Submit a pull request

See [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

---

**Last Updated**: 2025-10-27
