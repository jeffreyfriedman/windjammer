//! Partial `@derive(Clone)` / `@auto(Clone)` must still merge inferred traits (Debug, Copy, …).
//!
//! Dogfooding: game structs often use `@derive(Clone)` only; parent enums use `#[derive(Debug)]`
//! and require nested structs to implement `Debug`. Merging explicit derives with inference
//! fixes E0277 (`T doesn't implement Debug`).

use std::process::Command;
use tempfile::tempdir;

fn compile_wj_to_rust(source: &str) -> (String, bool) {
    let dir = tempdir().expect("tempdir");

    let wj_file = dir.path().join("test.wj");
    std::fs::write(&wj_file, source).unwrap();

    let _output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj_file.to_str().unwrap(),
            "--output",
            dir.path().to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj compiler");

    let src_dir = dir.path().join("src");
    let main_rs = if src_dir.join("main.rs").exists() {
        src_dir.join("main.rs")
    } else {
        dir.path().join("test.rs")
    };

    let rs_content = std::fs::read_to_string(&main_rs).unwrap_or_default();

    let rlib_output = dir.path().join("test.rlib");
    let rustc = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            "-o",
            rlib_output.to_str().unwrap(),
        ])
        .arg(&main_rs)
        .output()
        .expect("Failed to run rustc");

    (rs_content, rustc.status.success())
}

#[test]
fn test_partial_derive_clone_merges_debug_for_nested_enum() {
    let source = r#"
@derive(Clone)
pub struct Inner {
    x: i32,
}

pub enum Outer {
    V(Inner),
}

pub fn main() {
    let i = Inner { x: 1 }
    let o = Outer::V(i)
    println!("{:?}", o)
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(
        compiles,
        "Nested enum Debug requires Inner: Debug. rustc failed. Generated:\n{}",
        rs
    );
    assert!(
        rs.contains("derive(") && rs.contains("Debug") && rs.contains("Clone"),
        "Expected merged derive(Debug, Clone, ...) on Inner, got:\n{}",
        rs
    );
}

#[test]
fn test_partial_auto_clone_merges_debug() {
    let source = r#"
@auto(Clone)
pub struct Inner {
    x: i32,
}

pub fn main() {
    let i = Inner { x: 1 }
    println!("{:?}", i)
}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(
        compiles,
        "println! Debug requires Inner: Debug. rustc failed. Generated:\n{}",
        rs
    );
    assert!(
        rs.contains("Debug") && rs.contains("Clone"),
        "Expected merged derive on Inner, got:\n{}",
        rs
    );
}
