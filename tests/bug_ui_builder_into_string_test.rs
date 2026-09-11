#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
    feature = "integration_tests",
))]

//! FAILING REPRO — owned `string` builder formals must emit `impl Into<String>` so Rust
//! consumers can pass `&str` (windjammer-ui StatusChip / AuthFetch). Tip emits bare `String`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

const CHIP: &str = include_str!("fixtures/library_multipass/ui_builder_into_string.wj");

fn assert_into_string_formals(rs: &str) {
    assert!(
        rs.contains("impl Into<String>"),
        "RED: owned string builder formals must emit impl Into<String> (windjammer-ui). Got:\n{rs}"
    );
    assert!(
        rs.contains(".into()"),
        "RED: Into formals must call .into() when storing. Got:\n{rs}"
    );
}

#[test]
fn ui_builder_string_formal_must_emit_impl_into_string() {
    assert_into_string_formals(&test_utils::compile_single(CHIP));
}

#[test]
fn hexagonal_ui_builder_string_formal_must_emit_impl_into_string() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod domain
pub use domain::chip::StatusChip
"#,
    );
    project.add_file("domain/mod.wj", "pub mod chip\n");
    project.add_file("domain/chip.wj", CHIP);

    let map = project
        .compile()
        .expect("hexagonal StatusChip compile should succeed");
    let chip_key = map
        .keys()
        .find(|k| k.ends_with("chip.rs"))
        .cloned()
        .unwrap_or_else(|| panic!("missing chip.rs; keys: {:?}", map.keys().collect::<Vec<_>>()));
    assert_into_string_formals(map.get(&chip_key).expect("chip.rs"));
}

#[test]
fn ui_builder_string_formal_rust_str_call_site_must_cargo_check() {
    // Tip: StatusChip::new("paid") fails E0308 until Into lands.
    let tmp = TempDir::new().expect("tempdir");
    let wj = tmp.path().join("chip.wj");
    fs::write(&wj, CHIP).unwrap();
    let out = tmp.path().join("build");
    windjammer::compiler::build_project(
        &wj,
        &out,
        windjammer::CompilationTarget::Rust,
        false,
    )
    .expect("transpile chip");

    // Rust consumer mirrors windjammer-ui call sites.
    fs::write(
        out.join("caller_main.rs"),
        r#"
use into_string_repro::StatusChip;
fn main() {
    let _ = StatusChip::new("paid").label("Paid");
}
"#,
    )
    .unwrap();
    fs::write(
        out.join("Cargo.toml"),
        r#"
[package]
name = "into_string_repro"
version = "0.1.0"
edition = "2021"
[lib]
path = "chip.rs"
[[bin]]
name = "caller"
path = "caller_main.rs"
"#,
    )
    .unwrap();

    let shared = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("wj_integration_verify");
    let status = Command::new("cargo")
        .current_dir(&out)
        .env("CARGO_TARGET_DIR", &shared)
        .args(["check", "--bin", "caller", "--quiet"])
        .status()
        .expect("spawn cargo check");
    assert!(
        status.success(),
        "RED: Rust &str call sites on WJ string builders must cargo-check (impl Into<String>)"
    );
}
