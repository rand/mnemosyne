# Mnemosyne Makefile
#
# Convenient shortcuts for common development tasks

.PHONY: help build install test clean doctor check lint format

# Default target: show help
help:
	@echo "Mnemosyne Development Commands"
	@echo ""
	@echo "Build & Install:"
	@echo "  make build        Build release binary"
	@echo "  make install      Build and install with proper code signing"
	@echo ""
	@echo "Testing:"
	@echo "  make test         Run all tests"
	@echo "  make check        Run cargo check"
	@echo "  make doctor       Run mnemosyne doctor health check"
	@echo ""
	@echo "Code Quality:"
	@echo "  make lint         Run clippy linter"
	@echo "  make format       Format code with rustfmt"
	@echo ""
	@echo "Cleanup:"
	@echo "  make clean        Remove build artifacts"
	@echo ""

# Build release binary (suppresses warnings for clean output)
build:
	@RUSTFLAGS="-A warnings" cargo build --release 2>&1 | grep -v "^warning:" || true

# Build with warnings visible (for development)
build-verbose:
	cargo build --release

# Build and install with proper macOS code signing
install:
	@./scripts/build-and-install.sh

# Run all tests
test:
	cargo test --all

# Run cargo check (fast compile check)
check:
	cargo check --all

# Run mnemosyne doctor health check
doctor:
	@if [ -x "$$HOME/.cargo/bin/mnemosyne" ]; then \
		$$HOME/.cargo/bin/mnemosyne doctor; \
	elif [ -x "./target/release/mnemosyne" ]; then \
		./target/release/mnemosyne doctor; \
	else \
		echo "Error: mnemosyne binary not found. Run 'make install' first."; \
		exit 1; \
	fi

# Run clippy linter
lint:
	cargo clippy --all-targets --all-features

# Format code with rustfmt
format:
	cargo fmt --all

# Clean build artifacts
clean:
	cargo clean
	rm -rf target/
	@echo "Build artifacts cleaned"
