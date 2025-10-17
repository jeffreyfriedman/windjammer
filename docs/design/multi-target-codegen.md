# Multi-Target Code Generation Architecture

**Version:** 0.32.0  
**Status:** Design Document  
**Author:** Windjammer Team

---

## Overview

This document describes the architecture for supporting multiple compilation targets (Rust, JavaScript, WebAssembly) from a single Windjammer AST, ensuring that:

1. **No Interference**: New targets don't break existing Rust/WASM pipelines
2. **Code Reuse**: Shared optimization passes benefit all targets
3. **Target-Specific**: Each backend can apply target-specific optimizations
4. **Maintainability**: Clean separation of concerns

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                         Windjammer Source                        │
└─────────────────────┬───────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Lexer → Parser → AST                          │
│                   (Unchanged - Existing)                         │
└─────────────────────┬───────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Type Checker & Inference                       │
│                   (Unchanged - Existing)                         │
└─────────────────────┬───────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────────┐
│              Target-Agnostic Optimization Passes                 │
│  • String Interning (Phase 11)                                  │
│  • Dead Code Elimination (Phase 12)                             │
│  • Loop Optimization (Phase 13)                                 │
│  • Escape Analysis (Phase 14)                                   │
│  • SIMD Vectorization (Phase 15)                                │
│                   (Refactored - Extract Common Logic)            │
└─────────────────────┬───────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Target Selection                              │
│               (New - Based on --target flag)                     │
└──────────┬──────────────────────┬─────────────────┬─────────────┘
           │                      │                 │
           ▼                      ▼                 ▼
┌──────────────────┐   ┌──────────────────┐   ┌──────────────────┐
│  Rust Backend    │   │  JavaScript      │   │  WASM Backend    │
│  (Existing)      │   │  Backend (NEW)   │   │  (Existing)      │
│                  │   │                  │   │                  │
│  • Defer Drop    │   │  • Async/await   │   │  • Memory opt    │
│  • Inline hints  │   │  • Web Workers   │   │  • Size opt      │
│  • Rust idioms   │   │  • Polyfills     │   │  • JS interop    │
└────────┬─────────┘   └────────┬─────────┘   └────────┬─────────┘
         │                      │                       │
         ▼                      ▼                       ▼
┌──────────────────┐   ┌──────────────────┐   ┌──────────────────┐
│   .rs + Cargo    │   │  .js + .d.ts +   │   │   .wasm + .js    │
│                  │   │  package.json    │   │   bindings       │
└──────────────────┘   └──────────────────┘   └──────────────────┘
```

---

## Key Design Principles

### 1. Shared AST and Type Information

**No Duplication:**
- All targets use the **same** AST from `parser.rs`
- All targets use the **same** type checking from `type_checker.rs`
- No target-specific AST modifications (use IR if needed)

**Benefits:**
- Bugs fixed once affect all targets
- New language features automatically available everywhere
- Consistent semantics across targets

### 2. Target-Agnostic Optimization Layer

**Extract Common Optimizations:**
```rust
// src/optimizer/common.rs (NEW)
pub trait OptimizationPass {
    fn name(&self) -> &str;
    fn optimize(&self, program: &mut Program) -> OptimizationStats;
    fn is_target_agnostic(&self) -> bool { true }
}

// Existing optimizations become target-agnostic
pub struct StringInterningPass;
pub struct DeadCodeEliminationPass;
pub struct LoopOptimizationPass;
// etc.
```

**Benefits:**
- JavaScript benefits from Rust optimizations (dead code elimination, etc.)
- Rust benefits from any new optimizations added for JS
- Clear separation between agnostic and target-specific passes

### 3. Backend Trait for Code Generation

```rust
// src/codegen/backend.rs (NEW)
pub trait CodegenBackend {
    type Output;
    
    fn name(&self) -> &str;
    fn generate(&self, program: &Program, config: &CodegenConfig) -> Result<Self::Output>;
    fn target_specific_optimizations(&self) -> Vec<Box<dyn OptimizationPass>>;
}

// Existing Rust backend
pub struct RustBackend;
impl CodegenBackend for RustBackend {
    type Output = String; // Rust source code
    // ...
}

// New JavaScript backend
pub struct JavaScriptBackend;
impl CodegenBackend for JavaScriptBackend {
    type Output = JavaScriptOutput; // JS + source maps + .d.ts
    // ...
}

// Existing WASM backend (refactored)
pub struct WasmBackend;
impl CodegenBackend for WasmBackend {
    type Output = WasmOutput; // .wasm + JS bindings
    // ...
}
```

**Benefits:**
- Clean abstraction - backends don't know about each other
- Easy to add new targets (Python, C, etc.)
- Testable in isolation

### 4. Non-Intrusive Refactoring

**Phase 1: Extract without Breaking**
```rust
// src/codegen/mod.rs (REFACTORED)
pub mod backend;      // NEW: Backend trait
pub mod rust;         // MOVED: Existing RustCodeGenerator → rust/mod.rs
pub mod javascript;   // NEW: JavaScript backend
pub mod wasm;         // MOVED: Existing WASM code → wasm/mod.rs
pub mod common;       // NEW: Shared utilities

// Keep existing public API unchanged
pub fn generate_rust(program: &Program) -> String {
    let backend = rust::RustBackend::new();
    backend.generate(program, &Default::default()).unwrap()
}
```

**Phase 2: Gradually Adopt New API**
```rust
// src/main.rs (ENHANCED)
let target = matches.get_one::<String>("target")
    .map(|s| s.as_str())
    .unwrap_or("rust");

let output = match target {
    "rust" => codegen::generate_rust(&program),
    "js" => codegen::generate_javascript(&program),
    "wasm" => codegen::generate_wasm(&program),
    _ => return Err(format!("Unknown target: {}", target)),
};
```

**Benefits:**
- Existing code keeps working
- No big-bang refactor
- Incremental migration path

---

## Implementation Plan

### Step 1: Refactor Existing Code (Non-Breaking)

**Goal:** Prepare for multi-target without breaking anything

**Tasks:**
1. Move `src/codegen.rs` → `src/codegen/rust.rs`
2. Create `src/codegen/backend.rs` with trait
3. Implement `CodegenBackend` for `RustBackend`
4. Keep `src/codegen/mod.rs` with existing public API
5. Run all tests - **ensure 100% pass**

**Changes:**
```rust
// Before: src/codegen.rs
pub fn generate_rust(program: &Program) -> String { ... }

// After: src/codegen/mod.rs
mod backend;
mod rust;

pub use rust::generate_rust; // Re-export existing API
```

**Validation:**
- `cargo test` - all tests pass
- `cargo build --release` - compiles
- No behavioral changes

### Step 2: Extract Common Optimizations

**Goal:** Identify optimization passes that benefit all targets

**Candidates:**
- ✅ **String Interning** - JS strings can be interned too
- ✅ **Dead Code Elimination** - Remove unused functions in JS
- ✅ **Loop Optimization (LICM)** - JS engines benefit from hoisted code
- ⚠️ **Escape Analysis** - Rust-specific (stack allocation), skip for JS
- ⚠️ **SIMD Vectorization** - Rust-specific, but could inspire JS TypedArray opts
- ✅ **Constant Folding** - Always beneficial

**Refactor:**
```rust
// src/optimizer/common.rs
pub struct OptimizationPipeline {
    target_agnostic: Vec<Box<dyn OptimizationPass>>,
    rust_specific: Vec<Box<dyn OptimizationPass>>,
    js_specific: Vec<Box<dyn OptimizationPass>>,
}

impl OptimizationPipeline {
    pub fn for_target(target: Target) -> Self {
        let mut pipeline = Self {
            target_agnostic: vec![
                Box::new(StringInterningPass),
                Box::new(DeadCodeEliminationPass),
                Box::new(LoopOptimizationPass),
            ],
            rust_specific: vec![
                Box::new(EscapeAnalysisPass),
                Box::new(SimdVectorizationPass),
                Box::new(DeferDropPass),
            ],
            js_specific: vec![
                Box::new(AsyncAwaitOptimizationPass),
                Box::new(ClosureOptimizationPass),
            ],
        };
        
        // Return target-appropriate passes
        match target {
            Target::Rust | Target::Wasm => {
                pipeline.target_agnostic.extend(pipeline.rust_specific);
            }
            Target::JavaScript => {
                pipeline.target_agnostic.extend(pipeline.js_specific);
            }
        }
        
        pipeline
    }
}
```

### Step 3: Implement JavaScript Backend

**Goal:** Generate ES2020+ JavaScript with source maps

**Features:**
- Function declarations
- Classes (for structs)
- Closures
- Async/await (for Windjammer's `go` keyword)
- Module exports (ESM)
- Source maps

**Example:**
```windjammer
// Input: hello.wj
fn greet(name: string) {
    println!("Hello, ${name}!")
}

fn main() {
    greet("World")
}
```

```javascript
// Output: hello.js
export function greet(name) {
    console.log(`Hello, ${name}!`);
}

export function main() {
    greet("World");
}

// Auto-run main if not imported
if (import.meta.url === `file://${process.argv[1]}`) {
    main();
}
```

### Step 4: Add TypeScript Definitions

**Goal:** Generate `.d.ts` for IDE support

```typescript
// Output: hello.d.ts
export declare function greet(name: string): void;
export declare function main(): void;
```

### Step 5: Source Map Generation

**Goal:** Enable debugging Windjammer source in browser DevTools

**Library:** Use `source-map` crate

**Output:**
```json
// hello.js.map
{
  "version": 3,
  "sources": ["hello.wj"],
  "names": [],
  "mappings": "AAAA;AACA;AACA",
  "file": "hello.js"
}
```

### Step 6: CLI Integration

```bash
# Rust (default, unchanged)
wj build hello.wj

# JavaScript (new)
wj build --target=js hello.wj
wj build --target=javascript hello.wj --output dist/

# WASM (existing, refactored)
wj build --target=wasm hello.wj
```

---

## Benefits to Existing Targets

### Rust Backend Improvements

**1. Better Modularity:**
- Clear separation of concerns
- Easier to test individual components
- Cleaner architecture

**2. Shared Optimizations:**
- Any new optimization for JS can benefit Rust
- Example: If we add better closure optimization for JS, Rust might benefit too

**3. Better Testing:**
- Cross-target tests ensure correctness
- Example: Same input, verify Rust and JS produce equivalent behavior

### WASM Backend Improvements

**1. Unified Pipeline:**
- WASM can share more with Rust than with JS
- Common optimizations for both compiled targets

**2. JS Interop:**
- Better JS generation helps WASM<->JS bindings
- Example: TypeScript definitions for WASM exports

**3. Smaller WASM Binaries:**
- Dead code elimination improvements help WASM size

---

## Testing Strategy

### 1. Backend-Agnostic Tests

```rust
// tests/codegen/common.rs
#[test]
fn test_hello_world_all_targets() {
    let source = r#"
        fn main() {
            println!("Hello, World!")
        }
    "#;
    
    let program = parse_and_check(source);
    
    // Rust target
    let rust_code = codegen::generate_rust(&program);
    assert!(rust_code.contains("println!"));
    
    // JavaScript target
    let js_code = codegen::generate_javascript(&program);
    assert!(js_code.contains("console.log"));
    
    // WASM target
    let wasm_output = codegen::generate_wasm(&program);
    assert!(wasm_output.exports.contains(&"main".to_string()));
}
```

### 2. Target-Specific Tests

```rust
// tests/codegen/javascript_specific.rs
#[test]
fn test_async_await_translation() {
    let source = r#"
        async fn fetch_data() -> string {
            http_get("https://api.example.com").await
        }
    "#;
    
    let js_code = codegen::generate_javascript(&parse(source));
    assert!(js_code.contains("async function"));
    assert!(js_code.contains("await"));
}
```

### 3. Semantic Equivalence Tests

```rust
// tests/integration/semantic_equivalence.rs
#[test]
fn test_fibonacci_all_targets() {
    let source = include_str!("fixtures/fibonacci.wj");
    
    // Compile to all targets
    let rust_binary = compile_rust(source);
    let js_script = compile_js(source);
    
    // Run both
    let rust_output = run_binary(rust_binary, &["10"]);
    let js_output = run_node(js_script, &["10"]);
    
    // Should produce same result
    assert_eq!(rust_output, "55");
    assert_eq!(js_output, "55");
}
```

---

## Migration Path (Backward Compatibility)

### Phase 1: Internal Refactoring (v0.32.0-alpha)
- Move code to new structure
- Keep existing public API
- All tests pass

### Phase 2: Add JavaScript Support (v0.32.0-beta)
- JavaScript backend implemented
- `--target=js` flag works
- Rust and WASM still default, unchanged

### Phase 3: Stabilize (v0.32.0)
- Production-ready JavaScript output
- Comprehensive documentation
- All targets well-tested

### Phase 4: Unified API (v0.33.0+)
- Optional: Deprecate old API
- Encourage new `CodegenBackend` trait
- Remove old code (if needed)

---

## Success Criteria

✅ **No Regressions:**
- All existing Rust/WASM tests pass unchanged
- Performance not degraded
- Binary size not increased

✅ **JavaScript Quality:**
- Readable output (not obfuscated)
- Source maps work in DevTools
- TypeScript definitions accurate
- npm packages work out of box

✅ **Shared Benefits:**
- At least 3 optimization passes shared across targets
- Test coverage improves for all targets
- Documentation clearer about compiler architecture

---

## Future Enhancements

### More Targets
- Python (via same architecture)
- C (for embedded systems)
- LLVM IR (for ultimate optimization)

### Better Optimization Sharing
- Intermediate Representation (IR) layer
- More sophisticated analysis passes
- Target-specific hints in IR

### Cross-Target Testing
- Fuzzing that compares all targets
- Property-based testing
- Formal verification of semantic equivalence

---

## Conclusion

This architecture ensures that adding JavaScript support:
1. **Doesn't break** existing Rust/WASM pipelines
2. **Improves** code quality through better modularity
3. **Enables** future targets easily
4. **Maintains** backward compatibility

The key insight: **Treat backends as pluggable modules that share a common foundation.**

---

*This is a living document. Update as implementation progresses.*

