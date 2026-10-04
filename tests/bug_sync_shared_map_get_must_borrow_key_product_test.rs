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

//! P3.664: product `wj-sync` SharedMap get must borrow owned String keys.
//!
//! ```ignore
//! match g.get(key) { … }   // after m.inner.lock()
//! ```
//! Tip emits bare `g.get(key)` → E0308. Isolate
//! `mutex_guard_hashmap_string_key_must_borrow` (P3.660/P3.576) covers the
//! multipass shape; this gates the published package. Do not reshape sync.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn sync_pkg() -> Option<PathBuf> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..8 {
        let sibling = dir.join("windjammer-ecosystem/packages/wj-sync");
        if sibling.join("src").exists() {
            return Some(sibling);
        }
        if let Some(parent) = dir.parent() {
            let uncle = parent.join("windjammer-ecosystem/packages/wj-sync");
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
fn sync_shared_map_get_must_borrow_key_product() {
    let pkg = sync_pkg().unwrap_or_else(|| {
        panic!(
            "wj-sync src not found by walking from {}",
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
        .expect("wj build wj-sync");
    assert!(
        build.status.success(),
        "P3.664 wj-sync transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let shared = fs::read_to_string(out.join("shared.rs")).unwrap_or_default();
    eprintln!("P3.664 wj-sync shared.rs get sites:");
    for line in shared.lines().filter(|l| l.contains(".get(") || l.contains("contains_key(")) {
        eprintln!("{line}");
    }

    let get_region = {
        let start = shared.find("fn shared_map_get(").unwrap_or(0);
        &shared[start..shared.len().min(start + 420)]
    };
    let bare_get = get_region.contains(".get(key)") && !get_region.contains(".get(&key)");

    assert!(
        !bare_get,
        "P3.664 RED: SharedMap get must borrow String key (get(&key)):\n{get_region}\n{shared}"
    );
}
