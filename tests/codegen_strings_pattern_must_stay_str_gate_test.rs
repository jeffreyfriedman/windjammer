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

//! FAILING REPRO — `strings.split` / `starts_with` delimiter/prefix must stay `&str`.
//!
//! Runtime (`windjammer_runtime::strings::{split,starts_with}`) takes `&str` for the
//! pattern argument. Std stubs declare owned `string`, and multipass `--module-file`
//! dogfood emits `String::from("lit")` → WJ0003 / E0308.
//!
//! Desired: emit bare `"lit"` (or `.as_str()`), never `String::from("lit")`.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn assert_no_owned_pattern(generated: &str, needle: &str) {
    assert!(
        !generated.contains(&format!("String::from(\"{needle}\")")),
        "pattern `{needle}` must not be String::from. Generated:\n{generated}"
    );
    assert!(
        !generated.contains(&format!("\"{needle}\".to_string()")),
        "pattern `{needle}` must not be .to_string(). Generated:\n{generated}"
    );
}

#[test]
fn strings_split_and_starts_with_literals_must_stay_str() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("t.wj");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
    fs::write(
        &src,
        r#"
use std::strings

pub fn split_pipe(line: string) -> Vec<string> {
    strings.split(line + "", "|")
}

pub fn starts_acct(id: string) -> bool {
    strings.starts_with(id + "", "acct~")
}

fn main() {}
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
            "--check",
            "--module-file",
            "--no-cargo",
        ])
        .output()
        .expect("run wj");
    assert!(
        build.status.success(),
        "wj build --check --module-file must allow split/starts_with with &str patterns. stderr=\n{}\nstdout=\n{}",
        String::from_utf8_lossy(&build.stderr),
        String::from_utf8_lossy(&build.stdout)
    );

    let generated = test_utils::compile_single(
        r#"
use std::strings

pub fn split_pipe(line: string) -> Vec<string> {
    strings.split(line + "", "|")
}

pub fn starts_acct(id: string) -> bool {
    strings.starts_with(id + "", "acct~")
}
"#,
    );
    assert_no_owned_pattern(&generated, "|");
    assert_no_owned_pattern(&generated, "acct~");
}
