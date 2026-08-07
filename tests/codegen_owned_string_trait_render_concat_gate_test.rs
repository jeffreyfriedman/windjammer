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

//! Gate (GREEN) — `string + local` after a **cross-module** trait `.render()` must borrow.
//!
//! Same-file trait methods correctly emit `html + &chip`. When `.render()` is defined
//! in a sibling module and called across `use crate::…`, tip omits `&` on the owned
//! `String` local → E0308 (`expected &str, found String`).
//!
//! Language-only; no product/repo names.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn cross_module_trait_render_concat_must_borrow_rhs() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    let out = tmp.path().join("out");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&out).unwrap();

    fs::write(src.join("mod.wj"), "pub mod chips\npub mod views\n").unwrap();
    fs::write(
        src.join("chips.wj"),
        r#"
pub trait Renderable {
    fn render(self) -> string
}

pub struct Label {
    text: string,
}

impl Renderable for Label {
    fn render(self) -> string {
        self.text
    }
}
"#,
    )
    .unwrap();
    fs::write(
        src.join("views.wj"),
        r#"
use crate::chips::{Label, Renderable}

pub fn wrap_label(text: string) -> string {
    let chip = Label { text: text }.render()
    let mut html = "<span>"
    html = html + chip
    html = html + "</span>"
    html
}

pub fn banner_plus_body(ok: bool) -> string {
    let banner = if ok {
        "ok"
    } else {
        "err"
    }
    let body = Label { text: "body" }.render()
    banner + body
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

    let views_rs = fs::read_to_string(out.join("views.rs")).unwrap_or_else(|_| {
        fs::read_dir(&out)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .find_map(|e| {
                let p = e.path();
                let name = p.file_name()?.to_string_lossy().into_owned();
                if name.contains("views") && p.extension().is_some_and(|x| x == "rs") {
                    fs::read_to_string(p).ok()
                } else {
                    None
                }
            })
            .unwrap_or_default()
    });

    assert!(
        !views_rs.contains("html + chip") && !views_rs.contains("banner + body"),
        "cross-module trait render concat RHS must borrow (&chip / &body). Got:\n{views_rs}"
    );
    assert!(
        views_rs.contains("html + &chip") || views_rs.contains("+ &chip"),
        "expected borrowed chip concat. Got:\n{views_rs}"
    );
    assert!(
        views_rs.contains("banner + &body") || views_rs.contains("+ &body"),
        "expected borrowed body concat. Got:\n{views_rs}"
    );

    let crate_dir = tmp.path().join("crate");
    fs::create_dir_all(crate_dir.join("src")).unwrap();
    fs::write(
        crate_dir.join("Cargo.toml"),
        r#"[package]
name = "render_concat_gate"
version = "0.0.0"
edition = "2021"
[lib]
path = "src/lib.rs"
"#,
    )
    .unwrap();
    let chips = fs::read_to_string(out.join("chips.rs")).unwrap_or_default();
    // views.rs already has `use crate::chips…` — rewrite for flat lib.
    let views_body = views_rs
        .replace("use crate::chips::{Label, Renderable};", "")
        .replace("use crate::chips::Label;", "")
        .replace("use crate::chips::Renderable;", "");
    let lib = format!(
        "#![allow(dead_code, unused)]\nmod chips {{\n{chips}\n}}\nmod views {{\nuse super::chips::{{Label, Renderable}};\n{views_body}\n}}\n"
    );
    fs::write(crate_dir.join("src/lib.rs"), lib).unwrap();
    let check = Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(&crate_dir)
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "cargo check must succeed after render-concat borrow. stderr=\n{}\nemitted views=\n{views_rs}",
        String::from_utf8_lossy(&check.stderr)
    );
}
