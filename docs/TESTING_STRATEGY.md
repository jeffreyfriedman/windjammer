# Testing Strategy

**Related:** [`USER_LIBRARY_TEST_GAPS.md`](USER_LIBRARY_TEST_GAPS.md) — why `--module-file` /
outbound-`build/` dogfood libraries still use Rust `cargo test` instead of `wj test`.

Principle: **a test is one file in the right place, and a test nothing runs is a CI failure.**
This document describes the mechanism as implemented; `tests/README.md` is the contributor
cheat sheet.

## 1. Where tests live

| Kind | Location | Runner |
|------|----------|--------|
| Compiler integration tests (Rust) | `tests/*.rs`, `tests/<dir>/**/*.rs` containing `#[test]` | `cargo test --release --test all` |
| Unit tests | `#[cfg(test)]` in `src/**` | `cargo test --release --lib` |
| Crate tests | `crates/*/tests/*.rs` | `cargo test --release --workspace` |
| `.wj` corpus (compile checks) | `tests/*.wj`, `tests/conformance/**`, `tests/auto_clone`, `tests/windjammer_tests`, `tests/integration_wj`, `tests/fixtures/*.wj`, `tests/error_system*` | `tests/wj_corpus_test.rs` (part of `all`) |
| Windjammer-language tests (user projects) | `@test` / `test_*` functions in any `.wj` | `wj test` |

## 2. How the `all` binary is assembled

`autotests = false`; `[[test]] name = "all"` points at `tests/all.rs`, which `include!`s
`$OUT_DIR/all_tests_generated.rs`. `build.rs` writes that file using
`build_support/test_modules.rs`:

1. each top-level `tests/*.rs` (except `all.rs`) becomes `mod <file_stem>`;
2. each `.rs` below a sub-directory that contains `#[test]` becomes `mod <path__joined>`;
3. `common/`, `fixtures/`, hidden directories and `*.expected.rs` are never modules;
4. two files that map to one module name abort the build with both paths in the message.

One binary means one link step (~800 fewer links). The rule is covered by
`tests/build_test_module_discovery_test.rs`.

## 3. Feature groups

The six features `parser_tests`, `analyzer_tests`, `codegen_tests`, `interpreter_tests`,
`conformance_tests`, `integration_tests` select a group. Headers:

* `#![cfg(any(not(any(<all six>)), feature = "<group>"))]` — full suite, or that group;
* `#![cfg(not(any(<all six>)))]` — full suite only (regression and linter tests);
* no header — always.

`--all-features` enables all six, which compiles out every `not(any(...))` file and still
reports green. Therefore: **tests never run with `--all-features`** (CI, Makefile and docs
use plain `cargo test`; `--all-features` is for clippy/check only), and
`tests/feature_gate_guard_test.rs` contains a test that fails when all six are on, plus a test
that keeps its cfg list in sync with `Cargo.toml`.

## 4. Orphan check

`scripts/check-orphan-tests.sh` (CI job `orphan-tests`, `make test-orphans`) lists tracked test
candidates (`tests/**` `.rs`/`.wj`/`.sh`, `crates/*/tests/**`, `*_test.wj`, `test_*.wj`) and
fails on any that is not reached. Reached means: top-level `tests/*.rs`; matched by a
`// consumes: <glob>` line in a `tests/*.rs` runner (`*` stays inside one directory, `**`
crosses); mentioned by path or file name in a runner (`tests/*.rs`, `src/**`, `scripts/*`,
`Makefile`, `.github/**`); or allow-listed in `tests/ORPHAN_ALLOWLIST.txt` with a reason.
`*.skip`, `*.disabled` and `*.bak` files always fail: use `#[ignore = "reason"]`.

## 5. Ratchet

`.config/known-test-failures.txt` lists failures accepted for now (one test path per line,
each a genuine compiler bug). CI (`cargo nextest run --release --workspace --profile ci`) and
`scripts/check-test-ratchet.sh` fail on any failure not in the file and report entries that
now pass. Do not add entries to hide new failures.

## 6. `wj test` discovery

See `tests/README.md` ("What `wj test` discovers") and `src/test_runner/MODULE.md`. In short:
every `.wj` under the root is scanned; `@test`/`test_*` functions at top level, in inline
mods and in receiver-less impl functions are tests; test-named files are always parsed;
failures are aggregated and fatal; zero tests is an error unless `--allow-empty`; `--list`
prints the result; duplicate stems get unique module names; `--filter` only declares modules
that were compiled.

## 7. CI

`.github/workflows/test.yml`: formatting and clippy (`--all-features` allowed), `orphan-tests`,
version/lockfile checks, then the matrix job: `cargo test --release --workspace --no-run`,
`cargo nextest run --release --workspace --profile ci` with the ratchet comparison,
doctests, multi-target tests.
