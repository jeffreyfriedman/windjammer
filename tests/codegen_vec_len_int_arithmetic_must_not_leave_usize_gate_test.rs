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
))]

//! FAILING REPRO — `vec.len()` in `int` arithmetic must not leave `usize`.
//!
//! Windjammer `int` is `i64`. Tip currently emits:
//!   `let total = items.len();`  // usize
//!   `done * 100 / total`        // E0277 i64 / usize
//!
//! Expected: coerce `.len()` to `i64` when the value flows into `int` ops
//! (assignment to `int`, `/`, `*`, `+` with `int`), or type `total` as `i64`.
//!
//! Language-only; no product/repo names.
//! Blocks `--module-file` dogfood regen when progress = (done * 100) / tasks.len().

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn vec_len_in_int_division_must_not_leave_usize() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("t.wj");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
    fs::write(
        &src,
        r#"
pub fn progress_percent(items: Vec<string>, done: int) -> int {
    let total = items.len()
    if total == 0 {
        return 100
    }
    (done * 100) / total
}

fn main() {
    let items = vec!["a", "b"]
    let _ = progress_percent(items, 1)
}
"#,
    )
    .unwrap();

    let wj = env!("CARGO_BIN_EXE_wj");
    let build = Command::new(wj)
        .args([
            "build",
            src.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("run wj");
    assert!(
        build.status.success(),
        "wj build must succeed. stderr=\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("t.rs")).unwrap_or_else(|_| {
        fs::read_dir(&out)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .find_map(|e| {
                let p = e.path();
                if p.extension().is_some_and(|x| x == "rs") {
                    fs::read_to_string(p).ok()
                } else {
                    None
                }
            })
            .unwrap_or_default()
    });
    assert!(
        !rs.is_empty(),
        "expected generated Rust under {}",
        out.display()
    );

    // Must cargo-check: i64 / usize is the dogfood failure mode.
    let crate_dir = tmp.path().join("crate");
    fs::create_dir_all(crate_dir.join("src")).unwrap();
    fs::write(
        crate_dir.join("Cargo.toml"),
        r#"[package]
name = "len_int_div_gate"
version = "0.1.0"
edition = "2021"
[lib]
path = "src/lib.rs"
"#,
    )
    .unwrap();
    fs::write(crate_dir.join("src/lib.rs"), &rs).unwrap();
    let check = Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(&crate_dir)
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "vec.len() used in int division must cargo-check as i64 (not usize). \
         stderr=\n{}\ngenerated=\n{rs}",
        String::from_utf8_lossy(&check.stderr)
    );
}
