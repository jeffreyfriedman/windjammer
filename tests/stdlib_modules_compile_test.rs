//! TDD: Every windjammer_runtime-mapped std module must compile with --check.
//!
//! Reuses conformance stdlib tests plus dedicated http fixture.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

fn wj_check(source: &Path, work: &Path) {
    let output = Command::new(test_utils::wj_binary())
        .args([
            "build",
            source.to_str().unwrap(),
            "-o",
            work.to_str().unwrap(),
            "--check",
        ])
        .output()
        .expect("wj build");

    if !output.status.success() {
        panic!(
            "stdlib compile check failed for {}:\nstdout: {}\nstderr: {}",
            source.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn stdlib_http_server_compiles_with_runtime() {
    let wj_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/stdlib_http_server.wj");
    let work = tempdir().expect("tempdir");
    wj_check(&wj_path, work.path());

    let generated =
        std::fs::read_to_string(work.path().join("stdlib_http_server.rs")).expect("game.rs");
    assert!(
        generated.contains("windjammer_runtime::http"),
        "expected runtime http import:\n{generated}"
    );
}

#[test]
fn stdlib_conformance_modules_compile_with_check() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let conformance_dir = manifest.join("tests/conformance/stdlib");
    let entries: Vec<_> = std::fs::read_dir(&conformance_dir)
        .expect("read conformance/stdlib")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "wj"))
        .collect();

    assert!(
        !entries.is_empty(),
        "expected at least one stdlib conformance .wj under tests/conformance/stdlib"
    );

    for entry in entries {
        let work = tempdir().expect("tempdir");
        wj_check(&entry.path(), work.path());
    }
}

#[test]
fn stdlib_runtime_imports_resolve_in_generated_rust() {
    let wj_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/stdlib_imports_only.wj");
    let work = tempdir().expect("tempdir");
    wj_check(&wj_path, work.path());

    let generated =
        std::fs::read_to_string(work.path().join("stdlib_imports_only.rs")).expect("rs output");
    for needle in [
        "windjammer_runtime::http",
        "windjammer_runtime::json",
        "windjammer_runtime::env",
        "windjammer_runtime::time",
        "windjammer_runtime::log_mod",
        "windjammer_runtime::csv_mod",
        "windjammer_runtime::regex_mod",
    ] {
        assert!(
            generated.contains(needle),
            "expected runtime import {needle} in:\n{generated}"
        );
    }
}
