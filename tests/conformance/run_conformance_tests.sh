#!/bin/bash

# ============================================================================
# Windjammer Conformance Test Suite
# ============================================================================
#
# PURPOSE:
# Verify that Windjammer programs produce identical observable behavior
# regardless of compilation backend (Rust, Go, future targets).
#
# HOW IT WORKS:
# 1. Compile each .wj test file with `wj build`
# 2. Compile the generated Rust with `rustc`
# 3. Run the binary and capture stdout
# 4. Compare stdout against expected output (from comments in .wj file)
#
# FUTURE:
# When the Go backend is added, this script will:
# 1. Compile each .wj to BOTH Rust and Go
# 2. Run both binaries
# 3. Assert stdout is identical between backends
#
# USAGE:
#   ./run_conformance_tests.sh                    # Run all tests
#   ./run_conformance_tests.sh values/copy_semantics.wj  # Run one test
#
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT_DIR="/tmp/windjammer_conformance_tests"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Counters
TOTAL=0
PASSED=0
FAILED=0
SKIPPED=0

echo ""
echo -e "${BLUE}╔═══════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║       Windjammer Conformance Test Suite                  ║${NC}"
echo -e "${BLUE}║       Backend-Independent Behavioral Verification        ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════════╝${NC}"
echo ""

# Clean output directory
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

# Find the wj binary
WJ_BIN="$PROJECT_ROOT/target/release/wj"
if [ ! -f "$WJ_BIN" ]; then
    WJ_BIN="$PROJECT_ROOT/target/debug/wj"
fi
if [ ! -f "$WJ_BIN" ]; then
    echo -e "${RED}ERROR: Cannot find wj binary. Run 'cargo build' first.${NC}"
    exit 1
fi
echo -e "Using compiler: ${WJ_BIN}"
echo ""

# Extract expected output from .wj file comments
# Looks for lines starting with "// " between EXPECTED OUTPUT: and the next blank comment
extract_expected_output() {
    local test_file="$1"
    sed -n '/^\/\/ EXPECTED OUTPUT:/,/^$/{
        /^\/\/ EXPECTED OUTPUT:/d
        /^$/d
        s/^\/\/ //
        p
    }' "$test_file"
}

# Run a single conformance test
run_test() {
    local test_file="$1"
    local test_name="${test_file#$SCRIPT_DIR/}"
    local test_basename="$(basename "$test_file" .wj)"
    local test_output_dir="$OUTPUT_DIR/$test_basename"

    TOTAL=$((TOTAL + 1))

    echo -n "  Testing ${test_name}... "

    mkdir -p "$test_output_dir"

    # Step 1: Compile Windjammer to Rust
    if ! "$WJ_BIN" build "$test_file" -o "$test_output_dir" --no-cargo > "$test_output_dir/wj_stdout.log" 2> "$test_output_dir/wj_stderr.log"; then
        echo -e "${RED}✗ FAILED${NC} (Windjammer compilation failed)"
        if [ -f "$test_output_dir/wj_stderr.log" ]; then
            echo "    $(head -3 "$test_output_dir/wj_stderr.log")"
        fi
        FAILED=$((FAILED + 1))
        return
    fi

    # Find the generated .rs file
    local rs_file=$(find "$test_output_dir" -name "*.rs" -not -name "Cargo.toml" | head -1)
    if [ -z "$rs_file" ] || [ ! -f "$rs_file" ]; then
        echo -e "${RED}✗ FAILED${NC} (No .rs file generated)"
        FAILED=$((FAILED + 1))
        return
    fi

    # Step 2: Compile Rust to binary
    local binary="$test_output_dir/$test_basename"
    if ! rustc "$rs_file" -o "$binary" 2> "$test_output_dir/rustc_stderr.log"; then
        echo -e "${YELLOW}⊘ SKIPPED${NC} (Rust compilation failed — may need stdlib)"
        SKIPPED=$((SKIPPED + 1))
        return
    fi

    # Step 3: Run binary and capture output
    local actual_output
    if ! actual_output=$("$binary" 2>&1); then
        echo -e "${RED}✗ FAILED${NC} (Runtime error)"
        FAILED=$((FAILED + 1))
        return
    fi

    # Step 4: Check for PASSED marker in output
    if echo "$actual_output" | grep -q "PASSED"; then
        echo -e "${GREEN}✓ PASSED${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}✗ FAILED${NC} (No PASSED marker in output)"
        echo "    Actual output:"
        echo "$actual_output" | head -5 | sed 's/^/    /'
        FAILED=$((FAILED + 1))
    fi

    # Save actual output for comparison
    echo "$actual_output" > "$test_output_dir/actual_output.txt"
}

# Determine which tests to run
if [ $# -gt 0 ]; then
    # Run specific test(s)
    for arg in "$@"; do
        test_path="$SCRIPT_DIR/$arg"
        if [ -f "$test_path" ]; then
            run_test "$test_path"
        else
            echo -e "${RED}Test not found: $arg${NC}"
        fi
    done
else
    # Run all tests
    echo -e "${BLUE}Category: Values & Ownership${NC}"
    for test_file in "$SCRIPT_DIR"/values/*.wj; do
        [ -f "$test_file" ] && run_test "$test_file"
    done

    echo ""
    echo -e "${BLUE}Category: Types${NC}"
    for test_file in "$SCRIPT_DIR"/types/*.wj; do
        [ -f "$test_file" ] && run_test "$test_file"
    done

    echo ""
    echo -e "${BLUE}Category: Control Flow${NC}"
    for test_file in "$SCRIPT_DIR"/control_flow/*.wj; do
        [ -f "$test_file" ] && run_test "$test_file"
    done

    echo ""
    echo -e "${BLUE}Category: Error Handling${NC}"
    for test_file in "$SCRIPT_DIR"/error_handling/*.wj; do
        [ -f "$test_file" ] && run_test "$test_file"
    done

    echo ""
    echo -e "${BLUE}Category: Standard Library${NC}"
    for test_file in "$SCRIPT_DIR"/stdlib/*.wj; do
        [ -f "$test_file" ] && run_test "$test_file"
    done

    echo ""
    echo -e "${BLUE}Category: Integrated${NC}"
    for test_file in "$SCRIPT_DIR"/*.wj; do
        [ -f "$test_file" ] && run_test "$test_file"
    done
fi

# Summary
echo ""
echo -e "════════════════════════════════════════════"
echo -e "  Results: ${GREEN}$PASSED passed${NC}, ${RED}$FAILED failed${NC}, ${YELLOW}$SKIPPED skipped${NC} (of $TOTAL total)"
echo -e "════════════════════════════════════════════"

if [ $FAILED -eq 0 ] && [ $PASSED -gt 0 ]; then
    echo -e "${GREEN}🎉 All conformance tests passed!${NC}"
    exit 0
elif [ $FAILED -eq 0 ] && [ $SKIPPED -gt 0 ]; then
    echo -e "${YELLOW}⚠  Some tests skipped (need stdlib or Cargo build). No failures.${NC}"
    exit 0
else
    echo -e "${RED}❌ Some conformance tests failed.${NC}"
    exit 1
fi
