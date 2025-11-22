#!/bin/bash
# Comprehensive test script for Windjammer language compiler
# Tests compilation, examples, and integration tests

set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Windjammer Compiler Test Suite${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Track results
PASSED=0
FAILED=0
FAILED_TESTS=()

# Test a command and track results
test_command() {
    local name="$1"
    local cmd="$2"
    
    echo -e "${YELLOW}Testing:${NC} $name"
    if eval "$cmd" > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} $name passed"
        ((PASSED++))
    else
        echo -e "${RED}✗${NC} $name failed"
        ((FAILED++))
        FAILED_TESTS+=("$name")
    fi
}

# ============================================================================
# Part 1: Build Windjammer Compiler
# ============================================================================

echo -e "${BLUE}Part 1: Building Windjammer Compiler${NC}"
echo "----------------------------------------"
test_command "Windjammer compiler build" "cargo build --release"
echo ""

# ============================================================================
# Part 2: Example Compilation (Rust examples only)
# ============================================================================

echo -e "${BLUE}Part 2: Rust Examples Compilation${NC}"
echo "----------------------------------------"

# Check Rust examples compile
for example in examples/*.rs; do
    if [ -f "$example" ]; then
        basename=$(basename "$example" .rs)
        test_command "Example: $basename" "cargo check --example $basename"
    fi
done
echo ""

# ============================================================================
# Part 3: Workspace Tests
# ============================================================================

echo -e "${BLUE}Part 3: Workspace Tests${NC}"
echo "----------------------------------------"

test_command "Lib tests" "cargo test --workspace --lib"
test_command "Benchmark tests (compile)" "cargo test --benches --no-run"
test_command "Compiler integration tests" "cargo test --test compiler_tests"
echo ""

# ============================================================================
# Part 4: Clippy Checks
# ============================================================================

echo -e "${BLUE}Part 4: Clippy Checks${NC}"
echo "----------------------------------------"

test_command "Windjammer clippy" "cargo clippy --package windjammer --lib --bins -- -D warnings"
test_command "Runtime clippy" "cargo clippy --package windjammer-runtime --lib -- -D warnings"
test_command "LSP clippy" "cargo clippy --package windjammer-lsp --lib -- -D warnings"
test_command "MCP clippy" "cargo clippy --package windjammer-mcp --lib -- -D warnings"
echo ""

# ============================================================================
# Summary
# ============================================================================

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Test Results${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo -e "${GREEN}Passed:${NC} $PASSED"
echo -e "${RED}Failed:${NC} $FAILED"
echo ""

if [ $FAILED -gt 0 ]; then
    echo -e "${RED}Failed tests:${NC}"
    for test in "${FAILED_TESTS[@]}"; do
        echo -e "  ${RED}✗${NC} $test"
    done
    echo ""
    exit 1
else
    echo -e "${GREEN}All tests passed! 🎉${NC}"
    echo ""
    exit 0
fi
