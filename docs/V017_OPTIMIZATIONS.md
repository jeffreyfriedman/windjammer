# v0.17.0 Compiler Optimizations

## Overview

This document describes the multi-phase compiler optimization pipeline implemented in v0.17.0. The goal is to generate Rust code that performs competitively with hand-written Rust (target: ≥95% performance).

## Architecture

The optimization pipeline runs after parsing and before code generation:

```
Source Code → Lexer → Parser → Analyzer (Optimizations) → Code Generator → Rust Code
```

### Analyzer (`src/analyzer.rs`)

The analyzer performs multiple optimization passes:
1. **Inline Hints** - Marks functions for inlining
2. **Clone Optimization** - Eliminates unnecessary `.clone()` calls
3. **Struct Mapping** - Optimizes struct construction
4. **String Analysis** - Prepares for string capacity optimization

### Code Generator (`src/codegen.rs`)

The code generator consumes optimization hints and generates optimized Rust code.

## Phase 1: Inline Hints

### Purpose
Reduce function call overhead by adding `#[inline]` attributes to appropriate functions.

### Heuristics

**Always Inline:**
- Stdlib wrapper functions (thin wrappers around Rust crates)
- Small functions (≤ 50 lines of code)
- Trivial functions (single expression/statement)

**Never Inline:**
- `main` function
- Test functions (`@test` decorator)
- Async functions (`@async` decorator)

### Implementation

```rust
fn should_inline_function(&self, func: &FunctionDecl) -> bool {
    // Check blacklist
    if func.name == "main" || func.decorators.contains("test") || func.decorators.contains("async") {
        return false;
    }
    
    // Check if stdlib wrapper
    if self.is_stdlib_wrapper(func) {
        return true;
    }
    
    // Check size and complexity
    let loc = self.count_lines(func);
    loc <= 50 || self.is_trivial(func)
}
```

### Impact
- Reduced function call overhead in tight loops
- Better LLVM optimization opportunities

## Phase 2: Clone Optimization

### Purpose
Eliminate unnecessary `.clone()` calls that impact performance due to unnecessary allocations.

### Analysis

Tracks variable usage patterns:
- **Reads**: How many times a variable is read
- **Writes**: How many times a variable is written
- **Escapes**: Whether variable escapes function scope
- **In Loop**: Whether variable is used in loop context

### Optimization Rules

Variables can have `.clone()` eliminated if:
1. **Only Read** - Variable never written and doesn't escape
2. **Single Use** - Variable read once and doesn't escape
3. ~~**Local Only** - Variable doesn't escape function~~ (removed - too aggressive)

### Critical: Loop Context Tracking

**Bug Fixed in v0.17.0**: Variables used in loops MUST preserve `.clone()` calls.

Each loop iteration needs its own copy of the data. Without tracking loop context, the optimizer was incorrectly removing clones, causing "value moved" errors.

```rust
// BEFORE (incorrect optimization):
let name = String::from("user");
for i in 0..1000 {
    let user = User { name: name };  // ERROR: value moved in first iteration
}

// AFTER (correct):
let name = String::from("user");
for i in 0..1000 {
    let user = User { name: name.clone() };  // OK: each iteration gets a copy
}
```

### Implementation

```rust
fn detect_unnecessary_clones(&self, func: &FunctionDecl) -> Vec<CloneOptimization> {
    let mut usage: HashMap<String, (reads, writes, escapes, in_loop)> = HashMap::new();
    
    // Analyze usage patterns
    for stmt in &func.body {
        self.analyze_statement_for_clones(stmt, &mut usage, 0);
    }
    
    // Identify safe optimizations
    for (var, (reads, writes, escapes, in_loop)) in usage {
        // NEVER optimize variables used in loops!
        if in_loop {
            continue;
        }
        
        // Apply other rules...
    }
}
```

### Impact
- Reduced allocations for single-use variables
- Improved performance in non-loop contexts
- Critical correctness fix for loop scenarios

## Phase 3: Struct Mapping Optimization

### Purpose
Generate idiomatic Rust struct construction patterns.

### Patterns Detected

1. **Field Shorthand**
```rust
// Before:
Point { x: x, y: y }

// After (optimized):
Point { x, y }
```

2. **FromRow Pattern** (future)
```rust
// Detect SQL row → struct mapping
User::from_row(row)
```

3. **Builder Pattern** (future)
```rust
// Detect builder-style construction
User::builder().name(name).email(email).build()
```

### Implementation

```rust
fn detect_struct_mappings(&self, func: &FunctionDecl) -> Vec<StructMappingOptimization> {
    // Scan for struct literals
    for stmt in &func.body {
        if let Statement::Let { value: Expression::StructLiteral { fields, .. }, .. } = stmt {
            // Check if field shorthand applies
            for (field_name, field_expr) in fields {
                if let Expression::Identifier(var_name) = field_expr {
                    if var_name == field_name {
                        // Can use shorthand!
                    }
                }
            }
        }
    }
}
```

### Impact
- Cleaner, more idiomatic generated code
- Slight compilation time improvement (less text to parse)
- Better readability for debugging

## Phase 4: String Operation Analysis

### Purpose
Prepare for string capacity pre-allocation optimization.

### Patterns Detected

1. **String Interpolation** (via `format!` macro)
2. **Concatenation Chains** (`a + b + c + d`)
3. **Loop Accumulation** (building strings in loops)

### Capacity Estimation

```rust
fn detect_string_optimizations(&self, func: &FunctionDecl) -> Vec<StringOptimization> {
    // Example: format!("User: {}, Email: {}", name, email)
    // Estimated capacity: 14 (static) + name.len() + email.len()
    
    // Example: loop accumulation
    // Estimated capacity: iterations * avg_item_size
}
```

### Implementation Status

**Phase 4 Status**: Infrastructure complete, optimization hooks ready but not yet applied in codegen.

### Future Work

Code generator will use capacity hints:
```rust
// Future optimization:
let mut result = String::with_capacity(estimated_capacity);
result.push_str(&format!(...));
```

## Performance Results

### Benchmark: Minimal TaskFlow API

**Test**: 3,500 struct operations (1000 Users + 500 Projects + 2000 Tasks)

```
Pure Rust:    0.229s  (100% baseline)
Windjammer:   0.338s  (67.8% of Rust speed)
Gap:          ~47% slower
```

### Analysis

**What's Working:**
- ✅ Inline hints improving call overhead
- ✅ Clone optimization reducing allocations (outside loops)
- ✅ Struct shorthand generating cleaner code

**Performance Gap:**
- Code generation may not be as efficient as hand-written Rust
- Potential issues:
  - Unnecessary moves/copies we haven't detected
  - Suboptimal type conversions (e.g., `&str` → `String`)
  - Missing lifetime elision opportunities
  - Not using `std::hint::black_box` to prevent over-optimization

**Next Steps:**
1. Analyze generated code for inefficiencies
2. Implement more aggressive optimizations
3. Add escape analysis
4. Improve type inference to reduce conversions

## Debugging Optimizations

### Enable Optimization Tracing

```bash
# Set environment variable to see optimization decisions
WINDJAMMER_DEBUG_OPTIMIZATIONS=1 wj build main.wj
```

### Inspect Generated Code

```bash
# Compare generated code with/without optimizations
wj build --no-optimize main.wj -o build_unopt
wj build main.wj -o build_opt
diff -u build_unopt/main.rs build_opt/main.rs
```

## Testing

### Optimization Test Suite

Location: `tests/optimization_tests.wj`

Tests each optimization phase:
- Inline hint generation
- Clone elimination
- Struct shorthand
- String capacity estimation

### Running Tests

```bash
cargo test optimization
cargo test --test feature_tests
```

## Future Improvements

### v0.18.0+ Roadmap

1. **Close Performance Gap**
   - Target: 90-95% of Rust speed
   - Analyze generated code for hotspots
   - Implement escape analysis
   - Better type inference

2. **Advanced Optimizations**
   - String capacity pre-allocation (Phase 4)
   - Lifetime elision
   - Move instead of clone where safe
   - Const folding

3. **Profile-Guided Optimization**
   - Collect runtime profiling data
   - Optimize hot paths aggressively
   - Use PGO for better inlining decisions

## References

- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [LLVM Optimization Passes](https://llvm.org/docs/Passes.html)
- [Rust Compiler Optimization Levels](https://doc.rust-lang.org/cargo/reference/profiles.html)

## Changelog

- **v0.17.0**: Initial 4-phase optimization pipeline
  - Phase 1: Inline hints
  - Phase 2: Clone optimization (with loop bug fix)
  - Phase 3: Struct mapping
  - Phase 4: String analysis (infrastructure)

