# Windjammer Go Backend: Detailed Design Plan

**Status:** Planning (not yet implementing)  
**Author:** Design discussion, Feb 2026  
**Prerequisites:** Stable Rust backend, formalized semantic contract  

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Motivation](#motivation)
3. [Prerequisites — Why Not Yet](#prerequisites--why-not-yet)
4. [Core Design Principle: Analyzer as Safety Layer](#core-design-principle-analyzer-as-safety-layer)
5. [Windjammer Semantic Contract](#windjammer-semantic-contract)
6. [Language Feature Mapping: Windjammer → Go](#language-feature-mapping-windjammer--go)
7. [Architecture](#architecture)
8. [Conformance Testing Strategy](#conformance-testing-strategy)
9. [Known Divergence Points & Mitigations](#known-divergence-points--mitigations)
10. [Implementation Phases](#implementation-phases)
11. [Open Questions](#open-questions)
12. [Decision: Why Not an Interpreter](#decision-why-not-an-interpreter)
13. [Future: Additional Backends](#future-additional-backends)

---

## Executive Summary

Windjammer currently transpiles to Rust, inheriting Rust's memory safety, performance,
and ecosystem. However, Rust's compilation speed (10–60+ seconds for non-trivial
projects) creates a slow iteration cycle during development.

This plan proposes adding a **Go backend** as an alternative transpilation target.
Go compiles in sub-second times, is memory-safe (via GC), and has a rich ecosystem.
The key design insight is that both backends would be **semantically equivalent** —
not a "loose" dev mode, but a full backend that produces identical observable behavior.

**The critical architectural decision:** Windjammer's **analyzer** enforces safety
(ownership, move semantics, mutation rules) at compile time, **before** code reaches
any backend. The backend is a translation layer, not a safety layer. This means the
Go backend doesn't sacrifice safety — it just implements the same contract differently.

---

## Motivation

### The Problem

```
Windjammer source  →  Rust source  →  rustc  →  binary
                                       ^^^^
                                    10-60+ seconds
```

During active development (especially game engine work), the edit-compile-test cycle
is dominated by `rustc` compilation time. Incremental compilation helps but doesn't
eliminate the fundamental cost.

### The Proposal

```
Development:   Windjammer  →  Go source  →  go build  →  binary  (<1 second)
Production:    Windjammer  →  Rust source → rustc     →  binary  (full safety + perf)
```

Developers iterate quickly with the Go backend during development, then compile with
the Rust backend for production releases. Both backends produce programs with identical
observable behavior.

### Why Transpilation, Not Interpretation

An interpreter was considered and rejected. Transpilation to a real compiled language
gives you:

| Benefit | Interpreter | Go Backend | Rust Backend |
|---------|:-----------:|:----------:|:------------:|
| Fast iteration | ✅ Instant | ✅ Sub-second | ❌ Slow |
| Real performance | ❌ 10-100x slower | ✅ Fast | ✅ Fastest |
| Ecosystem access | ❌ None | ✅ Go ecosystem | ✅ Rust ecosystem |
| Binary distribution | ❌ Need runtime | ✅ Static binary | ✅ Static binary |
| Hardware access (GPU, audio) | ❌ Need FFI bridge | ✅ Via cgo or pure Go | ✅ Via crates |
| Real concurrency | ❌ Build from scratch | ✅ Goroutines | ✅ Tokio/threads |
| Memory safety | ❓ Must build GC | ✅ GC | ✅ Borrow checker |

The interpreter only wins on iteration speed, and loses everything else. For a game
engine project that needs hardware access, real performance, and ecosystem libraries,
transpilation is the correct approach.

---

## Prerequisites — Why Not Yet

**The Go backend should NOT be implemented until the Rust backend is stable.** Here's why:

### 1. The Analyzer Is Still Evolving

The ownership inference engine — which would become the shared safety layer for both
backends — is still being actively debugged through dogfooding. Recent sessions have
fixed bugs in:

- Operator precedence and ownership
- Array indexing parameter inference
- Parameter mutability detection
- Trait implementation parameter signatures
- Binary operation ownership for Copy types
- Self field access patterns
- Nested field mutation
- Assignment detection edge cases

Each of these fixes changes what "correct Windjammer behavior" means. Adding a Go
backend now means every analyzer fix requires updates to TWO code generators instead
of one.

### 2. The Semantic Contract Is Implicit

Right now, Windjammer's semantics are defined by "whatever the Rust backend generates."
There's no formal specification of what move semantics, mutation rules, or evaluation
order should be. Before adding a second backend, we need to **formalize** these
semantics so both backends can implement the same contract. Building the Rust backend
to completion forces us to discover and resolve all the edge cases that the formal
contract needs to cover.

### 3. Language Features Are Still Being Added

Pattern matching, traits, generics, closures, the module system — all are still being
refined. Adding a second backend multiplies the implementation cost of every feature
change by 2x. It's far more efficient to stabilize features on one backend first.

### 4. The Game Engine Isn't Compiling Yet

The primary dogfooding target (windjammer-game) still has compilation errors. Until
it compiles cleanly on the Rust backend, we don't have confidence that the language
features are complete enough to warrant a second backend.

### Readiness Criteria

The Go backend implementation should begin when:

- [ ] **Windjammer-game compiles cleanly** on the Rust backend (zero errors)
- [ ] **Analyzer ownership inference is stable** (no bug fixes for 2+ weeks)
- [ ] **Core language features are complete**: enums, pattern matching, traits,
      generics, closures, error handling, module system
- [ ] **Semantic contract is formalized** (see section below)
- [ ] **Conformance test suite exists** (backend-independent tests)

**Estimated timeline to readiness:** When the Rust backend reaches "1.0-ish" stability.

---

## Core Design Principle: Analyzer as Safety Layer

### The Key Insight

In most languages, safety is enforced by the target:
- Rust enforces memory safety at compile time (borrow checker)
- Go enforces memory safety at runtime (garbage collector)
- C enforces... nothing (programmer's problem)

In Windjammer, **the analyzer IS the safety layer**. It runs before any backend sees
the code:

```
Windjammer Source
       │
       ▼
┌──────────────┐
│    Lexer     │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│    Parser    │
└──────┬───────┘
       │
       ▼
┌──────────────────────────────────────────────────────────────┐
│                       ANALYZER                                │
│                                                               │
│  ✅ Ownership inference (who owns what)                       │
│  ✅ Move detection (value consumed → can't reuse)             │
│  ✅ Mutation analysis (what gets modified)                     │
│  ✅ Type checking (types match, generics resolve)             │
│  ✅ Exhaustiveness checking (match covers all cases)          │
│  ✅ Borrow conflict detection (no aliased mutation)           │
│                                                               │
│  If the analyzer rejects the code, NO backend sees it.        │
│  If the analyzer approves the code, ANY backend can safely    │
│  generate it.                                                 │
└──────────────────────┬───────────────────────────────────────┘
                       │
          ┌────────────┼────────────┐
          │            │            │
          ▼            ▼            ▼
   ┌────────────┐ ┌─────────┐ ┌──────────┐
   │ Rust       │ │ Go      │ │ Future   │
   │ Backend    │ │ Backend │ │ Backends │
   │            │ │         │ │          │
   │ &/&mut/own │ │ */value │ │ ...      │
   │ zero-cost  │ │ GC      │ │          │
   │ no runtime │ │ fast    │ │          │
   └────────────┘ └─────────┘ └──────────┘
```

### What This Means in Practice

**The analyzer rejects unsafe code regardless of backend:**

```windjammer
fn bad_example() {
    let items = vec![1, 2, 3]
    consume(items)       // items moved here
    println(items.len()) // ❌ ANALYZER ERROR: use of moved value 'items'
}
```

This error fires during analysis, **before** Rust codegen or Go codegen ever runs.
The Go backend doesn't get a chance to silently accept this (which raw Go would).

**The analyzer approves safe code for any backend:**

```windjammer
fn good_example() {
    let items = vec![1, 2, 3]
    let n = read_only(items)  // items borrowed (analyzer knows: read-only)
    println(items.len())      // ✅ items still available
}
```

The Rust backend generates `read_only(&items)`. The Go backend generates
`readOnly(items)` (Go slices are reference types). Both preserve the semantic: items
is read, not consumed, and remains available.

### Compile-Time Safety Checklist

The analyzer must enforce ALL of these before any backend runs:

| Safety Property | Enforcement Point |
|-----------------|-------------------|
| Use-after-move | Analyzer: track moved variables, reject reuse |
| Mutation of immutable binding | Analyzer: track `let` vs `let mut` semantics |
| Type mismatches | Analyzer: type inference + checking |
| Exhaustive pattern matching | Analyzer: coverage analysis |
| Aliased mutable references | Analyzer: borrow conflict detection |
| Uninitialized variables | Analyzer: definite assignment analysis |
| Unreachable code | Analyzer: control flow analysis |
| Null/None access | Analyzer: Option types enforced |

If any of these checks currently live in the Rust codegen (rather than the analyzer),
they must be **lifted to the analyzer** before the Go backend is implemented. This is
Phase 0 of implementation.

---

## Windjammer Semantic Contract

This section defines what Windjammer programs MEAN, independent of backend. Both the
Rust and Go backends must preserve these semantics.

### 1. Value Semantics

**Copy types** (int, int32, uint, float, bool): Always independent copies. Assigning
or passing a copy type creates a new, independent value.

```windjammer
let a = 42
let b = a    // b is an independent copy
b = 99       // a is still 42
```

**Move types** (string, Vec, structs, enums): Passing or assigning transfers ownership.
The original binding becomes invalid.

```windjammer
let name = "hello"
let other = name     // name is moved to other
// name is no longer valid here (analyzer enforces this)
```

**Clone**: Explicitly creating an independent copy of a move type.

```windjammer
let name = "hello"
let other = name.clone()  // explicit copy
// both name and other are valid
```

### 2. Parameter Passing

The analyzer determines how each parameter is used, then each backend implements the
optimal passing strategy:

| Analyzer Determination | Rust Backend | Go Backend |
|------------------------|-------------|------------|
| Read-only, copy type | `param: i32` (by value) | `param int` (by value) |
| Read-only, move type | `param: &String` (borrow) | `param string` (Go strings are immutable) |
| Read-only, struct | `param: &MyStruct` (borrow) | `param MyStruct` (by value) or `param *MyStruct` (pointer, for large structs) |
| Mutated | `param: &mut T` (mut borrow) | `param *T` (pointer) |
| Consumed (moved) | `param: T` (owned) | `param T` (by value, caller's copy invalidated by analyzer) |

The **observable behavior** is identical:
- Read-only params: caller's value unchanged after call
- Mutated params: caller sees the mutation
- Consumed params: caller can't use the value anymore (analyzer enforces)

### 3. Mutation Rules

- `let` bindings are immutable by default
- Mutation requires either `let mut` or the analyzer detecting mutation through method calls
- Struct fields can only be mutated through `&mut self` methods (analyzer infers this)
- The analyzer tracks mutation through all code paths

### 4. Move Semantics

- Non-copy values are moved on assignment, function call (when consumed), or return
- After a move, the source binding is invalid (analyzer error to reuse)
- The analyzer tracks moves through control flow (if/else, match, loops)
- Conditional moves are flagged as errors (value might or might not be moved)

### 5. Error Handling

- `Result<T, E>` represents fallible operations
- `Option<T>` represents nullable values
- `?` operator propagates errors
- Both backends must implement these types with identical behavior

**Rust backend:** Maps directly to `std::result::Result` and `std::option::Option`.  
**Go backend:** Generates wrapper types:

```go
type Result[T any] struct {
    value T
    err   error
    isOk  bool
}

type Option[T any] struct {
    value T
    isSome bool
}
```

### 6. Pattern Matching

- Must be exhaustive (analyzer checks this)
- Bindings in patterns introduce new variables
- Guards (`if` conditions in match arms) are evaluated left-to-right
- First matching arm wins

### 7. Evaluation Order

- Expressions evaluate left-to-right
- Function arguments evaluate left-to-right
- Short-circuit evaluation for `&&` and `||`
- Deterministic across all backends

### 8. Concurrency (Future)

Windjammer's concurrency primitives will be defined at the language level:
- Channels (maps to Rust `mpsc` / Go `chan`)
- Spawn (maps to Rust `thread::spawn` / Go `go`)
- Mutex/Lock (maps to Rust `Mutex` / Go `sync.Mutex`)

Raw shared mutable state is not permitted in Windjammer. All sharing goes through
language-defined primitives, ensuring both backends are data-race-safe.

---

## Language Feature Mapping: Windjammer → Go

### Primitive Types

| Windjammer | Rust | Go |
|-----------|------|-----|
| `int` | `i64` | `int64` |
| `int32` | `i32` | `int32` |
| `uint` | `u64` | `uint64` |
| `float` | `f64` | `float64` |
| `bool` | `bool` | `bool` |
| `string` | `String` | `string` |

### Composite Types

| Windjammer | Rust | Go |
|-----------|------|-----|
| `Vec<T>` | `Vec<T>` | `[]T` |
| `[T; N]` | `[T; N]` | `[N]T` |
| `HashMap<K, V>` | `HashMap<K, V>` | `map[K]V` |
| `Option<T>` | `Option<T>` | `*T` or wrapper type |
| `Result<T, E>` | `Result<T, E>` | `(T, error)` or wrapper type |
| `(T1, T2)` (tuple) | `(T1, T2)` | Generated struct |

### Structs

```windjammer
struct Player {
    name: string,
    hp: int,
    position: Vec2,
}
```

**Rust:**
```rust
struct Player {
    name: String,
    hp: i64,
    position: Vec2,
}
```

**Go:**
```go
type Player struct {
    Name     string
    Hp       int64
    Position Vec2
}
```

Note: Go exports fields with capital first letter. Internal field access in generated
code uses the capitalized names. Source maps track the mapping.

### Enums (Algebraic Data Types)

```windjammer
enum Shape {
    Circle(radius: float),
    Rect(width: float, height: float),
    Point,
}
```

**Rust:**
```rust
enum Shape {
    Circle { radius: f64 },
    Rect { width: f64, height: f64 },
    Point,
}
```

**Go:**
```go
type Shape interface {
    isShape()
}

type ShapeCircle struct {
    Radius float64
}
func (ShapeCircle) isShape() {}

type ShapeRect struct {
    Width  float64
    Height float64
}
func (ShapeRect) isShape() {}

type ShapePoint struct{}
func (ShapePoint) isShape() {}
```

### Pattern Matching

```windjammer
match shape {
    Circle(r) => 3.14159 * r * r,
    Rect(w, h) => w * h,
    Point => 0.0,
}
```

**Rust:**
```rust
match shape {
    Shape::Circle { radius: r } => 3.14159 * r * r,
    Shape::Rect { width: w, height: h } => w * h,
    Shape::Point => 0.0,
}
```

**Go:**
```go
func() float64 {
    switch s := shape.(type) {
    case ShapeCircle:
        return 3.14159 * s.Radius * s.Radius
    case ShapeRect:
        return s.Width * s.Height
    case ShapePoint:
        return 0.0
    default:
        panic("non-exhaustive match") // unreachable: analyzer guarantees exhaustiveness
    }
}()
```

### Traits → Interfaces

```windjammer
trait Drawable {
    fn draw(self, canvas: Canvas)
    fn bounds(self) -> Rect
}
```

**Rust:**
```rust
trait Drawable {
    fn draw(&self, canvas: &mut Canvas);
    fn bounds(&self) -> Rect;
}
```

**Go:**
```go
type Drawable interface {
    Draw(canvas *Canvas)
    Bounds() Rect
}
```

### Trait Implementations

```windjammer
impl Drawable for Circle {
    fn draw(self, canvas: Canvas) {
        canvas.draw_circle(self.center, self.radius)
    }
    fn bounds(self) -> Rect {
        Rect { x: self.center.x - self.radius, ... }
    }
}
```

**Go:** Methods on the struct (Go interfaces are implicit/structural):
```go
func (c Circle) Draw(canvas *Canvas) {
    canvas.DrawCircle(c.Center, c.Radius)
}
func (c Circle) Bounds() Rect {
    return Rect{X: c.Center.X - c.Radius, ...}
}
// Circle automatically satisfies Drawable interface
```

### Generics

```windjammer
fn first<T>(items: Vec<T>) -> Option<T> {
    if items.len() > 0 {
        return Some(items[0])
    }
    return None
}
```

**Rust:**
```rust
fn first<T: Clone>(items: &Vec<T>) -> Option<T> {
    if items.len() > 0 {
        Some(items[0].clone())
    } else {
        None
    }
}
```

**Go:**
```go
func first[T any](items []T) *T {
    if len(items) > 0 {
        v := items[0]
        return &v
    }
    return nil
}
```

### Closures

```windjammer
let doubled = items.map(|x| x * 2)
```

**Rust:**
```rust
let doubled: Vec<_> = items.iter().map(|x| x * 2).collect();
```

**Go:**
```go
doubled := make([]int64, len(items))
for i, x := range items {
    doubled[i] = x * 2
}
```

Note: Go doesn't have map/filter/reduce on slices natively (as of Go 1.23, there are
iterators in `slices` package). The Go backend may generate explicit loops for
functional-style operations.

---

## Architecture

### File Structure

```
windjammer/src/codegen/
├── backend.rs          # CodegenBackend trait, Target enum, factory
├── mod.rs              # generate() dispatcher
├── rust/               # Existing Rust backend (22 files, ~11k lines)
│   ├── backend.rs
│   ├── generator.rs
│   ├── expressions.rs
│   ├── statements.rs
│   ├── items.rs
│   ├── types.rs
│   ├── operators.rs
│   └── ...
├── go/                 # NEW: Go backend
│   ├── mod.rs          # GoBackend struct, impl CodegenBackend
│   ├── generator.rs    # Core AST traversal and Go code generation
│   ├── expressions.rs  # Expression → Go translation
│   ├── statements.rs   # Statement → Go translation
│   ├── items.rs        # Top-level items (structs, enums, functions, traits)
│   ├── types.rs        # Windjammer types → Go types
│   ├── operators.rs    # Binary/unary operators
│   ├── patterns.rs     # Pattern matching → type switches
│   ├── enums.rs        # ADT → interface + structs translation
│   └── stdlib.rs       # Go implementations of Windjammer stdlib types
├── javascript/         # Existing JavaScript backend
└── wasm.rs             # Existing WebAssembly backend
```

### Target Enum Extension

```rust
// In backend.rs
pub enum Target {
    Rust,
    Go,          // NEW
    JavaScript,
    WebAssembly,
}
```

### CLI Extension

```
wj build                      # Default: Rust backend
wj build --target rust        # Explicit: Rust backend
wj build --target go          # NEW: Go backend
wj run --target go            # NEW: Compile with Go, then run
wj check                      # Analyzer only (no backend) — validates safety
```

### Go Standard Library Shim

A small Go module that provides Windjammer's standard library types:

```
windjammer/std_go/             # Go implementations of WJ stdlib
├── go.mod
├── wj.go                     # Core types: Result, Option
├── collections.go            # Vec helpers, HashMap helpers
├── fmt.go                    # println, format, etc.
├── math.go                   # Math functions
└── ...
```

This is auto-imported by the generated Go code.

---

## Conformance Testing Strategy

The most critical part of the multi-backend architecture is proving equivalence.

### Backend-Independent Test Suite

```
windjammer/tests/conformance/
├── values/
│   ├── copy_semantics.wj      # Verify copy types are independent
│   ├── move_semantics.wj      # Verify moved values can't be reused
│   ├── clone_semantics.wj     # Verify clone creates independent copy
│   └── mutation.wj            # Verify mutation is visible to caller
├── types/
│   ├── enums.wj               # ADTs work correctly
│   ├── pattern_matching.wj    # Match expressions produce correct results
│   ├── generics.wj            # Generic functions/types work
│   └── traits.wj              # Trait dispatch works correctly
├── control_flow/
│   ├── if_else.wj             # Conditional execution
│   ├── loops.wj               # For, while, loop
│   ├── match.wj               # Pattern matching control flow
│   └── closures.wj            # Closure capture and execution
├── error_handling/
│   ├── result.wj              # Result propagation
│   ├── option.wj              # Option handling
│   └── question_mark.wj       # ? operator
└── stdlib/
    ├── vec.wj                 # Vec operations
    ├── hashmap.wj             # HashMap operations
    └── string.wj              # String operations
```

### Test Runner

Each conformance test:
1. Compiles to Rust, runs, captures output
2. Compiles to Go, runs, captures output
3. Asserts outputs are identical

```bash
wj test --conformance           # Run all conformance tests on all backends
wj test --conformance --target go  # Run conformance tests on Go only
```

### What "Identical" Means

- Same stdout output
- Same exit codes
- Same error behavior (panics at the same points)
- Floating point: within epsilon tolerance (IEEE 754 allows minor divergence)

---

## Known Divergence Points & Mitigations

These are areas where Rust and Go behavior naturally differs. Each must have a
mitigation strategy:

### 1. Use-After-Move

| | Rust | Go (raw) | Windjammer |
|---|---|---|---|
| Move then reuse | Compile error | Silently works (GC) | **Analyzer error** (before either backend) |

**Mitigation:** Analyzer rejects use-after-move. Go backend never sees this code.

### 2. Data Races

| | Rust | Go (raw) | Windjammer |
|---|---|---|---|
| Shared mutable state | Compile error (Send/Sync) | Runtime race condition | **Analyzer restricts to safe primitives** |

**Mitigation:** Windjammer only exposes channels, mutexes, and other safe concurrency
primitives. No raw shared mutable state. Both backends implement these safely.

### 3. Integer Overflow

| | Rust (release) | Go | Windjammer |
|---|---|---|---|
| Overflow behavior | Wraps silently | Wraps silently | **Wraps** (matching both) |

**Mitigation:** Both wrap. Consistent behavior. Future: add `checked_add()` etc.

### 4. Deterministic Destruction (RAII)

| | Rust | Go | Windjammer |
|---|---|---|---|
| Drop timing | Deterministic (end of scope) | Non-deterministic (GC) | **Best-effort in Go** |

**Mitigation:** For file handles, network connections, etc., the Go backend generates
`defer` statements at scope boundaries. Not perfectly deterministic, but correct for
resource cleanup. Windjammer can add explicit `defer` or `with` blocks for
resources that need deterministic cleanup.

### 5. Stack Overflow

| | Rust | Go |
|---|---|---|
| Deep recursion | Stack overflow / segfault | Goroutine stack grows dynamically |

**Mitigation:** Acceptable divergence. Go is actually safer here. Document that deeply
recursive code may behave differently.

### 6. Floating Point Precision

| | Rust | Go |
|---|---|---|
| IEEE 754 | Yes | Yes |
| Optimization reordering | Possible in release mode | Less aggressive |

**Mitigation:** Both use IEEE 754 f64. Minor differences possible in optimized Rust
builds. Conformance tests use epsilon comparison for floats.

### 7. String Representation

| | Rust | Go |
|---|---|---|
| String encoding | UTF-8, owned `String` | UTF-8, immutable `string` |
| Mutability | `String` is mutable | Strings are immutable (new string on mutation) |

**Mitigation:** The Go backend uses `strings.Builder` for mutable string operations
and returns new strings. Observable behavior (the final string value) is identical.
Performance characteristics differ but semantics match.

---

## Implementation Phases

### Phase 0: Formalize and Lift (Pre-requisite)

**Goal:** Ensure all safety checks live in the analyzer, not in the Rust codegen.

**Tasks:**
- [ ] Audit Rust codegen for any safety checks that should be in the analyzer
- [ ] Lift use-after-move detection to analyzer (verify it's there, not implicit in Rust)
- [ ] Lift mutation tracking to analyzer (verify it's complete)
- [ ] Lift exhaustiveness checking to analyzer
- [ ] Write the formal Windjammer Semantic Contract document (expand section 5 above)
- [ ] Create initial conformance test suite (10-20 core tests)

**Estimated effort:** 1-2 weeks  
**Depends on:** Stable Rust backend

### Phase 1: Go Backend Scaffold

**Goal:** Minimal Go backend that compiles trivial programs.

**Tasks:**
- [ ] Add `Target::Go` to the `Target` enum
- [ ] Create `codegen/go/mod.rs` implementing `CodegenBackend`
- [ ] Implement basic type mapping (primitives, string)
- [ ] Implement function generation (parameters, return types, bodies)
- [ ] Implement `let` bindings, assignments, return statements
- [ ] Implement basic expressions (literals, identifiers, binary ops, unary ops)
- [ ] Generate `go.mod` and `main.go`
- [ ] Wire into CLI: `wj build --target go`
- [ ] Run `go build` on generated code
- [ ] First conformance test passing on both backends

**Estimated effort:** 1-2 weeks

### Phase 2: Structs and Methods

**Goal:** Structs, impl blocks, method calls work.

**Tasks:**
- [ ] Struct declaration → Go struct
- [ ] Struct literal construction
- [ ] Field access
- [ ] Impl blocks → Go methods
- [ ] Self parameter → Go receiver (value or pointer based on mutation analysis)
- [ ] Method call generation

**Estimated effort:** 1 week

### Phase 3: Enums and Pattern Matching

**Goal:** Algebraic data types and match expressions work.

**Tasks:**
- [ ] Enum → Go interface + concrete structs pattern
- [ ] Simple pattern matching → type switch
- [ ] Nested pattern matching → nested type assertions
- [ ] Pattern bindings (extracting fields)
- [ ] Match guards → if conditions in cases
- [ ] Exhaustiveness (generate `default: panic()` for safety)

**Estimated effort:** 1-2 weeks

### Phase 4: Traits and Generics

**Goal:** Trait system and generic functions/types work.

**Tasks:**
- [ ] Trait declaration → Go interface
- [ ] Trait implementation → Go methods (implicit interface satisfaction)
- [ ] Default trait methods → generated on each implementing type
- [ ] Generic functions → Go generic functions
- [ ] Generic structs → Go generic structs
- [ ] Trait bounds → Go type constraints

**Estimated effort:** 1-2 weeks

### Phase 5: Standard Library and Error Handling

**Goal:** Windjammer stdlib works on Go backend.

**Tasks:**
- [ ] Create `std_go/` module with Go stdlib implementations
- [ ] Vec operations (push, pop, len, iter, map, filter)
- [ ] HashMap operations (insert, get, remove, iter)
- [ ] Option type (Some, None, unwrap, map, and_then)
- [ ] Result type (Ok, Err, unwrap, map, ?)
- [ ] String operations (format, split, trim, etc.)
- [ ] println/print/eprintln
- [ ] Math functions

**Estimated effort:** 1 week

### Phase 6: Closures, Iterators, Advanced Features

**Goal:** Remaining language features work.

**Tasks:**
- [ ] Closures → Go anonymous functions
- [ ] Closure capture semantics (analyzer determines what's captured)
- [ ] Iterator methods (map, filter, fold) → loops or Go iterators
- [ ] Range expressions
- [ ] Tuple types → generated structs
- [ ] Type casting

**Estimated effort:** 1 week

### Phase 7: Integration and Polish

**Goal:** Full conformance, game engine compatibility (minus FFI).

**Tasks:**
- [ ] Run full conformance test suite (target: 100% pass rate)
- [ ] Source map generation for Go output
- [ ] `go fmt` integration for idiomatic output
- [ ] Error message mapping (Go compile errors → Windjammer source locations)
- [ ] Performance benchmarking (compile time comparison)
- [ ] Documentation

**Estimated effort:** 1-2 weeks

### Total Estimated Effort: 8-12 weeks

---

## Open Questions

### 1. FFI Strategy for Go Backend

The Rust backend uses `extern fn` to call Rust crates (SDL2, etc.). The Go backend
would need a different FFI story. Options:

- **Option A:** Pure Go libraries (go-sdl2, etc.) — different library, same API
- **Option B:** Shared C interface — both backends call C, Rust via FFI, Go via cgo
- **Option C:** Abstract engine interface — Windjammer defines the API, each backend
  provides an implementation
- **Option D:** Defer this entirely — Go backend for pure logic, Rust for engine code

Recommendation: **Option D for now.** The Go backend is for fast iteration on game
LOGIC (dialogue systems, inventory, AI, etc.), not rendering. Rendering/FFI code
always compiles with Rust.

### 2. Go Module Management

Generated Go code needs a `go.mod`. Questions:
- Should `wj build --target go` auto-generate `go.mod`?
- How do we manage Go dependencies (if the user calls Go ecosystem libraries)?
- Where does the generated Go code live? (Same `build/` directory?)

### 3. Option Type Representation

Two approaches in Go:

**Pointer-based (idiomatic Go):**
```go
func find(id int64) *Player { ... }  // nil = None
```

**Wrapper type (safer, matches semantics better):**
```go
func find(id int64) Option[Player] { ... }
```

Recommendation: Wrapper type for correctness. The analyzer guarantees you handle
None/Some, which aligns with explicit Option type rather than nil checks.

### 4. Error Handling: Result vs (T, error)

**Idiomatic Go:**
```go
func parse(s string) (int64, error) { ... }
```

**Result wrapper (matches Windjammer semantics):**
```go
func parse(s string) Result[int64] { ... }
```

Recommendation: Result wrapper initially (matches semantics exactly), with possible
idiomatic Go mode later.

### 5. Performance of Generated Go Code

Go code generated from Windjammer patterns (interface-based enums, wrapper types) may
be less performant than hand-written Go. This is acceptable because:
- The Go backend is for iteration speed, not runtime performance
- Production builds use the Rust backend
- Generated code can be optimized later

### 6. Struct Field Visibility

Go uses capitalized names for exported fields. Windjammer uses snake_case. The Go
backend needs a naming convention:

```windjammer
struct Player { hit_points: int }
```

Options:
- `HitPoints` (Go convention, readable)
- `Hit_points` (preserve underscores, capitalize first letter)

Recommendation: `HitPoints` (proper Go convention). Source maps handle the mapping.

---

## Decision: Why Not an Interpreter

For the record, an interpreter was evaluated and rejected:

| Criterion | Interpreter | Go Backend | Winner |
|-----------|:-----------:|:----------:|:------:|
| Iteration speed | Instant | Sub-second | Interpreter (marginal) |
| Runtime performance | 10-100x slower | ~2x slower than Rust | **Go** |
| Ecosystem access | None | Full Go ecosystem | **Go** |
| Game engine support | Cannot run (needs FFI) | Can run pure logic | **Go** |
| Binary distribution | Need runtime | Static binary | **Go** |
| Concurrency | Must build from scratch | Goroutines | **Go** |
| Implementation effort | 3-4 weeks (basic) | 8-12 weeks | Interpreter |
| Maintenance burden | Separate runtime | Just another codegen | **Go** |
| Real-world utility | Testing/REPL only | Full development | **Go** |

The sub-second difference between "instant" and "sub-second" is negligible in practice.
Everything else favors the Go backend.

---

## Future: Additional Backends

The multi-backend architecture opens doors beyond Go:

| Backend | Value Proposition | Feasibility |
|---------|-------------------|-------------|
| **Go** | Fast iteration, GC, goroutines | This plan |
| **C** | Maximum portability, embedded systems | Medium (no GC, manual memory management challenging) |
| **Zig** | Fast compilation, no GC, C interop | High (similar to Rust but faster compile) |
| **LLVM IR** | Direct native compilation, maximum performance | High effort but ultimate goal |
| **JavaScript** | Web deployment (already partially exists) | Already in progress |

The key insight remains: **the analyzer enforces Windjammer semantics, backends just
translate.** Any new backend only needs to implement the semantic contract, not
re-implement safety checking.

---

## Summary

The Go backend is a **semantically-equivalent alternative compilation target** for
Windjammer, not a loose "dev mode." Safety is enforced by the analyzer at compile time,
before any backend runs. Both backends produce programs with identical observable
behavior, differing only in performance characteristics and available ecosystem.

**When to implement:** After the Rust backend is stable, the analyzer is mature, and
the Windjammer semantic contract is formalized.

**Why:** Sub-second compilation for rapid development iteration, with the Rust backend
available for production builds with maximum performance and safety.

**Key principle:** The compiler does the hard work, not the developer — and now the
compiler does it for multiple targets.
