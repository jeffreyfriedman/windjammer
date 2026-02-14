.PHONY: help test test-all test-stdlib test-ui test-game test-integration \
        test-quick test-verbose clippy fmt check build build-all \
        example-ui example-game example-http example-physics example-audio \
        test-examples clean doc setup-hooks check-versions

# Default target
help:
	@echo "Windjammer - Development Commands"
	@echo ""
	@echo "Setup:"
	@echo "  make setup-hooks       - Install git hooks (version checks)"
	@echo "  make check-versions    - Check version consistency across crates"
	@echo ""
	@echo "Testing:"
	@echo "  make test              - Run all tests"
	@echo "  make test-quick        - Quick test (stdlib smoke test only)"
	@echo "  make test-stdlib       - Test standard library"
	@echo "  make test-ui           - Test UI framework"
	@echo "  make test-game         - Test game engine"
	@echo "  make test-integration  - Integration tests only"
	@echo "  make test-verbose      - Run tests with output"
	@echo "  make test-examples     - Run all example tests"
	@echo ""
	@echo "Code Quality:"
	@echo "  make check             - Run all checks (fmt + clippy + test)"
	@echo "  make fmt               - Format code"
	@echo "  make clippy            - Run clippy linter"
	@echo ""
	@echo "Building:"
	@echo "  make build             - Build main compiler"
	@echo "  make build-all         - Build all crates"
	@echo "  make doc               - Generate documentation"
	@echo ""
	@echo "Examples:"
	@echo "  make example-ui        - Run UI counter demo"
	@echo "  make example-game      - Run game app test"
	@echo "  make example-http      - Run HTTP server test"
	@echo "  make example-physics   - Run physics test"
	@echo "  make example-audio     - Run audio test"
	@echo ""
	@echo "Cleanup:"
	@echo "  make clean             - Clean build artifacts"

# =============================================================================
# Testing
# =============================================================================

# Run all tests
test:
	@echo "🧪 Running all tests..."
	cargo test --all-features --workspace

# Run all tests with nocapture
test-verbose:
	@echo "🧪 Running all tests (verbose)..."
	cargo test --all-features --workspace -- --nocapture

# Quick smoke test
test-quick:
	@echo "🧪 Quick smoke test..."
	cargo test -p windjammer-runtime --test smoke_test

# Test standard library
test-stdlib:
	@echo "🧪 Testing standard library..."
	cargo test -p windjammer-runtime --all-features

# Test stdlib integration tests
test-stdlib-integration:
	@echo "🧪 Testing stdlib integration..."
	cargo test -p windjammer-runtime --test integration_tests

# Test stdlib smoke tests
test-stdlib-smoke:
	@echo "🧪 Testing stdlib smoke tests..."
	cargo test -p windjammer-runtime --test smoke_test

# Test individual stdlib modules
test-stdlib-http:
	@echo "🧪 Testing std::http..."
	cargo test -p windjammer-runtime http

test-stdlib-db:
	@echo "🧪 Testing std::db..."
	cargo test -p windjammer-runtime db

test-stdlib-json:
	@echo "🧪 Testing std::json..."
	cargo test -p windjammer-runtime json

test-stdlib-fs:
	@echo "🧪 Testing std::fs..."
	cargo test -p windjammer-runtime fs

# Test UI framework
test-ui:
	@echo "🧪 Testing UI framework..."
	cargo test -p windjammer-ui --all-features

# Test UI components
test-ui-vdom:
	@echo "🧪 Testing UI VDOM..."
	cargo test -p windjammer-ui vdom

test-ui-reactivity:
	@echo "🧪 Testing UI reactivity..."
	cargo test -p windjammer-ui reactivity

test-ui-renderer:
	@echo "🧪 Testing UI renderer..."
	cargo test -p windjammer-ui simple_renderer

# Test game engine
test-game:
	@echo "🧪 Testing game engine..."
	cargo test -p windjammer-game --all-features

# Test game components
test-game-ecs:
	@echo "🧪 Testing game ECS..."
	cargo test -p windjammer-game ecs

test-game-physics:
	@echo "🧪 Testing game physics..."
	cargo test -p windjammer-game physics --features 3d

test-game-rendering:
	@echo "🧪 Testing game rendering..."
	cargo test -p windjammer-game rendering

test-game-audio:
	@echo "🧪 Testing game audio..."
	cargo test -p windjammer-game audio --features audio

# Test compiler
test-compiler:
	@echo "🧪 Testing compiler..."
	cargo test -p windjammer

# Test LSP
test-lsp:
	@echo "🧪 Testing LSP..."
	cargo test -p windjammer-lsp

# Test MCP
test-mcp:
	@echo "🧪 Testing MCP..."
	cargo test -p windjammer-mcp

# Integration tests
test-integration:
	@echo "🧪 Running integration tests..."
	cargo test --test '*' --workspace

# =============================================================================
# Code Quality
# =============================================================================

# Run all quality checks
check: fmt clippy test
	@echo "✅ All checks passed!"

# Format code
fmt:
	@echo "📝 Formatting code..."
	cargo fmt --all

# Check formatting without modifying
fmt-check:
	@echo "📝 Checking code formatting..."
	cargo fmt --all -- --check

# Run clippy
clippy:
	@echo "📎 Running clippy..."
	cargo clippy --all-features --workspace -- -D warnings

# Run clippy on specific crate
clippy-compiler:
	@echo "📎 Clippy: windjammer..."
	cargo clippy -p windjammer --all-features -- -D warnings

clippy-lsp:
	@echo "📎 Clippy: windjammer-lsp..."
	cargo clippy -p windjammer-lsp --all-features -- -D warnings

clippy-mcp:
	@echo "📎 Clippy: windjammer-mcp..."
	cargo clippy -p windjammer-mcp --all-features -- -D warnings

clippy-ui:
	@echo "📎 Clippy: windjammer-ui..."
	cargo clippy -p windjammer-ui --all-features -- -D warnings

clippy-game:
	@echo "📎 Clippy: windjammer-game..."
	cargo clippy -p windjammer-game --all-features -- -D warnings

# =============================================================================
# Building
# =============================================================================

# Build main compiler
build:
	@echo "🔨 Building compiler..."
	cargo build --release

# Build all crates
build-all:
	@echo "🔨 Building all crates..."
	cargo build --all-features --workspace --release

# Build debug
build-debug:
	@echo "🔨 Building (debug)..."
	cargo build --all-features --workspace

# Build examples
build-examples:
	@echo "🔨 Building examples..."
	cargo build --examples --all-features --workspace

# Generate documentation
doc:
	@echo "📚 Generating documentation..."
	cargo doc --all-features --workspace --no-deps --open

# Generate documentation without opening
doc-quiet:
	@echo "📚 Generating documentation..."
	cargo doc --all-features --workspace --no-deps

# =============================================================================
# Examples
# =============================================================================

# Run all example tests
test-examples: example-ui example-physics example-audio example-game
	@echo "✅ All examples completed!"

# UI counter demo
example-ui:
	@echo "🎨 Running UI counter demo..."
	cargo run --example counter_test -p windjammer-ui

# Game app test
example-game:
	@echo "🎮 Running game app test..."
	cargo run --example game_app_test -p windjammer-game --features "3d,audio" 2>/dev/null || \
	cargo run --example game_app_test -p windjammer-game --features "3d"

# Physics test
example-physics:
	@echo "⚙️  Running physics test..."
	cargo run --example physics_test -p windjammer-game --features 3d

# Audio test  
example-audio:
	@echo "🔊 Running audio test..."
	cargo run --example audio_test -p windjammer-game --features audio 2>/dev/null || \
	echo "⚠️  Audio feature requires audio hardware"

# Rendering test
example-rendering:
	@echo "🎨 Running rendering test..."
	cargo run --example rendering_test -p windjammer-game

# Window test (desktop)
example-window:
	@echo "🪟 Running window test..."
	cargo run --example window_test -p windjammer-ui --features desktop

# HTTP server test (requires creating example first)
example-http:
	@echo "🌐 HTTP server example..."
	@echo "To test HTTP server:"
	@echo "  1. cargo run -- examples/serve_ui_wasm.wj"
	@echo "  2. curl http://127.0.0.1:8080/"

# =============================================================================
# Benchmarks
# =============================================================================

bench:
	@echo "⚡ Running benchmarks..."
	cargo bench --workspace

bench-compiler:
	@echo "⚡ Benchmarking compiler..."
	cargo bench -p windjammer

# =============================================================================
# Cleanup
# =============================================================================

# Clean build artifacts
clean:
	@echo "🧹 Cleaning build artifacts..."
	cargo clean

# Clean and rebuild
rebuild: clean build-all

# =============================================================================
# Development Workflow
# =============================================================================

# Pre-commit checks (mimics pre-commit hook)
pre-commit: fmt-check clippy test
	@echo "✅ Pre-commit checks passed!"

# Quick development cycle
dev: fmt clippy test-quick
	@echo "✅ Quick dev cycle complete!"

# Full CI simulation
ci: fmt-check clippy test build-all
	@echo "✅ CI checks complete!"

# Watch and test (requires cargo-watch)
watch:
	@echo "👀 Watching for changes..."
	cargo watch -x "test --all-features"

# Watch specific crate
watch-stdlib:
	cargo watch -x "test -p windjammer-runtime"

watch-ui:
	cargo watch -x "test -p windjammer-ui"

watch-game:
	cargo watch -x "test -p windjammer-game"

# =============================================================================
# Release
# =============================================================================

# Prepare release
release-check: check doc
	@echo "✅ Release ready!"
	@echo ""
	@echo "Next steps:"
	@echo "  1. Review CHANGELOG.md"
	@echo "  2. git commit -am 'chore: prepare release'"
	@echo "  3. git push origin feature-branch"
	@echo "  4. Create pull request"

# Show project stats
stats:
	@echo "📊 Project Statistics"
	@echo "====================="
	@echo ""
	@echo "Lines of code:"
	@find src crates -name '*.rs' | xargs wc -l | tail -1
	@echo ""
	@echo "Test count:"
	@cargo test --workspace --all-features -- --list 2>/dev/null | grep -c "test " || echo "Run 'make test' first"
	@echo ""
	@echo "Crates:"
	@ls -1 crates/ | wc -l
	@echo ""
	@echo "Dependencies:"
	@cargo tree --workspace --depth 1 | wc -l

# Show version info
version:
	@echo "Windjammer Version Info"
	@echo "======================="
	@grep '^version' Cargo.toml | head -1
	@echo ""
	@echo "Crate versions:"
	@for crate in windjammer-lsp windjammer-mcp windjammer-ui windjammer-game windjammer-runtime; do \
		echo -n "  $$crate: "; \
		grep '^version' crates/$$crate/Cargo.toml | head -1 | cut -d'"' -f2; \
	done

# =============================================================================
# Advanced Testing
# =============================================================================

# Test with specific features
test-wasm:
	@echo "🧪 Testing WASM target..."
	cargo test --target wasm32-unknown-unknown -p windjammer-ui

# Test with minimal features
test-minimal:
	@echo "🧪 Testing minimal features..."
	cargo test --no-default-features

# Memory leak detection (requires valgrind)
test-memory:
	@echo "🧪 Memory leak detection..."
	@which valgrind > /dev/null || (echo "❌ valgrind not installed" && exit 1)
	cargo build --tests
	valgrind --leak-check=full --show-leak-kinds=all \
		target/debug/deps/windjammer-* --test-threads=1

# =============================================================================
# Utilities
# =============================================================================

# Count TODOs in codebase
todos:
	@echo "📝 TODOs in codebase:"
	@grep -r "TODO" src crates --include="*.rs" | wc -l

# Find TODOs with context
todos-list:
	@echo "📝 TODO items:"
	@grep -rn "TODO" src crates --include="*.rs"

# Show recent changes
changes:
	@echo "📝 Recent commits:"
	@git log --oneline -10

# Show current branch
branch:
	@echo "Current branch: $$(git branch --show-current)"
	@echo "Status:"
	@git status -s

# =============================================================================
# Git Hooks & Version Management
# =============================================================================

# Install git hooks for version consistency checks
setup-hooks:
	@echo "🔧 Setting up git hooks..."
	@git config core.hooksPath .githooks
	@echo "✅ Git hooks installed (.githooks/)"
	@echo "   pre-commit: checks version consistency in staged Cargo.toml files"
	@echo "   pre-push:   validates tag versions match Cargo.toml before pushing"

# Check version consistency across all workspace crates
check-versions:
	@./scripts/check-versions.sh