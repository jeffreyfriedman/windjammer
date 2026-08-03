# User-library test gaps (`wj test` vs dogfood crates)

**Audience:** language / tooling owners  
**Date:** 2026-08-02 (updated 2026-08-03)  
**Context:** Dogfood library packages historically ran **Rust** `cargo test` against WJ-generated `build/lib.rs` even though Windjammer advertises a full test suite (`wj test`, `std::testing` / `std::test`, `@test`). Tip closes the harness gaps so packages can migrate to Windjammer tests.

This document lists the **concrete gaps** that block migrating those tests to Windjammer today. It is language-only (no product names required to act on the items).

---

## What already works

Windjammer **does** have a real test stack for the happy path:

| Piece | Status |
|-------|--------|
| `wj test` CLI | Discovers `*_test.wj` / files under `tests/` |
| `@test` decorator **or** `test_*` fn names | Both recognized in discovery |
| `std::testing` / `std::test` asserts | `assert`, `assert_eq`, `assert_contains`, Option/Result helpers |
| Library under test | Harness can compile the package lib into a temp crate and link tests |
| Path deps (incl. special-case `windjammer-ui`) | Partially supported in `test_execution` |

Examples: `examples/testing_examples.wj`, `examples/syntax_tests/32_testing/`, `std/testing.wj`, `std/test/mod.wj`.

**Dogfood parity flags (tip / v0.50.0):**

| Flag | Purpose |
|------|---------|
| `--module-file --library` | Same post-transpile layout as `wj build --library --module-file` |
| `--use-build-dir DIR` | Link pre-built outbound tree; skip library recompile |
| `--use-project-cargo` | Merge project root `Cargo.toml` deps/features into test lib crate |
| `--no-runtime-copy [--runtime-path PATH]` | Path-dep `windjammer-runtime` without recursive copy into temp |
| `-o/--output DIR` | Outbound dir for fresh library compile (when not using `--use-build-dir`) |
| `--no-generate-cargo-toml` | Skip auto Cargo.toml for library compile |

Integration gates: `tests/wj_test_*_test.rs`.

---

## Gaps — status

### 1. Custom `--module-file` / outbound `build/` layouts — **CLOSED**

`wj test --module-file --library` runs the same post-steps as `wj build` (`apply_library_build_post_steps`: scoped `mod.rs`, strip `main()`).

`wj test --use-build-dir build` links a pre-built tree without recompiling it (mtime of `build/lib.rs` unchanged). When the project root `Cargo.toml` owns `[lib] path = "build/lib.rs"`, the harness path-deps the **project root** (dogfood layout).

### 2. Dual Cargo identity (WJ crate vs Rust crate) — **CLOSED (opt-in)**

`wj test --use-project-cargo` merges project root `[dependencies]` (paths, features, `default-features`) into the temp test-library `Cargo.toml`. `windjammer-ui` is no longer forced to `desktop` when the project manifest already declares the dep or specifies features.

### 3. Discovery UX vs docs / examples mismatch — **CLOSED**

Empty-state help now mentions `tests/`, `*_test.wj`, and `@test`. Examples still mix APIs (`mock`, `bench`, `property`) — see remaining items.

### 4. Harness fragility (runtime / sandbox copy) — **CLOSED (opt-in)**

`wj test --no-runtime-copy` (and `--runtime-path`) writes a Cargo path dep to `windjammer-runtime` instead of copying into `{temp}/crates/windjammer-runtime`. Default remains copy-for-self-contained temp trees (backward compatible).

### 5. Tip ownership asymmetry for “pure WJ” tests — **CLOSED (gates green)**

Related tip gates are green:

- `rust_leakage_string_literal_to_string_forbidden_test` — forbid `"…".to_string()` in WJ source  
- `codegen_bare_string_literal_owned_method_arg_gate_test` — bare lit into owned method must auto-own  
- `codegen_empty_string_option_match_arm_gate_test` — `None => ""` vs `Some(s) => s`  
- `codegen_owned_string_trait_render_concat_gate_test` — cross-module `.render()` concat borrow  

Do not reintroduce `"literal".to_string()` in `.wj`. New UI-builder edge cases still belong as language gates, not product workarounds.

### 6. No first-class “integration test against generated lib in-place” — **CLOSED**

`wj test --use-build-dir build` satisfies in-place testing against the CI-shipped artifact without recompile.

---

## Recommended acceptance criteria

A dogfood library package can:

1. Keep SOT in `src/**/*.wj` with **no** string-literal `.to_string()`. — **yes** (lint + codegen gates)  
2. Add `tests/foo_test.wj` using `@test` + `std::testing::assert_contains` / `assert_eq`. — **yes**  
3. Run `wj test` with the **same** compile flags as `wj build` for that package. — **yes** (`--module-file`, `--library`, `-o`)  
4. Path-depend on UI crates (web features) without manual Cargo.toml surgery. — **yes** (`--use-project-cargo`)  
5. Drop the parallel Rust `tests/*.rs` harness without losing coverage. — **yes** for tip; migrate package-by-package  

Recommended dogfood invocation:

```bash
wj test --module-file --library --use-project-cargo --no-runtime-copy
# when CI already ships build/:
wj test --use-build-dir build --use-project-cargo --no-runtime-copy
```

---

## Interim guidance for product agents

- Prefer WJ SOT + language gates for tip bugs.  
- Keep Rust tests thin: call `crate_api::…`, assert strings/ints — no duplicated product logic.  
- When migrating a suite to `wj test`, use `--module-file --use-project-cargo --no-runtime-copy` for dogfood libs; add `--use-build-dir build` when CI already ships `build/`.  
- Do not reintroduce `"literal".to_string()` in `.wj` to pacify tip; file/extend gates instead.

---

## Remaining work

| Item | Notes |
|------|--------|
| Default `--no-runtime-copy` | Today opt-in; become default when `wj.toml` declares a runtime path |
| `std::testing` advanced APIs in library tests | `mock`, `bench`, `property` — verify per API, document one happy path |
| Auto-detect dogfood layout | Infer `--use-build-dir` / `--use-project-cargo` from existing `Cargo.toml` + `build/` |
| Package migrations | Convert remaining Rust adapter tests to `*_test.wj` package-by-package |
