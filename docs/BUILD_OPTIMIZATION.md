# Build Optimization Guide

This document describes the build optimizations applied to Mnemosyne for faster compilation.

## Phase 1: Quick Wins (Implemented)

### Results
- **Baseline clean build**: 2m 58s
- **Optimized clean build**: 2m 46s (12s faster, ~7% improvement)
- **Incremental builds**: ~3-4s (excellent incremental performance)

### Optimizations Applied

#### 1. Disabled Debug Symbols in Dev Builds
**File**: `.cargo/config.toml`
```toml
[profile.dev]
debug = false  # Reduces build time by 10-20%
```

**Impact**: Faster compilation, smaller binaries
**Trade-off**: Less detailed debugging info (use `debug = "line-tables-only"` if needed)

#### 2. Optimized Tokio Features
**File**: `Cargo.toml`
```toml
# Before
tokio = { version = "1.35", features = ["full", "tracing"] }

# After
tokio = { version = "1.35", features = ["macros", "rt-multi-thread", "sync", "time", "net", "io-util", "tracing"] }
```

**Impact**: Reduces unnecessary feature compilation
**Features used**:
- `macros`: `#[tokio::main]`, `#[tokio::test]`
- `rt-multi-thread`: Multi-threaded runtime
- `sync`: RwLock, mpsc channels
- `time`: timeouts, sleep, intervals
- `net`: Network I/O
- `io-util`: I/O utilities
- `tracing`: Runtime tracing integration

#### 3. Configured sccache
**File**: `.cargo/config.toml`
```toml
[env]
RUSTC_WRAPPER = "sccache"
```

**Installation**:
```bash
cargo install sccache
```

**Impact**:
- First build: Same speed (cache warming)
- Subsequent clean builds: 2-3x faster
- Shared cache across projects
- 10 GiB cache by default

**Cache location**: `~/Library/Caches/Mozilla.sccache` (macOS)

### Configuration Files Modified
1. `.cargo/config.toml` - Build settings, sccache, profiles
2. `Cargo.toml` - Tokio features optimization

## Phase 2: Workspace Restructuring (Deferred)

Workspace restructuring would provide 30-50% improvement for incremental builds by:
- Splitting into 6 crates: core, services, orchestration, ics, bindings, main
- Enabling parallel compilation across crates
- Better incremental compilation boundaries
- Faster `cargo check` cycles

**Status**: Deferred due to complexity
- Requires resolving circular dependencies (WorkItem types in storage)
- Extensive import updates across ~100 files
- Risk of breaking tests and references

**Recommendation**: Consider for future major refactoring when:
- Build times become critical bottleneck
- Architecture naturally separates into distinct layers
- Time available for thorough testing and validation

## Quick Reference

### Development Workflow
```bash
# Fast type-checking (no codegen)
cargo check

# Build with optimizations applied
cargo build

# Incremental build after small changes
cargo build  # ~3-4s with our optimizations

# Clean build (force recompile)
cargo clean && cargo build  # ~2m 46s
```

### Viewing Build Timings
```bash
cargo build --timings
# Opens HTML report in browser
open target/cargo-timings/cargo-timing-*.html
```

### sccache Management
```bash
# View cache statistics
sccache --show-stats

# Clear cache
sccache --stop-server
rm -rf ~/Library/Caches/Mozilla.sccache
sccache --start-server
```

### Profile Variants

Our `.cargo/config.toml` defines:

- **`dev`** (default): Fast compilation, no optimization, no debug symbols
- **`release`**: Full optimization, LTO, single codegen unit (slow build, fast binary)
- **`fast-release`**: Thin LTO, parallel codegen, good optimization (faster build, still fast binary)

```bash
# Use fast-release for testing near-production performance
cargo build --profile fast-release
```

## Advanced Optimizations (Not Yet Implemented)

### 1. Linker Optimization
**macOS**: System linker is already fast, lld not needed
**Linux**: Consider `mold` or `lld` for 20-50% faster linking

### 2. Codegen Units
Increase for faster builds (reduces optimization):
```toml
[profile.dev]
codegen-units = 256  # Default is 256, reduces to 1 for release
```

### 3. Pre-built Dependencies
Consider vendoring or pre-building heavy dependencies like tree-sitter parsers.

### 4. Nightly Features (Experimental)
```bash
RUSTFLAGS="-Zthreads=8" cargo +nightly build
```

## Troubleshooting

### sccache Not Working
```bash
# Check if wrapper is configured
grep RUSTC_WRAPPER .cargo/config.toml

# Verify sccache is in PATH
which sccache

# Check sccache server status
sccache --show-stats
```

### Slow Incremental Builds
```bash
# Clear incremental cache and rebuild
rm -rf target/debug/incremental
cargo build
```

### Out of Disk Space
```bash
# sccache uses up to 10 GiB
sccache --show-stats

# Reduce cache size
export SCCACHE_CACHE_SIZE="5G"
```

## Measurement Methodology

### Baseline Measurement
```bash
# Clean build
cargo clean
time cargo build --timings

# Incremental build
touch src/main.rs
time cargo build
```

### Fair Comparison
- Same machine, no other processes
- Warm filesystem cache (run twice, measure second)
- Same Rust toolchain version
- Clear sccache between baseline and optimized runs for clean comparison

## References

- [Rust Performance Book - Build Configuration](https://nnethercote.github.io/perf-book/build-configuration.html)
- [rustc Dev Guide - Incremental Compilation](https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation-in-detail.html)
- [sccache Documentation](https://github.com/mozilla/sccache)
- [Cargo Book - Build Performance](https://doc.rust-lang.org/nightly/cargo/guide/build-performance.html)

---

**Last Updated**: 2025-11-01
**Rust Version**: 1.89.0
**Baseline**: 2m 58s → **Optimized**: 2m 46s (7% improvement)
