#!/bin/bash
# Comprehensive test script for all Windjammer examples
# Tests both native game examples and WASM UI examples

set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Windjammer Examples Test Suite${NC}"
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
    echo ""
}

# ============================================================================
# Part 1: Build Windjammer Compiler
# ============================================================================

echo -e "${BLUE}Part 1: Building Windjammer Compiler${NC}"
echo "----------------------------------------"
test_command "Windjammer compiler build" "cargo build --release"

# ============================================================================
# Part 2: Game Framework Examples (Native)
# ============================================================================

echo -e "${BLUE}Part 2: Game Framework Examples${NC}"
echo "----------------------------------------"

test_command "Physics simulation" "cargo run --example physics_test -p windjammer-game-framework"
test_command "Audio system" "cargo run --example audio_playback_test -p windjammer-game-framework --features audio"
test_command "Texture system" "cargo run --example texture_test -p windjammer-game-framework"

# Window/rendering tests require a display, so we just check if they compile
test_command "Window example (compile)" "cargo build --example window_test -p windjammer-game-framework"
test_command "Sprite example (compile)" "cargo build --example sprite_test -p windjammer-game-framework"
test_command "Game loop example (compile)" "cargo build --example game_loop_test -p windjammer-game-framework"
test_command "Rendering example (compile)" "cargo build --example rendering_test -p windjammer-game-framework"

# ============================================================================
# Part 3: UI Framework WASM Build
# ============================================================================

echo -e "${BLUE}Part 3: UI Framework WASM Build${NC}"
echo "----------------------------------------"

test_command "WASM UI framework build" "cd crates/windjammer-ui && wasm-pack build --target web && cd ../.."

# ============================================================================
# Part 4: Integration Tests
# ============================================================================

echo -e "${BLUE}Part 4: Integration Tests${NC}"
echo "----------------------------------------"

test_command "All workspace tests" "cargo test --workspace --quiet"

# ============================================================================
# Part 5: Clippy Checks
# ============================================================================

echo -e "${BLUE}Part 5: Clippy Checks${NC}"
echo "----------------------------------------"

test_command "Windjammer clippy" "cargo clippy --package windjammer -- -D warnings"
test_command "Runtime clippy" "cargo clippy --package windjammer-runtime -- -D warnings"
test_command "UI clippy" "cargo clippy --package windjammer-ui -- -D warnings"
test_command "Game framework clippy" "cargo clippy --package windjammer-game-framework -- -D warnings"
test_command "LSP clippy" "cargo clippy --package windjammer-lsp -- -D warnings"
test_command "MCP clippy" "cargo clippy --package windjammer-mcp -- -D warnings"

# ============================================================================
# Summary
# ============================================================================

echo ""
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
