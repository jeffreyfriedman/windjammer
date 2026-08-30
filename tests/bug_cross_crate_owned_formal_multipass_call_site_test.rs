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

//! FAILING REPRO — multipass app cross-module calls into owned `String` formals.
//!
//! Ecosystem patterns on tip `wj` (2026-08-30):
//! - `wj-fetch` `pretty(body)` → E0308 (`String` vs `&str`) without `own(body)` forwarder
//! - `wj-auth-api` `render_html("<!DOCTYPE…", vars)` → E0308 without `own(lit)` forwarder
//!
//! Related: `bug_app_multipass_cross_crate_owned_forwarder_module_file_test` (`own()` locals);
//! this gate covers **bare binding** and **string literal** call sites.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const JSON_UTIL: &str = r#"
pub fn pretty(body: string) -> string {
    body
}
"#;

const FORMAT: &str = r#"
use crate::json_util::pretty

pub fn format_body(body: string) -> string {
    pretty(body)
}
"#;

const TEMPLATE: &str = r#"
use std::collections::HashMap

pub fn render_html(template: string, vars: HashMap<string, string>) -> string {
    template
}
"#;

const PAGES: &str = r#"
use std::collections::HashMap
use crate::template::render_html

pub fn welcome_page() -> string {
    let mut vars = HashMap::new()
    vars.insert("title", "hello")
    render_html("<html>{{title}}</html>", vars)
}
"#;

#[test]
fn cross_module_owned_param_must_move_not_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod json_util
pub mod format
"#,
    );
    test.add_file("json_util.wj", JSON_UTIL);
    test.add_file("format.wj", FORMAT);

    let map = test
        .compile()
        .expect("cross-module owned param compile should succeed");
    let format_rs = map.get("format.rs").expect("format.rs");
    let bad = format_rs.contains("pretty(&body)") || format_rs.contains("pretty(& body)");
    assert!(
        !bad,
        "RED: owned `String` formal must receive moved binding, not borrow:\n{format_rs}"
    );
    test.cargo_check()
        .expect("moved owned param must cargo-check");
}

#[test]
fn cross_module_string_literal_must_own_not_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod template
pub mod pages
"#,
    );
    test.add_file("template.wj", TEMPLATE);
    test.add_file("pages.wj", PAGES);

    let map = test
        .compile()
        .expect("cross-module literal call compile should succeed");
    let pages_rs = map.get("pages.rs").expect("pages.rs");
    let template_rs = map.get("template.rs").expect("template.rs");

    let demoted = template_rs.contains("template: &str");
    let bad_literal = pages_rs.contains("render_html(\"<html")
        && !pages_rs.contains(".to_string()")
        && pages_rs.contains("render_html(\"");
    if demoted {
        assert!(
            !bad_literal,
            "RED: string literal into demoted `&str` must borrow or auto-own; pages:\n{pages_rs}"
        );
    } else if bad_literal {
        panic!(
            "RED: string literal into owned `String` formal must auto-own; pages:\n{pages_rs}"
        );
    }
    test.cargo_check()
        .expect("literal call site must cargo-check");
}
