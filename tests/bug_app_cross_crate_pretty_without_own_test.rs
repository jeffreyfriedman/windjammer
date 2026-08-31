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

//! Regression guard — cross-crate `pretty(body)` must move owned `String`, not borrow.
//!
//! Ecosystem `wj-fetch` keeps `pretty(own(body))` as a defensive workaround; tip emit is
//! already correct for bare `pretty(body)` in this metadata fixture.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn cross_crate_pretty_body_without_own_must_move_not_borrow() {
    let tmp = TempDir::new().expect("tempdir");

    let lib_src = tmp.path().join("json_util_src");
    fs::create_dir_all(&lib_src).expect("mkdir json_util_src");
    fs::write(lib_src.join("mod.wj"), "pub mod json_util\n").unwrap();
    fs::write(
        lib_src.join("json_util.wj"),
        r#"pub fn pretty(body: string) -> string {
    body
}
"#,
    )
    .unwrap();

    let lib_gen = tmp.path().join("json_util_gen");
    let lib_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            lib_src.to_str().unwrap(),
            "--output",
            lib_gen.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("json_util build");
    assert!(
        lib_build.status.success(),
        "json_util library build failed:\n{}",
        String::from_utf8_lossy(&lib_build.stderr)
    );

    let metadata_path = lib_gen.join("metadata.json");
    assert!(
        metadata_path.exists(),
        "json_util must emit metadata.json for cross-crate calls"
    );

    let app_src = tmp.path().join("fetch_src");
    fs::create_dir_all(&app_src).expect("mkdir fetch_src");
    fs::write(
        app_src.join("format.wj"),
        r#"
use wj_json_util::pretty

pub fn format_body(body: string) -> string {
    pretty(body)
}
"#,
    )
    .unwrap();

    let app_gen = tmp.path().join("fetch_gen");
    let app_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            app_src.join("format.wj").to_str().unwrap(),
            "--output",
            app_gen.to_str().unwrap(),
            "--no-cargo",
            "--metadata",
            &format!("wj_json_util={}", metadata_path.display()),
        ])
        .output()
        .expect("format build");
    assert!(
        app_build.status.success(),
        "format build failed:\n{}",
        String::from_utf8_lossy(&app_build.stderr)
    );

    let generated = fs::read_to_string(app_gen.join("format.rs")).expect("format.rs");
    assert!(
        !generated.contains("pretty(&body") && !generated.contains("pretty(& body"),
        "owned cross-crate formal must receive moved String, not borrow:\n{generated}"
    );
    assert!(
        generated.contains("pretty(body)"),
        "expected bare move into owned String formal:\n{generated}"
    );
}
