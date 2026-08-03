//! Gate: std::http `HttpMethod` enum must compile and match on `ServerRequest.method`.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn stdlib_http_method_enum_matches_on_server_request() {
    let wj_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/stdlib_http_method_gate.wj");
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
            "HttpMethod gate build failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let rs_path = work.path().join("stdlib_http_method_gate.rs");
    let main_content = std::fs::read_to_string(&rs_path).expect("stdlib_http_method_gate.rs");
    assert!(
        main_content.contains("windjammer_runtime::http"),
        "expected runtime http import in generated Rust:\n{main_content}"
    );
    assert!(
        main_content.contains("HttpMethod"),
        "expected HttpMethod in generated Rust:\n{main_content}"
    );
}
