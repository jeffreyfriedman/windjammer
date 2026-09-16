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

//! P3.315: nested `while` + `substring(s, i, i+1)` + `i = i + 1` (wj-duration
//! `parse_ms` shape) must not emit `(i + 1_i32) as usize` / `i += 1 as i32`.
//!
//! Tip GREEN (2026-09-16): nested loops emit `i + 1_usize` / `i += 1`.
//! Remaining duration blocker is P3.317 (`(n * mult) as i32`).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str = r#"
use std::strings

fn is_digit(ch: string) -> bool {
    ch >= "0" && ch <= "9"
}

pub fn parse_ms(s: string) -> Result<int, string> {
    let mut i = 0
    let mut total = 0
    while i < strings.len(s) {
        let start = i
        while i < strings.len(s) && is_digit(strings.substring(s, i, i + 1)) {
            i = i + 1
        }
        let num_text = strings.substring(s, start, i)
        let unit_start = i
        while i < strings.len(s) && !is_digit(strings.substring(s, i, i + 1)) {
            i = i + 1
        }
        let _ = num_text
        let _ = unit_start
        if i == start {
            return Err("bad")
        }
        total = total + 1
    }
    Ok(total)
}
"#;

fn bad_i32_peer(rs: &str) -> bool {
    // Index-width only — do not flag unrelated `total += 1 as i32` on int accumulators.
    rs.contains("i + 1_i32")
        || rs.contains("i+1_i32")
        || rs.contains("(i + 1_i32)")
        || rs.contains("i += 1 as i32")
        || rs.contains("i += 1_i32")
        || rs.contains("i + 1i32")
}

#[test]
fn module_file_nested_while_substring_i_plus_one_must_not_emit_i32() {
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
    if bad_i32_peer(&generated) {
        eprintln!("P3.315 RED:\n{generated}");
    }
    assert!(
        !bad_i32_peer(&generated),
        "RED P3.315: nested while substring i+1 must not emit i32 peers onto usize:\n{generated}"
    );

    let check = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "cargo check failed (duration nested parse_ms shape):\n{}\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
}
