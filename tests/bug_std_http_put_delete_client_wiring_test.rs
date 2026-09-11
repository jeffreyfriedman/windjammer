//! P3.222 (`wj-proxy`): `http.put` / `patch` / `delete` / `head` must exist on
//! `windjammer_runtime::http` with the same shapes as `post` / `get`.
//!
//! Adapter `forward_put` currently fails E0425: `cannot find function 'put' in module 'http'`.
//! Codegen also owns the body (`body.to_string()`) instead of borrowing like `http::post`.

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

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn std_http_put_must_link_and_borrow_like_post() {
    let source = r#"
use std::http

pub fn put_text(url: string, body: string) -> Result<string, string> {
    match http.put(url, body) {
        Ok(response) => response.text(),
        Err(e) => Err(e),
    }
}
"#;
    let generated = test_utils::assert_stdlib_runtime_links(source, &["http::put"]);
    assert!(
        generated.contains("http::put(&url, &body)"),
        "http::put must borrow owned String url and body like post, got:\n{generated}"
    );
}

#[test]
fn std_http_patch_must_link_and_borrow_like_post() {
    let source = r#"
use std::http

pub fn patch_text(url: string, body: string) -> Result<string, string> {
    match http.patch(url, body) {
        Ok(response) => response.text(),
        Err(e) => Err(e),
    }
}
"#;
    let generated = test_utils::assert_stdlib_runtime_links(source, &["http::patch"]);
    assert!(
        generated.contains("http::patch(&url, &body)"),
        "http::patch must borrow owned String url and body like post, got:\n{generated}"
    );
}

#[test]
fn std_http_delete_must_link_like_get() {
    let source = r#"
use std::http

pub fn delete_url(url: string) -> Result<string, string> {
    match http.delete(url) {
        Ok(response) => response.text(),
        Err(e) => Err(e),
    }
}
"#;
    let generated = test_utils::assert_stdlib_runtime_links(source, &["http::delete"]);
    assert!(
        generated.contains("http::delete(&url)") || generated.contains("http::delete(url"),
        "http::delete must emit a runtime delete call, got:\n{generated}"
    );
}

#[test]
fn std_http_head_must_link_like_get() {
    let source = r#"
use std::http

pub fn head_url(url: string) -> Result<string, string> {
    match http.head(url) {
        Ok(response) => response.text(),
        Err(e) => Err(e),
    }
}
"#;
    let generated = test_utils::assert_stdlib_runtime_links(source, &["http::head"]);
    assert!(
        generated.contains("http::head(&url)") || generated.contains("http::head(url"),
        "http::head must emit a runtime head call, got:\n{generated}"
    );
}
