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

//! FAILING REPRO / lockstep gate — match arms yielding `string` must unify as owned.
//!
//! Dogfood (`finance-screens` `json.wj` analytics schema parse):
//! ```ignore
//! let dims = match dims_raw.find("]") {
//!     Some(end) => dims_raw.substring(0, end + 1),
//!     None => dims_raw,
//! }
//! ```
//! Tip may emit `Some` arm as `String` and `None` as `&str` (E0308), blocking screens regen.
//!
//! Desired: both arms owned `String` under `wj build --check --module-file`.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn match_string_arms_substring_vs_binding_must_unify_owned() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("t.wj");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
    fs::write(
        &src,
        r#"
pub fn clip_or_all(raw: string) -> string {
    match raw.find("]") {
        Some(end) => raw.substring(0, end + 1),
        None => raw,
    }
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
        "wj build --check must unify match string arms (substring vs binding). stderr=\n{}\nstdout=\n{}",
        String::from_utf8_lossy(&build.stderr),
        String::from_utf8_lossy(&build.stdout)
    );
}
