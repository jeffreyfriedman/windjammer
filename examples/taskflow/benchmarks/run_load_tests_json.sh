#!/bin/bash

# TaskFlow API Load Testing Script with JSON Output
# Enhanced version that produces machine-readable JSON for dashboard

set -e

RESULTS_DIR="results"
TIMESTAMP=$(date +%Y-%m-%d_%H-%M-%S)
JSON_OUTPUT="results/comparison_${TIMESTAMP}.json"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${GREEN}TaskFlow API Load Testing Suite (JSON Output)${NC}"
echo "=============================================="
echo ""

# Check if wrk is installed
if ! command -v wrk &> /dev/null; then
    echo -e "${RED}Error: wrk is not installed${NC}"
    echo "Install with:"
    echo "  macOS: brew install wrk"
    echo "  Linux: sudo apt-get install wrk"
    exit 1
fi

# Check if jq is installed (for JSON processing)
if ! command -v jq &> /dev/null; then
    echo -e "${YELLOW}Warning: jq is not installed (recommended for JSON processing)${NC}"
    echo "Install with:"
    echo "  macOS: brew install jq"
    echo "  Linux: sudo apt-get install jq"
fi

# Create results directory
mkdir -p "$RESULTS_DIR"

# Initialize JSON output
cat > "$JSON_OUTPUT" << 'EOF'
{
  "timestamp": "TIMESTAMP_PLACEHOLDER",
  "environment": {
    "os": "OS_PLACEHOLDER",
    "cpu": "CPU_PLACEHOLDER",
    "rust_version": "RUST_VERSION_PLACEHOLDER",
    "windjammer_version": "WINDJAMMER_VERSION_PLACEHOLDER"
  },
  "optimizations": {
    "phase1_inline": true,
    "phase2_clone_elimination": true,
    "phase3_struct_mapping": false,
    "phase4_advanced": false
  },
  "results": {
    "windjammer": {},
    "rust": {}
  }
}
EOF

# Replace placeholders
sed -i.bak "s/TIMESTAMP_PLACEHOLDER/$TIMESTAMP/" "$JSON_OUTPUT"
sed -i.bak "s/OS_PLACEHOLDER/$(uname -s)/" "$JSON_OUTPUT"
sed -i.bak "s/CPU_PLACEHOLDER/$(sysctl -n machdep.cpu.brand_string 2>/dev/null || echo 'Unknown')/" "$JSON_OUTPUT"
sed -i.bak "s/RUST_VERSION_PLACEHOLDER/$(rustc --version | awk '{print $2}')/" "$JSON_OUTPUT"
sed -i.bak "s/WINDJAMMER_VERSION_PLACEHOLDER/$(wj --version 2>/dev/null | awk '{print $2}' || echo '0.17.0')/" "$JSON_OUTPUT"
rm -f "${JSON_OUTPUT}.bak"

# Function to extract metrics and convert to JSON
extract_metrics_json() {
    local file=$1
    local test_name=$2
    
    local rps=$(grep "Requests/sec:" "$file" | awk '{print $2}' | tr -d ',' || echo "0")
    local transfer=$(grep "Transfer/sec:" "$file" | awk '{print $2}' | tr -d ',' || echo "0")
    local avg=$(grep "Latency" "$file" | head -1 | awk '{print $2}' || echo "0")
    local p50=$(grep "50%" "$file" | awk '{print $2}' || echo "0")
    local p75=$(grep "75%" "$file" | awk '{print $2}' || echo "0")
    local p90=$(grep "90%" "$file" | awk '{print $2}' || echo "0")
    local p99=$(grep "99%" "$file" | awk '{print $2}' || echo "0")
    local requests=$(grep "requests in" "$file" | awk '{print $1}' || echo "0")
    local duration=$(grep "requests in" "$file" | awk '{print $3}' | tr -d ',' || echo "0")
    
    cat << EOF
    "$test_name": {
      "requests_per_sec": $rps,
      "transfer_per_sec": "$transfer",
      "latency": {
        "avg": "$avg",
        "p50": "$p50",
        "p75": "$p75",
        "p90": "$p90",
        "p99": "$p99"
      },
      "total_requests": $requests,
      "duration": "$duration"
    }
EOF
}

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
    echo "  Duration: ${duration}s, Connections: $connections, Threads: $threads"
    
    if [ -z "$data" ]; then
        wrk -t$threads -c$connections -d${duration}s \
            --latency \
            "$url" \
            > "$output_file" 2>&1 || echo "Error running benchmark" > "$output_file"
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
            > "$output_file" 2>&1 || echo "Error running benchmark" > "$output_file"
    fi
    
    echo -e "${GREEN}  ✓ Complete${NC}"
}

# Test function for an implementation
test_implementation() {
    local impl=$1
    local base_url=$2
    
    echo ""
    echo -e "${GREEN}Testing $impl implementation${NC}"
    echo "Base URL: $base_url"
    echo ""
    
    # Wait for server to be ready
    echo "Waiting for server to be ready..."
    local ready=false
    for i in {1..30}; do
        if curl -s "$base_url/health" > /dev/null 2>&1; then
            echo -e "${GREEN}Server is ready!${NC}"
            ready=true
            break
        fi
        sleep 1
        echo -n "."
    done
    echo ""
    
    if [ "$ready" = false ]; then
        echo -e "${RED}Error: Server did not become ready${NC}"
        return 1
    fi
    
    # Test 1: Health Check
    run_benchmark \
        "Health Check" \
        "$base_url/health" \
        "GET" \
        "" \
        30 \
        100 \
        4 \
        "$RESULTS_DIR/${impl}_health.txt"
    
    # Test 2: High Concurrency
    run_benchmark \
        "High Concurrency" \
        "$base_url/health" \
        "GET" \
        "" \
        60 \
        500 \
        8 \
        "$RESULTS_DIR/${impl}_high_concurrency.txt"
    
    echo -e "${GREEN}$impl testing complete!${NC}"
}

# Main execution
echo "This script will benchmark both implementations and output JSON."
echo "Make sure both servers are running:"
echo "  - Windjammer: http://localhost:3000"
echo "  - Rust: http://localhost:3001"
echo ""

# Test Windjammer
test_implementation "windjammer" "http://localhost:3000"

# Test Rust
test_implementation "rust" "http://localhost:3001"

# Build JSON results
echo ""
echo -e "${BLUE}Building JSON output...${NC}"

# Extract metrics and update JSON
WJ_HEALTH_JSON=$(extract_metrics_json "$RESULTS_DIR/windjammer_health.txt" "health")
WJ_HIGH_JSON=$(extract_metrics_json "$RESULTS_DIR/windjammer_high_concurrency.txt" "high_concurrency")

RUST_HEALTH_JSON=$(extract_metrics_json "$RESULTS_DIR/rust_health.txt" "health")
RUST_HIGH_JSON=$(extract_metrics_json "$RESULTS_DIR/rust_high_concurrency.txt" "high_concurrency")

# Update JSON with jq (if available) or manual sed
if command -v jq &> /dev/null; then
    # Use jq for proper JSON manipulation
    jq ".results.windjammer = {$WJ_HEALTH_JSON,$WJ_HIGH_JSON}" "$JSON_OUTPUT" > "$JSON_OUTPUT.tmp" && mv "$JSON_OUTPUT.tmp" "$JSON_OUTPUT"
    jq ".results.rust = {$RUST_HEALTH_JSON,$RUST_HIGH_JSON}" "$JSON_OUTPUT" > "$JSON_OUTPUT.tmp" && mv "$JSON_OUTPUT.tmp" "$JSON_OUTPUT"
else
    # Fallback: manual JSON construction
    cat > "$JSON_OUTPUT" << EOF
{
  "timestamp": "$TIMESTAMP",
  "environment": {
    "os": "$(uname -s)",
    "cpu": "$(sysctl -n machdep.cpu.brand_string 2>/dev/null || echo 'Unknown')",
    "rust_version": "$(rustc --version | awk '{print $2}')",
    "windjammer_version": "$(wj --version 2>/dev/null | awk '{print $2}' || echo '0.17.0')"
  },
  "optimizations": {
    "phase1_inline": true,
    "phase2_clone_elimination": true,
    "phase3_struct_mapping": false,
    "phase4_advanced": false
  },
  "results": {
    "windjammer": {
      $WJ_HEALTH_JSON,
      $WJ_HIGH_JSON
    },
    "rust": {
      $RUST_HEALTH_JSON,
      $RUST_HIGH_JSON
    }
  }
}
EOF
fi

echo -e "${GREEN}✅ All benchmarks complete!${NC}"
echo ""
echo "Results:"
echo "  - JSON: $JSON_OUTPUT"
echo "  - Raw: $RESULTS_DIR/*_*.txt"
echo ""
echo "To view JSON:"
echo "  cat $JSON_OUTPUT | jq"
echo ""
echo "To serve for dashboard:"
echo "  cp $JSON_OUTPUT ../frontend/public/latest_results.json"

