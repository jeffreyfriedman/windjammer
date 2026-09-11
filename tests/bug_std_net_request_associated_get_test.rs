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

//! FAILING REPRO — `Request::get(url)` must codegen native `std::net` client, not `http::Request`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SIMPLE_GET: &str = include_str!("fixtures/library_multipass/net_request_associated_get.wj");

fn assert_native_net_request_get_emitted(generated: &str) {
    assert!(
        generated.contains("net::Request::get")
            || generated.contains("native::net::Request::get")
            || generated.contains("platform::native::net::Request::get"),
        "RED: Request::get must qualify native net client; got:\n{generated}"
    );
    assert!(
        !generated.contains("http::Request::get")
            && !generated.contains("windjammer_runtime::http::Request"),
        "must not lower std::net::Request::get to http server Request; got:\n{generated}"
    );
}

#[test]
fn std_net_request_associated_get_must_emit_native_client() {
    let generated = test_utils::compile_single(SIMPLE_GET);
    assert_native_net_request_get_emitted(&generated);
}

#[test]
fn std_net_request_associated_get_multipass_must_emit_native_client() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod adapters
pub use adapters::simple_get
"#,
    );
    project.add_file("adapters/mod.wj", "pub mod http_get\n");
    project.add_file("adapters/http_get.wj", SIMPLE_GET);

    let map = project
        .compile()
        .expect("simple_get multipass compile should succeed");
    let adapter = map
        .get("adapters/http_get.rs")
        .expect("adapters/http_get.rs");
    assert_native_net_request_get_emitted(adapter);
}
