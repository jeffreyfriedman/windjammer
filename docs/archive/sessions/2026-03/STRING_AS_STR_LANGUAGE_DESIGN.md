# Language Design: `.as_str()` in Windjammer

## The Question
Should Windjammer allow `.as_str()` on `string` parameters, or is this a language inconsistency?

## Cross-Backend Analysis

### Rust Backend
```rust
fn process(name: &str) {
    match name.as_str() { ... }  // ❌ E0658: Unstable feature
}
fn process(name: String) {
    match name.as_str() { ... }  // ✅ Valid
}
```

### Go Backend
```go
func process(name string) {
    switch name {  // ✅ Strings work directly
        case "warrior": ...
    }
    // NO .as_str() CONCEPT
}
```

### JavaScript Backend
```javascript
function process(name) {
    switch(name) {  // ✅ Strings work directly
        case "warrior": ...
    }
    // NO .as_str() CONCEPT
}
```

### Interpreter Backend
```
Direct pattern matching on strings - NO .as_str()
```

## Language Consistency Problem

**Current Windjammer code:**
```windjammer
pub fn from_name(name: string) -> Type {
    match name.as_str() {  // ← Rust-specific!
        "warrior" => ...,
    }
}
```

**Problems:**
1. ❌ `.as_str()` is **Rust-specific** - doesn't exist in Go/JS/Interpreter
2. ❌ Users must **understand Rust internals** (String vs &str) to use it correctly
3. ❌ **Inconsistent**: Sometimes needed, sometimes redundant, sometimes errors
4. ❌ **Violates "Compiler does the hard work"** - user shouldn't think about this

## The Right Design (Windjammer Philosophy)

### ✅ What Users Should Write
```windjammer
pub fn from_name(name: string) -> Type {
    match name {  // ← Clean, cross-backend, automatic
        "warrior" => ...,
    }
}
```

### ✅ What Compiler Should Generate

**Rust:**
- If `name` inferred as `&str`: `match name { ... }`
- If `name` inferred as `String`: `match name.as_str() { ... }`
- **Compiler adds `.as_str()` only when needed**

**Go:**
```go
switch name {
    case "warrior": ...
}
```

**JavaScript:**
```javascript
switch(name) {
    case "warrior": ...
}
```

## Decision: PROHIBIT `.as_str()` in Windjammer Source

### Rationale

1. **Cross-backend consistency** - Go/JS/Interpreter don't have this concept
2. **Compiler intelligence** - Compiler knows when Rust needs `.as_str()`, user doesn't need to
3. **Simpler mental model** - `string` type works directly in match, no conversion needed
4. **"Inference when it doesn't matter"** - String representation is a mechanical detail
5. **Prevents confusion** - Users won't wonder "when do I use .as_str()?"

### Implementation Plan

**Phase 1: Make it work automatically (TDD)**
- ✅ Test: Match on `string` param without `.as_str()` compiles to valid Rust
- ✅ Codegen: Auto-add `.as_str()` in Rust backend when `String` type is inferred
- ✅ Codegen: Skip `.as_str()` in Rust backend when `&str` type is inferred

**Phase 2: Lint against explicit `.as_str()` (Future)**
- Add analyzer warning: "Redundant .as_str() - Windjammer handles string conversions automatically"
- Help message: "Remove .as_str() from match statement"
- Error code: W0001 (warning, not error initially)

**Phase 3: Deprecate completely (Future)**
- Turn warning into error
- Update all game code to remove `.as_str()`
- Document in migration guide

## Comparison with Rust

**Rust forces explicitness:**
```rust
let s: String = "hello".to_string();
match s.as_str() {  // ← MUST write this
    "hello" => ...
}
```

**Windjammer infers it:**
```windjammer
let s: string = "hello"
match s {  // ← Compiler handles conversion
    "hello" => ...
}
```

This is **exactly analogous** to:
- **Rust**: `fn foo(x: &i32)` ← explicit &
- **Windjammer**: `fn foo(x: i32)` ← compiler infers & when needed

## Conclusion

**YES, prohibit `.as_str()` in Windjammer source.**

It's:
- ❌ Rust-specific (doesn't generalize to other backends)
- ❌ Exposes implementation details users shouldn't care about
- ❌ Violates "Compiler does the hard work"
- ❌ Inconsistent with Windjammer's inference philosophy

**The compiler should:**
1. **Auto-convert** `string` in match contexts (add `.as_str()` in Rust codegen when needed)
2. **Warn on explicit `.as_str()`** (Phase 2 - future)
3. **Error on explicit `.as_str()`** (Phase 3 - future, after game code updated)

**This is the Windjammer way: Simple source, smart compiler.**
