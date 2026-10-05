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

//! P3.669 product gate: `wj-json-util::merge_values` must not move `overlay`
//! into owned `take_field` inside the key loop (E0382). Prefer demoting
//! `take_field(value: Value)` → `&Value` since the body only calls `json.get`.

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
fn json_util_product_merge_values_must_cargo_check() {
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
        "P3.669 wj-json-util transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let lib = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    eprintln!(
        "P3.669 product take_field/merge sites:\n{}",
        lib.lines()
            .filter(|l| l.contains("take_field") || l.contains("fn merge_values"))
            .collect::<Vec<_>>()
            .join("\n")
    );

    let take_borrowed = lib.contains("fn take_field(value: &Value")
        || lib.contains("fn take_field(value: & Value");
    let overlay_safe = lib.contains("take_field(&overlay")
        || lib.contains("take_field(& overlay")
        || lib.contains("take_field(overlay.clone()");

    assert!(
        take_borrowed || overlay_safe,
        "P3.669 RED product: demote take_field or clone/borrow overlay:\n{lib}"
    );
    if lib.contains("fn take_field(value: Value") {
        assert!(
            !lib.contains("take_field(overlay,") && !lib.contains("take_field(overlay ,"),
            "P3.669 RED product: owned take_field must not move overlay in loop:\n{lib}"
        );
    }

    // cargo-check the generated package when possible
    let check = Command::new("cargo")
        .current_dir(&out)
        .args(["check", "--offline"])
        .output();
    if let Ok(check) = check {
        assert!(
            check.status.success(),
            "P3.669 product cargo check failed:\n{}\n{}",
            String::from_utf8_lossy(&check.stdout),
            String::from_utf8_lossy(&check.stderr)
        );
    }
}
