# tests/book_repros

## Purpose
Minimal programs for compiler or diagnostic bugs found while writing The Windjammer Book (docs/book). Each is listed in `docs/known-gaps.md` under "Found while writing the book" and pinned by a test in `tests/book_test.rs` that asserts the bug is still present, so fixing it fails the test and prompts removing the repro and the known-gaps row.

## Layout
- `explain_ownership_interpolation_span.wj`: `wj explain-ownership` prints a nonsense column (`@8:17640014`) for field reads inside the second interpolation of a string.
- `generic_largest_needs_clone.wj`: a generic function that copies `items[0]` out of a `Vec<T>` with `T: PartialOrd` runs interpreted and fails compiled with WJ0015.
- `dir_module/`: a project whose `util/mod.wj` function takes a `Vec<int>`; interpreted it prints 2, compiled it fails with WJ0003 (the file-module form `util.wj` compiles).
- `iter_filter_collect.wj`, `iter_find_closure.wj`, `hashmap_entry_literal_keys.wj`: collection idioms that run interpreted and are rejected compiled (WJ0029, WJ0003, WJ0003).
- `module_struct/`: a struct from a file module reaches `main` only through a function result; interpreted prints two lines, compiled fails with WJ0003 unless `main` imports the type.
- `interpolation_leading_literal.wj`: a string literal at the start of an interpolation fails to parse (WJ0011).
- `map_err_literal.wj`: `map_err(|_| "literal")` into `Result<int, string>`; interpreted fine, compiled WJ0003.
- `module_const.wj`, `optional_chain.wj`, `map_get_string_key.wj`: constructs that run interpreted and are rejected compiled (WJ0001, WJ0099, WJ0003).
- `async_await_compiled.wj`, `pure_attribute_compiled.wj`: constructs that run interpreted and are rejected compiled (WJ0099, WJ0099). `defer_in_loop.wj`: a loop-body `defer` runs compiled and not interpreted.

## Invariants
- A repro is the smallest program that shows the bug, free of project names (public hygiene).
