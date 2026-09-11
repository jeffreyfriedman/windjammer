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

//! FAILING REPRO — `Request::post(url, body).header(...).timeout(secs).send()` must qualify
//! native `std::net` client (`wj-fetch` POST class). DRY with get-timeout chain pattern.

#[path = "common/test_utils.rs"]
mod test_utils;

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const TIMED_POST: &str = include_str!("fixtures/library_multipass/net_request_post_header_timeout.wj");

fn assert_native_net_post_chain(generated: &str) {
    assert!(
        generated.contains("Request::post") || generated.contains("net::Request::post"),
        "must emit Request::post; got:\n{generated}"
    );
    assert!(
        generated.contains(".header("),
        "must emit .header(...); got:\n{generated}"
    );
    assert!(
        generated.contains(".timeout("),
        "must emit .timeout(secs); got:\n{generated}"
    );
    assert!(
        generated.contains("native::net")
            || generated.contains("platform::native::net")
            || generated.contains("windjammer_runtime::net::Request"),
        "RED: POST builder chain must qualify native net client; got:\n{generated}"
    );
    assert!(
        !generated.contains("http::Request::post")
            && !generated.contains("windjammer_runtime::http::Request"),
        "must not lower std::net Request builder to http server Request; got:\n{generated}"
    );
}

#[test]
fn std_net_request_post_header_timeout_chain_must_emit_native_client() {
    let generated = test_utils::compile_single(TIMED_POST);
    assert_native_net_post_chain(&generated);
}

#[test]
fn std_net_request_post_header_timeout_chain_hexagonal_must_emit_native_client() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod adapters
"#,
    );
    project.add_file("adapters/mod.wj", "pub mod http_post\n");
    project.add_file("adapters/http_post.wj", TIMED_POST);

    let map = project
        .compile()
        .expect("timed_post hexagonal compile should succeed");
    let adapter = map
        .get("adapters/http_post.rs")
        .expect("adapters/http_post.rs");
    assert_native_net_post_chain(adapter);
}
