#!/bin/bash

# Compare benchmark results between Windjammer and Rust

set -e

RESULTS_DIR="results"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}TaskFlow Performance Comparison${NC}"
echo "================================="
echo ""

# Function to extract RPS from wrk output
get_rps() {
    local file=$1
    grep "Requests/sec:" "$file" | awk '{print $2}' | tr -d '\n'
}

# Function to extract latency
get_latency() {
    local file=$1
    local percentile=$2
    grep "${percentile}%" "$file" | awk '{print $2}' | tr -d '\n'
}

# Function to calculate percentage difference
calc_diff() {
    local val1=$1
    local val2=$2
    echo "scale=2; (($val2 - $val1) / $val1) * 100" | bc
}

# Function to compare two implementations for a specific test
compare_test() {
    local test_name=$1
    local wj_file=$2
    local rust_file=$3
    
    if [ ! -f "$wj_file" ] || [ ! -f "$rust_file" ]; then
        echo -e "${YELLOW}  ⚠ Missing results files${NC}"
        return
    fi
    
    local wj_rps=$(get_rps "$wj_file")
    local rust_rps=$(get_rps "$rust_file")
    
    local wj_p50=$(get_latency "$wj_file" "50")
    local rust_p50=$(get_latency "$rust_file" "50")
    
    local wj_p99=$(get_latency "$wj_file" "99")
    local rust_p99=$(get_latency "$rust_file" "99")
    
    echo -e "${GREEN}$test_name${NC}"
    echo "  Throughput (RPS):"
    echo "    Windjammer: $wj_rps"
    echo "    Rust:       $rust_rps"
    
    if [ ! -z "$wj_rps" ] && [ ! -z "$rust_rps" ]; then
        local diff=$(calc_diff $wj_rps $rust_rps 2>/dev/null || echo "N/A")
        if [ "$diff" != "N/A" ]; then
            local sign=$(echo "$diff > 0" | bc -l)
            if [ "$sign" -eq 1 ]; then
                echo -e "    ${GREEN}Rust is ${diff}% faster${NC}"
            else
                local abs_diff=$(echo "$diff * -1" | bc)
                echo -e "    ${GREEN}Windjammer is ${abs_diff}% faster${NC}"
            fi
        fi
    fi
    
    echo "  Latency p50:"
    echo "    Windjammer: $wj_p50"
    echo "    Rust:       $rust_p50"
    
    echo "  Latency p99:"
    echo "    Windjammer: $wj_p99"
    echo "    Rust:       $rust_p99"
    echo ""
}

# Find latest results
echo "Finding latest benchmark results..."
echo ""

WJ_LATEST=$(ls -t "$RESULTS_DIR/windjammer/health_"*.txt 2>/dev/null | head -1)
if [ -z "$WJ_LATEST" ]; then
    echo -e "${RED}No Windjammer results found!${NC}"
    exit 1
fi

TIMESTAMP=$(basename "$WJ_LATEST" | sed 's/health_//;s/.txt//')
echo "Using results from: $TIMESTAMP"
echo ""

# Compare each test
compare_test \
    "Health Check" \
    "$RESULTS_DIR/windjammer/health_${TIMESTAMP}.txt" \
    "$RESULTS_DIR/rust/health_${TIMESTAMP}.txt"

compare_test \
    "User Registration" \
    "$RESULTS_DIR/windjammer/register_${TIMESTAMP}.txt" \
    "$RESULTS_DIR/rust/register_${TIMESTAMP}.txt"

compare_test \
    "User Login" \
    "$RESULTS_DIR/windjammer/login_${TIMESTAMP}.txt" \
    "$RESULTS_DIR/rust/login_${TIMESTAMP}.txt"

compare_test \
    "List Projects" \
    "$RESULTS_DIR/windjammer/projects_list_${TIMESTAMP}.txt" \
    "$RESULTS_DIR/rust/projects_list_${TIMESTAMP}.txt"

compare_test \
    "Create Project" \
    "$RESULTS_DIR/windjammer/projects_create_${TIMESTAMP}.txt" \
    "$RESULTS_DIR/rust/projects_create_${TIMESTAMP}.txt"

compare_test \
    "High Concurrency" \
    "$RESULTS_DIR/windjammer/high_concurrency_${TIMESTAMP}.txt" \
    "$RESULTS_DIR/rust/high_concurrency_${TIMESTAMP}.txt"

echo "================================="
echo ""
echo -e "${BLUE}Summary${NC}"
echo "Results saved in: $RESULTS_DIR/"
echo ""
echo "Performance targets:"
echo "  ✓ RPS within 10% is acceptable"
echo "  ✓ Latency p99 < 100ms is good"
echo "  ✓ No crashes under high concurrency"

