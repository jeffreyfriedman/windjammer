# src/stdlib_surface/

## Purpose
Declarative tables of the built-in standard-library surface (ADR-0018). Today: the known-method name table (`known_methods.rs`, shared by `wj check` WJ0015 and the interpreter parity gate), the enumerable collection methods
(`group_by`, `count_by`, `unique`, ...) with argument kinds, result shapes and the Rust runtime name, and the lowering table (`lowering.rs`): one row per std method with a rule per portable target, read from the `@js` /
`@go` / `@example` decorators on `std/rust_core.wji` (docs/design/table-driven-std-surface.md). The Go and JavaScript emitters print a migrated method (`Float`, `Integer`, `Char`, `String`) by interpreting its row; the portable typer reads the row's declared result type.

## Inputs/Outputs
In: a method name and argument count. Out: a `Method` descriptor. For the lowering table: the declared traits with their decorators as text (`RawTrait`, handed over by `typing::core_surface`). Out: `LoweringTable { rows: (CoreType, method) -> Row }`.

## Invariants
- One entry per (name, arity); Rust names are `wj_`-prefixed (they never collide with Rust std inherent methods).
- `std/enumerable.wj` declares the same names (checked by `tests/enumerable_surface_test.rs`).
- The lowering vocabulary (`Guard`, `Hook`, `Modifier`, `Fact`, `Rule`) names type shapes and emitter capabilities, never std methods; its variant counts are pinned by `tests/std_lowering_table_test.rs` (G6). A malformed decorator is an error, never a dropped rule.

## Owns
Which collection methods exist, their argument kinds and result shapes.

## Never decides
Ownership, clones, integer widths (IR solver), how a backend prints a call.

## Allowed imports
None (layer `support`).

## Entry points
`enumerable::lookup`, `enumerable::METHODS`. `lowering::LoweringTable::from_declarations`, `LoweringTable::row`, `Row::select` (the first rule whose guards hold). `std_functions::exists(module, name)`: the functions each std module provides in some engine (generated table `std_functions_table.rs`, gate tests/std_functions_surface_test.rs), used by `check::std_calls` (WJ0002 for `strings.index_of`).

## Tests
Unit tests in `enumerable.rs`; `tests/enumerable_surface_test.rs`; `tests/std_lowering_table_test.rs` (G1, G4-G6, G8), `tests/std_row_examples_test.rs` (G3), `tests/portable_emit_golden_test.rs` (G2); conformance programs `tests/conformance_programs/16x_coll_*.wj`.
