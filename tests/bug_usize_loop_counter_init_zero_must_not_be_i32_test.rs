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

//! P3.591: `let mut i = 0` used as a `usize` index/len peer must not emit
//! `let mut i: usize = 0_i32`.
//!
//! Product `wj-find` / `wj-form-parse` domain loops:
//!   `let mut i: usize = 0_i32;` → E0308.
//! Distinct from P3.577 toml `start = i + 1_usize as i64` (assign width).

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn find_app() -> Option<PathBuf> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..8 {
        let sibling = dir.join("windjammer-ecosystem/apps/wj-find");
        if sibling.join("src").exists() {
            return Some(sibling);
        }
        if let Some(parent) = dir.parent() {
            let uncle = parent.join("windjammer-ecosystem/apps/wj-find");
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
fn usize_loop_counter_init_zero_must_not_be_i32() {
    let app = find_app().unwrap_or_else(|| {
        panic!(
            "wj-find src not found by walking from {}",
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
        .expect("wj build wj-find");
    assert!(
        build.status.success(),
        "P3.591 wj-find transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let find_rs = fs::read_to_string(out.join("domain").join("find.rs")).unwrap_or_default();
    eprintln!("P3.591 find.rs i bindings:\n{}", {
        find_rs
            .lines()
            .filter(|l| l.contains("let mut i"))
            .collect::<Vec<_>>()
            .join("\n")
    });

    assert!(
        !find_rs.contains("usize = 0_i32") && !find_rs.contains(": usize = 0_i32"),
        "P3.591 RED: usize loop counter must not init as 0_i32 (wj-find / form-parse):\n{find_rs}"
    );
}
