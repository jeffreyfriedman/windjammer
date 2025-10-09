#!/bin/bash

# TaskFlow API Load Testing Script
# Uses wrk to benchmark HTTP endpoints

set -e

RESULTS_DIR="results"
TIMESTAMP=$(date +%Y-%m-%d_%H-%M-%S)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}TaskFlow API Load Testing Suite${NC}"
echo "================================"
echo ""

# Check if wrk is installed
if ! command -v wrk &> /dev/null; then
    echo -e "${RED}Error: wrk is not installed${NC}"
    echo "Install with:"
    echo "  macOS: brew install wrk"
    echo "  Linux: sudo apt-get install wrk"
    exit 1
fi

# Create results directory
mkdir -p "$RESULTS_DIR/windjammer"
mkdir -p "$RESULTS_DIR/rust"

# Function to run a single benchmark
run_benchmark() {
    local name=$1
    local url=$2
    local method=$3
    local data=$4
    local duration=$5
    local connections=$6
    local threads=$7
    local output_file=$8
    
    echo -e "${YELLOW}Running: $name${NC}"
    echo "  URL: $url"
    echo "  Method: $method"
    echo "  Duration: ${duration}s"
    echo "  Connections: $connections"
    echo "  Threads: $threads"
    
    if [ -z "$data" ]; then
        wrk -t$threads -c$connections -d${duration}s \
            --latency \
            "$url" \
            > "$output_file"
    else
        wrk -t$threads -c$connections -d${duration}s \
            --latency \
            -s <(cat <<EOF
wrk.method = "$method"
wrk.headers["Content-Type"] = "application/json"
wrk.body = '$data'
EOF
) \
            "$url" \
            > "$output_file"
    fi
    
    echo -e "${GREEN}  ✓ Complete${NC}"
    echo ""
}

# Function to extract metrics from wrk output
extract_metrics() {
    local file=$1
    local rps=$(grep "Requests/sec:" "$file" | awk '{print $2}')
    local p50=$(grep "50%" "$file" | awk '{print $2}')
    local p99=$(grep "99%" "$file" | awk '{print $2}')
    
    echo "  RPS: $rps"
    echo "  p50: $p50"
    echo "  p99: $p99"
}

# Test function for an implementation
test_implementation() {
    local impl=$1
    local base_url=$2
    local results_subdir="$RESULTS_DIR/$impl"
    
    echo -e "${GREEN}Testing $impl implementation${NC}"
    echo "Base URL: $base_url"
    echo ""
    
    # Wait for server to be ready
    echo "Waiting for server to be ready..."
    for i in {1..30}; do
        if curl -s "$base_url/health" > /dev/null 2>&1; then
            echo -e "${GREEN}Server is ready!${NC}"
            break
        fi
        sleep 1
    done
    
    # Test 1: Health Check (warmup + baseline)
    run_benchmark \
        "Health Check" \
        "$base_url/health" \
        "GET" \
        "" \
        10 \
        100 \
        4 \
        "$results_subdir/health_${TIMESTAMP}.txt"
    
    # Test 2: User Registration
    run_benchmark \
        "User Registration" \
        "$base_url/api/v1/auth/register" \
        "POST" \
        '{"username":"bench_user","email":"bench@example.com","password":"password123"}' \
        30 \
        50 \
        4 \
        "$results_subdir/register_${TIMESTAMP}.txt"
    
    # Test 3: User Login
    run_benchmark \
        "User Login" \
        "$base_url/api/v1/auth/login" \
        "POST" \
        '{"username":"bench_user","password":"password123"}' \
        30 \
        100 \
        4 \
        "$results_subdir/login_${TIMESTAMP}.txt"
    
    # Test 4: List Projects (simulates auth'd user)
    run_benchmark \
        "List Projects" \
        "$base_url/api/v1/projects" \
        "GET" \
        "" \
        30 \
        100 \
        4 \
        "$results_subdir/projects_list_${TIMESTAMP}.txt"
    
    # Test 5: Create Project
    run_benchmark \
        "Create Project" \
        "$base_url/api/v1/projects" \
        "POST" \
        '{"name":"Benchmark Project","description":"Testing performance"}' \
        30 \
        50 \
        4 \
        "$results_subdir/projects_create_${TIMESTAMP}.txt"
    
    # Test 6: High Concurrency Test
    run_benchmark \
        "High Concurrency (Health)" \
        "$base_url/health" \
        "GET" \
        "" \
        60 \
        500 \
        8 \
        "$results_subdir/high_concurrency_${TIMESTAMP}.txt"
    
    echo -e "${GREEN}$impl testing complete!${NC}"
    echo ""
    
    # Print summary
    echo "=== $impl Summary ==="
    echo ""
    echo "Health Check:"
    extract_metrics "$results_subdir/health_${TIMESTAMP}.txt"
    echo ""
    echo "Registration:"
    extract_metrics "$results_subdir/register_${TIMESTAMP}.txt"
    echo ""
    echo "Login:"
    extract_metrics "$results_subdir/login_${TIMESTAMP}.txt"
    echo ""
    echo "High Concurrency:"
    extract_metrics "$results_subdir/high_concurrency_${TIMESTAMP}.txt"
    echo ""
}

# Main execution
echo "This script will benchmark both Windjammer and Rust implementations."
echo "Make sure both servers are running:"
echo "  - Windjammer: http://localhost:3000"
echo "  - Rust: http://localhost:3001"
echo ""
read -p "Press Enter to continue or Ctrl+C to cancel..."

# Test Windjammer
test_implementation "windjammer" "http://localhost:3000"

# Test Rust
test_implementation "rust" "http://localhost:3001"

echo -e "${GREEN}All benchmarks complete!${NC}"
echo "Results saved to: $RESULTS_DIR/"
echo ""
echo "To compare results, run: ./compare_results.sh"

