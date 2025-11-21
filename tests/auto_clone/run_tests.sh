#!/bin/bash

# Auto-Clone Test Suite Runner

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT_DIR="/tmp/windjammer_auto_clone_tests"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "🧪 Windjammer Auto-Clone Test Suite"
echo "===================================="
echo ""

# Clean output directory
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

# Test files
TESTS=(
    "test_simple_variables.wj"
    "test_field_access.wj"
    "test_method_calls.wj"
    "test_index_expressions.wj"
    "test_combined_patterns.wj"
)

PASSED=0
FAILED=0

for test in "${TESTS[@]}"; do
    echo -n "Testing $test... "
    
    TEST_OUTPUT="$OUTPUT_DIR/${test%.wj}"
    
    # Compile Windjammer to Rust
    if ! "$PROJECT_ROOT/target/debug/wj" build "$SCRIPT_DIR/$test" -o "$TEST_OUTPUT" > /dev/null 2>&1; then
        echo -e "${RED}✗ FAILED${NC} (Windjammer compilation failed)"
        FAILED=$((FAILED + 1))
        continue
    fi
    
    # Compile Rust
    cd "$TEST_OUTPUT"
    if ! cargo build --quiet 2>&1 | grep -v "warning:" > /dev/null; then
        echo -e "${RED}✗ FAILED${NC} (Rust compilation failed)"
        FAILED=$((FAILED + 1))
        continue
    fi
    
    # Run test
    if ! cargo run --quiet 2>&1 | grep -q "All.*tests passed"; then
        echo -e "${RED}✗ FAILED${NC} (Test execution failed)"
        FAILED=$((FAILED + 1))
        continue
    fi
    
    echo -e "${GREEN}✓ PASSED${NC}"
    PASSED=$((PASSED + 1))
done

echo ""
echo "===================================="
echo -e "Results: ${GREEN}$PASSED passed${NC}, ${RED}$FAILED failed${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}🎉 All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}❌ Some tests failed${NC}"
    exit 1
fi

