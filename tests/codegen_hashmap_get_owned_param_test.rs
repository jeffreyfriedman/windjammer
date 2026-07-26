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

//! FAILING REPRO (for compiler session):
//! When a function takes `HashMap` by value but the body only calls `.get()`,
//! codegen emits `fn f(query: &HashMap<...>)` while call sites still pass owned
//! `HashMap`, causing E0308. Platform workaround: iterate by value instead of `.get()`.
//!
//! Expected (green): either keep owned param signature, or auto-ref at all call sites.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn hashmap_get_body_should_not_flip_owned_param_to_ref() {
    let source = r#"
use std::collections::HashMap

pub struct RouteQueryParams {
    pub as_of: Option<string>,
}

pub fn parse_query_params(query: HashMap<string, string>) -> RouteQueryParams {
    let as_of = match query.get("as_of") {
        Some(value) => Some(value + ""),
        None => None,
    }
    RouteQueryParams { as_of: as_of }
}

fn main() {
    let mut q = HashMap::new()
    q.insert("as_of", "2026-07-25")
    let _ = parse_query_params(q)
}
"#;

    let result = test_utils::compile_single(source);

    // Desired: owned HashMap parameter preserved (matches WJ source signature).
    // Current bug: becomes `&HashMap` because body only borrows via `.get()`.
    assert!(
        result.contains("parse_query_params(query: std::collections::HashMap")
            || result.contains("parse_query_params(query: HashMap"),
        "expected owned HashMap param in codegen, got:\n{}",
        result
    );
    assert!(
        !result.contains("parse_query_params(query: &std::collections::HashMap")
            && !result.contains("parse_query_params(query: &HashMap"),
        "HashMap.get() must not flip owned param to &HashMap. Got:\n{}",
        result
    );
}
