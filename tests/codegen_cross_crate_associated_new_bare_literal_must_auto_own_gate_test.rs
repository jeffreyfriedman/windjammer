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
))]

//! FAILING REPRO — bare string literals into owned `string` formals on
//! **cross-crate** associated `Type::new` must auto-own in codegen.
//!
//! Same-crate `Column::new("Name")` already emits `"Name".to_string()` (signature
//! resolved). Tip intentionally stopped method-name heuristics when the signature
//! is absent (`do not guess from method names like new/from`). External crates
//! (path deps / prebuilt UI) often have **no WJ signature**, so tip now emits a
//! bare `"Name"` (`&str`) into `fn new(header: String)` → E0308.
//!
//! Desired: still auto-own at the Rust boundary for owned `string` formals even
//! when the callee is an external / unresolved associated `::new` — without
//! requiring `"…".to_string()` in WJ source.
//!
//! Language-only; no product/repo names.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn write_ext_crate(root: &std::path::Path) {
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "extcol"
version = "0.0.0"
edition = "2021"
[lib]
path = "src/lib.rs"
"#,
    )
    .unwrap();
    fs::write(
        root.join("src/lib.rs"),
        r#"
pub struct Column {
    pub header: String,
}

impl Column {
    pub fn new(header: String) -> Self {
        Self { header }
    }
}
"#,
    )
    .unwrap();
}

#[test]
fn cross_crate_associated_new_bare_literal_must_auto_own() {
    let tmp = TempDir::new().unwrap();
    let ext = tmp.path().join("extcol");
    let src = tmp.path().join("src");
    let out = tmp.path().join("out");
    write_ext_crate(&ext);
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&out).unwrap();

    fs::write(src.join("mod.wj"), "pub mod view\n").unwrap();
    fs::write(
        src.join("view.wj"),
        r#"
use extcol::Column

pub fn make() -> Column {
    Column::new("Name")
}
"#,
    )
    .unwrap();

    let wj = env!("CARGO_BIN_EXE_wj");
    let build = Command::new(wj)
        .args([
            "build",
            src.join("mod.wj").to_str().unwrap(),
            "--module-file",
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("run wj");
    assert!(
        build.status.success(),
        "wj build must succeed. stderr=\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let view_rs = fs::read_to_string(out.join("view.rs")).unwrap_or_default();
    assert!(
        view_rs.contains("Column::new(\"Name\".to_string())")
            || view_rs.contains("Column::new(String::from(\"Name\"))")
            || view_rs.contains("Column::new(\"Name\".to_owned())"),
        "cross-crate Type::new(string) bare lit must auto-own in codegen. Got:\n{view_rs}"
    );
    assert!(
        !view_rs.contains("Column::new(\"Name\")"),
        "must not pass bare &str into owned String formal on external ::new. Got:\n{view_rs}"
    );

    // cargo-check against the external Rust crate (no WJ analysis of formals).
    let crate_dir = tmp.path().join("crate");
    fs::create_dir_all(crate_dir.join("src")).unwrap();
    let ext_path = ext.display().to_string().replace('\\', "/");
    fs::write(
        crate_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "cross_crate_new_lit_gate"
version = "0.0.0"
edition = "2021"
[lib]
path = "src/lib.rs"
[dependencies]
extcol = {{ path = "{ext_path}" }}
"#
        ),
    )
    .unwrap();
    let view = view_rs
        .replace("use crate::", "use ")
        .replace("use super::*;\n", "");
    fs::write(
        crate_dir.join("src/lib.rs"),
        format!("#![allow(dead_code, unused)]\n{view}\n"),
    )
    .unwrap();
    let check = Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(&crate_dir)
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "cross-crate bare lit → owned ::new must cargo check without WJ-source .to_string(). stderr=\n{}\nview=\n{view_rs}",
        String::from_utf8_lossy(&check.stderr)
    );
}
