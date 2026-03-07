# Windjammer v0.41.0 Release Notes

**Release date:** 2026-02-14

## Highlights

- **Multi-backend architecture**: Compile to Rust, Go, JavaScript, or run instantly with the built-in interpreter
- **`trait` in type positions**: Write `trait Foo` and let the compiler choose between static and dynamic dispatch
- **Cross-backend conformance**: 26 tests verify all 4 backends produce byte-identical output
- **Zero-warning codegen**: Compiler-generated Rust code is clean — no unused variables, no unnecessary parens, no clippy complaints

---

## New Feature: `trait` in Type Positions

Windjammer v0.41.0 introduces the ability to use `trait` directly as a type annotation. Instead of choosing between Rust's `impl Trait` (static dispatch) and `dyn Trait` (dynamic dispatch), you write `trait TraitName` and the compiler infers the correct dispatch strategy based on context.

```
// Windjammer source
fn describe(item: trait Describable) -> string {
    item.describe()
}

fn collect_items() -> Vec<trait Describable> {
    // ...
}
```

**Inference rules:**

| Context | Windjammer | Generated Rust |
|---------|-----------|----------------|
| Bare parameter | `x: trait Foo` | `x: impl Foo` |
| Bare return | `-> trait Foo` | `-> impl Foo` |
| In `Vec<>` | `Vec<trait Foo>` | `Vec<Box<dyn Foo>>` |
| In `Option<>` | `Option<trait Foo>` | `Option<Box<dyn Foo>>` |
| Behind `&` | `&trait Foo` | `&dyn Foo` |
| Behind `&mut` | `&mut trait Foo` | `&mut dyn Foo` |
| In `Box<>` | `Box<trait Foo>` | `Box<dyn Foo>` |

This embodies the Windjammer philosophy: **the compiler handles mechanical details so developers can focus on problem-solving.**

---

## New Feature: Multi-Backend Architecture

Windjammer now supports 4 execution backends:

| Backend | Command | Use case |
|---------|---------|----------|
| **Rust** | `wj build --target rust` | Production builds, maximum performance |
| **Go** | `wj build --target go` | Fast iteration, Go ecosystem integration |
| **JavaScript** | `wj build --target javascript` | Browser apps, Node.js, serverless |
| **Interpreter** | `wj run --interpret` | Instant feedback, REPL, rapid prototyping |

All backends honor the **Windjammer Semantic Contract** — a formal specification ensuring consistent behavior across execution targets.

### Backend feature matrix

| Feature | Rust | Go | JS | Interpreter |
|---------|:----:|:--:|:--:|:-----------:|
| Arithmetic, control flow, functions | Yes | Yes | Yes | Yes |
| Structs + methods | Yes | Yes | Yes | Yes |
| Unit enums + match | Yes | Yes | Yes | Yes |
| Data-carrying enums | Yes | -- | -- | Yes |
| Match guards | Yes | Yes | Yes | Yes |
| String interpolation (`${}`) | Yes | Yes | Yes | Yes |
| Variable shadowing | Yes | Yes | Yes | Yes |
| Static constructors (`Type::new`) | Yes | Yes | Yes | Yes |
| Vec push/len/index | Yes | Yes | Yes | Yes |
| If/else as expression | Yes | -- | -- | Yes |
| `trait` in type positions | Yes | Partial | -- | -- |
| Closures, generics, traits | Yes | -- | -- | Partial |

---

## Compiler Improvements

### Automatic Unused Variable Prefixing

The compiler now detects unused variables and automatically prefixes them with `_` in generated Rust code. This applies to:

- Function parameters
- `let` bindings
- For-loop iteration variables

This eliminates 300+ warnings that previously cluttered compiler output during development.

### Wildcard Pattern in For-Loops

`for _ in 0..10 { ... }` now works correctly. The parser previously rejected the `_` pattern in for-loop position.

### Struct Pattern Shorthand

Generated Rust code now uses shorthand syntax: `Foo { x, y }` instead of `Foo { x: x, y: y }`.

### HashMap Comment False-Positive

`HashMap` or `HashSet` appearing in code comments no longer triggers spurious `use std::collections::HashMap` imports in generated Rust.

---

## Bug Fixes

### Rust Backend (1 fix)

- **String interpolation in println**: `println("Hello, ${name}!")` was generating `println!(format!("Hello, {}!", name))` (nested macro) instead of the correct `println!("Hello, {}!", name)`

### Go Backend (7 fixes)

- **Static constructors**: `Point::new(x, y)` now generates `NewPoint(x, y int64) Point` (Go naming convention)
- **Unit enum variants in match**: Enum variant patterns now correctly recognized in type switch generation
- **Enum variant instantiation**: `Color::Red` generates `ColorRed{}` (struct literal, not bare type name)
- **Match case labels**: Correctly converts `Color::Red` to `ColorRed` in `case` labels
- **Match guards with returns**: Added if-else chain for matches containing guard expressions
- **Exhaustive switch fallthrough**: Adds `panic("unreachable match")` after exhaustive switches for Go's missing-return analysis
- **Operator precedence**: Precedence-aware parenthesization (`2 * (r.W + r.H)` instead of `2 * r.W + r.H`)

### JavaScript Backend (4 fixes)

- **Variable shadowing**: Rename pass (`x` -> `x$1` -> `x$2`) instead of illegal re-declaration of `let` in same scope
- **format! to template literals**: `format!("Hello, {}!", name)` generates `` `Hello, ${name}!` ``
- **Match guard deduplication**: Multiple arms binding the same variable now share a single `let` declaration
- **println + format unwrapping**: Interpolated strings correctly flow through `console.log()`

### CI & Infrastructure (5 fixes)

- Go backend tests gracefully skip when `go` is not installed (fixes macOS CI panics)
- JS backend test runner strips auto-run block to prevent double `main()` execution on Ubuntu
- Replaced `map_or(false, ...)` with `is_some_and(...)` per clippy
- Pre-commit hook auto-formats and stages changes
- Build system detects stale `Cargo.toml` files

---

## Testing

- **26** cross-backend conformance tests (all 4 backends produce identical output)
- **22** Go backend tests (15 extended + 7 basic)
- **16** JavaScript backend tests
- **20** interpreter tests (basic + advanced + regression)
- **237** unit tests
- **279** integration test files
- **12** `trait`-in-type-position tests covering all dispatch inference rules
- **Zero regressions** across the full test suite

---

## Stats

- 79 files changed, 12,239 insertions, 217 deletions
- 15 commits on `feature/v0.41.0-compiler-improvements`

---

## What's Next (v0.42.0)

- Data-carrying enum support for Go and JavaScript backends
- If/else expression support for Go backend
- Closure and generics support for Go and JavaScript backends
- Lifetime inference (automatic lifetime annotations)
- Continued dogfooding with the Windjammer game engine
