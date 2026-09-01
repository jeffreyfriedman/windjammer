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

//! FAILING REPRO — multipass HTTP adapter must pass owned `HttpReply` to `to_response` and owned
//! `string` bodies into `ServerResponse` helpers.
//!
//! Ecosystem `wj-notes-api` / `wj-proxy` on wj 0.50.0 (2026-08-31) emits `to_response(&mut reply)`,
//! `handle(..., &body, ...)`, and `json_response(&ServerResponse::bad_request(body))` with `body: &str`
//! → E0308 across all hexagonal HTTP apps.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const HTTP_ADAPTER: &str = include_str!("fixtures/library_multipass/http_adapter_owned_reply.wj");

#[test]
fn multipass_http_adapter_must_use_owned_reply_and_body() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod http_adapter
"#,
    );
    test.add_file("http_adapter.wj", HTTP_ADAPTER);

    let map = test
        .compile()
        .expect("http adapter fixture should compile");
    let adapter = map.get("http_adapter.rs").expect("http_adapter.rs");

    assert!(
        !adapter.contains("to_response(&mut"),
        "RED: to_response must take owned HttpReply; emitted:\n{adapter}"
    );
    assert!(
        !adapter.contains("json_response(&ServerResponse::bad_request(body))"),
        "RED: json_response must receive owned ServerResponse built from owned body; emitted:\n{adapter}"
    );
    test.cargo_check()
        .expect("multipass HTTP adapter fixture must cargo-check");
}
