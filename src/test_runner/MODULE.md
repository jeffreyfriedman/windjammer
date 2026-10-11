# test_runner

**Purpose:** implement `wj test`: discover Windjammer tests, compile them into a throw-away
harness crate next to the project library, run `cargo test`, report.

**Inputs:** `TestRunOptions` (path, filter, `list`, `allow_empty`, library/layout flags).
**Outputs:** process exit status (non-zero on test failure, scan errors, or zero tests),
human or JSON report.

## Files
- `test_discovery.rs` — walks the root, decides which files to parse, returns `Discovery`
  (files with tests, per-file errors) and assigns collision-free module names.
- `test_extraction.rs` — pure AST walk: `@test`/`test_*` functions at top level, in inline
  `mod`s and receiver-less `impl` functions.
- `test_harness.rs` — pure helpers: filter selection, `#[test]` marking, harness `lib.rs`.
- `test_execution.rs` — library compile, FFI copy, harness crate generation (IO).
- `layout_detect.rs`, `options.rs`, `test_reporting.rs`, `util.rs`.

## Invariants
- The naming rule lives in `crate::test_module_gate` (shared with module-system gating).
- One bad file never hides the others; all scan errors are reported together and are fatal.
- Finding zero tests (or a filter matching none) is an error unless `--allow-empty`.
- The harness `lib.rs` declares exactly the modules that were compiled.
- Module names are unique: stem if unique, else root-relative path joined by `__`.

## Never decides
Ownership/codegen behaviour, which cargo features a project uses.

## Allowed imports
`lexer`, `parser`, `compiler` (build entry points), `cargo_cache`, `config`, `test_module_gate`.

## Entry points
`run_tests`, `run_tests_with_options`.

## How to test
`cargo test --lib test_runner`, `cargo test --release --test all wj_test_`.
