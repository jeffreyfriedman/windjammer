# Windjammer Abstractions - Design Document

## Problem Statement

Currently, Windjammer exposes Rust implementation details directly:
- `@wasm_bindgen` - ties us to a specific Rust crate
- `use wasm_bindgen.prelude` - exposes Rust module structure
- Direct use of crate names in user code

This creates tight coupling and prevents us from:
- Swapping out backend implementations
- Supporting multiple compilation targets
- Evolving the language independently

## Design Principles

1. **Implementation Independence**: User code should not depend on specific Rust crates
2. **Semantic Naming**: Use Windjammer-native terms that describe intent, not implementation
3. **Future-Proof**: Allow backend swapping without breaking user code
4. **Zero-Config**: Common cases should "just work" without explicit decorators

## Proposed Abstractions

### WASM Export Decorator

**Current (BAD)**:
```windjammer
@wasm_bindgen
fn greet(name: string) -> string { ... }

@wasm_bindgen
impl Universe { ... }
```

**Proposed (GOOD)**:
```windjammer
@export  // or @wasm, or @public
fn greet(name: string) -> string { ... }

@export
impl Universe { ... }
```

**Rationale**:
- `@export` is implementation-agnostic
- Could compile to `#[wasm_bindgen]`, `#[no_mangle]`, or other backends
- Future: Could target JavaScript, Python FFI, etc.

---

### Imports/Use Statements

**Current (BAD)**:
```windjammer
use wasm_bindgen.prelude
use web_sys
use js_sys
```

**Proposed Option 1 - Implicit (BEST)**:
```windjammer
// No imports needed! Compiler auto-includes based on @export decorator
@export
fn greet(name: string) -> string { ... }
```

**Proposed Option 2 - Windjammer Namespaces**:
```windjammer
use std::wasm     // Windjammer's WASM abstraction
use std::web      // Web APIs (maps to web_sys)
use std::js       // JavaScript interop (maps to js_sys)
```

**Rationale**:
- Hide Rust crate structure
- Allow backend implementation to change
- Make imports optional when possible

---

### Standard Library Structure

**Proposed Namespace Hierarchy**:
```
std/
  ├── wasm/         → WASM-specific functionality
  ├── web/          → Browser APIs (DOM, Canvas, etc.)
  ├── js/           → JavaScript interop
  ├── http/         → HTTP client/server (wraps reqwest, hyper)
  ├── json/         → JSON serialization (wraps serde_json)
  ├── fs/           → File system (wraps std::fs)
  ├── path/         → Path manipulation
  ├── io/           → I/O operations
  ├── time/         → Date/time (wraps chrono)
  ├── crypto/       → Cryptography (wraps ring)
  ├── encoding/     → Base64, hex, etc.
  ├── net/          → Networking
  ├── sync/         → Concurrency primitives
  ├── testing/      → Test framework
  ├── fmt/          → Formatting
  ├── strings/      → String utilities
  ├── math/         → Math functions
  ├── collections/  → Data structures
  ├── regex/        → Regular expressions
  ├── cli/          → Command-line parsing (wraps clap)
  └── log/          → Logging (wraps tracing)
```

**Implementation Mapping** (internal, hidden from users):
```rust
// std.http → reqwest + hyper
// std.json → serde_json
// std.web  → web_sys
// std.js   → js_sys
// etc.
```

---

### Other Decorator Abstractions

**Current (BAD)**:
```windjammer
@derive(Debug, Clone, Serialize, Deserialize)
struct User {
    @serde(rename = "user_name")
    name: string,
}
```

**Proposed (GOOD)**:
```windjammer
@auto  // Infers common traits
struct User {
    @rename("user_name")  // Implementation-agnostic
    name: string,
}
```

**Decorator Mapping**:
- `@export` → `#[wasm_bindgen]` or `#[no_mangle]`
- `@test` → `#[test]`
- `@async` → `async` keyword (or future: multiple runtimes)
- `@rename(...)` → `#[serde(rename = "...")]`
- `@skip` → `#[serde(skip)]`
- `@default(...)` → `#[serde(default = "...")]`
- `@validate(...)` → validation logic (could use validator crate)

---

## Migration Path

### Phase 1: Support Both (v0.4.0)
```windjammer
// Old way still works (deprecated warning)
@wasm_bindgen
fn greet() { ... }

// New way preferred
@export
fn greet() { ... }
```

### Phase 2: Deprecate Old Way (v0.5.0)
```windjammer
// Emit deprecation warnings for @wasm_bindgen
@wasm_bindgen  // WARNING: Use @export instead
fn greet() { ... }
```

### Phase 3: Remove Old Way (v1.0.0)
```windjammer
// Only new abstractions supported
@export
fn greet() { ... }
```

---

## Implementation Strategy

### 1. Decorator Mapping Layer
Create a `decorator_to_rust()` function that maps Windjammer decorators to Rust attributes:

```rust
fn decorator_to_rust(&self, decorator: &str, target: DecoratorTarget) -> String {
    match (decorator, target) {
        ("export", DecoratorTarget::Function) => "#[wasm_bindgen]",
        ("export", DecoratorTarget::Impl) => "#[wasm_bindgen]",
        ("export", DecoratorTarget::Struct) => "#[wasm_bindgen]",
        ("test", _) => "#[test]",
        ("rename", _) => "#[serde(rename = \"...\")]",
        // Fallback: allow pass-through for now
        (name, _) => format!("#[{}]", name),
    }
}
```

### 2. Implicit Import System
When `@export` is detected, automatically inject necessary imports:

```rust
fn generate_implicit_imports(&self, items: &[Item]) -> Vec<String> {
    let mut imports = Vec::new();
    
    if has_export_decorator(items) {
        imports.push("use wasm_bindgen::prelude::*;".to_string());
    }
    
    if uses_web_apis(items) {
        imports.push("use web_sys::*;".to_string());
    }
    
    if uses_js_interop(items) {
        imports.push("use js_sys::*;".to_string());
    }
    
    imports
}
```

### 3. Standard Library Facade
Create `std/` directory with Windjammer wrappers:

```
std/
  wasm.wj     → exports @export, etc.
  http.wj     → wraps reqwest with nice API
  json.wj     → wraps serde_json
  ...
```

---

## Benefits

1. **Future-Proof**: Can swap Rust crates without breaking user code
2. **Cleaner Syntax**: More intuitive for Windjammer users
3. **Better Errors**: Can provide Windjammer-specific error messages
4. **Multiple Backends**: Could target different runtimes (Tokio, async-std, etc.)
5. **Versioning**: Can evolve APIs independently of underlying crates

---

## Examples

### Before (v0.3.0)
```windjammer
use wasm_bindgen.prelude
use web_sys

@wasm_bindgen
fn greet(name: string) -> string {
    format!("Hello, {}", name)
}

@wasm_bindgen
struct Counter {
    value: i32,
}

@wasm_bindgen
impl Counter {
    fn new() -> Counter {
        Counter { value: 0 }
    }
}
```

### After (v0.4.0+)
```windjammer
// No imports needed!

@export
fn greet(name: string) -> string {
    format!("Hello, {}", name)
}

@export
struct Counter {
    value: i32,
}

@export
impl Counter {
    fn new() -> Counter {
        Counter { value: 0 }
    }
}
```

**Or with explicit imports (if preferred)**:
```windjammer
use std::wasm  // Brings in @export and WASM utilities

@export
fn greet(name: string) -> string {
    format!("Hello, {}", name)
}
```

---

## Decision Points

1. **Decorator Name**: `@export`, `@wasm`, `@public`, or `@extern`?
   - **Recommendation**: `@export` (clear intent, not tied to WASM)

2. **Import Style**: Implicit vs explicit?
   - **Recommendation**: Implicit for WASM (zero-config), explicit for stdlib modules

3. **Backward Compatibility**: Support old syntax?
   - **Recommendation**: Yes, with deprecation warnings in v0.4.0-0.6.0

4. **Standard Library**: Wrappers or direct Rust interop?
   - **Recommendation**: Thin wrappers with Windjammer-idiomatic APIs

---

## TODO for v0.4.0

- [ ] Implement `@export` decorator
- [ ] Add implicit import injection
- [ ] Create decorator mapping layer
- [ ] Add deprecation warnings for `@wasm_bindgen`
- [ ] Update examples to use new syntax
- [ ] Start basic stdlib modules (http, json, fs)
- [ ] Update documentation
- [ ] Write migration guide

---

## Questions for Discussion

1. Should we support `@wasm_bindgen` indefinitely for Rust interop power users?
2. Should stdlib modules be opt-in (`use std::http`) or auto-imported?
3. Do we need a way to "escape" to raw Rust attributes when needed?
4. Should `@export` be the default for top-level functions in WASM projects?

---

**Status**: Draft - Ready for review and implementation
**Version**: v0.4.0
**Author**: Windjammer Team
**Date**: 2025-10-04

