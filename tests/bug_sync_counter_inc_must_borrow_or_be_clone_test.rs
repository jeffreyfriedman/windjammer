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

//! P3.594: `counter_inc` / `counter_get` are documented borrow hot paths but
//! product `wj-sync` keeps owned `c: Counter` while tests reuse `c` (codegen
//! inserts `.clone()`). `Counter { Arc<AtomicI64> }` is not `Clone` → E0599.
//!
//! Flat Arc-Counter isolates false-GREEN (`counter_inc(&Counter)`). Gate the
//! product package. Do not reshape wj-sync tests to manual Arc clones.
//! (P3.593 was WDB-432 indexed Copy cast.)

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
fn sync_counter_inc_must_borrow_or_be_clone() {
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
        "P3.594 wj-sync transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let shared_rs = fs::read_to_string(out.join("shared.rs")).unwrap_or_else(|_| {
        fs::read_to_string(out.join("lib.rs")).unwrap_or_default()
    });
    let inc = shared_rs
        .lines()
        .find(|l| l.contains("fn counter_inc("))
        .unwrap_or("")
        .to_string();
    eprintln!("P3.594 counter_inc: {inc}");

    let borrows = inc.contains("&Counter") || inc.contains("c: &");
    let counter_block = {
        let start = shared_rs.find("struct Counter").unwrap_or(0);
        &shared_rs[start.saturating_sub(80)..shared_rs.len().min(start + 200)]
    };
    let has_clone = counter_block.contains("Clone") && counter_block.contains("Counter");

    assert!(
        borrows || has_clone,
        "P3.594 RED: counter_inc must borrow Counter (or Counter must Clone) — \
         product tests reuse c → E0599:\n{inc}\n{counter_block}"
    );
}
