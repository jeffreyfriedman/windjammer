#!/bin/bash
# Error System End-to-End Tests
# Tests that error translation and source mapping work correctly

set -e

echo "🧪 Running Error System End-to-End Tests"
echo "=========================================="
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

TESTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="/tmp/wj_error_tests"
PASSED=0
FAILED=0

# Clean output directory
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

# Function to run a single test
run_test() {
    local test_file=$1
    local expected_error=$2
    local test_name=$(basename "$test_file" .wj)
    
    echo "Testing: $test_name"
    echo "  File: $test_file"
    echo "  Expected: $expected_error"
    
    # Build and capture output
    local output=$(cargo run -- build "$test_file" -o "$OUTPUT_DIR/$test_name" --check 2>&1 || true)
    
    # Check if expected error message appears
    if echo "$output" | grep -q "$expected_error"; then
        echo -e "  ${GREEN}✓ PASS${NC}: Error message translated correctly"
        ((PASSED++))
    else
        echo -e "  ${RED}✗ FAIL${NC}: Expected error message not found"
        echo "  Output:"
        echo "$output" | head -20
        ((FAILED++))
    fi
    
    echo ""
}

# Run all tests
echo "Test 1: Type Mismatch"
run_test "$TESTS_DIR/test_type_mismatch.wj" "Type mismatch"

echo "Test 2: Function Not Found"
run_test "$TESTS_DIR/test_function_not_found.wj" "Function not found"

echo "Test 3: Ownership Error"
run_test "$TESTS_DIR/test_ownership.wj" "Ownership error"

echo "Test 4: Mutability Error"
run_test "$TESTS_DIR/test_mutability.wj" "Cannot modify"

echo "Test 5: Variable Not Found"
run_test "$TESTS_DIR/test_variable_not_found.wj" "Variable not found"

# Summary
echo "=========================================="
echo "Test Summary:"
echo -e "  ${GREEN}Passed: $PASSED${NC}"
echo -e "  ${RED}Failed: $FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}🎉 All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}❌ Some tests failed${NC}"
    exit 1
fi

