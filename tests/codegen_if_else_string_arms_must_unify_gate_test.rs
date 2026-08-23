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

//! FAILING REPRO / lockstep gate — if/else arms that yield `string` must unify as owned.
//!
//! Dogfood (`domain/actor.wj` human_principal_from_sub):
//! ```ignore
//! let display = if email.len() > 0 { email } else { sub + "" }
//! ```
//! Tip multipass may emit `email: &str` and leave if-arm as `&str` while else is `String`
//! (E0308 / alloc macro if/else incompatible types).
//!
//! Desired: both arms owned `String`; prefer auto-own of formals used in string if/else.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn if_else_string_arms_must_unify_owned() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("t.wj");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
    fs::write(
        &src,
        r#"
use std::strings

pub fn pick_display(sub: string, email: string) -> string {
    if strings.len(email + "") > 0 {
        email + ""
    } else {
        sub + ""
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
        "wj build --check must unify if/else string arms as owned. stderr=\n{}\nstdout=\n{}",
        String::from_utf8_lossy(&build.stderr),
        String::from_utf8_lossy(&build.stdout)
    );
}
