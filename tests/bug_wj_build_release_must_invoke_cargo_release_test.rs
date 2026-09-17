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

//! FAILING REPRO — `wj build --release` must pass `--release` to `cargo build`.
//!
//! Ecosystem `wj-sync` fair benches need release codegen; tip CLI takes `-r` /
//! `--release` but `cli/build.rs` binds it as `_release` and always runs
//! `cargo build` (dev). That made debug WJ look ~4× slower than release Rust.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn wj_build_release_must_invoke_cargo_release() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    fs::create_dir_all(&src).unwrap();

    fs::write(
        temp.path().join("wj.toml"),
        r#"[package]
name = "wj-release-gate"
version = "0.1.0"
edition = "2025"
"#,
    )
    .unwrap();

    fs::write(
        src.join("main.wj"),
        r#"fn main() {
    println("ok")
}
"#,
    )
    .unwrap();

    let wj = test_utils::wj_binary();
    let output = Command::new(&wj)
        .args([
            "build",
            "--release",
            src.to_str().unwrap(),
            "--output",
            build.to_str().unwrap(),
        ])
        .current_dir(temp.path())
        .output()
        .expect("run wj build --release");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "wj build --release failed:\nstdout={stdout}\nstderr={stderr}"
    );

    let release_bin = build.join("target/release/wj_release_gate");
    let debug_bin = build.join("target/debug/wj_release_gate");
    let combined = format!("{stdout}{stderr}");
    assert!(
        release_bin.exists() || combined.contains("Finished `release`"),
        "wj build --release must run `cargo build --release` (P3.350).\n\
         expected {release_bin:?} or Finished `release` in output.\n\
         debug_exists={} stdout=\n{stdout}\nstderr=\n{stderr}",
        debug_bin.exists()
    );
    assert!(
        !debug_bin.exists() || release_bin.exists() || combined.contains("Finished `release`"),
        "release flag must not leave only a debug artifact:\n{combined}"
    );
}
