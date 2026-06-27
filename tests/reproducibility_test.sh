#!/bin/bash
# TDD: Test baseline error count reproducibility
#
# Problem: Error counts vary wildly between builds (17k → 4k → 20k)
# This test verifies if error counts are deterministic for the same source.
#
# Requires:
#   WJ_GAME_DIR — path to a Windjammer game project that uses `wj game build`
# Optional:
#   WJ_COMPILER — path to wj binary (defaults to repo target/release/wj)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
COMPILER="${WJ_COMPILER:-$REPO_ROOT/target/release/wj}"
GAME_DIR="${WJ_GAME_DIR:?Set WJ_GAME_DIR to a game project root}"

if [[ ! -x "$COMPILER" ]]; then
  echo "Compiler not found: $COMPILER" >&2
  exit 1
fi

echo "=== Reproducibility Test ==="
echo "Compiler: $COMPILER"
echo "Game dir: $GAME_DIR"
echo "Running game build 3 times to check error count stability"
echo ""

cd "$GAME_DIR"
git status --short | head -5 || true
echo ""

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
