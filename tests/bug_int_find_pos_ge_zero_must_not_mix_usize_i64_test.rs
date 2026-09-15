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

//! FAILING REPRO — `find`/`int` position `>= 0` emits `as usize >= 0_i64` (`wj-timefmt`).
//!
//! ```ignore
//! let plus_pos = find_char(text, "+")  // returns int (-1 if missing)
//! if plus_pos >= 0 { … }
//! ```
//! Tip casts LHS to usize while keeping `0_i64` → E0308.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str = r#"
use std::strings

fn find_char(text: string, ch: string) -> int {
    let mut i = 0
    while i < strings.len(text) {
        if strings.substring(text, i, i + 1) == ch {
            return i
        }
        i = i + 1
    }
    -1
}

pub fn has_plus(text: string) -> bool {
    let plus_pos = find_char(text, "+")
    plus_pos >= 0
}
"#;

#[test]
fn int_find_pos_ge_zero_must_not_mix_usize_i64() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("lib.wj"), SOURCE).unwrap();
    let out = tmp.path().join("gen");

    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build");
    assert!(
        build.status.success(),
        "library build failed:\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let generated = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    assert!(
        !generated.contains("as usize >= 0_i64")
            && !generated.contains("as usize >= 0_i32"),
        "RED P3.299: int find-pos >= 0 must not mix usize cast with i64 zero:\n{generated}"
    );

    let check = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "cargo check failed (int find-pos >= 0):\n{}\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
}
