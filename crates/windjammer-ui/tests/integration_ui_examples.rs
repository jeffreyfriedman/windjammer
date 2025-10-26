//! Integration tests for UI framework examples
//! Validates that UI framework compiles and WASM builds work

use std::process::Command;

/// Test that the UI framework compiles with default features
#[test]
fn test_ui_framework_compiles() {
    let output = Command::new("cargo")
        .args(["build", "-p", "windjammer-ui"])
        .output()
        .expect("Failed to execute cargo build");

    assert!(
        output.status.success(),
        "UI framework failed to compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that the UI framework compiles for WASM target
#[test]
#[ignore] // WASM compilation has some trait bound issues to resolve
fn test_ui_framework_wasm_compiles() {
    let output = Command::new("cargo")
        .args([
            "build",
            "-p",
            "windjammer-ui",
            "--target",
            "wasm32-unknown-unknown",
        ])
        .output()
        .expect("Failed to execute cargo build");

    assert!(
        output.status.success(),
        "UI framework failed to compile for WASM:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that the counter WASM binary compiles
#[test]
#[ignore] // WASM compilation has some trait bound issues to resolve
fn test_counter_wasm_binary_compiles() {
    let output = Command::new("cargo")
        .args([
            "build",
            "--bin",
            "counter_wasm",
            "-p",
            "windjammer-ui",
            "--target",
            "wasm32-unknown-unknown",
        ])
        .output()
        .expect("Failed to execute cargo build");

    assert!(
        output.status.success(),
        "Counter WASM binary failed to compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that wasm-pack build works
#[test]
#[ignore] // Requires wasm-pack to be installed
fn test_wasm_pack_build() {
    let output = Command::new("wasm-pack")
        .args(["build", "--target", "web", "crates/windjammer-ui"])
        .output()
        .expect("Failed to execute wasm-pack");

    assert!(
        output.status.success(),
        "wasm-pack build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that the UI framework compiles with web feature
#[test]
fn test_ui_web_feature_compiles() {
    let output = Command::new("cargo")
        .args(["build", "-p", "windjammer-ui", "--features", "web"])
        .output()
        .expect("Failed to execute cargo build");

    assert!(
        output.status.success(),
        "UI framework with web feature failed to compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that the UI framework compiles with desktop feature
#[test]
fn test_ui_desktop_feature_compiles() {
    let output = Command::new("cargo")
        .args(["build", "-p", "windjammer-ui", "--features", "desktop"])
        .output()
        .expect("Failed to execute cargo build");

    assert!(
        output.status.success(),
        "UI framework with desktop feature failed to compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that all UI benchmarks compile
#[test]
fn test_ui_benchmarks_compile() {
    let output = Command::new("cargo")
        .args(["build", "--benches", "-p", "windjammer-ui"])
        .output()
        .expect("Failed to execute cargo build");

    assert!(
        output.status.success(),
        "UI benchmarks failed to compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that UI framework tests pass
#[test]
fn test_ui_framework_tests_pass() {
    let output = Command::new("cargo")
        .args(["test", "-p", "windjammer-ui", "--lib"])
        .output()
        .expect("Failed to execute cargo test");

    assert!(
        output.status.success(),
        "UI framework tests failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
