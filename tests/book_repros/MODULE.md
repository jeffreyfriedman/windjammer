# tests/book_repros

## Purpose
Minimal programs for compiler or diagnostic bugs found while writing The Windjammer Book (docs/book). Each is listed in `docs/known-gaps.md` under "Found while writing the book" and pinned by a test in `tests/book_test.rs` that asserts the bug is still present, so fixing it fails the test and prompts removing the repro and the known-gaps row.

## Layout
- `explain_ownership_interpolation_span.wj`: `wj explain-ownership` prints a nonsense column (`@8:17640014`) for field reads inside the second interpolation of a string.
- `generic_largest_needs_clone.wj`: a generic function that copies `items[0]` out of a `Vec<T>` with `T: PartialOrd` runs interpreted and fails compiled with WJ0015.
- `iter_find_closure.wj`: a collection idiom that runs interpreted and is rejected compiled (WJ0003).
- `module_struct/`: a struct from a file module reaches `main` only through a function result; interpreted prints two lines, compiled fails with WJ0003 unless `main` imports the type.
- `interpolation_leading_literal.wj`: a string literal at the start of an interpolation fails to parse (WJ0011).
- `module_const.wj`, `map_get_string_key.wj`: constructs that run interpreted and are rejected compiled (WJ0001, WJ0003).
- `defer_in_loop.wj`: a loop-body `defer` runs compiled and not interpreted.

## Invariants
- A repro is the smallest program that shows the bug, free of project names (public hygiene).
