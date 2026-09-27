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

//! `json.to_string(payload)` must not emit `json::to_string(&mut payload)` (E0596).
//!
//! Ecosystem `wj-notes-api` `error_from_message` / `json_ok` / list GET:
//! `let payload = ErrorBody { error: message }` then `json.to_string(payload)`.
//! Simple `json_typed_serialize` isolates can stay GREEN while hexagonal product
//! still mut-borrows the local.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn json_to_string_payload_must_not_mut_borrow() {
    let tmp = TempDir::new().expect("tempdir");
    let wj = env!("CARGO_BIN_EXE_wj");

    let src = tmp.path().join("src");
    fs::create_dir_all(src.join("domain")).expect("mkdir domain");
    fs::write(src.join("mod.wj"), "pub mod domain\n").unwrap();
    fs::write(src.join("domain").join("mod.wj"), "pub mod api\n").unwrap();
    fs::write(
        src.join("domain").join("api.wj"),
        r#"
use std::json

struct ErrorBody {
    error: string,
}

fn own(value: string) -> string {
    value
}

pub fn error_from_message(status: u16, message: string) -> string {
    let message = own(message)
    let payload = ErrorBody { error: message }
    match json.to_string(payload) {
        Ok(text) => text,
        Err(_) => "{\"error\":\"unknown\"}",
    }
}

pub fn list_payload(items: Vec<string>) -> string {
    match json.to_string(items) {
        Ok(text) => text,
        Err(_) => "[]",
    }
}
"#,
    )
    .unwrap();

    let out = tmp.path().join("out");
    let build = Command::new(wj)
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("app build");
    assert!(
        build.status.success(),
        "json isolate transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let api_rs = fs::read_to_string(out.join("domain").join("api.rs")).unwrap_or_default();
    eprintln!("api.rs:\n{api_rs}");
    assert!(
        !api_rs.contains("to_string(&mut payload)")
            && !api_rs.contains("to_string(& mut payload)")
            && !api_rs.contains("to_string(&mut items)")
            && !api_rs.contains("to_string(& mut items)"),
        "json.to_string must not mut-borrow the local (E0596):\n{api_rs}"
    );
    assert!(
        api_rs.contains("to_string(&payload)")
            || api_rs.contains("to_string(& payload)")
            || api_rs.contains("to_string(payload)")
            || api_rs.contains("to_string(&items)")
            || api_rs.contains("to_string(& items)")
            || api_rs.contains("to_string(items)"),
        "json.to_string must emit a shared/owned call, not drop the arg:\n{api_rs}"
    );

    let check = Command::new("cargo")
        .current_dir(&out)
        .args(["check", "--quiet"])
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "json.to_string payload must cargo-check.\n{api_rs}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}
