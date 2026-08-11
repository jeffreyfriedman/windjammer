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

//! FAILING REPRO — `ServerRequest { method: "GET", … }` must coerce string → HttpMethod.
//!
//! `std::http::ServerRequest.method` is `HttpMethod`. Tip must not emit
//! `method: String::from("GET")` (E0308). Product workaround today:
//! `method: HttpMethod::GET`. Desired: bare `"GET"` in struct lit auto-coerces.
//!
//! Language-only; no product/repo names.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn server_request_method_string_literal_must_become_http_method() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("t.wj");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
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

fn main() {
    let _ = get_health()
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
        ])
        .output()
        .expect("run wj");
    assert!(
        build.status.success(),
        "wj build --check must succeed for method: \"GET\" struct lit. stderr=\n{}\nstdout=\n{}",
        String::from_utf8_lossy(&build.stderr),
        String::from_utf8_lossy(&build.stdout)
    );

    let rs = fs::read_to_string(out.join("t.rs")).unwrap_or_else(|_| {
        fs::read_dir(&out)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .find_map(|e| {
                let p = e.path();
                if p.extension().is_some_and(|x| x == "rs") {
                    fs::read_to_string(p).ok()
                } else {
                    None
                }
            })
            .unwrap_or_default()
    });

    assert!(
        !rs.contains("method: String::from(\"GET\")")
            && !rs.contains("method: \"GET\".to_string()"),
        "must not emit String for HttpMethod field. Got:\n{rs}"
    );
    assert!(
        rs.contains("HttpMethod::GET") || rs.contains("method: HttpMethod"),
        "expected HttpMethod enum in emit. Got:\n{rs}"
    );
}
