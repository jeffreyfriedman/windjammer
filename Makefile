.PHONY: test test-fast build clean fmt clippy check all

# Run all tests (single-threaded to avoid race conditions in LSP/MCP)
test:
	cargo test --workspace -- --test-threads=1

# Run tests with default parallelism (may deadlock in LSP/MCP)
test-fast:
	cargo test --workspace

# Build all crates in release mode
build:
	cargo build --workspace --release

# Clean build artifacts
clean:
	cargo clean

# Format code
fmt:
	cargo fmt --all

# Run clippy
clippy:
	cargo clippy --workspace --all-targets --all-features -- -D warnings

# Check without building
check:
	cargo check --workspace --all-targets --all-features

# Run all checks (fmt, clippy, test)
all: fmt clippy test
	@echo "✅ All checks passed!"

# Install pre-commit hooks
install-hooks:
	cp scripts/pre-commit .git/hooks/pre-commit
	chmod +x .git/hooks/pre-commit
	@echo "✅ Pre-commit hooks installed"

