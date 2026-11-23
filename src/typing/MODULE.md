# src/typing/

## Purpose
The single owner of static types (option C, `docs/design/numeric-width-redesign.md` section 7). Stage C1 holds the
typed surface of the Rust core types (`std/rust_core.wji`): `Vec`, `VecDeque`, `HashMap`, `BTreeMap`, `HashSet`,
`BTreeSet`, `Option`, `Result`, `String`, and the methods of every integer and float type (`Integer<T>`, `Float<T>`: `T` is the receiver's own type).

## Inputs/Outputs
In: a receiver's static type, a method name, a turbofish. Out: the instantiated parameter and result types.

## Invariants
- The surface is data (`std/rust_core.wji`, Windjammer syntax), parsed once with the Windjammer parser.
- Every entry agrees with rustc (`tests/core_surface_rustc_test.rs`).
- `usize` in a parameter is an index slot and in a result an index source (ADR-0028); `usize` stays a first-class type.

## Owns
Which core-type methods exist and their types.

## Never decides
Widths (the IR numeric solver does), ownership, how a backend prints a call.

## Allowed imports
Layer `types`. May import: lexer, parser, parser_impl, stdlib_surface (the lowering table's plain-text input types), type_classification.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`core_surface::method_signature`, `core_surface::declared`, `core_surface::entries`; `CoreSurface::lowering()` / `declared_traits()` hand the `@js` / `@go` / `@example` decorators of the declarations to `stdlib_surface::lowering` (docs/design/table-driven-std-surface.md).

## Tests
Unit tests in `core_surface.rs`; `tests/core_surface_rustc_test.rs` (every entry type-checks in rustc).
Run: `cargo test --lib typing::`.
