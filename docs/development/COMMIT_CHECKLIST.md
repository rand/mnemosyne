# Pre-Commit Checklist

Use this checklist before every commit to ensure quality and consistency.

## Code Quality

- [ ] **Code compiles** without errors
  ```bash
  cargo build --workspace --all-features
  ```

- [ ] **Tests pass** (or are updated appropriately)
  ```bash
  # For new code, write tests first!
  cargo test --workspace --all-features
  ```

- [ ] **No new Clippy warnings**
  ```bash
  cargo clippy --all-features 2>&1 | grep "warning:"
  ```

- [ ] **Code formatted**
  ```bash
  cargo fmt --all
  ```

## Documentation

- [ ] **Public APIs documented** with doc comments
- [ ] **Examples updated** if behavior changed
- [ ] **README/guides updated** if user-facing changes
- [ ] **FEATURE_BRANCH_STATUS.md updated** if milestone reached

## Testing

- [ ] **Unit tests** for new functions/methods
- [ ] **Integration tests** for cross-module changes
- [ ] **Examples verified** if applicable
  ```bash
  cargo run --example specification_workflow
  ```

- [ ] **Edge cases covered** (empty inputs, errors, etc.)

## Git Hygiene

- [ ] **Commit message** follows template (.gitmessage)
- [ ] **Track prefix** correct: [DSPy], [SpecFlow], [Integration], [Infra]
- [ ] **Atomic commit** (one logical change)
- [ ] **No debug code** (println!, dbg!, etc. removed)
- [ ] **No commented-out code** (delete or document why)

## Beads Tracking

- [ ] **Beads issue updated** if linked
  ```bash
  bd update bd-XXX --status in_progress
  bd update bd-XXX --comment "Progress note"
  ```

- [ ] **Export Beads** after commit
  ```bash
  bd export -o .beads/issues.jsonl
  git add .beads/issues.jsonl
  ```

## Track-Specific Checks

### [DSPy] Commits

- [ ] **Python tests pass** (if applicable)
  ```bash
  pytest src/orchestration/dspy_modules/ -v
  ```

- [ ] **No regression** in DSPy module performance
- [ ] **Fallback mechanisms** maintained for non-Python builds

### [SpecFlow] Commits

- [ ] **Round-trip serialization** tested for artifacts
- [ ] **CLI commands** tested if modified
- [ ] **Example updated** if artifact structure changed

### [Integration] Commits

- [ ] **Both tracks tested** independently
- [ ] **Integration points verified**
- [ ] **No cross-contamination** of concerns

### [Infra] Commits

- [ ] **No breaking changes** to existing code
- [ ] **Documentation explains** infrastructure changes
- [ ] **Backward compatibility** maintained

## Quality Gates (Before Marking Task Complete)

- [ ] **All acceptance criteria met** (from Beads issue)
- [ ] **No known bugs** introduced
- [ ] **Performance acceptable** (no >10% regressions)
- [ ] **Code reviewed** (self-review using this checklist)

## Post-Commit Actions

- [ ] **Working tree clean**
  ```bash
  git status  # Should show "nothing to commit"
  ```

- [ ] **Beads exported** and committed
  ```bash
  git log -1 --oneline  # Verify commit message
  ```

- [ ] **Todo list updated** if phase/task complete
- [ ] **Update status docs** if milestone reached

---

## Quick Command Reference

```bash
# Full quality check (run before commit)
cargo build --workspace --all-features && \
cargo test --workspace --all-features && \
cargo clippy --all-features && \
cargo fmt --all -- --check

# Python quality check (DSPy track)
pytest src/orchestration/dspy_modules/ -v --tb=short

# Example verification
cargo run --example specification_workflow
cargo run --example semantic_highlighting

# Beads workflow
bd list --status in_progress --json
bd update bd-XX --status in_progress
bd export -o .beads/issues.jsonl
```

---

## When to Skip Checks

**Documentation-only commits**: Can skip compile/test checks
**Work-in-progress commits**: Mark with `[WIP]` prefix, skip quality gates
**Emergency fixes**: Document reason in commit message

---

## Enforcement

This is a **living document**. If a check becomes obsolete or too burdensome, update this file and commit the change.

**Principle**: Better to update the checklist than to skip checks without documentation.
