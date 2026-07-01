#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// `stat_name: stat_name` in a struct literal must not `.into()` when the param is reused
/// in other fields (e.g. string interpolation → format!).
#[test]
fn test_struct_literal_owned_string_param_reused_without_move() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("dialog.wj");

    fs::write(
        &src,
        r##"
pub struct DialogStatCheck {
    pub stat_name: string,
    pub required_value: i32,
    pub success_text: string,
    pub failure_text: string,
}

impl DialogStatCheck {
    pub fn new(stat_name: string, required_value: i32) -> DialogStatCheck {
        DialogStatCheck {
            stat_name: stat_name,
            required_value: required_value,
            success_text: "[${stat_name}:${required_value}]",
            failure_text: "[${stat_name}:${required_value} FAILED]",
        }
    }
}
"##,
    )
    .unwrap();

    let out = tmp.path().join("out");
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("wj build");

    assert!(
        output.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let generated = fs::read_to_string(out.join("dialog.rs")).expect("dialog.rs");

    assert!(
        !generated.contains("stat_name.into()"),
        "owned string param must not be moved via .into() when reused in struct literal. Generated:\n{}",
        generated
    );

    // Verify rustc accepts the generated code
    let check = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .env_remove("CARGO_TARGET_DIR")
        .output()
        .expect("wj build with cargo");

    assert!(
        check.status.success(),
        "wj transpilation failed:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}
