# Windjammer compiler tests

**A test is one file in the right place.** Nothing is registered by hand, and a file that no
runner reaches fails CI (`scripts/check-orphan-tests.sh`).

## What runs, and how a file is found

| File | Reached by |
|------|-----------|
| `tests/*.rs` | `build.rs` generates one `mod` per file into the single `all` binary (`tests/all.rs` `include!`s the list). |
| `tests/<dir>/**/*.rs` containing `#[test]` | Same, module named by path: `a/b_test.rs` under `tests/` is `a__b_test`. |
| `tests/common/`, `tests/fixtures/`, `*.expected.rs` | Never modules: shared helpers (`#[path = ...] mod ...;`) and data. |
| `crates/*/tests/*.rs` | Cargo's normal integration-test discovery (`cargo test --workspace`). |
| `tests/*.wj`, `tests/conformance/**`, `tests/auto_clone`, `tests/windjammer_tests`, `tests/integration_wj`, `tests/fixtures/*.wj` | `tests/wj_corpus_test.rs` compiles each file; files that must fail to compile are in `tests/error_system*/`. Known-bad files are listed with a cause in `tests/wj_corpus_known_failures.txt`; a listed file that starts passing fails the run until its entry is removed. |
| `tests/*.wj` etc. referenced by name from a runner | Reached by that reference. |
| Anything else | Must be listed in `tests/ORPHAN_ALLOWLIST.txt` as `<glob> \| <reason>` (fixtures and helpers only), or deleted. |

Disabled files (`*.skip`, `*.disabled`, `*.bak`) are never allowed: use `#[ignore = "reason"]`
or delete the test.

## Commands

```bash
cargo test --release --test all                         # whole compiler suite
cargo test --release --test all -- ownership_method     # one module / test
cargo test --release --workspace                        # + crates/*/tests and unit tests
scripts/check-orphan-tests.sh                           # fail on unreached test files
scripts/check-test-ratchet.sh --release --test all      # fail only on NEW failures
```

**Never run tests with `--all-features`.** See "Feature groups" below; use `--all-features`
only for `clippy`/`check`. `tests/feature_gate_guard_test.rs` fails if you do.

## Feature groups

Opt-in suite features (`parser_tests`, `analyzer_tests`, `codegen_tests`, `interpreter_tests`,
`conformance_tests`, `integration_tests`) select a group of test files:

* A file headed `#![cfg(any(not(any(<six features>)), feature = "<group>"))]` runs in the full
  suite (no feature) **and** when its group is selected.
* A file headed `#![cfg(not(any(<six features>)))]` runs **only** in the full suite.
* A file with no header always runs.

Enabling all six at once therefore compiles out every `not(any(...))` file. CI runs
`cargo test` without `--all-features`; the guard test turns the mistake into a failure.

## What `wj test` discovers (user projects)

`wj test [PATH]` scans every `.wj` under the root (skipping hidden, `target`, `build`,
`node_modules`). Tests are `@test` / `@property_test` / `@test_cases` functions or `test_*`
functions, at top level, in inline `mod` blocks, or as receiver-less functions of non-generic
`impl` blocks. Files named `*_test.wj`, `*_tests.wj`, `test_*.wj` or under `tests/` are always
parsed; other files are parsed only if they mention `@test` or `test_`. `wj test --list` prints
what was found. A parse error in any scanned file is reported together with all the others and
fails the run; finding no tests fails the run unless `--allow-empty`. Duplicate file stems get
path-derived module names (`a/util_test.wj` and `b/util_test.wj` become `a__util_test`,
`b__util_test`). The single naming rule is `src/test_module_gate.rs`.

## Quality gates

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --release --test all
scripts/check-orphan-tests.sh
```
