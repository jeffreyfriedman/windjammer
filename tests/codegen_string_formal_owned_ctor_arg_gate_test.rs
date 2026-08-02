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

//! FAILING REPRO — `string` formals used as owned ctor args must stay / become `String`.
//!
//! Tip demotes `risk_band: string` → `&str`, then passes it into
//! `Badge::new(risk_band)` / similar owned-`String` constructors → E0308.
//! Prefer `"${risk_band}"` (or keep the formal owned) — never `"".to_string() + …`.
//!
//! Language-only; no product/repo names.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn string_formal_owned_ctor_arg_must_not_demote_to_str() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    let out = tmp.path().join("out");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&out).unwrap();

    fs::write(src.join("mod.wj"), "pub mod badge\npub mod view\n").unwrap();
    fs::write(
        src.join("badge.wj"),
        r#"
pub struct ScoreBadge {
    score: int,
    band: string,
}

impl ScoreBadge {
    pub fn new(score: int, band: string) -> ScoreBadge {
        ScoreBadge { score: score, band: band }
    }

    pub fn render(self) -> string {
        "${self.score}:${self.band}"
    }
}
"#,
    )
    .unwrap();
    fs::write(
        src.join("view.wj"),
        r#"
use crate::badge::{ScoreBadge}

pub fn dashboard(score: int, risk_band: string) -> string {
    ScoreBadge::new(score, risk_band).render()
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
        !view_rs.contains("ScoreBadge::new(score, risk_band)")
            || view_rs.contains("risk_band.to_string()")
            || view_rs.contains("risk_band: String"),
        "must pass owned String into ScoreBadge::new. Got:\n{view_rs}"
    );

    let crate_dir = tmp.path().join("crate");
    fs::create_dir_all(crate_dir.join("src")).unwrap();
    fs::write(
        crate_dir.join("Cargo.toml"),
        r#"[package]
name = "owned_ctor_gate"
version = "0.0.0"
edition = "2021"
[lib]
path = "src/lib.rs"
"#,
    )
    .unwrap();
    let badge = fs::read_to_string(out.join("badge.rs")).unwrap_or_default();
    let view = view_rs
        .replace("use crate::badge::{ScoreBadge};", "use super::badge::ScoreBadge;")
        .replace("use crate::badge::ScoreBadge;", "use super::badge::ScoreBadge;");
    fs::write(
        crate_dir.join("src/lib.rs"),
        format!("#![allow(dead_code, unused)]\nmod badge {{\n{badge}\n}}\nmod view {{\n{view}\n}}\n"),
    )
    .unwrap();
    let check = Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(&crate_dir)
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "owned string ctor arg must cargo check. stderr=\n{}\nview=\n{view_rs}",
        String::from_utf8_lossy(&check.stderr)
    );
}
