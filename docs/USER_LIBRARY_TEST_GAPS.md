# User-library test gaps (`wj test` vs dogfood crates)

**Audience:** language / tooling owners  
**Date:** 2026-08-02  
**Context:** LedgerKit `finance-screens` (and similar packages) still run **Rust** `cargo test` against WJ-generated `build/lib.rs` even though Windjammer advertises a full test suite (`wj test`, `std::testing` / `std::test`, `@test`).

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

### 2. Dual Cargo identity (WJ crate vs Rust crate)

Packages frequently keep:

- `wj.toml` for Windjammer, and  
- a separate Rust `Cargo.toml` that points at generated sources and path-deps.

`wj test` spins a **fresh** temp Cargo project and copies/rewrites deps. Drift vs the package’s real Cargo features (e.g. `windjammer-ui` `web` vs `desktop`, `default-features`) causes false reds or missing symbols.

**Need:** either consume the package’s Cargo.toml as source of truth for deps/features, or generate an equivalent that matches documented dogfood defaults.

### 3. Discovery UX vs docs / examples mismatch

CLI help text still says “functions starting with `test_`”, while discovery also accepts `@test`. Examples mix `std::testing`, `std::test`, and rich APIs (`mock`, `bench`, `property`) that may not all be wired for library tests.

**Need:** one documented happy path (`tests/foo_test.wj` + `@test` + `std::testing`) that `wj test` guarantees for libraries.

### 4. Harness fragility (runtime / sandbox copy)

`wj test` copies `windjammer-runtime` into the temp tree. That step fails under restricted environments and is a sharp edge for CI agents.

**Need:** depend on a published/path `windjammer-runtime` via Cargo like normal packages, without recursive copy from the compiler checkout.

### 5. Tip ownership asymmetry still breaks “pure WJ” tests

Even when `wj test` runs, library code must avoid Rustisms (`"lit".to_string()`) while tip still emits bare `&str` into some owned `String` formals (external UI builders). Tests that only call pure WJ helpers work; tests that compile UI dogfood hit the same codegen bugs as the app.

**Related gates (already filed):**

- `rust_leakage_string_literal_to_string_forbidden_test` — forbid `"…".to_string()` in WJ source  
- `codegen_bare_string_literal_owned_method_arg_gate_test` — bare lit into owned method must auto-own  
- `codegen_empty_string_option_match_arm_gate_test` — `None => ""` vs `Some(s) => s`  
- `codegen_owned_string_trait_render_concat_gate_test` — cross-module `.render()` concat borrow  

Until those are green, dogfood packages keep a small `owned_string` / interpolation workaround surface — and often keep Rust tests that already compile against tip’s emitted signatures.

### 6. No first-class “integration test against generated lib in-place”

Rust `#[cfg(test)]` / `tests/*.rs` can import `finance_screens` from the **exact** artifact CI ships. `wj test` always recompiles into a temp dir; failures are harder to bisect against `build/*.rs`.

**Need:** optional `wj test --use-build-dir build` (or similar) for dogfood packages.

---

## Recommended acceptance criteria

A dogfood library package can:

1. Keep SOT in `src/**/*.wj` with **no** string-literal `.to_string()`.  
2. Add `tests/foo_test.wj` using `@test` + `std::testing::assert_contains` / `assert_eq`.  
3. Run `wj test` with the **same** compile flags as `wj build` for that package.  
4. Path-depend on `windjammer-ui` (web features) without manual Cargo.toml surgery.  
5. Drop the parallel Rust `tests/*.rs` harness without losing coverage.

Until then, Rust tests against generated libs remain a **temporary adapter**, not a preference.

---

## Interim guidance for product agents

- Prefer WJ SOT + language gates for tip bugs.  
- Keep Rust tests thin: call `crate_api::…`, assert strings/ints — no duplicated product logic.  
- When migrating a suite to `wj test`, do it package-by-package after criteria 1–4 work on tip.  
- Do not reintroduce `"literal".to_string()` in `.wj` to pacify tip; file/extend gates instead.
