# Windjammer integration tests

Rust integration tests live under this directory. **Adding a file is enough** — no manual registration in `all.rs`, `mod.rs`, or `wj.toml`.

## How discovery works

1. **`build.rs`** scans `tests/*.rs` at compile time and generates `OUT_DIR/all_tests_generated.rs` with one `mod` per file (skipping `all.rs`, `lib.rs`, and `common/`).
2. **`tests/all.rs`** is a thin shell that `include!()`s that generated module list into a **single test binary** (one link step instead of ~800).
3. **`Cargo.toml`** sets `autotests = false` and declares one `[[test]]` target named `all`.

Downstream Windjammer projects (game crates) may still use auto-generated `tests/lib.rs` + a crate-root hook for `cargo test --lib`. The **compiler repo does not** — it uses the consolidated `all` binary only.

## Canonical commands

**Full suite** (default):

```bash
cd windjammer
cargo test --release --test all
```

**One module or test**:

```bash
cargo test --release --test all -- ownership_method_test
cargo test --release --test all -- test_snapshot_method_returns_self_type_gets_ref_self
```

**Fast iteration** (skip slow fixture subprocess tests):

```bash
cargo test --release --test all --features skip_fixtures
```

**Opt-in suite features** (subset of gated directories):

| Feature | Purpose |
|--------|---------|
| `parser_tests` | Parser, WGSL front matter |
| `analyzer_tests` | Analyzer, ownership inference |
| `codegen_tests` | Backend codegen |
| `interpreter_tests` | Interpreter runtime |
| `conformance_tests` | Cross-backend conformance |
| `integration_tests` | End-to-end / build / FFI |

```bash
cargo test --release --test all --features analyzer_tests
```

## `regression/` and `linter/`

Bug-regression and linter tests use `#![cfg(not(any(...suite features...)))]`. They run only in the **full** suite (no suite feature enabled).

## Shared helpers

`tests/common/` holds modules included via `#[path = ...] mod ...;` from individual tests. Helpers are not standalone test modules.

## Quality gates (compiler changes)

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --release --test all
```
