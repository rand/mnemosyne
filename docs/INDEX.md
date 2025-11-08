# Mnemosyne Documentation Index

**Last Updated**: 2025-11-08

Navigate the Mnemosyne documentation efficiently with this comprehensive index.

---

## 🚀 Getting Started

### For Users
- [**README**](../README.md) - Project overview, features, installation
- [**QUICK_START**](../QUICK_START.md) - Get up and running in 5 minutes
- [**INSTALL**](../INSTALL.md) - Detailed installation guide with troubleshooting

### For Developers
- [**AGENT_GUIDE**](../AGENT_GUIDE.md) - **START HERE** - Comprehensive guide for agents working in the project
- [**ARCHITECTURE**](../ARCHITECTURE.md) - System design and technical deep dive (1460 lines)
- [**CLAUDE**](../CLAUDE.md) - Development guidelines and Work Plan Protocol
- [**CONTRIBUTING**](../CONTRIBUTING.md) - How to contribute to Mnemosyne

---

## 📚 Technical References

### Core Documentation
- [**TYPES_REFERENCE**](TYPES_REFERENCE.md) - Complete type system reference
  - MemoryNote, Namespace, MemoryType, LinkType, WorkItem, etc.
  - JSON serialization formats
  - Type conversions and examples

- [**STORAGE_SCHEMA**](STORAGE_SCHEMA.md) - Database schema and query patterns
  - Tables, indexes, relationships
  - Migration history
  - Performance optimization
  - Troubleshooting common issues

### System Components
- [**MCP_SERVER**](../MCP_SERVER.md) - Model Context Protocol integration
  - 8 OODA-aligned tools
  - JSON-RPC protocol details
  - Tool usage examples

- [**ORCHESTRATION**](../ORCHESTRATION.md) - Multi-agent coordination system
  - 4 specialized agents (Orchestrator, Optimizer, Reviewer, Executor)
  - Work Plan Protocol phases
  - Quality gates and validation
  - Event persistence and SSE streaming

- [**EVALUATION**](../EVALUATION.md) - Privacy-preserving relevance scoring
  - Feature extraction
  - Feedback collection
  - Online learning

---

## 🎯 Feature Documentation

Located in [`features/`](features/) directory:

### Interactive Context Studio (ICS)
- [**ICS_README**](features/ICS_README.md) - Overview and capabilities
- [**ICS_ARCHITECTURE**](features/ICS_ARCHITECTURE.md) - Technical design
- [**ICS_KEYBOARD_SHORTCUTS**](features/ICS_KEYBOARD_SHORTCUTS.md) - Keybindings reference
- [**ics-integration-spec**](features/ics-integration-spec.md) - Integration specification

### Semantic Highlighting
- [**semantic_highlighting**](features/semantic_highlighting.md) - 3-tier system overview
- [**semantic-highlighter-test-plan**](features/semantic-highlighter-test-plan.md) - Testing strategy

### Vector Search & Evolution
- [**VECTOR_SEARCH**](features/VECTOR_SEARCH.md) - LibSQL vector embeddings
- [**EVOLUTION**](features/EVOLUTION.md) - Memory consolidation and decay
- [**EVOLUTION_ENHANCEMENTS**](features/EVOLUTION_ENHANCEMENTS.md) - Advanced features

### Privacy
- [**PRIVACY**](features/PRIVACY.md) - Privacy-preserving evaluation system

### RPC Server
- [**RPC**](features/RPC.md) - gRPC remote access to memory system
  - Full CRUD operations
  - Semantic search, graph traversal, hybrid recall
  - Streaming APIs for large datasets
  - Client examples (Python, Rust, Go)
  - Deployment configurations

---

## 📖 How-To Guides

Located in [`guides/`](guides/) directory:

- [**LLM Reviewer Guide**](guides/llm-reviewer.md) - Using the LLM-enhanced reviewer agent
- [**LLM Reviewer Setup**](guides/llm-reviewer-setup.md) - Configuration and troubleshooting
- [**RPC Getting Started**](guides/RPC_GETTING_STARTED.md) - Quick start guide for the gRPC server
- [**Migration Guide**](guides/migration.md) - Migrating from TUI to composable tools
- [**Workflows**](guides/workflows.md) - Common development workflows

---

## 📋 Specifications

Located in [`specs/`](specs/) directory:

### Architecture & Planning
- [**multi-agent-architecture**](specs/multi-agent-architecture.md) - Multi-agent system design
- [**project-plan**](specs/project-plan.md) - Project roadmap and milestones
- [**rust-implementation-spec**](specs/rust-implementation-spec.md) - Rust implementation details
- [**test-plan**](specs/test-plan.md) - Testing strategy and coverage targets

### Advanced Features
- [**background-processing-spec**](specs/background-processing-spec.md) - Tier 3 background processing
- [**incremental-analysis-spec**](specs/incremental-analysis-spec.md) - Incremental semantic analysis
- [**tier3-llm-integration-spec**](specs/tier3-llm-integration-spec.md) - LLM integration architecture

---

## 🔧 Build & Operations

- [**BUILD_OPTIMIZATION**](BUILD_OPTIMIZATION.md) - Build performance tuning
  - Phase 1: Quick wins (7% improvement)
  - sccache configuration
  - Tokio feature optimization
  - Future opportunities

- [**TROUBLESHOOTING**](../TROUBLESHOOTING.md) - Common issues and solutions
  - FTS trigger errors
  - Network warnings
  - Test failures
  - Database issues

- [**SECRETS_MANAGEMENT**](../SECRETS_MANAGEMENT.md) - API key management
  - Age-encrypted secrets
  - Three-tier key lookup
  - Cross-platform keychain support

---

## 📜 Historical Context

Located in [`historical/`](historical/) directory:

### Session Reports
- Archived session summaries, progress reports, and integration summaries
- Located in [`historical/session-reports/`](historical/session-reports/)

### Test Reports
- Historical test validation and namespace fix reports
- Located in [`historical/test-reports/`](historical/test-reports/)

### Planning Documents
- V2 planning documents, gap analysis, and option summaries
- Evolution planning and feature phasing

---

## 🗺️ Project Planning

- [**ROADMAP**](../ROADMAP.md) - Future plans and completed milestones
  - v1.0: Core memory system (COMPLETE)
  - v2.0: Vector search, RBAC, evolution (COMPLETE)
  - v2.1: ICS, semantic highlighting, composable tools (COMPLETE)
  - Future: Additional features and optimizations

- [**CHANGELOG**](../CHANGELOG.md) - Version history and changes
  - Detailed feature additions
  - Breaking changes
  - Performance improvements
  - Bug fixes

---

## 🌳 Coordination & Workflows

- [**BRANCH_ISOLATION**](BRANCH_ISOLATION.md) - Multi-agent branch coordination
- [**BRANCH_ISOLATION_TROUBLESHOOTING**](BRANCH_ISOLATION_TROUBLESHOOTING.md) - Conflict resolution
- [**COORDINATION_WORKFLOWS**](COORDINATION_WORKFLOWS.md) - Agent coordination patterns
- [**MIGRATION**](MIGRATION.md) - Migration from TUI wrapper to composable tools

---

## 🧪 Testing

- [**tests/e2e/README**](../tests/e2e/README.md) - E2E test suite documentation
  - Baseline vs. regression modes
  - Test categories and organization
  - Running and debugging tests

---

## 📦 Additional Resources

### Root Directory Documentation
- [**DOCUMENTATION**](../DOCUMENTATION.md) - Documentation standards
- [**CONTEXT_LOADING**](../CONTEXT_LOADING.md) - Context loading system
- [**TODO_ANALYSIS**](../TODO_ANALYSIS.md) - Technical debt tracking

### Historical Archive
- See [`historical/`](historical/) directory for:
  - Session summaries
  - Test reports
  - Decision logs
  - Status reports

---

## 🔍 Finding What You Need

### By Role

**New to Mnemosyne?**
→ [README](../README.md) → [QUICK_START](../QUICK_START.md) → [INSTALL](../INSTALL.md)

**Agent/Developer?**
→ [AGENT_GUIDE](../AGENT_GUIDE.md) → [ARCHITECTURE](../ARCHITECTURE.md) → [CLAUDE](../CLAUDE.md)

**Need API Reference?**
→ [TYPES_REFERENCE](TYPES_REFERENCE.md) → [STORAGE_SCHEMA](STORAGE_SCHEMA.md)

**Working on Features?**
→ [features/](features/) directory → relevant spec in [specs/](specs/)

**Troubleshooting?**
→ [TROUBLESHOOTING](../TROUBLESHOOTING.md) → [STORAGE_SCHEMA](STORAGE_SCHEMA.md) troubleshooting section

### By Topic

| Topic | Documents |
|-------|-----------|
| **Getting Started** | README, QUICK_START, INSTALL |
| **Development** | AGENT_GUIDE, CLAUDE, CONTRIBUTING |
| **Architecture** | ARCHITECTURE, ORCHESTRATION, MCP_SERVER |
| **Types & Schema** | TYPES_REFERENCE, STORAGE_SCHEMA |
| **Features** | features/ directory |
| **RPC Server** | features/RPC.md, guides/RPC_GETTING_STARTED.md, src/rpc/README.md |
| **Testing** | tests/e2e/README, specs/test-plan.md |
| **Build & Deploy** | BUILD_OPTIMIZATION, INSTALL |
| **Troubleshooting** | TROUBLESHOOTING, STORAGE_SCHEMA |
| **History** | CHANGELOG, ROADMAP, historical/ |

---

## 📝 Documentation Standards

When adding new documentation:

1. **Add to this index** - Keep INDEX.md up to date
2. **Use clear headings** - H1 for title, H2 for sections
3. **Include "Last Updated"** - Date at top of file
4. **Add See Also** - Link to related documents
5. **Use examples** - Code samples and JSON examples
6. **Update cross-references** - Ensure all links work

---

## 🔗 Quick Links

| Category | Link |
|----------|------|
| **Agent Guide** | [AGENT_GUIDE.md](../AGENT_GUIDE.md) |
| **Architecture** | [ARCHITECTURE.md](../ARCHITECTURE.md) |
| **Types** | [TYPES_REFERENCE.md](TYPES_REFERENCE.md) |
| **Schema** | [STORAGE_SCHEMA.md](STORAGE_SCHEMA.md) |
| **Features** | [features/](features/) |
| **Guides** | [guides/](guides/) |
| **Specs** | [specs/](specs/) |
| **Historical** | [historical/](historical/) |

---

**Questions?** Consult [AGENT_GUIDE.md](../AGENT_GUIDE.md) or [ARCHITECTURE.md](../ARCHITECTURE.md).
