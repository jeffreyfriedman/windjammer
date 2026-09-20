//! `std::url.parse` / `format` / `join` must wire for absolute URL helpers.
//!
//! Ecosystem `wj-url` is pure WJ today. Prefer std once these resolve; keep
//! query sugar thin-wrapping `encoding.form_*` / `wj-querystring`.

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
fn std_url_parse_codegen_resolves() {
    let source = r#"
use std::url

pub fn host_of(text: string) -> Result<string, string> {
    match url.parse(text) {
        Ok(u) => Ok(u.host),
        Err(e) => Err(e),
    }
}
"#;
    test_utils::assert_stdlib_runtime_links(source, &["url::parse"]);
}

#[test]
fn std_url_format_codegen_resolves() {
    let source = r#"
use std::url

pub fn dump(u: url.Url) -> string {
    url.format(u)
}
"#;
    test_utils::assert_stdlib_runtime_links_any(source, &["url::format", "url::format_url"]);
}

#[test]
fn std_url_join_codegen_resolves() {
    let source = r#"
use std::url

pub fn combine(base: string, rel: string) -> Result<string, string> {
    url.join(base, rel)
}
"#;
    test_utils::assert_stdlib_runtime_links_any(source, &["url::join", "url::join_url"]);
}
