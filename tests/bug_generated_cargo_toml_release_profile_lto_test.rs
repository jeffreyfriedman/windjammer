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

//! Generated Cargo.toml must honor `wj.toml` `[profile.release]` (LTO default).
//! Without LTO, cross-crate hot paths (wj-sync Shared/Counter) look falsely slow.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn generated_cargo_toml_defaults_lto_true_for_release() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    fs::create_dir_all(&src).unwrap();

    fs::write(
        temp.path().join("wj.toml"),
        r#"[package]
name = "lto-default"
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
            "--no-cargo",
            src.to_str().unwrap(),
            "--output",
            build.to_str().unwrap(),
        ])
        .current_dir(temp.path())
        .output()
        .expect("wj build");

    assert!(
        output.status.success(),
        "wj build failed:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let cargo = fs::read_to_string(build.join("Cargo.toml")).expect("Cargo.toml");
    assert!(
        cargo.contains("[profile.release]"),
        "missing [profile.release]:\n{cargo}"
    );
    assert!(
        cargo.contains("lto = true") || cargo.contains("lto=true"),
        "release profile must default lto = true (cross-crate inlining):\n{cargo}"
    );
    assert!(
        cargo.contains("opt-level"),
        "release profile must set opt-level:\n{cargo}"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn generated_cargo_toml_honors_wj_toml_profile_release() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    fs::create_dir_all(&src).unwrap();

    fs::write(
        temp.path().join("wj.toml"),
        r#"[package]
name = "lto-custom"
version = "0.1.0"
edition = "2025"

[profile.release]
opt-level = 2
lto = false
codegen-units = 16
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
            "--no-cargo",
            src.to_str().unwrap(),
            "--output",
            build.to_str().unwrap(),
        ])
        .current_dir(temp.path())
        .output()
        .expect("wj build");

    assert!(
        output.status.success(),
        "wj build failed:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let cargo = fs::read_to_string(build.join("Cargo.toml")).expect("Cargo.toml");
    assert!(
        cargo.contains("opt-level = 2"),
        "must honor wj.toml opt-level = 2:\n{cargo}"
    );
    assert!(
        cargo.contains("lto = false") || cargo.contains("lto=false"),
        "must honor wj.toml lto = false:\n{cargo}"
    );
    assert!(
        cargo.contains("codegen-units = 16"),
        "must honor wj.toml codegen-units:\n{cargo}"
    );
}
