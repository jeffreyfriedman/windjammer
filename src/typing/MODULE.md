# src/typing/

## Purpose
The single owner of static types (option C, `docs/design/numeric-width-redesign.md` section 7). Stage C1 holds the
typed surface of the Rust core types (`std/rust_core.wji`): `Vec`, `VecDeque`, `HashMap`, `BTreeMap`, `HashSet`,
`BTreeSet`, `Option`, `Result`, `String`, and the `f32` / `f64` math methods.

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
Layer `types`. May import: lexer, parser, parser_impl, type_classification.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`core_surface::method_signature`, `core_surface::declared`, `core_surface::entries`.

## Tests
Unit tests in `core_surface.rs`; `tests/core_surface_rustc_test.rs` (every entry type-checks in rustc).
Run: `cargo test --lib typing::`.
