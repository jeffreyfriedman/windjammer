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

//! FAILING REPRO — cross-crate app: `own()` forwarder into owned `String` formal emits `&local`
//! (E0308) when callee signatures come from dependency `metadata.json`.
//!
//! Same-crate multipass (`path_pkg` + `app` modules) is tip GREEN. Ecosystem `wj-path` /
//! `wj-template` / `wj-auth-api` fail until cross-crate call sites move owned locals.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn cross_crate_owned_forwarder_must_move_not_borrow() {
    let tmp = TempDir::new().expect("tempdir");

    let path_src = tmp.path().join("path_src");
    fs::create_dir_all(&path_src).expect("mkdir path_src");
    fs::write(
        path_src.join("path_pkg.wj"),
        r#"
pub fn join_path(left: string, right: string) -> string {
    left
}
"#,
    )
    .unwrap();

    let path_gen = tmp.path().join("path_gen");
    let path_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            path_src.to_str().unwrap(),
            "--output",
            path_gen.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("path_pkg build");
    assert!(
        path_build.status.success(),
        "path_pkg library build failed:\n{}",
        String::from_utf8_lossy(&path_build.stderr)
    );

    let metadata_path = path_gen.join("metadata.json");
    assert!(
        metadata_path.exists(),
        "path_pkg must emit metadata.json for cross-crate calls"
    );

    let app_src = tmp.path().join("app_src");
    fs::create_dir_all(&app_src).expect("mkdir app_src");
    fs::write(
        app_src.join("app.wj"),
        r#"
use path_pkg::join_path

pub fn resolve(left: string, right: string) -> string {
    let l = own(left)
    let r = own(right)
    join_path(l, r)
}

fn own(value: string) -> string {
    value
}
"#,
    )
    .unwrap();

    let app_gen = tmp.path().join("app_gen");
    let app_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            app_src.join("app.wj").to_str().unwrap(),
            "--output",
            app_gen.to_str().unwrap(),
            "--no-cargo",
            "--metadata",
            &format!("path_pkg={}", metadata_path.display()),
        ])
        .output()
        .expect("app build");
    assert!(
        app_build.status.success(),
        "app build failed:\n{}",
        String::from_utf8_lossy(&app_build.stderr)
    );

    let generated = fs::read_to_string(app_gen.join("app.rs")).expect("app.rs");
    let bad_emit = generated.contains("join_path(&l")
        || generated.contains("join_path(&left")
        || generated.contains("join_path( &l");
    assert!(
        !bad_emit,
        "RED: cross-crate owned String formal must receive moved local, not borrow:\n{generated}"
    );
    assert!(
        generated.contains("join_path(l") || generated.contains("join_path(l,"),
        "expected bare move into owned String formal:\n{generated}"
    );

    let cargo = Command::new("cargo")
        .arg("check")
        .current_dir(&app_gen)
        .output()
        .expect("cargo check");
    assert!(
        cargo.status.success(),
        "cross-crate moved forwarder must cargo-check.\nstderr:\n{}",
        String::from_utf8_lossy(&cargo.stderr)
    );
}
