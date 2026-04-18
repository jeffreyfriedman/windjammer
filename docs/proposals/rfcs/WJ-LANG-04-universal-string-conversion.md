# WJ-LANG-04: Universal String Conversion for Windjammer

**Status**: Draft  
**Author**: Windjammer Team  
**Created**: 2026-03-29  
**Category**: Language Design

## Summary

Windjammer deliberately removed `.to_string()` from `.wj` source as Rust leakage. Today, developers often fall back to `format!("{}", value)` for primitive-to-string conversion, which is verbose and embeds Rust macro syntax in user code. This RFC proposes a single idiomatic method, `.string()`, available on all Windjammer types, with compiler-generated backend mappings so user code stays backend-agnostic and concise.

## Problem Statement

1. **Rust leakage**: `.to_string()` names Rust’s `ToString` trait and ties mental models to one backend.
2. **`format!` in user code**: `format!("{}", x)` works for Rust codegen but is noisy, macro-heavy, and not a portable Windjammer idiom across Go, JavaScript, and the interpreter.
3. **No one obvious way**: Without a universal conversion, every type family invents ad hoc patterns, hurting readability and consistency with Windjammer’s “compiler does the hard work” philosophy.

We need a **short, universal, backend-neutral** surface for “give me a human-oriented string for this value.”

## Research: How Other Languages Handle Value-to-String

| Language | Primary mechanisms | Notes |
|----------|-------------------|--------|
| **Swift** | `String(value)` initializer; `"\(value)"` string interpolation; types can expose `.description` | Strong emphasis on interpolation + initializers; not a single method name on every type, but ergonomics are similar in practice. |
| **Kotlin** | `.toString()` on all types (inherited from `Any`) | Universal method name; JVM-centric naming but familiar to many developers. |
| **Rust** | `.to_string()` via `ToString`, auto for `Display` | Trait-based; name is idiomatic Rust but not neutral across backends. |
| **Zig** | `std.fmt.bufPrint()` (and related) | No universal instance method; requires buffer/format context; explicit and low-level. |
| **Go** | No single universal method; `strconv.Itoa()` for ints; `fmt.Sprintf()` for general formatting | Package-level functions; type-specific entry points. |
| **Python** | `str(value)` built-in | Function style, not a method; universal and short. |

Windjammer should pick **one** user-facing operation that reads like Windjammer—not like Rust macros or Go’s split between `strconv` and `fmt`.

## Proposed Design

### 1. Universal `.string()` method

Every Windjammer value (primitives, stdlib types, user-defined structs/enums, etc.) supports:

```windjammer
let n = 42
let s = n.string()  // "42"

let msg = "value is " + n.string()
```

**Semantics**: Returns a `String` (or the language’s canonical owned string type) representing the value for **display and logging**, not necessarily a lossless or parseable serialization.

### 2. Compiler-generated implementation by type category

The compiler **does not** require users to implement `.string()` by hand for ordinary types. It synthesizes calls appropriate to each backend:

| Category | Windjammer source | Intended backend behavior (examples) |
|----------|-------------------|----------------------------------------|
| Numeric primitives | `x.string()` | Rust: `.to_string()`; Go: `strconv.FormatInt` / `strconv.Itoa` / `fmt` as needed; JS: `String(x)`; Interpreter: native conversion |
| `bool`, other scalars | `x.string()` | Backend-appropriate boolean/string conversion |
| Custom type with **display**-style formatting (when Windjammer defines or infers it) | `x.string()` | Rust: `.to_string()` from `Display`; other backends: equivalent user-facing formatting |
| Custom type **without** display | `x.string()` | **Debug-style** representation (structured, stable enough for logs; not guaranteed API-stable across compiler versions unless specified later) |

Exact rules for “has Display” vs “fallback to debug” should align with Windjammer’s existing auto-derive and trait story as it evolves; this RFC establishes the **user-visible** contract: you always call `.string()`, and the compiler picks the best available representation.

### 3. Naming: `.string()` instead of `.to_string()`

- **`.to_string()`** is Rust’s vocabulary and implies the `ToString` trait.
- **`.string()`** is shorter, reads as English, and does not privilege one backend’s trait system.
- Matches Windjammer’s goal: **inference and synthesis where the name doesn’t need to encode backend mechanics**.

## Backend Compilation Strategy

| Backend | `.string()` lowering (illustrative) |
|---------|--------------------------------------|
| **Rust** | Primitives and `Display` types → `.to_string()`; fallback → `format!("{:?}", ...)` or derived debug output |
| **Go** | Integers → `strconv.Itoa` / `strconv.FormatInt`; general → `fmt.Sprintf("%v", ...)` or type-specific `.String()` when available |
| **JavaScript** | `String(value)` or `value.toString()` as appropriate for the lowered type |
| **Interpreter** | Direct conversion using interpreter runtime rules mirroring the table above |

The key point: **one spelling in `.wj`**, many correct lowerings in each backend.

## Design Principles

### 1. Backend-agnostic surface

User code never mentions `ToString`, `strconv`, or `format!` solely to get a default string.

### 2. One obvious way to stringify for logs and messages

Prefer `.string()` over scattering `format!` for simple concatenation and logging.

### 3. Compiler-owned defaults

Users opt into richer formatting (precision, padding, radix) via future formatting APIs or interpolation—not by learning backend macros today.

### 4. Consistency with “no Rust leakage”

`.string()` is valid Windjammer; `.to_string()` remains out of `.wj` files per project rules.

## Implementation Plan

### Phase 1: Core language + Rust backend

1. **Parser / AST**: Recognize method call `.string()` on any expression (or treat as builtin lowering).
2. **Type checker**: Resolve `.string()` as a synthetic method on all types, returning the canonical string type (`String`).
3. **Codegen (Rust)**:
   - Numeric and other primitives → `.to_string()`.
   - Types with `Display` → `.to_string()`.
   - Otherwise → debug formatting (e.g. `format!("{:?}", ...)`) until a finer policy exists.
4. **Tests**: `.wj` tests asserting string results for primitives and a few struct shapes (TDD in Windjammer).

### Phase 2: Go, JavaScript, interpreter

1. Extend each backend’s codegen with the lowering table (Itoa / Sprintf / `String()` / JS `String` / interpreter builtins).
2. Conformance tests alongside existing cross-backend tests.

### Phase 3: Documentation and stdlib

1. Document `.string()` in language reference.
2. Migrate examples from `format!("{}", x)` to `x.string()` where appropriate.

## Alternatives Considered

### A. Free function `str(value)` (Python-style)

**Pros**: Familiar to Python users; clearly not a method dispatch.  
**Cons**: Less consistent with Windjammer’s method-heavy ergonomics; may collide with other `str` meanings. **Rejected** for now in favor of uniform `.string()` on values.

### B. Keep `format!("{}", value)` as the idiomatic form

**Pros**: Already works for Rust backend.  
**Cons**: Verbose, macro syntax, poor fit for Go/JS mental models. **Rejected** as the primary idiom.

### C. Reintroduce `.to_string()` in `.wj` as “universal”

**Pros**: Familiar to Rust developers.  
**Cons**: Explicit Rust leakage and wrong signal for multi-backend language. **Rejected**.

### D. Multiple methods (`.display()`, `.debug()`, `.string()`)

**Pros**: Fine-grained control.  
**Cons**: Heavier API; this RFC optimizes for the common case first. **Deferred**—may appear later alongside explicit formatting traits.

## Future Work: String Interpolation

A natural follow-on is Windjammer-native interpolation, for example:

```windjammer
let x = 42
let msg = "value is ${x}"   // hypothetical syntax
```

Requirements would include: parsing, type checking, lowering to efficient backend code (no mandatory allocation per segment where avoidable), and clear interaction with `.string()` (interpolation likely uses the same underlying conversion rules).

## Decision Record

### Why a method instead of a macro?

Methods scale across backends without teaching users a preprocessor or Rust-specific `format!`. The compiler lowers a single construct everywhere.

### Why debug fallback for types without display?

Guarantees `.string()` always exists; avoids partial APIs. Exact stability of debug output can be tightened in a later RFC if needed.

### Why not reuse Kotlin’s `.toString()` name?

Universal in JVM-land, but still anchored to Java-style naming and does not improve over Rust’s `.to_string()` for Windjammer’s “not a Rust dialect” goal. `.string()` is neutral and short.

---

*This RFC complements the “no Rust leakage” rules: user-facing string conversion should look like Windjammer, while the compiler emits whatever each backend already does best.*
