#!/bin/bash

# Compare TaskFlow API benchmarks: Rust vs Windjammer
#
# This script runs both implementations' benchmarks and produces
# a side-by-side comparison showing the performance impact of 
# Phases 1 & 2 optimizations.

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo ""
echo -e "${BLUE}🏁 TaskFlow API Performance Comparison${NC}"
echo -e "${BLUE}=======================================${NC}"
echo ""
echo "This script will:"
echo "  1. Run Rust baseline benchmarks (100% performance)"
echo "  2. Run Windjammer benchmarks (with Phase 1 & 2 optimizations)"
echo "  3. Compare results and calculate performance ratio"
echo ""
echo -e "${YELLOW}Target:${NC} ≥95% of Rust performance"
echo ""

# Get script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
TASKFLOW_DIR="$(dirname "$SCRIPT_DIR")"

# Step 1: Run Rust benchmarks
echo -e "${YELLOW}Step 1: Running Rust benchmarks${NC}"
echo "--------------------------------"
cd "$TASKFLOW_DIR/rust"

cargo bench --no-fail-fast 2>&1 | tee /tmp/rust_bench_output.txt

echo -e "${GREEN}✓${NC} Rust benchmarks complete"
echo ""

# Step 2: Run Windjammer benchmarks
echo -e "${YELLOW}Step 2: Running Windjammer benchmarks${NC}"
echo "--------------------------------------"
cd "$TASKFLOW_DIR/windjammer/benches"

./run_benchmarks.sh 2>&1 | tee /tmp/windjammer_bench_output.txt

echo -e "${GREEN}✓${NC} Windjammer benchmarks complete"
echo ""

# Step 3: Extract and compare results
echo -e "${YELLOW}Step 3: Comparing results${NC}"
echo "------------------------"

# Parse Rust benchmark results
echo "Extracting Rust baseline results..."
RUST_JSON_SER=$(grep -A 1 "JSON Serialization/RegisterRequest" /tmp/rust_bench_output.txt | grep "time:" | awk '{print $2}')
RUST_JSON_DES=$(grep -A 1 "JSON Deserialization/LoginRequest" /tmp/rust_bench_output.txt | grep "time:" | awk '{print $2}')
RUST_BCRYPT=$(grep -A 1 "bcrypt_hash" /tmp/rust_bench_output.txt | grep "time:" | awk '{print $2}')
RUST_JWT_GEN=$(grep -A 1 "JWT Operations/generate" /tmp/rust_bench_output.txt | grep "time:" | awk '{print $2}')
RUST_JWT_VER=$(grep -A 1 "JWT Operations/verify" /tmp/rust_bench_output.txt | grep "time:" | awk '{print $2}')

echo "Extracting Windjammer results..."
# TODO: Parse Windjammer results similarly

# Step 4: Generate comparison report
echo ""
echo -e "${BLUE}📊 Performance Comparison Report${NC}"
echo -e "${BLUE}================================${NC}"
echo ""

cat > /tmp/windjammer_benchmark_comparison.md << 'EOF'
# TaskFlow API Microbenchmark Comparison

## Rust vs Windjammer (with Phase 1 & 2 Optimizations)

Generated: $(date +"%Y-%m-%d %H:%M:%S")

### Test Environment
- CPU: $(sysctl -n machdep.cpu.brand_string)
- Platform: $(uname -s) $(uname -m)
- Rust version: $(rustc --version)
- Windjammer version: $(wj --version)

### Optimization Status
- ✅ Phase 1: Inline hints (#[inline] for hot paths and stdlib)
- ✅ Phase 2: Smart borrow insertion (eliminate unnecessary clones)
- ⏳ Phase 3: Struct mapping optimization (pending)
- ⏳ Phase 4: Advanced optimizations (pending)

### Results

| Benchmark | Rust (baseline) | Windjammer | Ratio | Status |
|-----------|-----------------|------------|-------|--------|
| JSON Serialization (RegisterRequest) | ${RUST_JSON_SER} ns | TBD | TBD | ⏳ |
| JSON Deserialization (LoginRequest) | ${RUST_JSON_DES} ns | TBD | TBD | ⏳ |
| Password Hashing (bcrypt) | ${RUST_BCRYPT} ms | TBD | TBD | ⏳ |
| JWT Generate | ${RUST_JWT_GEN} µs | TBD | TBD | ⏳ |
| JWT Verify | ${RUST_JWT_VER} µs | TBD | TBD | ⏳ |
| Query Building (simple) | TBD ns | TBD | TBD | ⏳ |
| Query Building (complex) | TBD ns | TBD | TBD | ⏳ |

### Performance Analysis

**Overall Performance:**
- Average ratio: TBD% of Rust baseline
- Target: ≥95%
- Status: ⏳ Measuring...

**Impact of Optimizations:**
- Phase 1 (Inline hints): Expected 2-5% improvement
- Phase 2 (Clone elimination): Expected 10-15% improvement
- Combined expected: 12-20% improvement

**Next Steps:**
1. Phase 3: Struct mapping optimization (FromRow support)
2. Phase 4: Advanced optimizations
3. HTTP load testing with wrk
4. Svelte visualization dashboard

### Detailed Results

See full Criterion HTML reports:
- Rust: examples/taskflow/rust/target/criterion/report/index.html
- Windjammer: examples/taskflow/windjammer/build/target/criterion/report/index.html

EOF

cat /tmp/windjammer_benchmark_comparison.md

# Save to file
cp /tmp/windjammer_benchmark_comparison.md "$TASKFLOW_DIR/BENCHMARK_COMPARISON.md"

echo ""
echo -e "${GREEN}✅ Comparison complete!${NC}"
echo ""
echo "Full report saved to:"
echo "  $TASKFLOW_DIR/BENCHMARK_COMPARISON.md"
echo ""
echo "Criterion HTML reports:"
echo "  Rust:       $TASKFLOW_DIR/rust/target/criterion/report/index.html"
echo "  Windjammer: $TASKFLOW_DIR/windjammer/build/target/criterion/report/index.html"
echo ""

