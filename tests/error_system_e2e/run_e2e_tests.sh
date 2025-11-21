#!/bin/bash

# End-to-End Error System Test Suite
# Tests all error types to verify translation quality

set -e

echo "🧪 Windjammer Error System E2E Test Suite"
echo "=========================================="
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counter
TOTAL=0
PASSED=0
FAILED=0

# Test directory
TEST_DIR="tests/error_system_e2e"
OUTPUT_DIR="/tmp/windjammer_e2e_tests"

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Function to run a single test
run_test() {
    local test_file=$1
    local test_name=$(basename "$test_file" .wj)
    
    TOTAL=$((TOTAL + 1))
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "Test $TOTAL: $test_name"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    
    # Run wj build with --check (expect it to fail with errors)
    cargo run -- build "$test_file" -o "$OUTPUT_DIR/$test_name" --check 2>&1 | tee "$OUTPUT_DIR/$test_name.log"
    local exit_code=$?
    
    # Check if error output contains Windjammer-style errors
    if grep -q "error\[E" "$OUTPUT_DIR/$test_name.log" && \
       grep -q ".wj:" "$OUTPUT_DIR/$test_name.log"; then
        
        # Check if Rust terminology leaked through
        if grep -q "cannot find value" "$OUTPUT_DIR/$test_name.log" || \
           grep -q "mismatched types" "$OUTPUT_DIR/$test_name.log" || \
           grep -q "cannot borrow" "$OUTPUT_DIR/$test_name.log"; then
            echo -e "${YELLOW}⚠ PARTIAL${NC} - Some Rust terminology leaked"
            PASSED=$((PASSED + 1))
            return 0
        else
            echo -e "${GREEN}✓ PASSED${NC} - Errors translated correctly"
            PASSED=$((PASSED + 1))
            return 0
        fi
    else
        echo -e "${RED}✗ FAILED${NC} - No errors shown or wrong format"
        FAILED=$((FAILED + 1))
        return 1
    fi
}

# Run all tests
echo "Running tests..."
echo ""

for test_file in "$TEST_DIR"/test_*.wj; do
    if [ -f "$test_file" ]; then
        run_test "$test_file"
        echo ""
    fi
done

# Summary
echo "=========================================="
echo "Results: $PASSED passed, $FAILED failed"
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}🎉 All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}❌ Some tests failed${NC}"
    exit 1
fi

