# User-library test gaps (`wj test` vs dogfood crates)

**Audience:** language / tooling owners  
**Date:** 2026-08-10  
**Context:** Dogfood libraries often still run **Rust** `cargo test` against WJ-generated `build/lib.rs` even though Windjammer advertises a full test suite (`wj test`, `std::testing` / `std::test`, `@test`).

This document lists the **concrete gaps** that block migrating those tests to Windjammer today. It is language-only (no product names required to act on the items).

---

## Status on tip (2026-08-10)

**GREEN**

- `wj test --module-file --library`
- `--use-project-cargo` feature merge
- Combined `--use-build-dir` + `--use-project-cargo` on multipass stub (i64 / default int formals)
- `tests/wj_test_use_build_dir_i32_literal_gate_test.rs` — `@test` int lit vs explicit `i32` formal under `--use-build-dir`
- `tests/wj_test_use_build_dir_defaults_skip_wj_regen_gate_test.rs` — `--use-build-dir` defaults `SKIP_WJ_REGEN=1` for path-dep Cargo children

**RED gates filed (language-only)**

- `tests/wj_test_transpiles_discovered_wj_test_modules_gate_test.rs` — `wj test` discovers `*_test.wj` and emits `pub mod …` but does **not** write the transpiled `.rs` into the temp crate → cargo **E0583** (`file not found for module`)

**Dogfood**

- Prefer `SKIP_WJ_REGEN=1` for path-dep cargo until deliberate selective UI regen (generated trees can dirty)
- Rust `cargo test` remains interim until the transpile-discovered-modules gate greens; aspirational `tests/*_test.wj` pilots may land ahead of tip

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

So the gap is not “no testing language” — it is **dogfood / library-project fidelity**.

---

## Gaps that force Rust tests today

### 1. Custom `--module-file` / outbound `build/` layouts

Dogfood libraries often use:

```bash
wj build src/mod.wj --module-file -o build --no-cargo
```

then a hand-maintained `Cargo.toml` with `path = "build/lib.rs"`.

`wj test` assumes a **standard package layout** (`wj.toml` + `src/` → its own temp compile). It does **not**:

- honor an existing outbound `build/` tree as the lib under test, or  
- re-run the same `--module-file` Makefile contract the package uses for shipping.

**Need:** `wj test` must build the library the same way `make build` / CI builds it (flags, outbound dir, re-export patching).

**Status:** largely met via `--use-build-dir` + `--module-file` / `--library` on tip (see GREEN list).

### 2. Dual Cargo identity (WJ crate vs Rust crate)

Packages frequently keep:

- `wj.toml` for Windjammer, and  
- a separate Rust `Cargo.toml` that points at generated sources and path-deps.

`wj test` spins a **fresh** temp Cargo project and copies/rewrites deps. Drift vs the package’s real Cargo features (e.g. `windjammer-ui` `web` vs `desktop`, `default-features`) causes false reds or missing symbols.

**Need:** either consume the package’s Cargo.toml as source of truth for deps/features, or generate an equivalent that matches documented dogfood defaults.

**Status:** `--use-project-cargo` greens feature merge on tip.

### 3. Discovery UX vs docs / examples mismatch

CLI help text still says “functions starting with `test_`”, while discovery also accepts `@test`. Examples mix `std::testing`, `std::test`, and rich APIs (`mock`, `bench`, `property`) that may not all be wired for library tests.

**Need:** one documented happy path (`tests/foo_test.wj` + `@test` + `std::testing`) that `wj test` guarantees for libraries.

### 4. Harness fragility (runtime / sandbox copy)

`wj test` copies `windjammer-runtime` into the temp tree. That step fails under restricted environments and is a sharp edge for CI agents.

**Need:** depend on a published/path `windjammer-runtime` via Cargo like normal packages, without recursive copy from the compiler checkout.

### 5. Tip ownership asymmetry still breaks “pure WJ” tests

Even when `wj test` runs, library code must avoid Rustisms (`"lit".to_string()`) while tip still emits bare `&str` into some owned `String` formals (external UI builders). Tests that only call pure WJ helpers work; tests that compile UI dogfood hit the same codegen bugs as the app.

**Related gates (GREEN on tip as of fixture W0006 scrub + Option\<string\> payload own):**

- `rust_leakage_string_literal_to_string_forbidden_test` — forbid `"…".to_string()` in WJ source  
- `codegen_bare_string_literal_owned_method_arg_gate_test` — bare lit into owned method must auto-own  
- `codegen_empty_string_option_match_arm_gate_test` — `None => ""` vs `Some(s) => s`  
- `codegen_owned_string_trait_render_concat_gate_test` — cross-module `.render()` concat borrow  
- `codegen_enum_match_chars_json_field_gates_test` — enum qualify / chars / `Option<string>` field helper  
- `codegen_home_kpitile_compose_test` — KpiTile/KpiGrid compose without WJ-source `.to_string()`  
- `codegen_str_index_split_escape_gates_test` — index/split/escape + multipass formals  

Dogfood packages should no longer need `"lit".to_string()` workarounds for these patterns; remaining Rust harness tests are for other gaps (see above).

### 6. No first-class “integration test against generated lib in-place”

Rust `#[cfg(test)]` / `tests/*.rs` can import the **exact** artifact CI ships. `wj test` always recompiles into a temp dir; failures are harder to bisect against `build/*.rs`.

**Need:** optional `wj test --use-build-dir build` (or similar) for dogfood packages.

**Status:** flag exists and is GREEN for i32 / SKIP_WJ_REGEN; blocked on §7 for end-to-end `.wj` suites.

### 7. Discovered `*_test.wj` must be transpiled into the temp crate (RED)

Discovery finds `tests/foo_test.wj` and generates `pub mod foo_test;` in the temp `lib.rs`, but does not write `foo_test.rs`. Cargo then fails with **E0583** (`file not found for module`) before any assertion runs.

**Gate:** `tests/wj_test_transpiles_discovered_wj_test_modules_gate_test.rs`

**Need:** transpile each discovered `*_test.wj` into the temp test crate alongside the `pub mod` declaration.

---

## Recommended acceptance criteria

A dogfood library package can:

1. Keep SOT in `src/**/*.wj` with **no** string-literal `.to_string()`. — **partially met** (related ownership / leakage gates green)  
2. Add `tests/foo_test.wj` using `@test` + `std::testing` / `std::test` asserts. — **blocked** on §7 E0583 until tip greens the transpile gate  
3. Run `wj test` with the **same** compile flags as `wj build` for that package. — **met on tip** (`--module-file --library`, `--use-project-cargo`, `--use-build-dir` + i32 lit + SKIP_WJ_REGEN default)  
4. Path-depend on UI crates without manual Cargo.toml surgery. — **met for harness** (SKIP_WJ_REGEN default green); still prefer deliberate UI regen  

When migrating a suite to `wj test`, do it package-by-package after criteria 1–4 work on tip.
