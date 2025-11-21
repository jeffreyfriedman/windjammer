# Windjammer v0.11.0 Development Plan

## 🎯 Theme: Advanced Inference & Effect System

**Goal**: Extend inference to return types, error handling, and effects (Send/Sync)

**Timeline**: 2-3 weeks  
**Branch**: `feature/v0.11.0-enhanced-inference`

---

## 📋 Features Overview

| Feature | Priority | Complexity | Status |
|---------|----------|------------|--------|
| Named Bound Sets | High | Low | 🔜 Planned |
| Implicit Return Types (Phase 1) | High | High | 🔜 Planned |
| Smart Error Propagation | High | Medium | 🔜 Planned |
| Effect Inference (Send/Sync) | Medium | High | 🔜 Planned |
| More Stdlib Modules | Medium | Medium | 🔜 Planned |
| Additional Decorators | Low | Low | 🔜 Planned |

---

## 🏗️ Phase 1: Named Bound Sets (Deferred from v0.10.0)

### Goal

Reduce boilerplate for common trait combinations.

```windjammer
// Define once:
bound Printable = Display + Debug
bound Copyable = Clone + Copy
bound Serializable = Serialize + Deserialize

// Use everywhere:
fn process<T: Printable>(value: T) { ... }
fn store<T: Copyable>(value: T) { ... }
```

### Implementation

**Lexer**:
- Add `Token::Bound` keyword

**Parser**:
- Add `Item::BoundAlias { name: String, traits: Vec<String> }`
- Parse `bound Name = Trait + Trait`

**Codegen**:
- Store bound aliases in generator
- Expand aliases when generating type parameters
- Don't generate `type Name = ...` in Rust (it's just a compiler alias)

**Example 38**: Named Bound Sets

---

## 🏗️ Phase 2: Implicit Return Types

### Goal

Infer return types from function body (starting with simple cases).

```windjammer
// Write this:
fn add(a: int, b: int) {
    a + b  // Infers -> int
}

// Write this:
fn divide(a: int, b: int) {
    if b == 0 {
        Err("division by zero")
    } else {
        Ok(a / b)
    }
}
// Infers -> Result<int, string>
```

### Implementation Strategy

**Phase 2.1: Simple Cases**
- Infer from single-expression bodies
- Infer from explicit return statements
- Infer from if/match expressions

**Phase 2.2: Result Inference**
- Detect `Ok(...)` and `Err(...)` patterns
- Infer `Result<T, E>` return type
- Handle `?` operator

**Phase 2.3: Option Inference**
- Detect `Some(...)` and `None` patterns
- Infer `Option<T>` return type

### Challenges

1. **Type unification**: Multiple return paths must agree
2. **Recursive functions**: Need fixpoint iteration or explicit annotation
3. **Generic returns**: May need type hints
4. **Error messages**: Must be clear when inference fails

### Approach

- Start conservative: only infer for simple cases
- Require explicit return type for complex cases
- Provide helpful error messages

---

## 🏗️ Phase 3: Smart Error Propagation

### Goal

Automatically infer `Result<T, E>` return type when `?` operator is used.

```windjammer
// Write this:
fn read_config(path: string) {
    let content = fs::read_to_string(path)?  // Uses ?
    let config = json::parse(content)?       // Uses ?
    config
}

// Get this:
fn read_config(path: string) -> Result<Config, Error> { ... }
```

### Implementation

1. **Detect `?` operator in function body**
2. **Infer error type from `?` expressions**
3. **Infer success type from return value**
4. **Generate `Result<T, E>` return type**

### Edge Cases

- Multiple error types: may need `Box<dyn Error>` or explicit type
- No `?` but returns Result: keep as is
- Mix of `?` and non-Result returns: error

---

## 🏗️ Phase 4: Effect Inference (Send/Sync)

### Goal

Automatically infer `Send` and `Sync` bounds for concurrent code.

```windjammer
fn process_parallel<T>(items: Vec<T>) {
    spawn {
        // Closure uses T - compiler infers T: Send
        for item in items {
            process_item(item)
        }
    }
}
// Infers: fn process_parallel<T: Send>(items: Vec<T>)
```

### Implementation

1. **Detect `go` blocks (thread spawn)**
2. **Analyze captured variables**
3. **Infer `Send` for moved values**
4. **Infer `Sync` for shared references**
5. **Infer `'static` for thread spawn requirements**

### Challenges

- Closure capture analysis
- Lifetime requirements
- Arc/Mutex detection

---

## 🏗️ Phase 5: More Stdlib Modules

### Goal

Expand stdlib to cover more common use cases.

**New Modules**:
- `std/async`: Async runtime utilities (tokio wrapper)
- `std/db`: Database access (sqlx wrapper)
- `std/env`: Environment variables
- `std/process`: Process management
- `std/random`: Random number generation

**Example 39**: Stdlib async module  
**Example 40**: Stdlib database module

---

## 🏗️ Phase 6: Additional Decorators

### Goal

More decorators for common patterns.

### @benchmark

```windjammer
@benchmark
fn expensive_operation() {
    // Generates Criterion benchmark
}
```

### @memoize

```windjammer
@memoize
fn fibonacci(n: int) -> int {
    // Automatically cached
}
```

### @derive

```windjammer
@derive(Clone, Debug, PartialEq)
struct Point { x: int, y: int }
```

**Example 41**: Advanced decorators

---

## ✅ Success Criteria

### Must Have (Release Blockers)
- [ ] Named bound sets working
- [ ] Simple return type inference (single-expression bodies)
- [ ] Result inference from `Ok`/`Err` patterns
- [ ] Smart error propagation (infer from `?`)
- [ ] At least 2 new stdlib modules
- [ ] Examples 38-40
- [ ] Zero test failures
- [ ] Zero clippy warnings
- [ ] Documentation updates

### Should Have (Nice to Have)
- [ ] Send/Sync inference (or defer to v0.12.0)
- [ ] @benchmark decorator
- [ ] Example 41
- [ ] Performance benchmarks

### Could Have (Future)
- [ ] @memoize decorator (complex - needs runtime)
- [ ] Full effect system (v0.12.0+)
- [ ] Cross-function return type inference

---

## 🧪 Testing Strategy

### Return Type Inference Tests

```windjammer
// Test 1: Simple expression
fn double(x: int) { x * 2 }  // -> int

// Test 2: If expression
fn abs(x: int) { if x < 0 { -x } else { x } }  // -> int

// Test 3: Result from Ok/Err
fn parse(s: string) {
    if s == "42" { Ok(42) } else { Err("parse error") }
}  // -> Result<int, string>
```

### Error Propagation Tests

```windjammer
fn chain_operations() {
    let x = operation1()?
    let y = operation2(x)?
    Ok(y)
}  // -> Result<Y, Error>
```

---

## 📊 Development Phases

### Week 1: Foundation
- **Days 1-2**: Named bound sets (lexer, parser, codegen)
- **Days 3-5**: Simple return type inference (single expressions)
- **Milestone**: Named bounds working, simple return inference

### Week 2: Advanced Inference
- **Days 6-8**: Result/Option inference from patterns
- **Days 9-10**: Smart error propagation from `?`
- **Milestone**: Error propagation working

### Week 3: Polish & Ship
- **Days 11-13**: Stdlib modules (async, db/env)
- **Days 14-15**: Examples, documentation, testing
- **Milestone**: Ready to merge

---

## 🚀 Release Checklist

Before merging to main:

- [ ] All features implemented and tested
- [ ] Examples 38-41 work
- [ ] Zero test failures
- [ ] Zero clippy warnings
- [ ] `cargo fmt --all` clean
- [ ] Documentation updated:
  - [ ] README.md
  - [ ] CHANGELOG.md
  - [ ] GUIDE.md
- [ ] Performance benchmarks run
- [ ] PR comment prepared
- [ ] Release notes written

---

## 🔮 Looking Ahead to v0.12.0

Features planned for v0.12.0:
- **Full Effect System**: Complete Send/Sync/Static inference
- **Lifetime Inference**: Simple cases beyond Rust's elision
- **Cross-Function Inference**: Propagate constraints across calls
- **More Advanced Decorators**: @memoize, @retry, @timeout
- **Web Framework Integration**: @route, @middleware

---

## 📚 References

- `docs/INFERENCE_DESIGN.md` - Trait bound inference (v0.10.0)
- Rust Book: Chapter 10 (Generics, Traits, Lifetimes)
- Rust Book: Chapter 9 (Error Handling with Result and ?)
- Previous PRs: v0.10.0 (trait bound inference, decorators)

---

**Status**: Planning Complete ✅  
**Next Step**: Begin Day 1 - Named Bound Sets  
**Branch**: `feature/v0.11.0-enhanced-inference`  
**Target Release**: 2-3 weeks from start

**Core Philosophy**: Continue progressive disclosure - infer more, annotate less.

