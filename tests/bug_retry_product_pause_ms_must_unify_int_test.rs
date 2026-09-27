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

//! Product tip-out: `packages/wj-retry` `pause_ms` emits `(now as i32) < deadline`.
//! Blocks `wj-fetch` `$WJ test`. Do not reshape the package.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn retry_src() -> Option<PathBuf> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..8 {
        let pkg = dir.join("windjammer-ecosystem/packages/wj-retry/src");
        if pkg.join("lib.wj").exists() {
            return Some(pkg);
        }
        if let Some(parent) = dir.parent() {
            let uncle = parent.join("windjammer-ecosystem/packages/wj-retry/src");
            if uncle.join("lib.wj").exists() {
                return Some(uncle);
            }
            dir = parent.to_path_buf();
        } else {
            break;
        }
    }
    None
}

#[test]
fn retry_product_pause_ms_must_unify_int() {
    let src = retry_src().unwrap_or_else(|| {
        panic!(
            "wj-retry src not found by walking from {}",
            env!("CARGO_MANIFEST_DIR")
        )
    });

    let tmp = TempDir::new().expect("tempdir");
    let out = tmp.path().join("out");
    let wj = env!("CARGO_BIN_EXE_wj");
    let build = Command::new(wj)
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build wj-retry");
    assert!(
        build.status.success(),
        "wj-retry transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    eprintln!(
        "pause_ms sites:\n{}",
        rs.lines()
            .filter(|l| l.contains("now") || l.contains("deadline") || l.contains("pause_ms"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(
        !rs.contains("(now as i32) < deadline") && !rs.contains("(now as i32)< deadline"),
        "product pause_ms must unify timestamp_millis loop ints:\n{rs}"
    );
}
