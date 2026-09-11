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

//! FAILING REPRO — `use std::net::Request` must emit native net client imports.
//! Ecosystem `wj-fetch` `--timeout` adapter: bare `Request` resolves to `http::Request`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const TIMED_GET: &str = include_str!("fixtures/library_multipass/timed_net_get.wj");

fn assert_native_net_request_emitted(generated: &str) {
    assert!(
        generated.contains("native::net::Request")
            || generated.contains("platform::native::net"),
        "RED: std::net::Request must codegen native net imports; got:\n{generated}"
    );
    assert!(
        !generated.contains("http::Request {")
            && !generated.contains("windjammer_runtime::http::Request"),
        "must not map std::net::Request to http server Request; got:\n{generated}"
    );
}

#[test]
fn std_net_request_import_must_emit_native_net_types() {
    let generated = test_utils::compile_single(TIMED_GET);
    assert_native_net_request_emitted(&generated);
}

#[test]
fn std_net_request_import_multipass_must_emit_native_net_types() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod adapters
pub use adapters::timed_get
"#,
    );
    project.add_file("adapters/mod.wj", "pub mod http_get\n");
    project.add_file("adapters/http_get.wj", TIMED_GET);

    let map = project
        .compile()
        .expect("timed_get multipass compile should succeed");
    let adapter = map
        .get("adapters/http_get.rs")
        .expect("adapters/http_get.rs");
    assert_native_net_request_emitted(adapter);
}
