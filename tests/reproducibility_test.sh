#!/bin/bash
# TDD: Test baseline error count reproducibility
# 
# Problem: Error counts vary wildly between builds (17k → 4k → 20k)
# This test verifies if error counts are deterministic for the same source

set -e

COMPILER="/Users/jeffreyfriedman/src/wj/windjammer/target/release/wj"
GAME_DIR="/Users/jeffreyfriedman/src/wj/windjammer-game"

echo "=== Reproducibility Test ==="
echo "Running game build 3 times to check error count stability"
echo ""

# Ensure clean state
cd "$GAME_DIR"
git status --short | head -5
echo ""

# Run build 3 times, count errors
echo "Build 1..."
ERROR_COUNT_1=$("$COMPILER" game build --release 2>&1 | grep "Int inference error" | wc -l || true)
echo "  Errors: $ERROR_COUNT_1"

echo "Build 2..."
ERROR_COUNT_2=$("$COMPILER" game build --release 2>&1 | grep "Int inference error" | wc -l || true)
echo "  Errors: $ERROR_COUNT_2"

echo "Build 3..."
ERROR_COUNT_3=$("$COMPILER" game build --release 2>&1 | grep "Int inference error" | wc -l || true)
echo "  Errors: $ERROR_COUNT_3"

echo ""
echo "=== Results ==="
echo "Build 1: $ERROR_COUNT_1 errors"
echo "Build 2: $ERROR_COUNT_2 errors"
echo "Build 3: $ERROR_COUNT_3 errors"

# Check if all three are equal
if [ "$ERROR_COUNT_1" -eq "$ERROR_COUNT_2" ] && [ "$ERROR_COUNT_2" -eq "$ERROR_COUNT_3" ]; then
    echo ""
    echo "✅ PASS: Error counts are reproducible ($ERROR_COUNT_1 errors)"
    exit 0
else
    echo ""
    echo "❌ FAIL: Error counts vary! Non-deterministic inference or build state issue."
    echo "Variance: $(( ERROR_COUNT_1 > ERROR_COUNT_2 ? ERROR_COUNT_1 - ERROR_COUNT_2 : ERROR_COUNT_2 - ERROR_COUNT_1 )) errors between runs"
    exit 1
fi
