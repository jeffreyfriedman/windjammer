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

//! `Server::serve` handler closures must pass `ServerRequest` to user functions without
//! double-borrow (`&&ServerRequest`) when the runtime invokes `Fn(ServerRequest)`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn serve_closure_passes_request_to_handler_without_double_borrow() {
    let mut test = MultiFileTest::new();
    // Flat module layout so integration_test_helpers::assert_compiles_without_error works.
    test.add_file(
        "router.wj",
        r#"
use std::http::{ServerRequest, ServerResponse}

pub fn match_route(request: ServerRequest) -> Option<ServerResponse> {
    if request.path == "/health" {
        Some(ServerResponse::ok("ok"))
    } else {
        None
    }
}
"#,
    );
    test.add_file(
        "deps.wj",
        r#"
pub struct AppDeps {
    label: string,
}
"#,
    );
    test.add_file(
        "inbound.wj",
        r#"
use std::http::{ServerRequest, ServerResponse}
use deps::AppDeps
use router::match_route

pub fn handle_request(request: ServerRequest, _deps: AppDeps) -> ServerResponse {
    if let Some(response) = match_route(request) {
        return response
    }
    ServerResponse::not_found()
}
"#,
    );
    test.add_file(
        "server.wj",
        r#"
use std::http::*
use deps::AppDeps
use inbound::handle_request

pub fn run_server() {
    let deps = AppDeps { label: "demo" }
    let server = Server::new("0.0.0.0", 8080)
    match server.serve(|request| handle_request(request, deps)) {
        Ok(_) => {},
        Err(_) => {},
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("server.rs").expect("server.rs");

    // The ownership analyzer may infer handle_request(request: &ServerRequest)
    // since the function only reads from request. In that case the closure
    // call site correctly inserts &request (auto-borrow). Both patterns are
    // valid and should compile.
    let has_call = rs.contains("handle_request(request")
        || rs.contains("handle_request( request")
        || rs.contains("handle_request(&request");
    assert!(has_call, "serve closure must call handle_request. Got:\n{rs}");

    // Must NOT have double-borrow (&&request)
    assert!(
        !rs.contains("handle_request(&&request") && !rs.contains("handle_request( &&request"),
        "must not double-borrow request in serve closure. Got:\n{rs}"
    );
    test.assert_compiles_without_error();
}
