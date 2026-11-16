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

//! Cross-crate call sites take every argument's borrow or ownership from the callee's declaration, read from the
//! dependency's metadata: a read-only `Vec<u8>` parameter of the dependency is emitted `&Vec<u8>`, so the dependent
//! crate passes a borrow whether the argument is a local, a parameter or an explicit `.clone()` (rustc E0308
//! `expected &Vec<u8>, found Vec<u8>` when the clone was kept by value). The gate compiles both generated crates
//! with rustc: the dependency as an rlib, the dependent crate against it.

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn wj_build(src: &Path, out: &Path, extra: &[String]) {
    let mut args: Vec<String> = ["build", src.to_str().unwrap(), "--output", out.to_str().unwrap(), "--library", "--no-cargo", "--module-file", "--no-generate-cargo-toml", "--no-lint"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    args.extend(extra.iter().cloned());
    let out = Command::new(env!("CARGO_BIN_EXE_wj")).args(&args).output().expect("run wj");
    assert!(out.status.success(), "wj build failed:\n{}", String::from_utf8_lossy(&out.stderr));
}

fn rustc(args: &[&str]) -> std::process::Output {
    Command::new(std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string())).args(args).output().expect("run rustc")
}

#[test]
fn dependent_crate_borrows_for_a_read_only_dependency_parameter() {
    let tmp = TempDir::new().expect("tempdir");
    let a_src = tmp.path().join("bytes_src");
    fs::create_dir_all(&a_src).unwrap();
    fs::write(a_src.join("mod.wj"), "pub mod k\n").unwrap();
    fs::write(
        a_src.join("k.wj"),
        "pub fn total(v: Vec<u8>) -> u32 {\n    let mut s: u32 = 0\n    let mut i: usize = 0\n    while i < v.len() {\n        s = s + (v[i] as u32)\n        i = i + 1\n    }\n    s\n}\n\npub fn slice_bytes(data: Vec<u8>, start: usize, end: usize) -> Vec<u8> {\n    let mut out: Vec<u8> = Vec::new()\n    let mut i: usize = start\n    while i < end && i < data.len() {\n        out.push(data[i])\n        i = i + 1\n    }\n    out\n}\n",
    )
    .unwrap();
    let a_gen = tmp.path().join("bytes_gen");
    wj_build(&a_src, &a_gen, &[]);
    let k_rs = fs::read_to_string(a_gen.join("k.rs")).unwrap();
    assert!(k_rs.contains("pub fn total(v: &Vec<u8>)"), "the dependency borrows its read-only parameter:\n{k_rs}");

    let b_src = tmp.path().join("app_src");
    fs::create_dir_all(&b_src).unwrap();
    fs::write(b_src.join("mod.wj"), "pub mod m\n").unwrap();
    fs::write(
        b_src.join("m.wj"),
        "use bytes::k::total\nuse bytes::k::slice_bytes\n\npub fn with_clone(items: Vec<Vec<u8>>) -> u32 {\n    total(items[0].clone())\n}\n\npub fn with_local(items: Vec<Vec<u8>>) -> u32 {\n    let first = items[0].clone()\n    total(first)\n}\n\npub fn with_param(v: Vec<u8>) -> u32 {\n    total(v)\n}\n\npub fn head(data: Vec<u8>) -> Vec<u8> {\n    let body = slice_bytes(data.clone(), 0, data.len() - 1)\n    body\n}\n",
    )
    .unwrap();
    let b_gen = tmp.path().join("app_gen");
    wj_build(&b_src, &b_gen, &["--metadata".to_string(), format!("bytes={}", a_gen.join("metadata.json").display())]);
    let m_rs = fs::read_to_string(b_gen.join("m.rs")).unwrap();
    assert!(!m_rs.contains("total(items[0].clone())"), "an explicit clone argument is borrowed, not passed by value:\n{m_rs}");
    assert!(!m_rs.contains("slice_bytes(data.clone()"), "a clone of a parameter is borrowed:\n{m_rs}");

    let rlib = tmp.path().join("libbytes.rlib");
    let a = rustc(&["--edition", "2021", "--crate-type", "lib", "--crate-name", "bytes", a_gen.join("lib.rs").to_str().unwrap(), "-o", rlib.to_str().unwrap()]);
    assert!(a.status.success(), "dependency rlib:\n{}", String::from_utf8_lossy(&a.stderr));
    let rmeta = tmp.path().join("libapp.rmeta");
    let b = rustc(&[
        "--edition",
        "2021",
        "--crate-type",
        "lib",
        "--crate-name",
        "app",
        "--extern",
        &format!("bytes={}", rlib.display()),
        "--emit=metadata",
        "-o",
        rmeta.to_str().unwrap(),
        b_gen.join("lib.rs").to_str().unwrap(),
    ]);
    assert!(b.status.success(), "the dependent crate type-checks against the dependency:\n{}\n{m_rs}", String::from_utf8_lossy(&b.stderr));
}
