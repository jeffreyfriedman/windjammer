//! Nested `use std::http::*` must map types to `windjammer_runtime::http`, not `super::`.
//!
//! Ecosystem `wj-notes-api` hexagonal adapter: `src/adapters/http_server.wj`.
//! Auto-super sibling imports treated `HttpMethod` / `ServerRequest` / `ServerResponse`
//! as crate-local types and emitted `use super::HttpMethod` (E0432).

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
    feature = "codegen_tests",
))]

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn nested_http_adapter_project() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", "pub mod adapters\n");
    test.add_file(
        "adapters/mod.wj",
        "pub mod http_server\n\npub use http_server::handle\n",
    );
    test.add_file(
        "adapters/http_server.wj",
        r#"
use std::http::*

pub fn handle(req: ServerRequest) -> ServerResponse {
    let status = 200
    ServerResponse::new(status, req.body)
}

fn method_label(method: HttpMethod) -> string {
    if method == "GET" {
        return "GET"
    }
    "OTHER"
}
"#,
    );
    test
}

#[test]
fn nested_std_http_glob_does_not_emit_super_runtime_types() {
    let test = nested_http_adapter_project();
    let files = test
        .compile()
        .unwrap_or_else(|e| panic!("compile failed: {e}"));
    let rs = files.get("adapters/http_server.rs").unwrap_or_else(|| {
        panic!(
            "missing adapters/http_server.rs; have {:?}",
            files.keys().collect::<Vec<_>>()
        )
    });

    assert!(
        rs.contains("windjammer_runtime::http"),
        "nested std::http glob must map to windjammer_runtime::http, got:\n{rs}"
    );
    for forbidden in [
        "use super::HttpMethod",
        "use super::ServerRequest",
        "use super::ServerResponse",
    ] {
        assert!(
            !rs.contains(forbidden),
            "stdlib http types must not import via {forbidden}:\n{rs}"
        );
    }
}
