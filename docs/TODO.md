# Windjammer TODO - Feature Roadmap

## 🔴 P0 - Critical Missing Features

### Assignment Statements
**Status**: Not implemented  
**Blocked Test**: `test_ownership_inference_mut_borrowed`

**What's missing**:
- Parser support for assignment expressions (`x = value`)
- Analyzer support for tracking variable mutations
- Proper `&mut` inference when variables are reassigned

**Example that doesn't work**:
```windjammer
fn increment(x: int) {
    x = x + 1  // Parse error!
}
```

**Implementation needed**:
1. Add `Assignment` statement to AST
2. Parse `identifier = expression`
3. Use `Analyzer.variables` to track mutations
4. Infer `&mut` for parameters that are reassigned

---

## 🟠 P1 - Important Features

### 1. Local Variable Ownership Tracking
**Status**: Partially implemented (field exists, not used)

**What's missing**:
- Track local variable ownership modes
- Detect when locals are moved vs borrowed
- Generate appropriate Rust code

**Example**:
```windjammer
fn process() {
    let data = vec![1, 2, 3]
    consume(data)      // Move
    // data is invalid here
}
```

### 2. Closure Capture Analysis
**Status**: Not implemented

**What's missing**:
- Detect what variables closures capture
- Determine if capture is by value, by reference, or by mutable reference
- Generate `move` keyword when needed

**Example**:
```windjammer
fn create_counter() -> fn() -> int {
    let mut count = 0
    || {
        count = count + 1  // Captures count mutably
        count
    }
}
```

### 3. Move Semantics for Local Variables
**Status**: Not implemented

**What's missing**:
- Track when local variables are moved
- Prevent use-after-move
- Generate helpful error messages

---

## 🟡 P2 - Standard Library

### Core Modules (Designed, Not Implemented)
- `std/fs` - File system operations
- `std/http` - HTTP client/server
- `std/json` - JSON parsing
- `std/testing` - Test framework

**Status**: API designed (see `std/*/API.md`), no implementation

---

## 🟢 P3 - Enhancements

### 1. Error Mapping System
**Status**: Designed (see `docs/design/error-mapping.md`)

**Goal**: Map Rust compiler errors back to Windjammer source lines

### 2. Performance Benchmarks
**Status**: Not started

**Goal**: Prove "same performance as Rust" claim with real benchmarks

### 3. Advanced Trait Features
**Status**: Designed (see `docs/design/traits.md`)

**Features**:
- Trait bound inference
- Associated types as generics
- `@auto` derive inference

### 4. Doctests
**Status**: Designed (in ROADMAP.md)

**Goal**: Rust-style code examples in documentation

---

## 🔵 P4 - Tooling

### 1. Language Server (LSP)
**Status**: Started (see `crates/windjammer-lsp/`)

**Missing**:
- Actual implementation (currently skeleton)
- Autocomplete
- Go-to-definition
- Hover tooltips

### 2. VS Code Extension
**Status**: Started (see `editors/vscode/`)

**Missing**:
- Full language support
- Debugging integration
- Testing

### 3. Package Manager
**Status**: Not started

**Goal**: `wj add serde`, dependency management

---

## 📊 Implementation Priority

**Next 2 Weeks**:
1. Implement assignment statements (P0)
2. Test all examples (verify they compile)
3. Add more test cases

**Next Month**:
1. Local variable tracking (P1)
2. Closure capture analysis (P1)
3. Start stdlib implementation (P2)

**Next Quarter**:
1. Error mapping (P3)
2. Performance benchmarks (P3)
3. Advanced trait features (P3)

---

## 🧪 Testing Status

**Current**: 8/9 tests passing (1 ignored)
- ✅ Automatic reference insertion
- ✅ String interpolation
- ✅ Pipe operator
- ✅ Ternary operator
- ✅ Smart @auto derive
- ✅ Structs and impl blocks
- ✅ Combined features
- ✅ Ownership inference (borrowed)
- ❌ Ownership inference (mut borrowed) - blocked by assignment statements

**Examples**: Not tested systematically
- `examples/hello_world/` - Unknown
- `examples/http_server/` - Unknown
- `examples/wasm_game/` - Unknown
- `examples/cli_tool/` - Unknown

**Goal**: 100% test coverage, all examples compile

---

## 💭 Open Questions

1. **Assignment operator precedence**: How does `x = y = 5` work?
2. **Compound assignments**: Do we want `+=`, `-=`, etc.?
3. **Destructuring assignment**: `(x, y) = (1, 2)`?
4. **Pattern matching in let**: `let Some(x) = opt else { return }`?

---

## 📝 Notes

- The `Analyzer.variables` field exists but is unused (hence the warning)
- Once we implement assignments, we'll use it to track local variable states
- This will unlock proper `&mut` inference and move semantics

**Why the warning exists**: We added the field for future use but haven't implemented the features that need it yet.

