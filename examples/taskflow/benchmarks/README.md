# TaskFlow Benchmarking Suite

## Overview

This directory contains comprehensive benchmarks comparing Windjammer and Rust implementations of the TaskFlow API.

## Benchmark Types

### 1. HTTP Load Tests (`load_tests/`)
- Measures throughput (RPS) and latency under load
- Uses `wrk` for HTTP benchmarking
- Tests various concurrency levels and durations

### 2. Microbenchmarks (`microbenchmarks/`)
- Uses Criterion for precise measurements
- Tests individual operations (DB queries, serialization, etc.)
- Provides statistical analysis and regression detection

### 3. Memory Profiling (`memory/`)
- Tracks heap allocations
- Measures peak memory usage
- Identifies memory leaks

### 4. CPU Profiling (`cpu/`)
- Flamegraphs for hot path identification
- CPU usage under sustained load

## Running Benchmarks

### Quick Start

```bash
# Run all benchmarks
./run_all_benchmarks.sh

# Run just HTTP load tests
./run_load_tests.sh

# Run microbenchmarks
cd ../rust && cargo bench

# Compare results
./compare_results.sh
```

### Prerequisites

```bash
# Install wrk (HTTP load testing)
# macOS:
brew install wrk

# Linux:
sudo apt-get install wrk

# Install flamegraph tools
cargo install flamegraph

# Install heaptrack (optional, for memory profiling)
# macOS:
brew install heaptrack

# Linux:
sudo apt-get install heaptrack
```

## Benchmark Scenarios

### 1. Authentication Flow
- Register user
- Login
- Get current user
- **Expected:** < 5ms p95 latency, > 10k RPS

### 2. Project CRUD
- Create project
- List projects
- Update project
- Delete project
- **Expected:** < 10ms p95 latency, > 5k RPS

### 3. Task CRUD
- Create task
- List tasks by project
- Update task
- Search tasks
- **Expected:** < 15ms p95 latency, > 3k RPS

### 4. Complex Workflows
- Create project → Add members → Create tasks → Assign tasks
- **Expected:** < 50ms end-to-end

## Continuous Integration

Benchmarks run automatically on:
- Every PR to main
- Nightly builds
- Tagged releases

Performance regressions > 5% trigger warnings.
Performance regressions > 10% fail the build.

## Results Format

Results are stored in `results/` directory:

```
results/
├── windjammer/
│   ├── load_test_YYYY-MM-DD.json
│   ├── microbench_YYYY-MM-DD.json
│   └── memory_YYYY-MM-DD.json
└── rust/
    ├── load_test_YYYY-MM-DD.json
    ├── microbench_YYYY-MM-DD.json
    └── memory_YYYY-MM-DD.json
```

## Interpreting Results

### Good Performance Indicators
- ✅ p95 latency < 20ms for most endpoints
- ✅ > 5k RPS sustained throughput
- ✅ Memory usage < 100MB under load
- ✅ CPU usage efficient (< 80% at peak)
- ✅ No memory leaks (stable over time)

### Red Flags
- ⚠️ Latency spikes (p99 >> p95)
- ⚠️ Memory growth over time
- ⚠️ CPU usage near 100%
- ⚠️ Throughput degradation under load

## Goals

### Current Phase
1. **Establish baseline** for both implementations
2. **Identify optimization opportunities** in Windjammer
3. **Prove performance parity** (within 5%)

### Future Phases
1. **Optimize Windjammer compiler** to match/exceed Rust
2. **Demonstrate superiority** of naive Windjammer over naive Rust
3. **Track improvements** over time

## Contributing

When adding new benchmarks:
1. Add scenario to `scenarios/`
2. Update `run_all_benchmarks.sh`
3. Document expected performance
4. Add to CI workflow

## References

- [Criterion Documentation](https://bheisler.github.io/criterion.rs/book/)
- [wrk Documentation](https://github.com/wg/wrk)
- [Flamegraph Guide](https://www.brendangregg.com/flamegraphs.html)

