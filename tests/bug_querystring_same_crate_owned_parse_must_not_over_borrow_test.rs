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

//! Same-crate `parse(text: String)` / `stringify(pairs: Vec)` must not over-borrow
//! because a global homonym (`url::parse`, `HashMap::get`) emits `&str` / `&K`.
//!
//! Ecosystem `wj-querystring`: E0308 `expected String, found &String` at
//! `parse(&query)` and `expected Vec, found &Vec` at `stringify(&kept)`.
//! Inverse of `bug_notes_api_cross_crate_owned_into_demoted_str_must_auto_borrow_test`
//! (path-dep demoted `&str` must borrow). Shared≠Lock: one `parse` name.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn build_library(wj: &str, src_dir: &std::path::Path, out_dir: &std::path::Path) {
    let status = Command::new(wj)
        .args([
            "build",
            src_dir.to_str().unwrap(),
            "--output",
            out_dir.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("library build");
    assert!(
        status.status.success(),
        "library build failed:\n{}",
        String::from_utf8_lossy(&status.stderr)
    );
}

#[test]
fn same_crate_owned_parse_must_not_over_borrow() {
    let tmp = TempDir::new().expect("tempdir");
    let wj = env!("CARGO_BIN_EXE_wj");

    let src = tmp.path().join("qs_src");
    fs::create_dir_all(&src).expect("mkdir qs_src");
    fs::write(
        src.join("lib.wj"),
        r#"
use std::strings

pub fn parse(text: string) -> Vec<(string, string)> {
    let mut pairs = Vec::new()
    if strings.len(text) == 0 {
        return pairs
    }
    pairs.push((text, empty_string()))
    pairs
}

fn empty_string() -> string {
    ""
}

pub fn get(query: string, key: string) -> Option<string> {
    let pairs = parse(query)
    let mut i = 0
    while i < pairs.len() {
        let pair = pairs[i]
        if pair.0 == key {
            return Some(pair.1)
        }
        i = i + 1
    }
    None
}

pub fn stringify(pairs: Vec<(string, string)>) -> string {
    let mut pairs = pairs
    if pairs.len() == 0 {
        return empty_string()
    }
    let pair = pairs[0]
    pair.0
}

pub fn remove(query: string, key: string) -> string {
    let pairs = parse(query)
    stringify(pairs)
}

pub fn has(query: string, key: string) -> bool {
    match get(query, key) {
        Some(_) => true,
        None => false,
    }
}
"#,
    )
    .unwrap();

    let gen = tmp.path().join("qs_gen");
    build_library(wj, &src, &gen);
    let rs = fs::read_to_string(gen.join("lib.rs")).unwrap_or_default();
    eprintln!("generated:\n{rs}");

    assert!(
        rs.contains("text: String") || rs.contains("text:String"),
        "parse must keep owned String formal:\n{rs}"
    );
    assert!(
        !rs.contains("parse(&query)") && !rs.contains("parse(& query)"),
        "same-crate owned parse must not over-borrow query:\n{rs}"
    );
    assert!(
        !rs.contains("stringify(&pairs)") && !rs.contains("stringify(& pairs)"),
        "same-crate owned stringify must not over-borrow pairs:\n{rs}"
    );
    assert!(
        !rs.contains("get(&query") && !rs.contains("get(& query"),
        "same-crate get first arg is owned String — must not over-borrow:\n{rs}"
    );

    let cargo = gen.join("Cargo.toml");
    if cargo.exists() {
        let check = Command::new("cargo")
            .current_dir(&gen)
            .args(["check", "--quiet"])
            .output()
            .expect("cargo check");
        assert!(
            check.status.success(),
            "same-crate owned parse/stringify must cargo-check.\n{rs}\nstderr:\n{}",
            String::from_utf8_lossy(&check.stderr)
        );
    }
}
