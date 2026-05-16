#!/bin/bash

# Test all Windjammer examples and report results

TOTAL=0
PASSED=0
FAILED=0
FAILED_FILES=()

echo "Testing all Windjammer examples..."
echo "=================================="

# Create temp output directory
TEMP_OUT="/tmp/wj_test_output"
mkdir -p "$TEMP_OUT"

for file in $(find examples -name "*.wj" -type f | sort); do
    TOTAL=$((TOTAL + 1))
    
    # Get just the filename for output naming
    BASENAME=$(basename "$file" .wj)
    OUTPUT_DIR="$TEMP_OUT/${BASENAME}_$(date +%s)"
    
    # Detect if this is a component file (contains "view {" or "view{" as a standalone keyword)
    # Use word boundaries to avoid matching "preview", "overview", etc.
    if grep -qE '\bview\s*\{' "$file" 2>/dev/null; then
        TARGET="wasm"
    else
        TARGET="rust"
    fi
    
    # Try to build the file with appropriate target
    if wj build "$file" --output "$OUTPUT_DIR" --target "$TARGET" > /dev/null 2>&1; then
        PASSED=$((PASSED + 1))
        echo "✅ $file"
    else
        FAILED=$((FAILED + 1))
        FAILED_FILES+=("$file")
        echo "❌ $file"
    fi
done

echo ""
echo "=================================="
echo "Results: $PASSED/$TOTAL passed ($FAILED failed)"
if [ $TOTAL -gt 0 ]; then
    PERCENT=$(echo "scale=1; $PASSED * 100 / $TOTAL" | bc)
    echo "Pass rate: ${PERCENT}%"
fi

if [ $FAILED -gt 0 ] && [ $FAILED -le 10 ]; then
    echo ""
    echo "Failed files:"
    for file in "${FAILED_FILES[@]}"; do
        echo "  - $file"
    done
fi

# Cleanup
rm -rf "$TEMP_OUT"
