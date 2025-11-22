#!/bin/bash
# Run all CI checks locally before pushing
# This saves CI minutes and catches issues early

set -e

echo "🔍 Running all CI checks locally..."
echo ""

# Color codes
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track failures
FAILED=0

# 1. Formatting check
echo "📝 [1/5] Checking code formatting..."
if cargo fmt --all -- --check; then
    echo -e "${GREEN}✓${NC} Formatting check passed"
else
    echo -e "${RED}✗${NC} Formatting check failed. Run: cargo fmt --all"
    FAILED=1
fi
echo ""

# 2. Cargo check
echo "🔨 [2/5] Running cargo check..."
if cargo check --workspace --quiet; then
    echo -e "${GREEN}✓${NC} Cargo check passed"
else
    echo -e "${RED}✗${NC} Cargo check failed"
    FAILED=1
fi
echo ""

# 3. Tests
echo "🧪 [3/5] Running tests..."
if cargo test --workspace --lib --quiet; then
    echo -e "${GREEN}✓${NC} Tests passed"
else
    echo -e "${RED}✗${NC} Tests failed"
    FAILED=1
fi
echo ""

# 4. Clippy
echo "📎 [4/5] Running clippy..."
if cargo clippy --workspace --all-targets --quiet -- -D warnings; then
    echo -e "${GREEN}✓${NC} Clippy passed"
else
    echo -e "${RED}✗${NC} Clippy failed"
    FAILED=1
fi
echo ""

# 5. All targets check
echo "🎯 [5/5] Checking all targets..."
if cargo check --all-targets --workspace --quiet; then
    echo -e "${GREEN}✓${NC} All targets check passed"
else
    echo -e "${RED}✗${NC} All targets check failed"
    FAILED=1
fi
echo ""

# Summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ ALL CI CHECKS PASSED!${NC}"
    echo "   Safe to push to GitHub."
    exit 0
else
    echo -e "${RED}❌ SOME CHECKS FAILED${NC}"
    echo "   Fix the issues before pushing."
    exit 1
fi

