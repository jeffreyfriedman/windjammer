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

//! P3.661 / P3.669: product `wj-json-util` into `json::get` must match `&Value`.
//!
//! After P3.669 readonly demotion, helpers keep shared-ref formals:
//!
//! ```ignore
//! fn take_field(value: &Value, key: String) -> Option<Value> {
//!     json::get(value, &key)  // value already `&Value` — bare pass is correct
//! }
//! ```
//!
//! The pre-demotion bug was owned `value: Value` + bare `json::get(value, …)`
//! (E0308). That shape must still fail the gate. Demoted formals + bare
//! identifiers (or owned locals with `&value`) are GREEN.

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

/// True when `fn_name` emits owned `value: Value` and calls `json::get(value,`
/// without auto-borrowing (the P3.661 E0308 shape).
fn owned_value_bare_into_json_get(lib: &str, fn_name: &str) -> bool {
    let marker = format!("fn {fn_name}(");
    let start = match lib.find(&marker) {
        Some(i) => i,
        None => return false,
    };
    let region = &lib[start..lib.len().min(start + 420)];
    let formal_owned = region.contains("value: Value") && !region.contains("value: &Value");
    let bare_get = region.contains("json::get(value,") && !region.contains("json::get(&value,");
    formal_owned && bare_get
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
    for line in lib.lines().filter(|l| l.contains("json::get(") || l.contains("fn take_field") || l.contains("fn child_at")) {
        eprintln!("{line}");
    }

    let bad_take = owned_value_bare_into_json_get(&lib, "take_field");
    let bad_child = owned_value_bare_into_json_get(&lib, "child_at");

    assert!(
        !(bad_take || bad_child),
        "P3.661 RED: owned Value into json::get must auto-borrow or demote formal \
         (take_field={bad_take} child_at={bad_child}):\n{lib}"
    );

    // Hold P3.669 demotion: take_field / child_at should not stay owned Value
    // when the body only borrows into json::get / get_index.
    let take_start = lib.find("fn take_field(").unwrap_or(0);
    let take_region = &lib[take_start..lib.len().min(take_start + 160)];
    assert!(
        take_region.contains("value: &Value") || take_region.contains("json::get(&value,"),
        "P3.669 hold: take_field must demote to &Value or borrow at json::get:\n{take_region}"
    );
}
