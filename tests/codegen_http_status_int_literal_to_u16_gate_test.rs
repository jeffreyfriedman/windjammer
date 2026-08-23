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

//! FAILING REPRO — `ServerResponse::new(200, body)` / `::error(status, msg)` must accept
//! bare integer literals and `int` values for `status: u16` formals.
//!
//! Tip currently reports: Type conflict: must be U16 but was I64.
//! Product workaround: typed constructors (`ok` / `created` / `bad_request` / …)
//! and status branching instead of `ServerResponse::error(status, …)`.
//!
//! Language-only; no product/repo names.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn server_response_new_int_literal_must_coerce_to_u16() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("t.wj");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
    fs::write(
        &src,
        r#"
use std::http::ServerResponse

pub fn ok_csv(body: string) -> ServerResponse {
    ServerResponse::new(200, body)
}

pub fn err_from_int(status: int, message: string) -> ServerResponse {
    ServerResponse::error(status, message)
}

fn main() {
    let _ = ok_csv("a,b")
    let _ = err_from_int(404, "missing")
}
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
        ])
        .output()
        .expect("run wj");
    assert!(
        build.status.success(),
        "wj build --check --module-file must accept int literal / int → u16 status. stderr=\n{}\nstdout=\n{}",
        String::from_utf8_lossy(&build.stderr),
        String::from_utf8_lossy(&build.stdout)
    );
}
