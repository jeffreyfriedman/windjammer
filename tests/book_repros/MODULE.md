# tests/book_repros

## Purpose
Minimal programs for compiler or diagnostic bugs found while writing The Windjammer Book (docs/book). Each is listed in `docs/known-gaps.md` under "Found while writing the book" and pinned by a test in `tests/book_test.rs` that asserts the bug is still present, so fixing it fails the test and prompts removing the repro and the known-gaps row.

## Layout
- `module_struct/`: a struct from a file module reaches `main` only through a function result; interpreted prints two lines, compiled fails with WJ0003 unless `main` imports the type.
- `defer_in_loop.wj`: a loop-body `defer` runs compiled and not interpreted.
- `lone_script_mod/` (found by the ADR-0033 work; fixed): `wj build main.wj` of a lone script with a sibling `mod` generates the module beside `main.rs`; `tests/book_test.rs` keeps it as a regression test.

## Invariants
- A repro is the smallest program that shows the bug, free of project names (public hygiene).
