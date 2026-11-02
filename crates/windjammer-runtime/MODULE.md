# crates/windjammer-runtime

## Purpose
Rust implementations behind Windjammer's standard library: when a program does `use std::http`, generated code calls functions from this crate (http, fs, json, crypto, regex, time, collections, testing/mocks, platform and profiling layers).

## Inputs/Outputs
In: calls from generated Rust. Out: stdlib behavior. Signatures are scanned by the compiler (`stdlib_scanner`) to learn ownership requirements.

## Invariants
- Heavy modules are cargo-feature gated (`net`, `async`, `cli`, `crypto`, `datetime`, `jwt`, `regex`, `parallel`, `yaml`, `compress`, `csv`, `log`, `db`). The single `#[cfg(feature = "f")]` line directly above `pub mod m;` in `lib.rs` (and `platform/native/mod.rs`) is what the compiler reads to enable only the groups a program uses; keep that one-line shape. `default` enables everything.
- The public signature of each function is the ownership contract the analyzer reads; changing `&T`/`T` parameters changes generated code.
- Platform differences live in `platform/{native,wasm}`.
- No dependency on the compiler crate. `caps_core/` is the one exception in spirit: the compiler compiles that directory too (`#[path]`), so the capability model has a single source; it uses only `std`.
- Capabilities (ADR-0021): every public function that touches the OS calls `caps::*` first (audit: tests/effects_runtime_audit_test.rs); `caps::install` runs once at the start of a program's `main`, only when the compiler could not prove its calls; a denial exits 126. `cap.rs` is `std::cap`.

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
`durable_core` (shared with the interpreter via `#[path]`): journal file format, run identity, `RunJournal`; `durable`: `Journalable` encodings and `run`/`step` for compiled `durable fn` (ADR-0021 E4b; vectors in `tests/effects/durable_*vectors.json`).
