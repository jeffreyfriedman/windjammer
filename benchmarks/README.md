# TaskFlow API - Performance Benchmarks

This directory contains performance benchmarking infrastructure for the TaskFlow API, comparing Windjammer and Rust implementations.

## Baseline Results (v0.16.0)

**Test Environment:**
- Machine: macOS 23.5.0
- Rust: 1.90.0
- Date: October 2025

### Rust Implementation - Criterion Microbenchmarks

#### JSON Serialization
| Operation | Time | Notes |
|-----------|------|-------|
| RegisterRequest | 280.54 ns | User registration payload |
| CreateProjectRequest | 149.51 ns | Project creation payload |

#### JSON Deserialization
| Operation | Time | Notes |
|-----------|------|-------|
| LoginRequest | 135.55 ns | Login credentials |
| CreateTaskRequest | 291.13 ns | Task creation payload |

#### Password Hashing
| Operation | Time | Notes |
|-----------|------|-------|
| bcrypt_hash | 254.62 ms | Bcrypt with default cost (12) |

#### JWT Operations
| Operation | Time | Notes |
|-----------|------|-------|
| generate | 1.0046 µs | Generate JWT token |
| verify | 1.8997 µs | Verify JWT token |

#### Query Building
| Operation | Time | Notes |
|-----------|------|-------|
| simple_select | 39.921 ns | Simple SELECT query |
| complex_join | 74.520 ns | Complex JOIN query |

---

## Summary

**Rust Baseline Performance:**
- **JSON operations**: 135-291 ns (very fast)
- **Cryptography**: 255 ms for bcrypt (expected - intentionally slow)
- **JWT**: 1-2 µs (excellent)
- **Query building**: 40-75 ns (negligible)

**Key Insights:**
1. **Bcrypt dominates auth latency** - 99.9% of login time is password hashing
2. **JSON serialization** is extremely fast (~150-280 ns per operation)
3. **JWT operations** are efficient (1-2 µs)
4. **Query building** has negligible overhead

---

## Running Benchmarks

### Rust Implementation

```bash
cd examples/taskflow/rust
cargo bench
```

Results are saved to `target/criterion/` with HTML reports.

### Viewing Results

```bash
# Open HTML report
open examples/taskflow/rust/target/criterion/report/index.html
```

---

## Benchmark Structure

### Microbenchmarks (Criterion)

Located in `examples/taskflow/rust/benches/api_benchmarks.rs`

**Coverage:**
- JSON serialization/deserialization
- Password hashing (bcrypt)
- JWT generation/verification
- Query building

**Why these?**
- Represent critical hot paths in the API
- Easy to isolate and measure
- No database or network dependencies

### Load Tests (Future)

**Planned:**
- `wrk`-based HTTP endpoint testing
- Measures: RPS, p50/p95/p99 latency
- High concurrency testing (500 connections)
- Automated comparison scripts

---

## Interpretation Guide

### What to Look For

1. **Regression Detection**
   - > 5% slower: Warning (investigate)
   - > 10% slower: Failure (block merge)

2. **Outliers**
   - Some outliers are normal (OS scheduling, GC, etc.)
   - > 15% outliers: Investigate environmental issues

3. **Variance**
   - Low variance (< 5%): Consistent, reliable
   - High variance (> 20%): Investigate noise sources

### What NOT to Over-Optimize

1. **Query Building** (40-75 ns) - Already negligible
2. **JWT Operations** (1-2 µs) - Not a bottleneck
3. **JSON** (135-291 ns) - Serde is already optimal

### What MATTERS

1. **Database Query Execution** (not yet measured - requires running DB)
2. **Overall Request Latency** (end-to-end HTTP)
3. **Throughput under Load** (RPS with realistic concurrency)

---

## Next Steps

**For v0.16.0:**
- ✅ Establish Rust baseline (this document)
- ⏳ Build equivalent Windjammer benchmarks
- ⏳ Compare baseline performance
- ⏳ Document any gaps

**For v0.17.0:**
- Implement compiler optimizations to close gaps
- Add HTTP load testing (wrk)
- Add database integration tests
- CI/CD for continuous monitoring

---

## CI/CD Integration

### GitHub Actions Workflow

Located in `.github/workflows/benchmarks.yml`

**Triggers:**
- Pull requests (compare vs main)
- Main branch (update baseline)
- Nightly (track long-term trends)

**Actions:**
1. Run Criterion benchmarks
2. Compare results against baseline
3. Detect regressions (5% warning, 10% fail)
4. Comment results on PR
5. Upload artifacts (90-day retention)

**Baseline Storage:**
- Stored in `benchmarks/baselines/`
- Updated on main branch merges
- Historical tracking (90 days)

---

## Contributing

### Adding New Benchmarks

1. **Identify hot path** - Profile first, optimize later
2. **Isolate functionality** - No external dependencies
3. **Use black_box** - Prevent compiler optimizations
4. **Measure iterations** - Let Criterion decide sample size
5. **Document purpose** - Explain what and why

### Example:

```rust
fn my_benchmark(c: &mut Criterion) {
    c.bench_function("operation_name", |b| {
        b.iter(|| {
            // Use black_box to prevent optimization
            black_box(expensive_operation(black_box(input)))
        });
    });
}
```

---

## Resources

- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [wrk HTTP Benchmarking](https://github.com/wg/wrk)

---

## Baseline Archive

**v0.16.0 (October 2025):**
- Rust: See results above
- Windjammer: TBD

**Future releases will add comparison data here.**

---

**Last Updated:** v0.16.0  
**Next Review:** v0.17.0 (after compiler optimizations)

