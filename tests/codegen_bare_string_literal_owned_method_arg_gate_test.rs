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

//! Gate (GREEN) — bare string literals into owned `string` methods must auto-own in codegen.
//!
//! WJ source must not write `"msg".to_string()`. Tip emits ownership at the Rust
//! boundary when the callee formal is owned `String` (e.g. builder `.empty_message(String)`).
//!
//! Language-only; no product/repo names. Fixture uses a sibling-module builder.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn bare_string_literal_into_owned_string_method_must_auto_own() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    let out = tmp.path().join("out");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&out).unwrap();

    fs::write(src.join("mod.wj"), "pub mod ui\npub mod view\n").unwrap();
    fs::write(
        src.join("ui.wj"),
        r#"
pub struct Banner {
    message: string,
}

impl Banner {
    pub fn new() -> Banner {
        Banner { message: "" }
    }

    pub fn message(self, message: string) -> Banner {
        self.message = message
        self
    }

    pub fn render(self) -> string {
        self.message
    }
}
"#,
    )
    .unwrap();
    fs::write(
        src.join("view.wj"),
        r#"
use crate::ui::{Banner}

pub fn show() -> string {
    Banner::new().message("hello").render()
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
        view_rs.contains("message(\"hello\".to_string())")
            || view_rs.contains("message(String::from(\"hello\"))")
            || view_rs.contains("message(String::from(\"hello\").to_string())"),
        "bare literal into owned string method must auto-own in codegen. Got:\n{view_rs}"
    );
    assert!(
        !view_rs.contains(".message(\"hello\")"),
        "must not pass bare &str literal into owned String formal. Got:\n{view_rs}"
    );

    let crate_dir = tmp.path().join("crate");
    fs::create_dir_all(crate_dir.join("src")).unwrap();
    fs::write(
        crate_dir.join("Cargo.toml"),
        r#"[package]
name = "lit_owned_method_gate"
version = "0.0.0"
edition = "2021"
[lib]
path = "src/lib.rs"
"#,
    )
    .unwrap();
    let ui = fs::read_to_string(out.join("ui.rs")).unwrap_or_default();
    let view = view_rs
        .replace("use crate::ui::{Banner};", "use super::ui::Banner;")
        .replace("use crate::ui::Banner;", "use super::ui::Banner;");
    fs::write(
        crate_dir.join("src/lib.rs"),
        format!("#![allow(dead_code, unused)]\nmod ui {{\n{ui}\n}}\nmod view {{\n{view}\n}}\n"),
    )
    .unwrap();
    let check = Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(&crate_dir)
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "bare lit → owned string method must cargo check without WJ-source .to_string(). stderr=\n{}\nview=\n{view_rs}",
        String::from_utf8_lossy(&check.stderr)
    );
}
