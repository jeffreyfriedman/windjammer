//! TDD: std::http Server API must compile to windjammer_runtime::http and pass cargo check.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn stdlib_http_server_compiles_with_runtime_imports() {
    let wj_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/stdlib_http_server.wj");
    let work = tempdir().expect("tempdir");

    let output = Command::new(test_utils::wj_binary())
        .args([
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            work.path().to_str().unwrap(),
            "--check",
        ])
        .output()
        .expect("wj build");

    if !output.status.success() {
        panic!(
            "stdlib http server build failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let rs_path = work.path().join("stdlib_http_server.rs");
    let main_content = std::fs::read_to_string(&rs_path).expect("stdlib_http_server.rs");
    assert!(
        main_content.contains("windjammer_runtime::http"),
        "expected runtime http import in generated Rust:\n{main_content}"
    );
    assert!(
        main_content.contains("Server::new"),
        "expected Server API in generated Rust:\n{main_content}"
    );
}
