# Pull Request

## Description

**What does this PR do?**

A clear and concise description of the changes.

## Type of Change

- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Code refactoring
- [ ] Test improvements
- [ ] Dependency updates

## Related Issues

**Closes:** #[issue number]
**Related:** #[issue number], #[issue number]

## Changes Made

**Summary of changes:**

- Change 1: ...
- Change 2: ...
- Change 3: ...

**Files changed:**

- `path/to/file.rs`: [what changed]
- `path/to/test.rs`: [tests added]
- `docs/file.md`: [docs updated]

## Testing

**How has this been tested?**

- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Manual testing performed
- [ ] Tested on multiple platforms

**Test coverage:**

```bash
# Run tests
cargo test

# Check coverage (if applicable)
cargo tarpaulin
```

**Manual testing steps:**

1. Step 1: ...
2. Step 2: ...
3. Expected result: ...
4. Actual result: ...

**Tested on:**

- [ ] macOS
- [ ] Linux (specify distro: ____________)
- [ ] Windows (if applicable)

## Breaking Changes

**Does this PR introduce breaking changes?**

- [ ] No breaking changes
- [ ] Yes (describe below)

**If yes, describe the breaking changes:**

- What breaks: ...
- Migration path: ...
- Deprecation warnings: ...

**Updated documentation:**

- [ ] CHANGELOG.md updated
- [ ] Migration guide updated (if needed)
- [ ] API documentation updated
- [ ] README.md updated (if needed)

## Performance Impact

**Does this change affect performance?**

- [ ] No performance impact
- [ ] Performance improvement (describe below)
- [ ] Potential performance regression (describe below)

**If applicable, include benchmarks:**

```
Before:
[benchmark results]

After:
[benchmark results]
```

## Documentation

**Documentation updated:**

- [ ] Code comments added/updated
- [ ] API documentation updated
- [ ] User documentation updated
- [ ] Examples added/updated
- [ ] CHANGELOG.md entry added

**Documentation changes:**

- File 1: ...
- File 2: ...

## Checklist

**Before submitting:**

- [ ] I've read [CONTRIBUTING.md](../CONTRIBUTING.md)
- [ ] My code follows the project's style guidelines
- [ ] I've run `cargo fmt` to format my code
- [ ] I've run `cargo clippy` and addressed warnings
- [ ] I've added tests that prove my fix/feature works
- [ ] All existing tests pass (`cargo test`)
- [ ] I've updated documentation as needed
- [ ] I've added an entry to CHANGELOG.md (if user-facing change)
- [ ] My commits have clear, descriptive messages
- [ ] I've tested this on my local environment

**For Rust changes:**

- [ ] No new `unsafe` code (or justified in comments)
- [ ] Error handling uses `Result<T, E>` appropriately
- [ ] No new `unwrap()` or `expect()` in production code (use proper error handling)
- [ ] Public APIs have doc comments
- [ ] Breaking changes are documented

**For Python bindings (if applicable):**

- [ ] PyO3 bindings updated
- [ ] Type hints added
- [ ] Python tests added/updated
- [ ] Python documentation updated

**For MCP integration (if applicable):**

- [ ] MCP tools tested with Claude Code
- [ ] Tool descriptions are clear
- [ ] Input validation added
- [ ] Error messages are helpful

## Additional Context

**Add any other context about the PR:**

- Design decisions: ...
- Alternative approaches considered: ...
- Known limitations: ...
- Future improvements: ...

**Screenshots (if applicable):**

[Add screenshots for UI changes or visual documentation]

---

**For Maintainers:**

**Review checklist:**
- [ ] Code quality and style
- [ ] Test coverage adequate
- [ ] Documentation complete
- [ ] Breaking changes properly communicated
- [ ] CHANGELOG.md updated
- [ ] No security concerns
- [ ] Performance impact acceptable
