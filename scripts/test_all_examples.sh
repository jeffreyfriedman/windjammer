#!/bin/bash
# Test all Windjammer examples to ensure no regressions
# This script compiles every .wj file in the project

set -e  # Exit on first error

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Counters
TOTAL=0
PASSED=0
FAILED=0
SKIPPED=0

# Arrays to track results
declare -a FAILED_FILES
declare -a SKIPPED_FILES

echo "========================================="
echo "Testing All Windjammer Examples"
echo "========================================="
echo ""

# Build windjammer first
echo "Building windjammer compiler..."
cargo build --release --quiet
if [ $? -ne 0 ]; then
    echo -e "${RED}✗ Failed to build windjammer compiler${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Compiler built successfully${NC}"
echo ""

# Find all .wj files
WJ_FILES=$(find . -name "*.wj" -type f | grep -v "/build/" | grep -v "/target/" | grep -v "/pkg/" | sort)

# Files to skip (known issues, WIP, etc.)
SKIP_PATTERNS=(
    # Add patterns here if needed, e.g.:
    # "./examples/wip/"
    # "./test_"
)

should_skip() {
    local file=$1
    for pattern in "${SKIP_PATTERNS[@]}"; do
        if [[ "$file" == *"$pattern"* ]]; then
            return 0
        fi
    done
    return 1
}

echo "Found $(echo "$WJ_FILES" | wc -l | tr -d ' ') .wj files to test"
echo ""

# Test each file
for file in $WJ_FILES; do
    TOTAL=$((TOTAL + 1))
    
    # Check if should skip
    if should_skip "$file"; then
        echo -e "${YELLOW}⊘ SKIP${NC} $file"
        SKIPPED=$((SKIPPED + 1))
        SKIPPED_FILES+=("$file")
        continue
    fi
    
    # Try to compile
    if ./target/release/wj build "$file" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ PASS${NC} $file"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}✗ FAIL${NC} $file"
        FAILED=$((FAILED + 1))
        FAILED_FILES+=("$file")
    fi
done

echo ""
echo "========================================="
echo "Test Summary"
echo "========================================="
echo "Total:   $TOTAL"
echo -e "${GREEN}Passed:  $PASSED${NC}"
if [ $FAILED -gt 0 ]; then
    echo -e "${RED}Failed:  $FAILED${NC}"
fi
if [ $SKIPPED -gt 0 ]; then
    echo -e "${YELLOW}Skipped: $SKIPPED${NC}"
fi
echo ""

# Show failed files
if [ $FAILED -gt 0 ]; then
    echo -e "${RED}Failed Files:${NC}"
    for file in "${FAILED_FILES[@]}"; do
        echo "  - $file"
    done
    echo ""
fi

# Show skipped files
if [ $SKIPPED -gt 0 ]; then
    echo -e "${YELLOW}Skipped Files:${NC}"
    for file in "${SKIPPED_FILES[@]}"; do
        echo "  - $file"
    done
    echo ""
fi

# Exit with error if any tests failed
if [ $FAILED -gt 0 ]; then
    echo -e "${RED}❌ Some tests failed${NC}"
    exit 1
else
    echo -e "${GREEN}✅ All tests passed!${NC}"
    exit 0
fi

