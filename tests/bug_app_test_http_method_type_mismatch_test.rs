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

//! FAILING REPRO — `std::http::HttpMethod` in lib public API vs `tests/*_test.wj` call sites.
//!
//! Ecosystem `wj-webhook`: domain `handle(method: HttpMethod, …)` + tests
//! `app.handle(HttpMethod::GET, …)` → E0308 (`expected lib HttpMethod, found …`).
//! Workaround: public port accepts `string`; adapter matches enum internally.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn app_test_http_method_public_port_must_pass_wj_test() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    let src = root.join("src");
    let tests = root.join("tests");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&tests).unwrap();

    fs::write(
        root.join("wj.toml"),
        r#"[package]
name = "http-method-app"
version = "0.1.0"
edition = "2025"

[lib]
"#,
    )
    .unwrap();

    fs::write(
        src.join("lib.wj"),
        r#"use std::http::HttpMethod

pub struct WebhookApp {}

impl WebhookApp {
    pub fn handle(self, method: HttpMethod) -> int {
        match method {
            HttpMethod::GET => 200,
            HttpMethod::POST => 201,
            _ => 404,
        }
    }
}
"#,
    )
    .unwrap();

    fs::write(
        tests.join("handle_test.wj"),
        r#"use std::http::HttpMethod
use crate::WebhookApp

fn test_handle_get() {
    let app = WebhookApp {}
    let status = app.handle(HttpMethod::GET)
    assert_eq(status, 200)
}
"#,
    )
    .unwrap();

    let wj = test_utils::wj_binary();
    let output = Command::new(&wj)
        .arg("test")
        .current_dir(root)
        .output()
        .expect("run wj test");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}\n{stderr}");

    assert!(
        output.status.success(),
        "RED: lib HttpMethod public API + tests/*_test.wj must pass `wj test` (wj-webhook class).\n\
         Workaround: accept `string` at public port.\n{combined}"
    );
    assert!(
        combined.contains("passed") || combined.contains("All tests passed"),
        "expected passing tests in output:\n{combined}"
    );
}
