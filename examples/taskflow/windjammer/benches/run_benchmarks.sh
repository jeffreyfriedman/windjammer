#!/bin/bash

# Build and run TaskFlow API benchmarks (Windjammer version)
#
# This script transpiles the Windjammer benchmark code to Rust,
# then uses Criterion to run microbenchmarks and compare performance.

set -e

echo "🔧 Building TaskFlow API benchmarks (Windjammer version)"
echo "=========================================================="
echo ""

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

# Step 1: Transpile Windjammer code to Rust
echo -e "${YELLOW}Step 1: Transpiling Windjammer to Rust${NC}"
echo "---------------------------------------"

# Create build directory
mkdir -p build
cd build

# Transpile the benchmark file
wj build --output . ../benches/api_benchmarks.wj

echo -e "${GREEN}✓${NC} Transpilation complete"
echo ""

# Step 2: Create Cargo.toml for benchmarks
echo -e "${YELLOW}Step 2: Creating Cargo.toml${NC}"
echo "----------------------------"

cat > Cargo.toml << 'EOF'
[package]
name = "taskflow-api-wj"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
bcrypt = "0.15"
jsonwebtoken = "9.2"

[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "api_benchmarks"
harness = false
EOF

echo -e "${GREEN}✓${NC} Cargo.toml created"
echo ""

# Step 3: Run benchmarks with Criterion
echo -e "${YELLOW}Step 3: Running benchmarks${NC}"
echo "------------------------"

cargo bench

echo ""
echo -e "${GREEN}✅ Benchmarks complete!${NC}"
echo ""
echo "Results saved to:"
echo "  - target/criterion/report/index.html"
echo ""
echo "Open the HTML report with:"
echo "  open target/criterion/report/index.html"

