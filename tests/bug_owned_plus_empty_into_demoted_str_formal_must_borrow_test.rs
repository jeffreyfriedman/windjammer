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

//! P3.684: LedgerKit call sites `fn_taking_str(id + "")` where tip demotes
//! `string` formals to `&str` must auto-borrow the owned temp (not pass
//! `String` → E0308). Product drops `+ ""` interim until tip greens.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn owned_plus_empty_into_demoted_str_formal_must_borrow() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("lib.wj"),
        r#"
pub fn payload(entry_id: string, reference: string) -> string {
    entry_id + "|" + reference
}

pub fn build(result_id: string, result_reference: string) -> string {
    payload(result_id + "", result_reference + "")
}
"#,
    )
    .unwrap();

    let out = tmp.path().join("out");
    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
        ])
        .output()
        .expect("wj build");
    assert!(
        build.status.success(),
        "P3.684 transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    eprintln!("P3.684 emit:\n{rs}");

    let check = Command::new("cargo")
        .current_dir(&out)
        .args(["check", "--offline"])
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "P3.684 RED: owned + \"\" into demoted &str formal must borrow:\n{}\nemit:\n{rs}",
        String::from_utf8_lossy(&check.stderr)
    );
}
