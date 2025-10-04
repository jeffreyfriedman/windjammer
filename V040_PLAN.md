# v0.4.0 Development Plan

## 🎯 Goal
Create proper abstractions to decouple Windjammer from Rust implementation details, enabling future flexibility and a cleaner user experience.

## 🚨 Problem We're Solving

**Current Issue**: We're leaking Rust crate names into Windjammer code
```windjammer
@wasm_bindgen           // ❌ Ties us to a specific Rust crate
use wasm_bindgen.prelude // ❌ Exposes Rust module structure
```

**Consequences**:
- Can't swap out backend implementations
- Breaking changes if we upgrade/replace Rust crates
- Confusing for users not familiar with Rust ecosystem
- Limits future compilation targets

## ✅ Solution

**Use Windjammer-native abstractions**:
```windjammer
@export                 // ✅ Implementation-agnostic
// No imports needed!   // ✅ Compiler handles details
```

## 📋 v0.4.0 Roadmap

### Phase 1: Core Abstractions (Week 1)

#### 1. Implement `@export` Decorator
- Replace `@wasm_bindgen` with semantic `@export`
- Support for functions, structs, impl blocks
- Backward compatibility: keep `@wasm_bindgen` working with deprecation warning

#### 2. Implicit Import System
- Detect `@export` usage and auto-inject necessary Rust imports
- No more `use wasm_bindgen.prelude` in user code
- Smart detection of web APIs, JS interop needs

#### 3. Decorator Mapping Layer
- Internal mapping: Windjammer decorators → Rust attributes
- `@export` → `#[wasm_bindgen]`
- `@test` → `#[test]`
- `@rename("x")` → `#[serde(rename = "x")]`
- Extensible for future decorators

### Phase 2: Standard Library Foundation (Week 2)

#### 4. `std/http` Module
- Wrap `reqwest` with Windjammer-friendly API
- Simple HTTP client: `http.get()`, `http.post()`, etc.
- Auto-handle common patterns (JSON, auth, etc.)

#### 5. `std/json` Module
- Wrap `serde_json` with clean API
- `json.parse()`, `json.stringify()`
- Automatic serialization with `@auto`

#### 6. `std/fs` Module
- Wrap `std::fs` and `tokio::fs`
- Async file operations
- Path utilities

### Phase 3: Polish & Documentation (Week 3)

#### 7. Update Examples
- Migrate `wasm_hello` and `wasm_game` to use `@export`
- Create HTTP server example using `std/http`
- Create JSON API example

#### 8. Documentation
- Migration guide: v0.3.0 → v0.4.0
- Update GUIDE.md with new patterns
- Document all stdlib modules
- Philosophy doc on abstraction decisions

#### 9. Testing
- Test `@export` in all contexts
- Test implicit imports
- Test backward compatibility
- Add stdlib integration tests

## 🎨 Design Decisions

### 1. Decorator Naming
**Chosen**: `@export`

**Rationale**:
- Semantic: describes what you want (export to another language)
- Not tied to WASM specifically
- Could apply to C FFI, Python, JavaScript, etc.

**Alternatives considered**:
- `@wasm` - too specific
- `@public` - confuses with Rust's `pub`
- `@extern` - confusing with Rust extern
- `@ffi` - too low-level

### 2. Import Strategy
**Chosen**: Implicit (zero-config) for WASM, explicit for stdlib

**Example**:
```windjammer
// WASM: no imports needed
@export
fn greet() { ... }

// Stdlib: explicit
use std.http

fn main() {
    http.get("https://api.example.com")
}
```

**Rationale**:
- WASM is simple: just add `@export`
- Stdlib needs clarity about what's being used
- Can evolve to implicit stdlib later if desired

### 3. Backward Compatibility
**Chosen**: Support `@wasm_bindgen` with deprecation warnings in v0.4.0-0.6.0, remove in v1.0.0

**Migration Path**:
- v0.4.0: Both work, deprecation warning for old syntax
- v0.5.0-0.6.0: Continue deprecation warnings
- v1.0.0: Only new syntax supported

## 📊 Success Metrics

- [ ] All examples work with `@export` (no `@wasm_bindgen`)
- [ ] No Rust crate names in user-facing code
- [ ] At least 3 stdlib modules working (http, json, fs)
- [ ] Deprecation warnings emit correctly
- [ ] All tests passing
- [ ] Documentation complete

## 🚀 Implementation Order

1. ✅ Create design document (`docs/ABSTRACTIONS.md`)
2. Add `@export` decorator support to parser
3. Implement decorator mapping in codegen
4. Add implicit import injection
5. Update WASM examples to use `@export`
6. Test and verify
7. Create `std/http` module
8. Create `std/json` module  
9. Create `std/fs` module
10. Write migration guide
11. Update all documentation
12. Final testing and polish

## 💡 Future Considerations (v0.5.0+)

- More stdlib modules (time, crypto, regex, cli, log)
- Multiple compilation targets (Node.js, Deno, native)
- Plugin system for custom decorators
- Better error messages with Windjammer line numbers
- Performance benchmarks

## 🤔 Open Questions

1. **Escape hatch**: Should we allow raw Rust attributes via `@rust("attribute")`?
   - Pro: Power users can do anything
   - Con: Defeats purpose of abstractions

2. **Auto-detection**: Should we auto-detect WASM target from project structure?
   - If `Cargo.toml` has `crate-type = ["cdylib"]`, auto-enable WASM mode?

3. **Stdlib size**: Should stdlib be monolithic or modular?
   - Monolithic: Easier, everything works out of box
   - Modular: Smaller binaries, opt-in

4. **Naming conflicts**: What if user has `std` module?
   - Reserved keywords list?
   - Namespace prefix like `windjammer.std`?

## 📝 Notes

- Keep implementation simple and testable
- Prioritize user experience over implementation complexity
- Document all design decisions
- Write migration scripts if needed
- Consider adding `windjammer migrate` command to auto-upgrade code

---

**Status**: Ready to begin implementation  
**Target**: v0.4.0 release in ~3 weeks  
**Branch**: `feature/v0.4.0-stdlib-and-abstractions`

