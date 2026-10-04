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

//! P3.644: `slice`/`Vec::get(usize)` must pass the index by value, not `&usize`.
//!
//! Product `wj-sync` `pool.wj`:
//! ```ignore
//! match job_txs.get(idx) { … }   // idx: int
//! match job_txs.get(w2) { … }
//! ```
//! Tip emits `job_txs.get(&(idx as usize))` / `get(&(w2 as usize))` → E0277
//! (`SliceIndex` not implemented for `&usize`). Distinct from
//! `Vec::remove` owned-usize gate — this is `.get` on a Vec of mpsc Senders.
//! Do not reshape pool with casts / deref hacks.

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
fn slice_get_usize_index_must_not_borrow() {
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
        "P3.644 wj-sync transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let pool = fs::read_to_string(out.join("pool.rs")).unwrap_or_default();
    let get_sites: Vec<&str> = pool
        .lines()
        .filter(|l| l.contains(".get(") && (l.contains("idx") || l.contains("w2") || l.contains("usize")))
        .collect();
    eprintln!("P3.644 pool get sites:\n{}", get_sites.join("\n"));

    let bad = pool.contains(".get(&(idx as usize))")
        || pool.contains(".get(&(w2 as usize))")
        || pool
            .lines()
            .any(|l| l.contains(".get(&(") && l.contains("as usize)"));

    assert!(
        !bad,
        "P3.644 RED: Vec/slice .get must take owned usize, not &usize:\n{}",
        get_sites.join("\n")
    );
}
