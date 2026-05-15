# Windjammer integration tests

Rust integration tests live under this directory and are registered explicitly in the root `Cargo.toml` (`[[test]]` targets). Most crates use a crate-level `cfg` gate so you can **run a subset of suites** by enabling one or more `*_tests` Cargo features.

## Suite features

Declared in `Cargo.toml` under `[features]`:

| Feature | Directory | Purpose |
|--------|-----------|---------|
| `parser_tests` | `tests/parser/` | Parser, WGSL front matter, shader file detection |
| `analyzer_tests` | `tests/analyzer/` | Analyzer, ownership inference, type checking |
| `codegen_tests` | `tests/codegen/` | Backend codegen (Rust / Go / JS) |
| `interpreter_tests` | `tests/interpreter/` | Interpreter runtime |
| `conformance_tests` | `tests/conformance/` | Cross-backend conformance |
| `integration_tests` | `tests/integration/` | End-to-end / build / FFI / modules |

## Commands

**Full suite** (default—no suite feature flags): every gated directory plus `tests/regression/` and `tests/linter/` (those are only active when *no* suite feature above is enabled).

```bash
cd windjammer
cargo test --release
```

**One suite** (example: parser):

```bash
cargo test --release --features parser_tests
```

**Another suite** (example: Rust codegen):

```bash
cargo test --release --features codegen_tests
```

**Multiple suites**:

```bash
cargo test --release --features "parser_tests codegen_tests"
```

**Single integration test binary** (still respects the same feature gates):

```bash
cargo test --release --features parser_tests --test parser_expression_tests
```

## `regression/` and `linter/`

Bug-regression and linter tests are **not** tied to a suite feature. They are included only when you run the full suite (**no** `parser_tests`, `analyzer_tests`, `codegen_tests`, `interpreter_tests`, `conformance_tests`, or `integration_tests` feature is enabled). That keeps focused runs fast and avoids widening every partial suite with long regression matrices.

## How gating works

Each integration test source file starts with `#![cfg(any(...))]` (or `#![cfg(not(any(...))))]` for regression/linter):

- If **no** suite feature is set, the `not(any(...))` branch is true and **all** gated suites compile and run.
- If **any** suite feature is set, only files whose feature matches (or the same `not(any(...))` case for regression/linter) stay active.

Cargo may still build empty test harnesses for crates that are gated off; use `--test <name>` when you want to limit work to one binary.

## Shared helpers

`tests/common/` holds modules included via `#[path = ...] mod ...;` from individual tests. Helpers are not standalone `[[test]]` targets.
