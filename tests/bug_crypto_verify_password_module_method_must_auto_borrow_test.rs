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

//! P3.654 isolate: `crypto.verify_password(password, hash)` thin wrap must auto-borrow.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;

#[test]
fn crypto_verify_password_module_method_must_auto_borrow() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("lib.wj"),
        r#"
use std::crypto

pub fn verify_password(password: string, hash: string) -> Result<bool, string> {
    crypto.verify_password(password, hash)
}
"#,
    )
    .unwrap();

    let out = dir.path().join("out");
    let output = test_utils::run_wj_command([
        "build",
        "--no-cargo",
        "--library",
        "--module-file",
        src.to_str().unwrap(),
        "--output",
        out.to_str().unwrap(),
    ]);
    assert!(
        output.status.success(),
        "transpile failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let lib = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    eprintln!("P3.654 isolate lib.rs:\n{lib}");
    let ok = lib.contains("crypto::verify_password(&password, &hash)")
        || (lib.contains("password: &str") && lib.contains("hash: &str"));
    assert!(ok, "P3.654 RED: must auto-borrow or demote:\n{lib}");
}
