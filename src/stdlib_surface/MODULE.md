# src/stdlib_surface/

## Purpose
Declarative tables of the built-in standard-library surface (ADR-0018). Today: the known-method name table (`known_methods.rs`, shared by `wj check` WJ0015 and the interpreter parity gate) and the enumerable collection methods
(`group_by`, `count_by`, `unique`, ...) with argument kinds, result shapes and the Rust runtime name.

## Inputs/Outputs
In: a method name and argument count. Out: a `Method` descriptor.

## Invariants
- One entry per (name, arity); Rust names are `wj_`-prefixed (they never collide with Rust std inherent methods).
- `std/enumerable.wj` declares the same names (checked by `tests/enumerable_surface_test.rs`).

## Owns
Which collection methods exist, their argument kinds and result shapes.

## Never decides
Ownership, clones, integer widths (IR solver), how a backend prints a call.

## Allowed imports
None (layer `support`).

## Entry points
`enumerable::lookup`, `enumerable::METHODS`.

## Tests
Unit tests in `enumerable.rs`; `tests/enumerable_surface_test.rs`; conformance programs `tests/conformance_programs/16x_coll_*.wj`.
