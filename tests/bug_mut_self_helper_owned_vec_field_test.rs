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

//! P3.221/P3.223 (`wj-proxy`): read-only helper that clones `self.logs` then passes
//! the clone to an owned-`Vec` helper must emit `&self` so Mutex adapters cargo-check.

#[path = "common/test_utils.rs"]
mod test_utils;

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const DOMAIN: &str = include_str!("fixtures/library_multipass/proxy_stats_reply.wj");
const ADAPTER: &str = include_str!("fixtures/library_multipass/proxy_http_adapter.wj");

fn assert_stats_reply_borrows_self(rs: &str) {
    assert!(
        rs.contains("fn list_logs(&self)") || rs.contains("list_logs(&self)"),
        "list_logs pattern must borrow self. Got:\n{rs}"
    );
    assert!(
        rs.contains("fn stats_reply(&self)") || rs.contains("stats_reply(&self)"),
        "RED: stats_reply must borrow self like list_logs. Got:\n{rs}"
    );
    assert!(
        rs.contains("self.logs.clone()"),
        "explicit clone on field must emit .clone(). Got:\n{rs}"
    );
    assert!(
        !rs.contains("let snapshot = self.logs;"),
        "must not move self.logs when clone() requested. Got:\n{rs}"
    );
}

#[test]
fn test_readonly_vec_helper_method_must_borrow_self() {
    let rs = test_utils::compile_single(DOMAIN);
    assert_stats_reply_borrows_self(&rs);
}

#[test]
fn hexagonal_proxy_mutex_stats_reply_must_borrow_self() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod domain
pub mod adapters
"#,
    );
    project.add_file("domain/mod.wj", "pub mod app\n");
    project.add_file("domain/app.wj", DOMAIN);
    project.add_file("adapters/mod.wj", "pub mod http_server\n");
    project.add_file("adapters/http_server.wj", ADAPTER);

    let map = project
        .compile()
        .expect("hexagonal proxy compile should succeed");
    let domain_key = map
        .keys()
        .find(|k| k.ends_with("app.rs"))
        .cloned()
        .unwrap_or_else(|| panic!("missing app.rs; keys: {:?}", map.keys().collect::<Vec<_>>()));
    let domain_rs = map.get(&domain_key).expect("app.rs");
    assert_stats_reply_borrows_self(domain_rs);

    let adapter_key = map
        .keys()
        .find(|k| k.ends_with("http_server.rs"))
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "missing http_server.rs; keys: {:?}",
                map.keys().collect::<Vec<_>>()
            )
        });
    let adapter_rs = map.get(&adapter_key).expect("http_server.rs");
    assert!(
        adapter_rs.contains("handle_local("),
        "adapter must call handle_local; emitted:\n{adapter_rs}"
    );
}
