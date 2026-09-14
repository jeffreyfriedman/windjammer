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

//! FAILING REPRO — hexagonal adapter: owned `method_label` → demoted `method: &str`.
//!
//! Mirrors `wj-notes-api` / `wj-auth-api` before `handle_http`: multipass demotes
//! `App::handle`'s first formal to `&str` but emits owned `String` from
//! `method_label(req.method)` → E0308.
//!
//! Single-file gate `bug_owned_helper_into_demoted_str_formal_must_auto_borrow_test`
//! is tip GREEN; this multipass shape still RED on cargo-bin 0.50.0 product builds.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const DOMAIN_APP: &str = include_str!("fixtures/library_multipass/http_domain_method_label.wj");
const HTTP_ADAPTER: &str = include_str!("fixtures/library_multipass/http_adapter_method_label.wj");

#[test]
fn multipass_http_hexagonal_method_label_into_demoted_str_must_auto_borrow() {
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
        .expect("hexagonal method_label adapter should transpile");
    let domain = map
        .get("domain/app.rs")
        .expect("domain/app.rs");
    let adapter = map
        .get("adapters/http_server.rs")
        .expect("adapters/http_server.rs");

    let demoted = domain.contains("method: &str") || domain.contains("method:&str");
    if demoted {
        assert!(
            adapter.contains("handle(&method_label(")
                || adapter.contains("handle(&(method_label(")
                || adapter.contains("&method_label("),
            "RED: owned method_label into demoted &str must auto-borrow. adapter:\n{adapter}\ndomain:\n{domain}"
        );
    }

    test.cargo_check()
        .expect("hexagonal method_label → demoted &str must cargo-check (auto-borrow)");
}
