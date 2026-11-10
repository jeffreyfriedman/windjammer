# crates/windjammer-playground

## Purpose
The Windjammer interpreter for the book's browser playground (ADR-0034): `run(source)` compiled to `wasm32-unknown-unknown`, loaded by `docs/book/playground/` in a Web Worker.

## Inputs/Outputs
In: one program's source text, fuel and output limits, a seed. Out: `Outcome { stdout, stderr, diagnostics (wj check JSON), exit_code, steps }`, as JSON across the wasm boundary.

## Invariants
- A run is `wj run --interpret --sandbox --simulate --seed N --limits fuel=F,output=B` on one in-memory file: `wj check` first (errors refuse the run), then the tree walker under `Grant::sandbox`, the simulated world and the limits. `scripts/book/book_samples.wj --wasm` holds it to exactly that native command on every book sample.
- The wasm module imports nothing (`wj-wasm.mjs` refuses one that does): no clock, no entropy, no network. getrandom gets a fixed stream (`wasm.rs`).
- A workspace of its own (`[workspace]` in `Cargo.toml`, own `Cargo.lock`): the main workspace and its gates never build it. Build: `cargo build --release --target wasm32-unknown-unknown` from this directory (`.cargo/config.toml` sets the getrandom backend and a 32 MiB stack).

## Owns
The browser run configuration (`lib.rs`), the result shape (`outcome.rs`), the wasm exports (`wasm.rs`).

## Never decides
Language semantics, diagnostics or capability rules: all come from the `windjammer` library unchanged.

## Allowed imports
`windjammer` (no default features), `serde_json`; on wasm32 `getrandom` 0.2/0.3 for the custom backend.

## Entry points
`run`, `Options`, `Outcome`; wasm exports `wj_alloc`, `wj_free`, `wj_run`, `wj_result_ptr`.

## Test
`cargo test --release` here (native: output, fuel, output cap, diagnostics, panics, exit codes, denials, seeded world). Browser and parity: `scripts/book/playground_check.wj`, `docs/book/playground/tests/browser.test.mjs`.
