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

//! WDB-139: owned `string` param used in `== "lit"` + assigned into `String` field.
//!
//! Tip currently emits `impl Into<String>` then:
//!   - compares with `== &str` (E0369)
//!   - assigns bare param into `String` field without `.into()` (E0308)
//!
//! Product CQ-C5 also sees demoted `&str` + `.clone()` into `String` fields
//! (`body_label: request_label.clone()` in `observability_http_tcp_server_port.rs`).
//!
//! Expected: `String` formal (or demoted `&str` + `.to_string()` / `.into()`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod http
pub mod cap
"#;

const HTTP: &str = r#"
pub struct HttpServerRequest {
    pub body_label: string,
}

/// Cap leaf + read-only reuse — tip must keep assignable String (or .into/.to_string).
pub fn exchange(request_label: string) -> HttpServerRequest {
    let _has_metrics = request_label == "path=/v1/metrics"
    let _has_mcp = request_label == "path=/mcp"
    HttpServerRequest {
        body_label: request_label,
    }
}
"#;

const CAP: &str = r#"
use crate::http::exchange

pub fn cap_traces() -> string {
    let ex = exchange("POST path=/v1/traces")
    ex.body_label
}
"#;

fn wdb139_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("http.wj", HTTP);
    test.add_file("cap.wj", CAP);
    test
}

#[test]
fn wdb139_module_file_owned_string_into_string_field_must_compile() {
    let test = wdb139_fixture();
    let map = test
        .compile()
        .expect("WDB-139 multipass compile should succeed (codegen may still be wrong)");
    let http_rs = map.get("http.rs").expect("http.rs");

    let into_string = http_rs.contains("impl Into<String>");
    let missing_into = into_string
        && http_rs.contains("body_label: request_label")
        && !http_rs.contains("request_label.into()")
        && !http_rs.contains("request_label.to_string()");
    let demoted = http_rs.contains("request_label: &str");
    let bad_clone = http_rs.contains("body_label: request_label.clone()");
    let good = http_rs.contains("body_label: request_label.to_string()")
        || http_rs.contains("body_label: request_label.into()")
        || (http_rs.contains("request_label: String")
            && http_rs.contains("body_label: request_label"));

    eprintln!("WDB-139 http.rs:\n{http_rs}");
    eprintln!(
        "into_string={into_string} missing_into={missing_into} demoted={demoted} bad_clone={bad_clone} good={good}"
    );

    if missing_into {
        panic!(
            "WDB-139 RED: impl Into<String> formal must .into() into String field \
             (and must not == &str without binding). Product dogfood uses .to_string() \
             for demoted &str until tip lands."
        );
    }
    if demoted && bad_clone {
        panic!(
            "WDB-139 RED: demoted &str into owned String field must .to_string(), not .clone()."
        );
    }

    test.cargo_check().expect(
        "WDB-139: string param compared + stored into String field must cargo-check.",
    );
}
