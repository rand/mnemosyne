# Contributing to Mnemosyne

Thank you for your interest in contributing to Mnemosyne! This document provides guidelines and instructions for contributing to the project.

## Table of Contents

1. [Code of Conduct](#code-of-conduct)
2. [Getting Started](#getting-started)
3. [Development Setup](#development-setup)
4. [Development Workflow](#development-workflow)
5. [Code Standards](#code-standards)
6. [Testing Guidelines](#testing-guidelines)
7. [Documentation](#documentation)
8. [Pull Request Process](#pull-request-process)
9. [Issue Guidelines](#issue-guidelines)
10. [Project Phases](#project-phases)

---

## Code of Conduct

### Our Pledge

We are committed to providing a welcoming and inclusive experience for everyone. We expect all contributors to:

- Use welcoming and inclusive language
- Be respectful of differing viewpoints and experiences
- Accept constructive criticism gracefully
- Focus on what is best for the project and community
- Show empathy towards other community members

### Our Standards

**Acceptable behavior**:
- Professional and respectful communication
- Constructive feedback and collaboration
- Recognition of others' contributions
- Focus on technical merit

**Unacceptable behavior**:
- Harassment or discriminatory language
- Personal attacks or trolling
- Publishing others' private information
- Other conduct inappropriate in a professional setting

---

## Getting Started

### Prerequisites

- **Rust 1.75+**: Install via [rustup](https://rustup.rs/)
- **SQLite 3.43+**: Usually pre-installed on macOS/Linux
- **Git**: For version control
- **Anthropic API Key**: For testing LLM features (optional for most development)

### Quick Start

1. **Fork the repository** on GitHub

2. **Clone your fork**:
   ```bash
   git clone https://github.com/your-username/mnemosyne.git
   cd mnemosyne
   ```

3. **Add upstream remote**:
   ```bash
   git remote add upstream https://github.com/rand/mnemosyne.git
   ```

4. **Build the project**:
   ```bash
   cargo build
   ```

5. **Run tests**:
   ```bash
   cargo test
   ```

6. **Set up API key** (optional):
   ```bash
   cargo run -- config set-key
   ```

---

## Development Setup

### Recommended Tools

- **IDE**: VS Code with rust-analyzer extension
- **Formatter**: rustfmt (included with Rust toolchain)
- **Linter**: clippy (included with Rust toolchain)
- **Debugger**: LLDB (macOS/Linux) or GDB (Linux)

### VS Code Configuration

`.vscode/settings.json`:
```json
{
  "rust-analyzer.checkOnSave.command": "clippy",
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  }
}
```

### Environment Variables

```bash
# Optional: Enable debug logging
export RUST_LOG=debug

# Optional: Use test API key
export ANTHROPIC_API_KEY=sk-ant-test-...

# Optional: Custom database path
export MNEMOSYNE_DB_PATH=./test_mnemosyne.db
```

---

## Development Workflow

### Branch Strategy

- `main`: Stable, production-ready code
- `feature/*`: New features (e.g., `feature/hybrid-search`)
- `fix/*`: Bug fixes (e.g., `fix/fts5-trigger`)
- `docs/*`: Documentation updates (e.g., `docs/architecture`)
- `refactor/*`: Code refactoring (e.g., `refactor/storage-layer`)

### Creating a Feature Branch

```bash
# Update your local main
git checkout main
git pull upstream main

# Create feature branch
git checkout -b feature/your-feature-name

# Make changes and commit
git add .
git commit -m "Add your feature description"

# Push to your fork
git push origin feature/your-feature-name
```

### Keeping Your Branch Updated

```bash
# Fetch upstream changes
git fetch upstream

# Rebase on upstream main
git rebase upstream/main

# Force push to your fork (if already pushed)
git push --force-with-lease origin feature/your-feature-name
```

---

## Code Standards

### Rust Style Guide

Follow the [Rust Style Guide](https://doc.rust-lang.org/1.0.0/style/README.html) and use `rustfmt`:

```bash
cargo fmt
```

### Naming Conventions

- **Types**: `PascalCase` (e.g., `MemoryNote`, `LlmService`)
- **Functions**: `snake_case` (e.g., `get_api_key`, `enrich_memory`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `MAX_TOKENS`, `DEFAULT_MODEL`)
- **Modules**: `snake_case` (e.g., `storage`, `mcp_server`)

### Error Handling

Always use `Result<T, E>` for fallible operations:

```rust
// Good
pub fn get_memory(&self, id: MemoryId) -> Result<MemoryNote> {
    self.storage.get(id)
}

// Bad
pub fn get_memory(&self, id: MemoryId) -> MemoryNote {
    self.storage.get(id).unwrap() // Don't panic!
}
```

### Documentation Comments

All public APIs must have documentation:

```rust
/// Retrieves a memory by its unique identifier.
///
/// # Arguments
///
/// * `id` - The unique identifier of the memory
///
/// # Returns
///
/// * `Ok(MemoryNote)` - The requested memory
/// * `Err(MnemosyneError::NotFound)` - If memory doesn't exist
///
/// # Examples
///
/// ```
/// let memory = storage.get_memory(id)?;
/// println!("Found: {}", memory.summary);
/// ```
pub fn get_memory(&self, id: MemoryId) -> Result<MemoryNote> {
    // ...
}
```

### Clippy

Fix all clippy warnings before submitting:

```bash
cargo clippy -- -D warnings
```

### Common Patterns

**Async Functions**:
```rust
pub async fn enrich_memory(&self, content: &str) -> Result<MemoryNote> {
    // Use .await, not blocking calls
    let response = self.call_api(content).await?;
    Ok(response)
}
```

**Error Propagation**:
```rust
// Use ? operator for clean error propagation
pub fn process(&self) -> Result<()> {
    let data = self.read_data()?;
    let processed = self.transform(data)?;
    self.write_data(processed)?;
    Ok(())
}
```

**Builder Pattern**:
```rust
let memory = MemoryNote::builder()
    .content("Decision to use Rust")
    .namespace(Namespace::Global)
    .importance(8)
    .build()?;
```

---

## Testing Guidelines

### Test Organization

```
tests/
├── unit/           # Unit tests (alongside source)
├── integration/    # Integration tests
└── fixtures/       # Test data
```

### Unit Tests

Place unit tests in the same file as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_creation() {
        let memory = MemoryNote::new("test content");
        assert_eq!(memory.content, "test content");
    }

    #[tokio::test]
    async fn test_async_operation() {
        let result = some_async_fn().await;
        assert!(result.is_ok());
    }
}
```

### Integration Tests

Place integration tests in `tests/`:

```rust
// tests/integration/storage_test.rs
use mnemosyne::*;

#[tokio::test]
async fn test_storage_roundtrip() {
    let storage = SqliteStorage::new(":memory:").await.unwrap();
    let memory = MemoryNote::new("test");

    storage.store(&memory).await.unwrap();
    let retrieved = storage.get(memory.id).await.unwrap();

    assert_eq!(retrieved.content, memory.content);
}
```

### Test Coverage

**Targets**:
- Critical paths: 90%+
- Business logic: 80%+
- Error handling: 70%+
- Overall: 70%+

**Check coverage**:
```bash
cargo tarpaulin --out Html
open tarpaulin-report.html
```

### Test Guidelines

1. **Test names should be descriptive**:
   ```rust
   #[test]
   fn test_api_key_env_var_takes_precedence_over_keychain() {
       // ...
   }
   ```

2. **Use fixtures for complex test data**:
   ```rust
   fn create_test_memory() -> MemoryNote {
       MemoryNote::builder()
           .content("Test content")
           .namespace(Namespace::Global)
           .build()
           .unwrap()
   }
   ```

3. **Clean up test resources**:
   ```rust
   #[tokio::test]
   async fn test_with_cleanup() {
       let db = ":memory:";
       let storage = SqliteStorage::new(db).await.unwrap();

       // Test logic...

       storage.close().await.unwrap(); // Cleanup
   }
   ```

4. **Use `#[ignore]` for tests requiring external resources**:
   ```rust
   #[tokio::test]
   #[ignore] // Requires ANTHROPIC_API_KEY
   async fn test_llm_enrichment() {
       // ...
   }
   ```

---

## Documentation

### Code Documentation

**Required**:
- All public APIs
- Complex algorithms
- Non-obvious behavior

**Format**:
```rust
/// Brief one-line description.
///
/// Longer description with more details about behavior,
/// edge cases, and usage patterns.
///
/// # Arguments
///
/// * `param` - Description
///
/// # Returns
///
/// Description of return value
///
/// # Errors
///
/// Description of error conditions
///
/// # Examples
///
/// ```
/// let result = function(arg)?;
/// ```
pub fn function(param: Type) -> Result<ReturnType> {
    // ...
}
```

### User Documentation

Update these files for user-facing changes:

- `README.md`: Overview and quick start
- `INSTALL.md`: Installation instructions
- `MCP_SERVER.md`: API documentation
- `ARCHITECTURE.md`: System design

### Architecture Decision Records (ADRs)

For significant design decisions, add ADRs to `docs/adr/`:

```markdown
# ADR-001: Use SQLite for Storage

## Status
Accepted

## Context
Need a reliable, fast storage backend...

## Decision
Use SQLite with FTS5...

## Consequences
Positive: ...
Negative: ...
```

---

## Pull Request Process

### Before Submitting

**Checklist**:
- [ ] Code follows style guidelines
- [ ] Tests pass: `cargo test`
- [ ] No clippy warnings: `cargo clippy`
- [ ] Code formatted: `cargo fmt`
- [ ] Documentation updated
- [ ] CHANGELOG.md updated (if applicable)
- [ ] Commit messages are descriptive

### PR Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix (non-breaking change fixing an issue)
- [ ] New feature (non-breaking change adding functionality)
- [ ] Breaking change (fix or feature causing existing functionality to break)
- [ ] Documentation update

## Testing
How was this tested?

## Checklist
- [ ] Tests pass locally
- [ ] Clippy passes
- [ ] Documentation updated
- [ ] CHANGELOG updated

## Related Issues
Fixes #123
```

### Review Process

1. **Automated checks** must pass (CI/CD)
2. **At least one reviewer** must approve
3. **All comments** must be resolved
4. **Branch must be up-to-date** with main

### Merging

- Maintainers will merge approved PRs
- Use "Squash and merge" for feature branches
- Use "Rebase and merge" for hotfixes

---

## Issue Guidelines

### Before Creating an Issue

1. **Search existing issues** to avoid duplicates
2. **Check documentation** for answers
3. **Reproduce the bug** with minimal example
4. **Gather system information** (OS, Rust version, etc.)

### Issue Templates

#### Bug Report

```markdown
**Describe the bug**
Clear description of the issue

**To Reproduce**
Steps to reproduce:
1. ...
2. ...

**Expected behavior**
What should happen

**Actual behavior**
What actually happens

**Environment**
- OS: [e.g., macOS 14.0]
- Rust: [e.g., 1.75.0]
- Mnemosyne: [e.g., 0.1.0]

**Additional context**
Any other relevant information
```

#### Feature Request

```markdown
**Problem Statement**
What problem does this solve?

**Proposed Solution**
How should this work?

**Alternatives Considered**
What other approaches were considered?

**Additional Context**
Any other relevant information
```

### Issue Labels

- `bug`: Something isn't working
- `enhancement`: New feature or improvement
- `documentation`: Documentation updates
- `good first issue`: Good for newcomers
- `help wanted`: Extra attention needed
- `performance`: Performance improvements
- `security`: Security-related issues

---

## Project Phases

Mnemosyne is developed in 10 phases. Check [README.md](README.md) for current status.

### Current Focus Areas

**Phase 2 (Hybrid Search)**:
- Vector embeddings integration
- Hybrid ranking algorithm
- Performance optimization

**Phase 9 (Testing)**:
- Integration test suite
- E2E tests for MCP tools
- Performance benchmarks

**Phase 10 (Documentation)**:
- User guides and tutorials
- Video walkthroughs
- Example projects

### How to Contribute to Each Phase

**Phase 2 (Hybrid Search)**:
- Implement embedding generation
- Add vector similarity search
- Optimize hybrid ranking

**Phase 5 (Multi-Agent Integration)**:
- Create slash commands
- Develop hooks for session management
- Improve skills documentation

**Phase 6 (Agent Orchestration)**:
- Build agent-specific views
- Implement background evolution
- Add role-based access control

**Phase 8 (CLAUDE.md Integration)**:
- Document memory workflows
- Create decision trees
- Write integration guides

**Phase 9 (Testing)**:
- Write integration tests
- Create E2E test scenarios
- Develop benchmarks

---

## Getting Help

### Resources

- **Documentation**: [README.md](README.md), [ARCHITECTURE.md](ARCHITECTURE.md)
- **MCP API**: [MCP_SERVER.md](MCP_SERVER.md)
- **Installation**: [INSTALL.md](INSTALL.md)

### Communication

- **Issues**: For bugs and feature requests
- **Discussions**: For questions and ideas
- **Pull Requests**: For code contributions

### Maintainers

- **Lead**: @rand
- **Response Time**: Usually within 48 hours

---

## Recognition

Contributors will be acknowledged in:

- `CONTRIBUTORS.md` file
- Release notes
- Project README

Thank you for contributing to Mnemosyne!

---

**Version**: 0.1.0
**Last Updated**: 2024-10-26
