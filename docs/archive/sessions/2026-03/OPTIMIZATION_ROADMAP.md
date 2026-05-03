# Windjammer Compiler Optimization Roadmap

This document outlines the current and planned compiler optimizations in Windjammer, inspired by the [Rust Performance Book](https://nnethercote.github.io/perf-book/).

## Current Optimizations (v0.20.0) ✅

### Phase 0: Defer Drop (v0.20.0) ⚡
- **393x faster time-to-return** for large owned parameters
- Automatic `std::thread::spawn(move || drop(...))` generation
- Conservative safety checks (Send, no Drop side effects)
- **Impact**: Interactive applications, CLIs, web APIs

### Phase 1: Inline Hints
- Automatic `#[inline]` for small functions (< 10 lines)
- Stdlib wrapper functions always inlined
- **Impact**: ~5-10% performance improvement

### Phase 2: Clone Elimination
- Detects unnecessary `.clone()` calls
- Loop-aware analysis ensures correctness
- **Impact**: Reduced heap allocations, ~10-15% improvement

### Phase 3: Struct Shorthand
- Generates idiomatic Rust patterns (`Point { x, y }`)
- Cleaner, more efficient generated code
- **Impact**: Code size reduction, minor performance gain

### Phase 4: String Capacity Pre-allocation
- Optimizes `format!` with `String::with_capacity`
- Eliminates reallocation overhead
- **Impact**: ~20-30% faster string operations

### Phase 5: Compound Assignments
- Converts `x = x + 1` to `x += 1` automatically
- More efficient code patterns
- **Impact**: Minor performance gain, better codegen

### Phase 6: Constant Folding (Phase 7 in code)
- Evaluates constant expressions at compile time
- `2 + 3` becomes `5` in generated code
- **Impact**: Eliminates runtime computation

---

## Planned Optimizations (Future Versions)

### Phase 7: Lazy Static/Const (v0.21.0) 🎯

**Goal**: Convert runtime initialization to compile-time `const fn`

**Pattern**:
```windjammer
// User writes:
static REGEX_PATTERN = r"\d+"

// Compiler generates:
const REGEX_PATTERN: &str = r"\d+";  // Compile-time!
```

**Benefits**:
- Zero runtime overhead
- Faster startup time
- Smaller binary size

**Implementation**:
- Analyzer detects static initialization patterns
- Codegen emits `const` or `const fn` where possible
- Falls back to `lazy_static!` only when necessary

---

### Phase 8: SmallVec Optimization (v0.21.0) 🎯

**Goal**: Use stack allocation for small vectors

**Pattern**:
```windjammer
// User writes:
let items: Vec<int> = vec![1, 2, 3]

// Compiler generates (if profiled size usually <= 8):
let items: SmallVec<[i32; 8]> = smallvec![1, 2, 3];  // Stack!
```

**Benefits**:
- No heap allocation for small collections
- ~50-100% faster for small vectors
- Reduced memory fragmentation

**Implementation**:
- Analyzer tracks Vec usage patterns
- Generates SmallVec when beneficial (e.g., loop bounds suggest < 16 elements)
- Adds `smallvec` crate dependency automatically

**Safety**: SmallVec is drop-in compatible with Vec

---

### Phase 9: Cow (Clone-on-Write) Optimization (v0.22.0) 🎯

**Goal**: Reduce unnecessary allocations when data rarely changes

**Pattern**:
```windjammer
// User writes:
fn process(data: string) -> string {
    if needs_modification {
        data.to_uppercase()
    } else {
        data  // Wasteful!
    }
}

// Compiler generates:
fn process(data: Cow<'_, str>) -> Cow<'_, str> {
    if needs_modification {
        Cow::Owned(data.to_uppercase())
    } else {
        data  // Zero-cost!
    }
}
```

**Benefits**:
- Zero-cost when data isn't modified
- ~50% reduction in allocations for read-heavy workloads

**Implementation**:
- Analyzer detects conditional modification patterns
- Generates `Cow<'_, T>` where beneficial
- Preserves semantics (automatically converts to owned when needed)

---

### Phase 10: Unsafe Block Optimization (v0.23.0) ⚠️

**Goal**: Strategic `unsafe` for performance-critical paths

**Pattern**:
```windjammer
// User writes:
fn sum(arr: Vec<int>) -> int {
    let mut total = 0
    for i in 0..arr.len() {
        total += arr[i]  // Bounds check every iteration
    }
    total
}

// Compiler generates (with safety proof):
fn sum(arr: Vec<i32>) -> i32 {
    let mut total = 0;
    for i in 0..arr.len() {
        // SAFETY: i is guaranteed to be < arr.len() by loop condition
        unsafe { total += *arr.get_unchecked(i); }
    }
    total
}
```

**Benefits**:
- Eliminates bounds checks in proven-safe loops
- ~10-20% faster array-heavy code

**Implementation**:
- Analyzer proves safety invariants (loop bounds, array access patterns)
- Generates `unsafe` blocks only when provably safe
- Adds comprehensive safety comments
- **Conservative**: Only enables for obvious cases

**Safety**: All `unsafe` blocks include:
1. Safety proof in comments
2. Analyzer verification
3. Runtime assertions in debug builds

---

### Phase 11: Branch Prediction Hints (v0.23.0) 🎯

**Goal**: Help CPU predict branches in hot paths

**Pattern**:
```windjammer
// User writes:
if unlikely_error_condition {
    handle_error()
}

// Compiler generates:
if core::intrinsics::unlikely(unlikely_error_condition) {
    handle_error()
}
```

**Benefits**:
- ~5-10% faster in hot loops with predictable branches
- Better CPU pipeline utilization

**Implementation**:
- Analyzer detects error-handling patterns (`Result`, `Option`, `if err`)
- Generates `likely!`/`unlikely!` intrinsics
- Heuristics: error branches are unlikely, success branches are likely

---

### Phase 12: Loop Unrolling (v0.24.0) 🎯

**Goal**: Unroll small, fixed-size loops automatically

**Pattern**:
```windjammer
// User writes:
for i in 0..4 {
    process(arr[i])
}

// Compiler generates (unrolled):
process(arr[0]);
process(arr[1]);
process(arr[2]);
process(arr[3]);
```

**Benefits**:
- Eliminates loop overhead
- ~20-30% faster for small loops
- Enables further optimizations (SIMD, inlining)

**Implementation**:
- Analyzer detects fixed-size loops (range literals, const bounds)
- Unrolls loops with < 16 iterations
- Respects `#[no_unroll]` annotation (future)

---

### Phase 13: SIMD Hints (v0.25.0) 🔬

**Goal**: Auto-vectorize data-parallel operations

**Pattern**:
```windjammer
// User writes:
fn add_arrays(a: Vec<f32>, b: Vec<f32>) -> Vec<f32> {
    a.iter().zip(b).map(|(x, y)| x + y).collect()
}

// Compiler generates (with SIMD):
#[target_feature(enable = "avx2")]
fn add_arrays(a: Vec<f32>, b: Vec<f32>) -> Vec<f32> {
    // Use SIMD instructions for 8 floats at once
    // ...
}
```

**Benefits**:
- ~4-8x faster for data-parallel operations
- Especially powerful for numeric code

**Implementation**:
- Analyzer detects data-parallel patterns (map, zip, fold)
- Generates SIMD code using `std::simd` (when stable)
- Falls back to scalar code on unsupported platforms

**Status**: Waiting for `std::simd` stabilization in Rust

---

### Phase 14: Memory Layout Optimization (v0.26.0) 🎯

**Goal**: Optimize struct layout for cache performance

**Pattern**:
```windjammer
// User writes:
struct Data {
    flag: bool,      // 1 byte
    value: int,      // 4 bytes
    other_flag: bool // 1 byte
}

// Compiler generates (reordered):
#[repr(C)]
struct Data {
    value: i32,      // 4 bytes (aligned)
    flag: bool,      // 1 byte
    other_flag: bool, // 1 byte
    // 2 bytes padding
}
```

**Benefits**:
- Better cache utilization
- Reduced struct size (eliminate padding)
- ~5-10% performance improvement for struct-heavy code

**Implementation**:
- Analyzer sorts fields by size (descending)
- Generates `#[repr(C)]` for predictable layout
- Respects explicit field ordering (with `#[no_reorder]` annotation)

---

## Low-Priority Optimizations (Future)

### Phase 15: Profile-Guided Optimization (PGO)
- Collect runtime profiles
- Optimize based on actual usage patterns
- **Complexity**: High (requires instrumentation)

### Phase 16: Link-Time Optimization (LTO)
- Already enabled in Rust release mode
- No additional work needed

### Phase 17: Code Size Reduction
- `opt-level="z"` for embedded targets
- Trade performance for smaller binaries
- **Use Case**: Embedded systems, WASM

---

## Implementation Priority

**v0.21.0** (Next Release):
- Phase 7: Lazy Static/Const ✅
- Phase 8: SmallVec ✅
- Update documentation with Phase 0 (defer drop) results

**v0.22.0**:
- Phase 9: Cow ✅
- Phase 11: Branch Prediction ✅
- Performance benchmarking suite expansion

**v0.23.0**:
- Phase 10: Unsafe Blocks (conservative) ⚠️
- Phase 12: Loop Unrolling ✅
- Safety audit for all `unsafe` generation

**v0.24.0+**:
- Phase 13: SIMD (when `std::simd` is stable)
- Phase 14: Memory Layout
- Phase 15: PGO (experimental)

---

## Measurement Strategy

For each optimization phase, we measure:

1. **Performance Impact**: Before/after benchmarks (Criterion)
2. **Code Size Impact**: Binary size comparison
3. **Compile Time Impact**: Build time overhead
4. **Safety**: Miri testing for `unsafe` blocks

**Goal**: Each phase should provide:
- ≥5% performance improvement OR
- ≥10% code size reduction OR
- Enabler for future optimizations

---

## References

- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Defer Drop Research](https://abrams.cc/rust-dropping-things-in-another-thread)
- [LLVM Optimization Docs](https://llvm.org/docs/Passes.html)
- [Rust Compiler Internals](https://rustc-dev-guide.rust-lang.org/)

---

**Last Updated**: v0.20.0  
**Next Review**: v0.21.0 planning

