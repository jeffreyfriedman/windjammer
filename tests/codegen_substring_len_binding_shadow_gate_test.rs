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

//! FAILING REPRO — local `len` binding must not shadow `strings::len` in substring end index.
//!
//! Dogfood (`domain/bank_reconciliation.wj` `statement_rest_after_first`):
//! ```ignore
//! let len = strings.len(text + "")
//! strings.substring(text + "", 1, len)
//! ```
//! Tip may emit `strings::substring(..., |__cb0| len(&__cb0))` (E0618) instead of using `len` value.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn substring_end_index_len_binding_must_not_shadow_strings_len() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("t.wj");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
    fs::write(
        &src,
        r#"
use std::strings

pub fn rest_after_first(raw: string) -> string {
    let text = raw + ""
    let len = strings.len(text + "")
    if len == 0 {
        return "" + ""
    }
    strings.substring(text + "", 1, len)
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
        "substring end index from `len` binding must cargo-check. stderr=\n{}\nstdout=\n{}",
        String::from_utf8_lossy(&build.stderr),
        String::from_utf8_lossy(&build.stdout)
    );
}
