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

//! FAILING REPRO — `Request::get(url).timeout(secs).send()` must qualify native `std::net`
//! client methods (`wj-fetch` timeout adapter). Distinct from struct-literal `timed_net_get.wj`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const TIMED_CHAIN: &str = include_str!("fixtures/library_multipass/net_request_get_timeout_send.wj");

fn assert_native_net_timeout_chain(generated: &str) {
    assert!(
        generated.contains("Request::get") || generated.contains("net::Request::get"),
        "must emit Request::get; got:\n{generated}"
    );
    assert!(
        generated.contains(".timeout("),
        "must emit .timeout(secs); got:\n{generated}"
    );
    assert!(
        generated.contains("native::net")
            || generated.contains("platform::native::net")
            || generated.contains("windjammer_runtime::net::Request"),
        "RED: timeout chain must qualify native net client; got:\n{generated}"
    );
    assert!(
        !generated.contains("http::Request::get")
            && !generated.contains("windjammer_runtime::http::Request"),
        "must not lower std::net Request builder to http server Request; got:\n{generated}"
    );
}

#[test]
fn std_net_request_get_timeout_chain_must_emit_native_client() {
    let generated = test_utils::compile_single(TIMED_CHAIN);
    assert_native_net_timeout_chain(&generated);
}

#[test]
fn std_net_request_get_timeout_chain_hexagonal_must_emit_native_client() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod adapters
"#,
    );
    project.add_file("adapters/mod.wj", "pub mod http_get\n");
    project.add_file("adapters/http_get.wj", TIMED_CHAIN);

    let map = project
        .compile()
        .expect("timed_get timeout chain hexagonal compile should succeed");
    let adapter = map
        .get("adapters/http_get.rs")
        .expect("adapters/http_get.rs");
    assert_native_net_timeout_chain(adapter);
}
