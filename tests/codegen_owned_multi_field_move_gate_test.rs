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

//! Gate — WDB-096: moving multiple non-Copy fields from an owned struct into
//! owned formals must keep the root Owned and move fields (no `.clone()`).
//!
//! Pattern (PageRank buffer release):
//! ```text
//! return_f64(buf.scores)
//! return_f64(buf.next)
//! return_f64(buf.contrib)
//! ```

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn read_emitted_rs(out_dir: &Path, stem: &str) -> String {
    fs::read_to_string(out_dir.join(format!("{stem}.rs"))).unwrap_or_else(|_| {
        fs::read_dir(out_dir)
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
    })
}

fn build_source(source: &str, stem: &str) -> String {
    let tmp = TempDir::new().unwrap();
    let src_path = tmp.path().join(format!("{stem}.wj"));
    fs::write(&src_path, source).unwrap();
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
    let wj = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj)
        .args([
            "build",
            src_path.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("wj build");
    assert!(
        output.status.success(),
        "wj build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    read_emitted_rs(&out, stem)
}

const RELEASE_SOURCE: &str = r#"
pub struct Buf {
    pub scores: Vec<f64>,
    pub next: Vec<f64>,
    pub contrib: Vec<f64>,
}

fn return_f64(v: Vec<f64>) {}

pub fn release(buf: Buf) {
    return_f64(buf.scores)
    return_f64(buf.next)
    return_f64(buf.contrib)
}

pub fn release_via_lets(buf: Buf) {
    let scores = buf.scores
    let next = buf.next
    let contrib = buf.contrib
    return_f64(scores)
    return_f64(next)
    return_f64(contrib)
}
"#;

#[test]
fn test_wdb096_multi_field_call_moves_without_clone() {
    let rust = build_source(RELEASE_SOURCE, "pagerank_release");
    eprintln!("Generated Rust:\n{rust}");

    assert!(
        rust.contains("pub fn release(buf: Buf)")
            || rust.contains("pub fn release(mut buf: Buf)"),
        "WDB-096: release must keep owned Buf so fields can move. Got:\n{rust}"
    );
    assert!(
        !rust.contains("pub fn release(buf: &Buf)"),
        "WDB-096: must not demote buf to &Buf when moving fields into owned formals. Got:\n{rust}"
    );
    assert!(
        !rust.contains("buf.scores.clone()")
            && !rust.contains("buf.next.clone()")
            && !rust.contains("buf.contrib.clone()"),
        "WDB-096: distinct field moves into owned Vec formals must not clone. Got:\n{rust}"
    );
    assert!(
        rust.contains("return_f64(buf.scores)")
            && rust.contains("return_f64(buf.next)")
            && rust.contains("return_f64(buf.contrib)"),
        "WDB-096: expected bare field moves into return_f64. Got:\n{rust}"
    );
}

#[test]
fn test_wdb096_multi_field_let_extract_moves_without_clone() {
    let rust = build_source(RELEASE_SOURCE, "pagerank_release");
    eprintln!("Generated Rust:\n{rust}");

    assert!(
        rust.contains("pub fn release_via_lets(buf: Buf)")
            || rust.contains("pub fn release_via_lets(mut buf: Buf)"),
        "WDB-096: let-extract release must keep owned Buf. Got:\n{rust}"
    );
    assert!(
        !rust.contains("buf.scores.clone()")
            && !rust.contains("buf.next.clone()")
            && !rust.contains("buf.contrib.clone()"),
        "WDB-096: let-bound field extracts from owned Buf must move, not clone. Got:\n{rust}"
    );
}
