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

//! P3.322: usize index `start = i + 1` (then used as substring start) must not
//! emit `i + 1_usize as i64` / `as i32` (`wj-toml` array/table scanners).
//!
//! Related: P3.300/315 substring end width; this is assign of `i+1` into usize.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str = r#"
use std::strings

pub fn slice_after_comma(text: string) -> string {
    let mut i = 0
    let mut start = 0
    while i < strings.len(text) {
        let ch = strings.substring(text, i, i + 1)
        if ch == "," {
            start = i + 1
        }
        i = i + 1
    }
    strings.substring(text, start, strings.len(text))
}
"#;

fn bad_i_plus_one_cast(rs: &str) -> bool {
    rs.contains("1_usize as i64")
        || rs.contains("1_usize as i32")
        || rs.contains("+ 1_usize as i")
        || (rs.contains("start = i +") && rs.contains(" as i"))
}

#[test]
fn module_file_usize_i_plus_one_assign_must_stay_usize() {
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
    if bad_i_plus_one_cast(&generated) {
        eprintln!("P3.322 RED:\n{generated}");
    }
    assert!(
        !bad_i_plus_one_cast(&generated),
        "RED P3.322: usize start = i + 1 must stay usize width:\n{generated}"
    );

    let check = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "cargo check failed (toml i+1 assign shape):\n{}\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
}
