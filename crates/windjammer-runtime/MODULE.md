# crates/windjammer-runtime

## Purpose
Rust implementations behind Windjammer's standard library: when a program does `use std::http`, generated code calls functions from this crate (http, fs, json, crypto, regex, time, collections, testing/mocks, platform and profiling layers).

## Inputs/Outputs
In: calls from generated Rust. Out: stdlib behavior. Signatures are scanned by the compiler (`stdlib_scanner`) to learn ownership requirements.

## Invariants
- The public signature of each function is the ownership contract the analyzer reads; changing `&T`/`T` parameters changes generated code.
- Platform differences live in `platform/{native,wasm}`.
- No dependency on the compiler crate.

## Owns
Stdlib semantics, error types of stdlib calls, test helpers (`test.rs`, `mock*.rs`, `fixtures.rs`).

## Never decides
Compiler behavior, ownership inference, or name resolution.

## Allowed imports
Third-party crates only; must not import `windjammer`.

## Entry points
`src/lib.rs` re-exports one module per stdlib area (`http.rs`, `fs.rs`, `json.rs`, ...).

## Tests
`cargo test -p windjammer-runtime` (crates/windjammer-runtime/tests and inline tests).
