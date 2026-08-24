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

//! FAILING REPRO — `method: "GET"` must emit `HttpMethod::GET` *and* import `HttpMethod`.
//!
//! Tip coerces the string literal correctly, but if WJ source never names `HttpMethod`
//! (only `use std::http::ServerRequest`), emit uses `HttpMethod::GET` without importing
//! it (E0433). Desired: auto-import `HttpMethod` whenever a struct field coerces.
//!
//! Dogfood: `request_context.wj` keeps `use …::{ServerRequest, HttpMethod}` as workaround
//! while writing `method: "GET"`.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn http_method_string_lit_must_auto_import_httpmethod() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("t.wj");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
    // Intentionally import only ServerRequest — not HttpMethod.
    fs::write(
        &src,
        r#"
use std::http::ServerRequest

pub fn get_health() -> ServerRequest {
    ServerRequest {
        method: "GET",
        path: "/health",
        query: std::collections::HashMap::new(),
        headers: vec![],
        body: "",
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
            "--no-cargo",
        ])
        .output()
        .expect("run wj");
    assert!(
        build.status.success(),
        "wj --check must succeed with method: \"GET\" and no HttpMethod import. stderr=\n{}\nstdout=\n{}",
        String::from_utf8_lossy(&build.stderr),
        String::from_utf8_lossy(&build.stdout)
    );
}
