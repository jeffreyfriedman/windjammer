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

//! P3.325: demoted `&str` formal + `let mut core = text` (clone to String) then
//! `core = strings.substring(...)` must own the substring (`.to_string()`), not
//! assign `&str` into `String` (`wj-semver` split_build / split_pre).
//!
//! Product: E0308 expected `String`, found `&str` on assign and `Ok((core, …))`.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str = r#"
use std::strings

pub fn split_build_pub(text: string) -> Result<(string, string), string> {
    split_build(text)
}

fn split_build(text: string) -> Result<(string, string), string> {
    let mut core = text
    let mut build = ""
    let mut i = 0
    while i < strings.len(text) {
        let ch = strings.substring_chars(text, i, i + 1)
        if ch == "+" {
            core = strings.substring(text, 0, i)
            build = strings.substring(text, i + 1, strings.len(text))
            i = strings.len(text)
        } else {
            i = i + 1
        }
    }
    Ok((core, build))
}
"#;

fn bad_substring_into_string(rs: &str) -> bool {
    // Demoted &str formal with String local assigned from bare substring.
    let demoted = rs.contains("text: &str") || rs.contains("text:&str");
    let assign = rs.contains("core = strings::substring(")
        || rs.contains("build = strings::substring(");
    let owns = rs.contains("substring(") && rs.contains(".to_string()");
    demoted && assign && !owns
}

#[test]
fn module_file_demoted_str_substring_assign_must_own() {
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
    if bad_substring_into_string(&generated) {
        eprintln!("P3.325 RED:\n{generated}");
    }
    assert!(
        !bad_substring_into_string(&generated),
        "RED P3.325: substring into owned String local must .to_string() when formal is &str:\n{generated}"
    );

    let check = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "cargo check failed (semver split_build shape):\n{}\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
}
