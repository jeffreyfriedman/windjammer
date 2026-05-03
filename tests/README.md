# Windjammer compiler integration tests (`tests/`)

This directory contains Rust integration tests for the `windjammer` crate. Each `tests/*.rs` file is a separate test binary (unless configured otherwise in `Cargo.toml`).

## When to use what

| Goal | Use |
|------|-----|
| Parser/analyzer/codegen on **one** snippet, no filesystem | A dedicated `tests/foo_test.rs` with `Lexer` + `Parser` + `Analyzer` + `CodeGenerator`, or the `wj` CLI in a temp file (see `trait_impl_signature_match_test.rs`). |
| **Multi-file** projects, `use` across modules, **multipass** ownership / trait inference | [`integration_test_helpers.rs`](./integration_test_helpers.rs) [`MultiFileTest`](./integration_test_helpers.rs) + [`build_project_ext`](../src/compiler.rs) (same path as real library builds). |
| Nested `src/foo/bar.wj` trees, auto-imports | Temp dir + `build_project_ext` directly (see `nested_module_import_test.rs`, `cross_module_struct_literal_test.rs`). |

## `MultiFileTest` (file-based multipass)

Source of truth: [`integration_test_helpers.rs`](./integration_test_helpers.rs).

1. `MultiFileTest::new()` — temp project with `src/` and `build/`.
2. `add_file("module.wj", "...")` — paths are relative to `src/`.
3. `compile()` → `Result<HashMap<String, String>, String>` — keys are paths under `build/` (e.g. `holder.rs`).
4. `assert_contains("holder.rs", "fn foo")` — compiles then asserts on generated Rust.
5. `assert_compile_error("Parse error")` — expects `compile()` to fail; substring must appear in the error.
6. `assert_compiles_without_error()` — writes a flat `lib.rs` + verification `Cargo.toml`, then runs `cargo check` (slow in a cold tempdir; see ignored test in `integration_test_helpers_self_test.rs`).

### Include the helper in a new test crate

Cargo does not compile `tests/*.rs` as one crate. Pull the helper in with:

```rust
#[path = "integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
```

### Template for a new complex scenario

```rust
#[path = "integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn test_my_cross_file_bug() {
    let mut t = MultiFileTest::new();
    t.add_file(
        "defs.wj",
        r#"
pub struct Thing { pub n: i32 }
"#,
    );
    t.add_file(
        "user.wj",
        r#"
use defs::Thing

pub fn f() -> Thing {
    Thing { n: 1 }
}
"#,
    );
    t.assert_contains("user.rs", "Thing");
}
```

Avoid Rust reserved words as **module** names (`trait`, `impl`, `mod`, …) in file stems when you care about readable generated Rust.

## Related tests

- Cross-file trait ownership (manual multipass helper): `cross_file_trait_impl_test.rs`
- Multi-file multipass + `assert_contains` for trait signatures: `trait_impl_multi_file_test.rs`
- Single-file trait vs impl Rust typecheck (E0053): `trait_impl_signature_match_test.rs`
