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

//! FAILING REPRO — hexagonal `domain/` + `adapters/` multipass must emit owned HTTP reply/body.
//!
//! Mirrors `wj-notes-api` / `wj-auth-api` layout: `domain/app.wj` + `adapters/http_server.wj`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const DOMAIN_APP: &str = include_str!("fixtures/library_multipass/http_domain_reply.wj");
const HTTP_ADAPTER: &str = include_str!("fixtures/library_multipass/http_adapter_hexagonal.wj");

#[test]
fn multipass_http_hexagonal_adapter_must_use_owned_reply_and_body() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod domain
pub mod adapters
"#,
    );
    test.add_file("domain/mod.wj", "pub mod app\n");
    test.add_file("domain/app.wj", DOMAIN_APP);
    test.add_file("adapters/mod.wj", "pub mod http_server\n");
    test.add_file("adapters/http_server.wj", HTTP_ADAPTER);

    let map = test
        .compile()
        .expect("hexagonal http adapter should compile");
    let adapter = map
        .get("adapters/http_server.rs")
        .expect("adapters/http_server.rs");

    assert!(
        !adapter.contains("to_response(&mut"),
        "RED: hexagonal to_response must take owned HttpReply; emitted:\n{adapter}"
    );
    assert!(
        !adapter.contains("json_response(&ServerResponse::bad_request(body))"),
        "RED: hexagonal json_response must own body; emitted:\n{adapter}"
    );
    test.cargo_check()
        .expect("hexagonal HTTP adapter must cargo-check");
}
