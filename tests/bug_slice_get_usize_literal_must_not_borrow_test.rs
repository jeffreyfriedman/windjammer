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

//! P3.649: `slice`/`Vec::get(0)` integer literals must not emit `&0_usize`.
//!
//! Product `wj-cron` `parse_cron`:
//! ```ignore
//! match parts.get(0) { … }
//! match parts.get(1) { … }
//! ```
//! Tip emits `parts.get(&0_usize)` … → E0277. Blocks scheduler (path-dep cron).
//! Distinct from P3.644 (variable `idx as usize` in wj-sync pool). Do not reshape
//! cron with casts.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn cron_pkg() -> Option<PathBuf> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..8 {
        let sibling = dir.join("windjammer-ecosystem/packages/wj-cron");
        if sibling.join("src").exists() {
            return Some(sibling);
        }
        if let Some(parent) = dir.parent() {
            let uncle = parent.join("windjammer-ecosystem/packages/wj-cron");
            if uncle.join("src").exists() {
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
fn slice_get_usize_literal_must_not_borrow() {
    let pkg = cron_pkg().unwrap_or_else(|| {
        panic!(
            "wj-cron src not found by walking from {}",
            env!("CARGO_MANIFEST_DIR")
        )
    });

    let tmp = TempDir::new().expect("tempdir");
    let out = tmp.path().join("out");
    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .current_dir(&pkg)
        .args([
            "build",
            "src",
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build wj-cron");
    assert!(
        build.status.success(),
        "P3.649 wj-cron transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let lib = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    let bad_sites: Vec<&str> = lib
        .lines()
        .filter(|l| l.contains(".get(&") && l.contains("_usize"))
        .collect();
    eprintln!("P3.649 borrowed usize get sites:\n{}", bad_sites.join("\n"));

    assert!(
        bad_sites.is_empty(),
        "P3.649 RED: Vec/slice .get(N) must take owned usize, not &N_usize:\n{}",
        bad_sites.join("\n")
    );
}
