# Windjammer Semantic Contract

## Purpose

This document defines the **observable semantics** of the Windjammer language,
independent of any compilation backend. All backends (Rust, Go, future targets)
must produce programs that conform to these semantics.

The contract is verified by the **conformance test suite** in `tests/conformance/`.

## Core Principles

1. **Same source, same behavior** — a Windjammer program must produce identical
   observable output regardless of backend.
2. **Safety is enforced by the analyzer** — the Windjammer compiler checks
   mutability, ownership, and exhaustiveness before any backend runs.
3. **Backends differ only in performance characteristics** — runtime speed, binary
   size, and compilation time may differ, but behavior must not.

---

## 1. Type System

### 1.1 Primitive Types

| Windjammer Type | Semantics | Copy? |
|-----------------|-----------|-------|
| `int`           | 64-bit signed integer | Yes |
| `float`         | 64-bit IEEE 754 | Yes |
| `bool`          | `true` / `false` | Yes |
| `string`        | UTF-8, immutable content | No |
| `char`          | Unicode scalar value | Yes |

**Contract:** Arithmetic on `int` and `float` must produce identical results
across backends (no silent truncation, no platform-dependent rounding for basic ops).

### 1.2 Composite Types

| Type | Semantics |
|------|-----------|
| `Vec<T>` | Growable array, heap-allocated |
| `HashMap<K, V>` | Key-value map, unordered |
| `Option<T>` | `Some(T)` or `None` |
| `Result<T, E>` | `Ok(T)` or `Err(E)` |
| Structs | Named product types |
| Enums | Tagged unions |

**Contract:** All composite types are non-Copy unless all fields are Copy
(and the type has no heap-allocated fields).

### 1.3 Auto-Derive

The compiler automatically derives traits for user-defined types:
- `Copy` + `Clone`: if all fields are Copy
- `Clone`: if all fields are Clone (but not all Copy)
- `Debug`: always
- `PartialEq` + `Eq`: if all fields implement equality
- `Default`: if all fields implement Default

---

## 2. Variable Bindings

### 2.1 Immutability by Default

```windjammer
let x = 5        // immutable — cannot be reassigned or mutated
let mut y = 10   // mutable — can be reassigned and mutated
```

**Contract:**
- `let` bindings are immutable. Attempting to reassign, compound-assign,
  mutate a field, or call a mutating method is a **compile-time error**.
- `let mut` bindings are mutable. All mutation operations are allowed.
- The compiler emits helpful error messages suggesting `let mut` when mutation
  of an immutable binding is detected.

### 2.2 Shadowing

```windjammer
let x = 5
let x = x + 1    // shadows the previous x
```

**Contract:** `let` re-declarations in the same scope shadow previous bindings.
The shadowed binding is a new, independent variable.

---

## 3. Ownership and Borrowing

### 3.1 Automatic Ownership Inference

Windjammer does **not** require explicit `&`, `&mut`, or ownership annotations
on function parameters. The compiler infers the correct ownership mode.

**Inference rules:**
- **`&self`** (borrowed): when `self` is only read
- **`&mut self`** (mutably borrowed): when `self.field` is mutated or mutating methods called
- **`self`** (owned): when `self` is returned, used in binary ops (Copy types),
  or a non-Copy field is moved out (e.g., `return self.content`)
- **`&param`** (borrowed): when parameter is only read (non-Copy types)
- **`&mut param`** (mutably borrowed): when parameter fields are mutated
- **Owned param**: when parameter is a Copy type used in operators, or in trait impls

### 3.2 For-Loop Borrow Inference

```windjammer
let items: Vec<int> = vec![1, 2, 3]
for item in items {
    println("{}", item)
}
let n = items.len()   // items used after loop → auto-borrow
```

**Contract:**
- If the collection is used after the loop → auto-insert borrow (`&`)
- If the collection is NOT used after the loop → allow consumption (move)
- Field access iterables (e.g., `self.items`) are always borrowed
- Ranges are never affected

---

## 4. Control Flow

### 4.1 If / Else

```windjammer
if condition {
    // then branch
} else if other {
    // else-if branch
} else {
    // else branch
}
```

**Contract:** `if` is an expression when used as the last statement or in assignments.

### 4.2 Loops

```windjammer
for item in collection { ... }
for i in 0..10 { ... }
while condition { ... }
loop { ... }
```

**Contract:**
- `for` iterates over collections, ranges, or iterators
- `while` evaluates condition before each iteration
- `loop` is an infinite loop (exit with `break`)
- `break` and `continue` work in all loop types

### 4.3 Pattern Matching

```windjammer
match value {
    Pattern1 => expr1,
    Pattern2 => expr2,
    _ => default_expr,
}
```

**Contract:**
- Match must be exhaustive (all cases covered, enforced at compile time)
- Match is an expression
- Supports: literal patterns, identifier binding, struct destructuring,
  enum variant matching, wildcard `_`, tuple patterns

---

## 5. Functions

### 5.1 Declaration

```windjammer
fn name(param1: Type1, param2: Type2) -> ReturnType {
    // body
}
```

**Contract:**
- Last expression is the implicit return value (no `return` needed)
- Explicit `return` is also supported
- Parameters have automatic ownership inference (see Section 3)

### 5.2 Methods

```windjammer
impl MyStruct {
    fn method(self, other_param: int) -> int {
        self.field + other_param
    }
}
```

**Contract:**
- `self` ownership is automatically inferred
- Methods are called with dot notation: `obj.method(arg)`

---

## 6. Traits

```windjammer
trait Renderable {
    fn render(self) -> string
}

impl Renderable for MyStruct {
    fn render(self) -> string {
        self.name
    }
}
```

**Contract:**
- Trait methods define the interface
- Implementations must match the trait method signatures
- Default method implementations are supported
- Self ownership in trait methods follows the same inference rules

---

## 7. Standard Library

### 7.1 I/O

```windjammer
println("Hello, world!")
println("{} + {} = {}", a, b, a + b)
```

**Contract:**
- `println` writes to stdout with a trailing newline
- `print` writes to stdout without trailing newline
- Format strings use `{}` for interpolation

### 7.2 Collections

```windjammer
let mut v: Vec<int> = Vec::new()
v.push(1)
let n = v.len()

let mut m: HashMap<string, int> = HashMap::new()
m.insert("key", 42)
```

**Contract:** Collection operations must behave identically across backends.
Iteration order for `HashMap` is **not guaranteed** to be consistent
across backends (hash algorithms may differ).

---

## 8. Error Model

### 8.1 Compile-Time Errors (Analyzer)

These errors are caught **before** any backend runs:
- Immutability violations (mutating `let` binding)
- Exhaustiveness failures (non-exhaustive match)
- Type mismatches
- Unknown identifiers

### 8.2 Runtime Behavior

- Integer overflow: wrapping (consistent with Rust release mode)
- Array out-of-bounds: panic
- Division by zero: panic
- Unwrap on None/Err: panic

**Contract:** Runtime panics must produce a message identifying the error.
The exact format may differ between backends, but the error type must match.

---

## 9. Conformance Verification

### Test Format

Each conformance test is a `.wj` file with:
1. Expected output in comments (`// EXPECTED OUTPUT:`)
2. A `main()` function that exercises the feature
3. Output lines tagged with `[test_name]` for identification
4. A `PASSED` marker at the end

### Running Tests

```bash
cd tests/conformance
./run_conformance_tests.sh
```

The script:
1. Compiles each `.wj` file with `wj build`
2. Builds the generated target code
3. Runs the binary and captures stdout
4. Compares against expected output

When multiple backends are available, the script compiles to each backend
and asserts identical stdout.

---

## Version History

| Version | Changes |
|---------|---------|
| 0.41.0  | Initial semantic contract. Immutability enforcement, for-loop borrow inference, owned self for non-Copy field moves. |
