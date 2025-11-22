#!/bin/bash
# Test script for Windjammer language compiler
# Tests compilation and integration tests

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
    local show_output="${3:-false}"
    
    echo -e "${YELLOW}Testing:${NC} $name"
    
    if [ "$show_output" = "true" ]; then
        if eval "$cmd"; then
        echo -e "${GREEN}✓${NC} $name passed"
            PASSED=$((PASSED + 1))
        else
            echo -e "${RED}✗${NC} $name failed"
            FAILED=$((FAILED + 1))
            FAILED_TESTS+=("$name")
        fi
    else
        if eval "$cmd" > /dev/null 2>&1; then
            echo -e "${GREEN}✓${NC} $name passed"
            PASSED=$((PASSED + 1))
    else
        echo -e "${RED}✗${NC} $name failed"
            FAILED=$((FAILED + 1))
        FAILED_TESTS+=("$name")
        fi
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
# Part 2: Workspace Tests
# ============================================================================

echo -e "${BLUE}Part 2: Workspace Tests${NC}"
echo "----------------------------------------"

test_command "Lib tests" "cargo test --workspace --lib --quiet"
test_command "Compiler integration tests" "cargo test --test compiler_tests --quiet"
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
