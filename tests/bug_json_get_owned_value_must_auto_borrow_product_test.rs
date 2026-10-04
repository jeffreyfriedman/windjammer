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

//! P3.661: product `wj-json-util` owned `Value` into `json::get` must auto-borrow.
//!
//! ```ignore
//! fn take_field(value: Value, key: string) -> Option<Value> {
//!     json.get(value, key)  // runtime: get(value: &Value, key: &str)
//! }
//! ```
//! Tip emits `json::get(value, &key)` → E0308 (`&Value` expected). Sibling
//! `json::get_index(&value, idx)` and path_set's `json::get(&out, &head)` already
//! borrow. Distinct from get_index multipass gate. Do not reshape with manual `&`.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn json_util_pkg() -> Option<PathBuf> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..8 {
        let sibling = dir.join("windjammer-ecosystem/packages/wj-json-util");
        if sibling.join("src").exists() {
            return Some(sibling);
        }
        if let Some(parent) = dir.parent() {
            let uncle = parent.join("windjammer-ecosystem/packages/wj-json-util");
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
fn json_get_owned_value_must_auto_borrow_product() {
    let pkg = json_util_pkg().unwrap_or_else(|| {
        panic!(
            "wj-json-util src not found by walking from {}",
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
        .expect("wj build wj-json-util");
    assert!(
        build.status.success(),
        "P3.661 wj-json-util transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let lib = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    eprintln!("P3.661 wj-json-util lib.rs (get sites):\n");
    for line in lib.lines().filter(|l| l.contains("json::get(")) {
        eprintln!("{line}");
    }

    let bad_take = {
        let start = lib.find("fn take_field(").unwrap_or(0);
        let region = &lib[start..lib.len().min(start + 200)];
        region.contains("json::get(value,") && !region.contains("json::get(&value,")
    };
    let bad_child = {
        let start = lib.find("fn child_at(").unwrap_or(0);
        let region = &lib[start..lib.len().min(start + 360)];
        region.contains("json::get(value,") && !region.contains("json::get(&value,")
    };

    assert!(
        !(bad_take || bad_child),
        "P3.661 RED: json::get must auto-borrow owned Value \
         (take_field={bad_take} child_at={bad_child}):\n{lib}"
    );
}
