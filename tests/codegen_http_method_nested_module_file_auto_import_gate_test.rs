#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

//! Gate — `method: "GET"` in a nested `--module-file` graph must resolve HttpMethod.
//!
//! Prefer FQ emit (`windjammer_runtime::http::HttpMethod::GET`) so importing only
//! `ServerRequest` is enough; bare `HttpMethod::GET` requires an auto-import.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn http_method_string_lit_nested_module_file_must_auto_import() {
    let mut test = MultiFileTest::new();
    // Intentionally import only ServerRequest — not HttpMethod.
    test.add_file(
        "adapters/request.wj",
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
"#,
    );

    let map = test
        .compile()
        .expect("nested module-file compile should succeed");
    let rs = map
        .get("adapters/request.rs")
        .or_else(|| map.get("request.rs"))
        .expect("request.rs output");

    let fq = rs.contains("windjammer_runtime::http::HttpMethod::GET");
    let bare = rs.contains("HttpMethod::GET");
    assert!(
        fq || bare,
        "expected string→HttpMethod coerce under nested multipass. Got:\n{rs}"
    );
    let imports_httpmethod = rs.contains("use windjammer_runtime::http::HttpMethod")
        || rs.contains("HttpMethod,")
        || rs.contains("{ ServerRequest, HttpMethod }")
        || rs.contains("{HttpMethod")
        || rs.contains("::http::{ServerRequest, HttpMethod}")
        || rs.contains("use windjammer_runtime::http::{ServerRequest, HttpMethod}");
    assert!(
        fq || imports_httpmethod,
        "FQ HttpMethod path or auto-import required under nested --module-file. Got:\n{rs}"
    );

    test.cargo_check()
        .expect("HttpMethod string lit must cargo check without explicit import");
}
