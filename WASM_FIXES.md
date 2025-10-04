# WASM Game Fixes - Critical Bugs Discovered

## Overview

By working through the complex `wasm_game` example (Conway's Game of Life), we discovered and fixed **4 critical compiler bugs** that would have affected any real-world Windjammer project.

## Bugs Fixed

### 1. **Operator Precedence Bug in Binary Expressions**
**Problem**: The modulo operator `%` has higher precedence than `+` and `-`, but the codegen was generating code without proper parentheses.

**Example**:
```windjammer
let neighbor_row = (row + delta_row + self.height - 1) % self.height
```

**Generated (WRONG)**:
```rust
let neighbor_row = row + delta_row + self.height - 1 % self.height;
// Evaluates as: row + delta_row + self.height - (1 % self.height)
```

**Generated (FIXED)**:
```rust
let neighbor_row = (row + delta_row + self.height - 1) % self.height;
```

**Fix**: Added `op_precedence()` function and logic to wrap lower-precedence subexpressions in parentheses.

**Files Changed**: `src/codegen.rs`

---

### 2. **Missing Glob Imports for Use Statements**
**Problem**: Windjammer's Go-style `use wasm_bindgen.prelude` was being transpiled to `use wasm_bindgen::prelude;` instead of `use wasm_bindgen::prelude::*;`, causing "cannot find attribute" errors.

**Example**:
```windjammer
use wasm_bindgen.prelude
```

**Generated (WRONG)**:
```rust
use wasm_bindgen::prelude;  // Nothing imported!
```

**Generated (FIXED)**:
```rust
use wasm_bindgen::prelude::*;  // All items imported
```

**Fix**: Modified `generate_use()` to append `::*` for glob imports.

**Files Changed**: `src/codegen.rs`

---

### 3. **Impl Block Decorators Not Supported**
**Problem**: Decorators like `@wasm_bindgen` on `impl` blocks weren't being parsed or generated, causing WASM functions to not be exported to JavaScript.

**Example**:
```windjammer
@wasm_bindgen
impl Universe {
    fn new() -> Universe { ... }
}
```

**Generated (WRONG)**:
```rust
impl Universe {
    fn new() -> Universe { ... }  // NOT exported to JS!
}
```

**Generated (FIXED)**:
```rust
#[wasm_bindgen]
impl Universe {
    pub fn new() -> Universe { ... }  // Exported!
}
```

**Fix**: 
- Added `decorators: Vec<Decorator>` to `ImplBlock` struct
- Modified parser to collect decorators before `impl`
- Updated codegen to generate `#[attribute]` before impl blocks
- Added `in_wasm_bindgen_impl` flag to track when to add `pub`

**Files Changed**: `src/parser.rs`, `src/codegen.rs`

---

### 4. **Missing `pub` on Functions in `#[wasm_bindgen]` Impl Blocks**
**Problem**: Functions in `#[wasm_bindgen]` impl blocks must be `pub` to be exported to JavaScript, but the codegen wasn't adding it.

**Example**:
```windjammer
@wasm_bindgen
impl Universe {
    fn new() -> Universe { ... }
}
```

**Generated (WRONG)**:
```rust
#[wasm_bindgen]
impl Universe {
    fn new() -> Universe { ... }  // Not exported!
}
```

**Generated (FIXED)**:
```rust
#[wasm_bindgen]
impl Universe {
    pub fn new() -> Universe { ... }  // Exported!
}
```

**Fix**: Added logic to prepend `pub ` to functions when `in_wasm_bindgen_impl` is true.

**Files Changed**: `src/codegen.rs`

---

## Impact

These bugs would have affected:
- **Any WASM project** using `wasm-bindgen`
- **Any arithmetic** involving mixed precedence operators
- **All imports** from Rust crates
- **Type-driven codegen** where decorators control behavior

## Testing

- ✅ All 9 compiler tests pass
- ✅ `wasm_hello` example works (simple WASM functions)
- ✅ `wasm_game` example works (complex Conway's Game of Life at 60 FPS)
- ✅ Both examples tested in browser at `localhost:8080` and `localhost:8090`

## Conclusion

This demonstrates the critical value of **working through complex, real-world examples** during language development. Simple unit tests wouldn't have caught these issues, but a full game implementation exposed all four bugs immediately.

**Lesson**: Always build something real with your language!

