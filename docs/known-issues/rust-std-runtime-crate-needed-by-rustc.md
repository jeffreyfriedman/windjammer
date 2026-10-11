# Rust target: programs using `std::{math,strings,fs,time}` need the `windjammer_runtime` crate

`wj build --target rust --no-cargo` emits `use windjammer_runtime::...`; plain `rustc` cannot resolve it. The conformance
harness compiles Rust with `rustc` only, so the std-shim programs (110-113) are recorded as unsupported for the Rust column
(they still run on Go/JS against the independent oracle). A cargo-based Rust runner would remove these entries.
