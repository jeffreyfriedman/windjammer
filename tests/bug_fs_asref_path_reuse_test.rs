#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

//! `fs` AsRef<Path> contracts: reused path locals must not move.
//! Library builds (ecosystem packages) demote formals forwarded only to borrowing
//! fs APIs so `write → load → remove` works without clones.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[path = "common/test_utils.rs"]
mod test_utils;

fn build_lib_src(src: &str) -> String {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("wj.toml"),
        r#"[package]
name = "fs-asref-seed"
version = "0.1.0"
edition = "2025"

[lib]
"#,
    )
    .unwrap();
    fs::write(root.join("src/lib.wj"), src).unwrap();

    let out = root.join("out");
    let status = Command::new(test_utils::wj_binary())
        .args([
            "build",
            "--library",
            "--no-cargo",
            "-o",
        ])
        .arg(&out)
        .arg(root.join("src"))
        .current_dir(root)
        .status()
        .expect("run wj build");
    assert!(status.success(), "wj build --library failed");
    fs::read_to_string(out.join("lib.rs")).expect("read lib.rs")
}

#[test]
fn fs_write_then_reuse_path_does_not_move() {
    let generated = build_lib_src(
        r#"
use std::fs

pub fn write_then_read(path: string) -> Result<string, string> {
    match fs.write(path, "APP=1\n") {
        Ok(_) => {},
        Err(e) => return Err(e),
    }
    fs.read_to_string(path)
}
"#,
    );

    let write_moves =
        generated.contains("fs::write(path,") && !generated.contains("fs::write(&path");
    let read_moves = generated.contains("read_to_string(path)")
        && !generated.contains("read_to_string(&path)")
        && !generated.contains("fn write_then_read(path: &str)");
    assert!(
        !(write_moves && read_moves),
        "cannot move path into both write and read_to_string:\n{generated}"
    );
}

#[test]
fn load_path_forwarded_to_fs_read_demotes_to_borrow() {
    // Match-scrutinee form matches ecosystem packages (`wj-dotenv`); bare
    // `return fs.read_to_string(path)` still keeps owned formals today.
    let generated = build_lib_src(
        r#"
use std::fs

pub fn load(path: string) -> Result<string, string> {
    match fs.read_to_string(path) {
        Ok(content) => Ok(content),
        Err(e) => Err(e),
    }
}

pub fn write_load_remove(path: string) -> Result<(), string> {
    match fs.write(path, "x\n") {
        Ok(_) => {},
        Err(e) => return Err(e),
    }
    match load(path) {
        Ok(_) => {},
        Err(e) => return Err(e),
    }
    fs.remove_file(path)
}
"#,
    );
    assert!(
        generated.contains("fn load(path: &str)")
            || generated.contains("fn load(path: &String)"),
        "load must demote path to borrow so write→load→remove reuses it:\n{generated}"
    );
}
