//! TDD: Example 47 (simple HTTP server) must compile with sync Server API and no Rust leakage.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn example_47_simple_http_server_compiles_with_runtime() {
    let wj_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/syntax_tests/47_simple_http_server/main.wj");
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
            "example 47 build failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let rs_path = work.path().join("main.rs");
    let generated = std::fs::read_to_string(&rs_path).expect("main.rs");

    assert!(
        generated.contains("windjammer_runtime::http"),
        "expected runtime http import:\n{generated}"
    );
    assert!(
        generated.contains("Server::new"),
        "expected Server::new in generated Rust:\n{generated}"
    );
    assert!(
        generated.contains("windjammer_runtime::io::println"),
        "println must map to runtime io, not Rust macro:\n{generated}"
    );
    assert!(
        !generated.contains("println!("),
        "generated code must not leak Rust println! macro:\n{generated}"
    );
}
