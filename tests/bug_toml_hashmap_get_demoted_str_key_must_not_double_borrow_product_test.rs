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

//! P3.658: demoted `&str` HashMap key must not emit `get(&key)` (double borrow).
//!
//! Product `wj-toml`:
//! ```ignore
//! pub fn get(text: string, key: string) -> Option<string> {
//!     match map.get(key) { … }
//! }
//! ```
//! Tip demotes `key` to `&str` then still emits `map.get(&key)` → E0277
//! (`String: Borrow<&str>`). Correct is `map.get(key)` when key is already `&str`,
//! or keep owned `String` + `get(&key)`. Distinct from P3.654–657 owned→`&str`
//! call-site borrows. Do not reshape toml with casts.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn toml_pkg() -> Option<PathBuf> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..8 {
        let sibling = dir.join("windjammer-ecosystem/packages/wj-toml");
        if sibling.join("src").exists() {
            return Some(sibling);
        }
        if let Some(parent) = dir.parent() {
            let uncle = parent.join("windjammer-ecosystem/packages/wj-toml");
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
fn toml_hashmap_get_demoted_str_key_must_not_double_borrow_product() {
    let pkg = toml_pkg().unwrap_or_else(|| {
        panic!(
            "wj-toml src not found by walking from {}",
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
        .expect("wj build wj-toml");
    assert!(
        build.status.success(),
        "P3.658 wj-toml transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let lib = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    eprintln!("P3.658 wj-toml lib.rs get region:\n{}", {
        let start = lib.find("pub fn get(").unwrap_or(0);
        &lib[start..lib.len().min(start + 280)]
    });

    let get_region = {
        let start = lib.find("pub fn get(").unwrap_or(0);
        &lib[start..lib.len().min(start + 280)]
    };
    let demoted_key = get_region.contains("key: &str");
    let double_borrow = get_region.contains(".get(&key)");

    assert!(
        !(demoted_key && double_borrow),
        "P3.658 RED: demoted &str HashMap key must use get(key), not get(&key):\n{get_region}\n{lib}"
    );
}
