# Mnemosyne Documentation

**Complete documentation index and navigation**

Welcome to the Mnemosyne documentation! This page organizes all documentation by topic and experience level.

---

## 🚀 Getting Started

**New to Mnemosyne? Start here:**

- **[Quick Start Guide](QUICK_START.md)** ⚡
  Get running in 5 minutes. Store and retrieve your first memory.
  **Time**: 5 minutes | **Audience**: Everyone

- **[Installation Guide](INSTALL.md)** 📦
  Detailed installation instructions, configuration options, and verification steps.
  **Time**: 15 minutes | **Audience**: Everyone

- **[Architecture Overview](ARCHITECTURE_OVERVIEW.md)** 🏗️
  How Mnemosyne works (user-friendly explanation).
  **Time**: 10 minutes | **Audience**: Users wanting to understand the system

---

## 📖 User Guides

**Practical guides for using Mnemosyne effectively:**

- **[Common Workflows](docs/guides/workflows.md)** 🔄
  Real-world usage patterns: daily development, debugging, team collaboration, refactoring.
  **Time**: 20 minutes | **Audience**: Daily users

- **[MCP API Reference](MCP_SERVER.md)** 📡
  Complete reference for all 8 MCP tools with examples.
  **Time**: 15 minutes | **Audience**: Power users, integrators

- **[Slash Commands](docs/guides/workflows.md#using-slash-commands)** ⌨️
  Quick reference for Claude Code slash commands.
  **Time**: 2 minutes | **Audience**: Claude Code users

---

## 🔧 Configuration

**Set up and customize Mnemosyne:**

- **[Secrets Management](SECRETS_MANAGEMENT.md)** 🔐
  Secure API key configuration: environment variables, age encryption, OS keychain.
  **Time**: 10 minutes | **Audience**: Everyone installing Mnemosyne

- **[Hooks Configuration](HOOKS_TESTING.md)** 🪝
  Automatic memory capture: session-start, pre-compact, post-commit hooks.
  **Time**: 15 minutes | **Audience**: Users wanting automatic capture

- **[Multi-Agent Orchestration](ORCHESTRATION.md)** 🤖
  PyO3 setup, 4-agent architecture, performance tuning.
  **Time**: 30 minutes | **Audience**: Advanced users, contributors

---

## 🛠️ Development

**Contributing to Mnemosyne:**

- **[Contributing Guide](CONTRIBUTING.md)** 🤝
  How to contribute: setup, workflow, standards, testing, pull requests.
  **Time**: 30 minutes | **Audience**: Contributors

- **[Architecture Deep Dive](ARCHITECTURE.md)** 🏛️
  Technical internals: storage layer, LLM service, MCP protocol, graph traversal.
  **Time**: 45 minutes | **Audience**: Developers, architects

- **[Testing Guide](docs/test-reports/integration-test-guide.md)** 🧪
  Test strategy: unit, integration, LLM, E2E tests.
  **Time**: 20 minutes | **Audience**: Contributors

- **[Roadmap](ROADMAP.md)** 🗺️
  Development phases (10/10 complete), v2.0 plans, timeline.
  **Time**: 15 minutes | **Audience**: Contributors, stakeholders

---

## 📚 Reference

**Technical references and specifications:**

- **[Changelog](CHANGELOG.md)** 📝
  Version history, release notes, breaking changes.
  **Time**: 5 minutes | **Audience**: Upgraders

- **[Audit Report](AUDIT_REPORT.md)** ✅
  v1.0 quality assessment: 12 issues found, recommendations.
  **Time**: 20 minutes | **Audience**: Project managers, QA

- **[Migration Guide](docs/guides/migration.md)** 🔄
  Upgrading between versions: v0.1→v1.0, v1.0→v1.1, v1.x→v2.0.
  **Time**: 10 minutes per version | **Audience**: Upgraders

---

## 🔍 Troubleshooting

**Solving common issues:**

- **[Troubleshooting Guide](TROUBLESHOOTING.md)** 🔧
  Comprehensive solutions for installation, runtime, and development issues.
  **Time**: 5-15 minutes (find your issue) | **Audience**: Everyone

**Quick Issue Lookup:**

| Issue | Solution |
|-------|----------|
| "mnemosyne: command not found" | [Installation Issues](TROUBLESHOOTING.md#installation-issues) |
| "No API key found" | [API Key Errors](TROUBLESHOOTING.md#api-key-errors) |
| "Database is locked" | [Database Errors](TROUBLESHOOTING.md#database-initialization-errors) |
| "MCP server failed to start" | [MCP Configuration](TROUBLESHOOTING.md#mcp-configuration-not-detected) |
| Build failures | [Build Failures](TROUBLESHOOTING.md#build-failures) |
| Test failures | [Test Failures](TROUBLESHOOTING.md#test-failures) |

---

## 🎯 By Use Case

### I want to...

#### ...get started quickly
1. [Quick Start Guide](QUICK_START.md) (5 min)
2. [Slash Commands](docs/guides/workflows.md#using-slash-commands) (2 min)

#### ...install for my team
1. [Installation Guide](INSTALL.md) (15 min)
2. [Secrets Management](SECRETS_MANAGEMENT.md) (10 min)
3. [Team Knowledge Sharing](docs/guides/workflows.md#team-knowledge-sharing) (15 min)

#### ...understand how it works
1. [Architecture Overview](ARCHITECTURE_OVERVIEW.md) (10 min)
2. [Architecture Deep Dive](ARCHITECTURE.md) (45 min - optional)

#### ...use it daily
1. [Common Workflows](docs/guides/workflows.md) (20 min)
2. [MCP API Reference](MCP_SERVER.md) (15 min - reference)

#### ...contribute to development
1. [Contributing Guide](CONTRIBUTING.md) (30 min)
2. [Architecture Deep Dive](ARCHITECTURE.md) (45 min)
3. [Testing Guide](docs/test-reports/integration-test-guide.md) (20 min)
4. [Roadmap](ROADMAP.md) (15 min)

#### ...upgrade versions
1. [Changelog](CHANGELOG.md) (5 min - check what's new)
2. [Migration Guide](docs/guides/migration.md) (10 min per version)

#### ...troubleshoot issues
1. [Troubleshooting Guide](TROUBLESHOOTING.md) (5-15 min)
2. [GitHub Issues](https://github.com/rand/mnemosyne/issues) (search existing)
3. [GitHub Discussions](https://github.com/rand/mnemosyne/discussions) (ask community)

#### ...integrate with CI/CD
1. [CI/CD Workflow](docs/guides/workflows.md#cicd-integration) (15 min)
2. [Secrets Management](SECRETS_MANAGEMENT.md#cicd-best-practices) (10 min)

#### ...set up automatic capture
1. [Hooks Configuration](HOOKS_TESTING.md) (15 min)
2. [Hooks in Workflows](docs/guides/workflows.md#daily-development-session) (5 min)

---

## 📋 Documentation by Type

### Tutorials (Step-by-Step)
- [Quick Start Guide](QUICK_START.md) - First memory in 5 minutes
- [Installation Guide](INSTALL.md) - Complete setup
- [Common Workflows](docs/guides/workflows.md) - Real-world patterns

### How-To Guides (Task-Oriented)
- [Troubleshooting Guide](TROUBLESHOOTING.md) - Solve specific problems
- [Migration Guide](docs/guides/migration.md) - Upgrade between versions
- [Secrets Management](SECRETS_MANAGEMENT.md) - Configure API keys
- [Hooks Configuration](HOOKS_TESTING.md) - Set up automatic capture

### Explanations (Understanding)
- [Architecture Overview](ARCHITECTURE_OVERVIEW.md) - How it works (user-friendly)
- [Architecture Deep Dive](ARCHITECTURE.md) - Technical internals
- [Multi-Agent Orchestration](ORCHESTRATION.md) - PyO3 and 4-agent system

### Reference (Look-Up)
- [MCP API Reference](MCP_SERVER.md) - Complete tool documentation
- [Changelog](CHANGELOG.md) - Version history
- [Audit Report](AUDIT_REPORT.md) - Quality assessment
- [Roadmap](ROADMAP.md) - Future plans

---

## 🎓 Learning Path

### Beginner

**Goal**: Store and retrieve your first memory

1. [Quick Start Guide](QUICK_START.md) ⚡ (5 min)
2. Try slash commands in Claude Code (5 min)
3. [Architecture Overview](ARCHITECTURE_OVERVIEW.md) 🏗️ (10 min - optional)

**Total Time**: 10-20 minutes

### Intermediate

**Goal**: Use Mnemosyne effectively in daily work

1. [Common Workflows](docs/guides/workflows.md) 🔄 (20 min)
2. [MCP API Reference](MCP_SERVER.md) 📡 (15 min)
3. [Hooks Configuration](HOOKS_TESTING.md) 🪝 (15 min)

**Total Time**: 50 minutes

### Advanced

**Goal**: Master Mnemosyne, contribute to project

1. [Architecture Deep Dive](ARCHITECTURE.md) 🏛️ (45 min)
2. [Multi-Agent Orchestration](ORCHESTRATION.md) 🤖 (30 min)
3. [Contributing Guide](CONTRIBUTING.md) 🤝 (30 min)
4. [Testing Guide](docs/test-reports/integration-test-guide.md) 🧪 (20 min)

**Total Time**: 2 hours

---

## 🔗 External Resources

### Mnemosyne Project

- **[GitHub Repository](https://github.com/rand/mnemosyne)** - Source code
- **[Issue Tracker](https://github.com/rand/mnemosyne/issues)** - Bug reports, feature requests
- **[Discussions](https://github.com/rand/mnemosyne/discussions)** - Community Q&A
- **[Releases](https://github.com/rand/mnemosyne/releases)** - Version downloads

### Related Projects

- **[Claude Code](https://claude.com/claude-code)** - AI coding assistant
- **[Model Context Protocol](https://modelcontextprotocol.io/)** - MCP specification
- **[LibSQL](https://github.com/libsql/libsql)** - Database engine
- **[Age Encryption](https://age-encryption.org/)** - Secure file encryption

### Technologies Used

- **[Rust](https://www.rust-lang.org/)** - Core language
- **[PyO3](https://pyo3.rs/)** - Rust↔Python bindings
- **[Claude 3.5 Haiku](https://www.anthropic.com/news/claude-3-5-haiku)** - LLM for enrichment
- **[Tokio](https://tokio.rs/)** - Async runtime

---

## 📊 Documentation Stats

| Category | Files | Total Lines | Status |
|----------|-------|-------------|--------|
| Getting Started | 3 | ~1,200 | ✅ Complete |
| User Guides | 3 | ~2,000 | ✅ Complete |
| Configuration | 3 | ~1,800 | ✅ Complete |
| Development | 4 | ~5,000 | ✅ Complete |
| Reference | 4 | ~3,000 | ✅ Complete |
| Troubleshooting | 1 | ~1,200 | ✅ Complete |
| **Total** | **18** | **~14,200** | ✅ **Complete** |

---

## 💬 Getting Help

Can't find what you need? Multiple ways to get help:

### Self-Service
1. **Search this documentation** (Cmd/Ctrl+F)
2. **[Troubleshooting Guide](TROUBLESHOOTING.md)** - Common issues
3. **[GitHub Issues](https://github.com/rand/mnemosyne/issues?q=)** - Search existing issues

### Community
4. **[GitHub Discussions](https://github.com/rand/mnemosyne/discussions)** - Ask the community
5. **Discord/Slack** (coming soon) - Real-time chat

### Direct Support
6. **[Email](mailto:rand.arete@gmail.com)** - Direct to maintainers (48h response time)
7. **[File an Issue](https://github.com/rand/mnemosyne/issues/new)** - Bug reports, feature requests

---

## ✍️ Contributing to Documentation

Documentation improvements are always welcome!

**How to contribute:**

1. **Found an error?** [File an issue](https://github.com/rand/mnemosyne/issues/new?template=bug_report.md)
2. **Want to improve?** [Submit a PR](CONTRIBUTING.md#pull-request-process)
3. **Missing documentation?** [Request it](https://github.com/rand/mnemosyne/discussions)

**Documentation guidelines:**
- Clear, concise language
- Examples for all concepts
- Test all commands before documenting
- Update related docs when making changes

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed contribution guidelines.

---

## 📅 Documentation Maintenance

| Document | Last Updated | Next Review | Status |
|----------|-------------|-------------|--------|
| README.md | 2025-10-27 | v1.2 | ✅ Current |
| QUICK_START.md | 2025-10-27 | v1.2 | ✅ Current |
| INSTALL.md | 2025-10-27 | v1.2 | ✅ Current |
| TROUBLESHOOTING.md | 2025-10-27 | v1.2 | ✅ Current |
| ARCHITECTURE.md | 2025-10-27 | v2.0 | ✅ Current |
| All others | 2025-10-27 | As needed | ✅ Current |

---

**Welcome to Mnemosyne!** 🧠✨

Start with the [Quick Start Guide](QUICK_START.md) and build from there.

---

**Last Updated**: 2025-10-27
**Version**: 1.0.0
**Documentation Coverage**: 100%
