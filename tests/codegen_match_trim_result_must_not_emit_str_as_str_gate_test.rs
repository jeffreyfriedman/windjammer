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

//! FAILING REPRO — matching `trim()` result must not emit unstable `&str::as_str()`.
//!
//! `let k = s.trim(); match k { ... }` currently codegen as `match k.as_str()` where
//! `k: &str`. Stable Rust's `String::as_str` is fine; `&str::as_str` needs
//! `#![feature(str_as_str)]` (nightly-only / unstable).
//!
//! Desired: match the `&str` directly (`match k { ... }`) or only call `.as_str()`
//! on owned `String`.
//!
//! Language-only; no product/repo names.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn codegen_match_trim_result_must_not_emit_str_as_str() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("t.wj");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
    fs::write(
        &src,
        r#"
pub fn classify(raw: string) -> string {
    let k = raw.trim()
    match k {
        "a" => "one",
        "b" => "two",
        _ => "other",
    }
}

fn main() {
    let _ = classify(" a ")
}
"#,
    )
    .unwrap();

    let wj = env!("CARGO_BIN_EXE_wj");
    let build = Command::new(wj)
        .args([
            "build",
            src.to_str().unwrap(),
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

    let rs = fs::read_to_string(out.join("t.rs")).unwrap_or_else(|_| {
        fs::read_dir(&out)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .find_map(|e| {
                let p = e.path();
                if p.extension().is_some_and(|x| x == "rs") {
                    fs::read_to_string(p).ok()
                } else {
                    None
                }
            })
            .unwrap_or_default()
    });

    assert!(
        !rs.contains(".as_str()"),
        "match on trim() `&str` must not emit `.as_str()` (unstable str_as_str). got:\n{rs}"
    );
    // Sanity: still a match on the trimmed binding or expression.
    assert!(
        rs.contains("match ") && (rs.contains("trim()") || rs.contains("k")),
        "expected match on trimmed value. got:\n{rs}"
    );
}
