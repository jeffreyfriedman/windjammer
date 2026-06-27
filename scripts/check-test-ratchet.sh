#!/usr/bin/env bash
# Ratcheting test runner: fails only on NEW test failures.
# Known failures are tracked in .config/known-test-failures.txt
#
# Usage: ./scripts/check-test-ratchet.sh [cargo-test-args...]
#
# Examples:
#   ./scripts/check-test-ratchet.sh --release --test all
#   ./scripts/check-test-ratchet.sh --release --test all -- specific_test

set -o pipefail

KNOWN=".config/known-test-failures.txt"
ACTUAL_FILE=$(mktemp)
trap "rm -f $ACTUAL_FILE" EXIT

if [ ! -f "$KNOWN" ]; then
    echo "⚠️  No known-failures file at $KNOWN — running tests normally"
    exec cargo test "$@"
fi

echo "🔧 Running tests with ratcheting (known failures: $(wc -l < "$KNOWN" | tr -d ' '))"
echo ""

cargo test "$@" 2>&1 | tee /tmp/test-ratchet-output.txt
TEST_EXIT=${PIPESTATUS[0]}

if [ $TEST_EXIT -eq 0 ]; then
    echo ""
    echo "✅ All tests passed!"
    exit 0
fi

grep "^test .* \.\.\. FAILED$" /tmp/test-ratchet-output.txt | \
    sed 's/^test //' | sed 's/ \.\.\. FAILED$//' | \
    sort > "$ACTUAL_FILE"

ACTUAL_COUNT=$(wc -l < "$ACTUAL_FILE" | tr -d ' ')
if [ "$ACTUAL_COUNT" -eq 0 ]; then
    echo "❌ Tests failed but couldn't parse failure list"
    exit 1
fi

NEW_FAILURES=$(comm -23 "$ACTUAL_FILE" "$KNOWN")
FIXED_TESTS=$(comm -23 "$KNOWN" "$ACTUAL_FILE")

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 Ratchet Report"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "   Known failures: $(wc -l < "$KNOWN" | tr -d ' ')"
echo "   Actual failures: $ACTUAL_COUNT"

if [ -n "$FIXED_TESTS" ]; then
    FIXED_COUNT=$(echo "$FIXED_TESTS" | wc -l | tr -d ' ')
    echo ""
    echo "🎉 $FIXED_COUNT test(s) now PASSING — remove from known-test-failures.txt:"
    echo "$FIXED_TESTS" | sed 's/^/   ✓ /'
fi

if [ -n "$NEW_FAILURES" ]; then
    NEW_COUNT=$(echo "$NEW_FAILURES" | wc -l | tr -d ' ')
    echo ""
    echo "❌ $NEW_COUNT NEW failure(s) — these must be fixed:"
    echo "$NEW_FAILURES" | sed 's/^/   ✗ /'
    echo ""
    echo "To add to known failures (if intentional): append to $KNOWN"
    exit 1
fi

echo ""
echo "✅ No new failures introduced. Ratchet holds."
exit 0
