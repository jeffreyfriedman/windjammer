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

//! FAILING REPRO — hexagonal `wj-webhook` layout: `src/domain/*.wj` + lib re-export + `tests/*_test.wj`.
//!
//! Public port accepts `string` today; this gate asserts `HttpMethod` in lib API + package tests
//! must pass `wj test` without string adapter workaround.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn app_module_file_http_method_public_port_must_pass_wj_test() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    let src = root.join("src");
    let domain = src.join("domain");
    let tests = root.join("tests");
    fs::create_dir_all(&domain).unwrap();
    fs::create_dir_all(&tests).unwrap();

    fs::write(
        root.join("wj.toml"),
        r#"[package]
name = "http-method-module-file-app"
version = "0.1.0"
edition = "2025"

[lib]
"#,
    )
    .unwrap();

    fs::write(
        src.join("lib.wj"),
        r#"pub mod domain
pub use domain::webhook::WebhookApp
"#,
    )
    .unwrap();

    fs::write(
        domain.join("webhook.wj"),
        r#"use std::http::HttpMethod

pub struct WebhookApp {}

impl WebhookApp {
    pub fn handle(self, method: HttpMethod, path: string) -> int {
        match method {
            HttpMethod::GET => {
                if path == "/health" {
                    200
                } else {
                    404
                }
            },
            HttpMethod::POST => 201,
            _ => 405,
        }
    }
}
"#,
    )
    .unwrap();

    fs::write(
        tests.join("webhook_test.wj"),
        r#"use std::http::HttpMethod
use crate::WebhookApp

fn test_health_get() {
    let app = WebhookApp {}
    let status = app.handle(HttpMethod::GET, "/health")
    assert_eq(status, 200)
}

fn test_post_created() {
    let app = WebhookApp {}
    let status = app.handle(HttpMethod::POST, "/webhook")
    assert_eq(status, 201)
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
        "RED: module-file lib HttpMethod API + tests/*_test.wj must pass `wj test` (wj-webhook hexagonal class).\n\
         Workaround: public `handle(method: string, …)` + `parse_method`.\n{combined}"
    );
    assert!(
        combined.contains("passed") || combined.contains("All tests passed"),
        "expected passing tests in output:\n{combined}"
    );
}
