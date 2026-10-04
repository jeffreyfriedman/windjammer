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

//! P3.650: `Vec::remove(0)` integer literal must not emit `&0_usize`.
//!
//! Product `wj-proxy` ring-buffer trim:
//! ```ignore
//! let _ = self.logs.remove(0)
//! ```
//! Tip emits `self.logs.remove(&0_usize)` → E0308. Isolate
//! `bug_vec_remove_usize_no_ref` (named local) can false-GREEN; gate product.
//! Do not reshape proxy with casts.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn proxy_app() -> Option<PathBuf> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..8 {
        let sibling = dir.join("windjammer-ecosystem/apps/wj-proxy");
        if sibling.join("src").exists() {
            return Some(sibling);
        }
        if let Some(parent) = dir.parent() {
            let uncle = parent.join("windjammer-ecosystem/apps/wj-proxy");
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
fn vec_remove_usize_literal_must_not_borrow() {
    let app = proxy_app().unwrap_or_else(|| {
        panic!(
            "wj-proxy src not found by walking from {}",
            env!("CARGO_MANIFEST_DIR")
        )
    });

    let tmp = TempDir::new().expect("tempdir");
    let out = tmp.path().join("out");
    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .current_dir(&app)
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
        .expect("wj build wj-proxy");
    assert!(
        build.status.success(),
        "P3.650 wj-proxy transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let proxy = fs::read_to_string(out.join("domain").join("proxy.rs")).unwrap_or_default();
    let bad = proxy.contains("remove(&0_usize)") || proxy.contains("remove(&0)");
    eprintln!(
        "P3.650 remove sites:\n{}",
        proxy
            .lines()
            .filter(|l| l.contains("remove("))
            .collect::<Vec<_>>()
            .join("\n")
    );

    assert!(
        !bad,
        "P3.650 RED: Vec::remove(0) must take owned usize, not &0_usize:\n{proxy}"
    );
}
