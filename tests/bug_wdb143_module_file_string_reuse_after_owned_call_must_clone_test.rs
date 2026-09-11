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

//! WDB-143: `string` passed into an owned formal then reused in a struct field
//! must clone (or borrow) — tip currently moves and then reuses → E0382.
//!
//! Product: `document_otlp_dremel_kv_pipeline_port.rs`
//!   `encode(..., service, attrs)` then `service_name: service`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod encode
pub mod pipe
"#;

const ENCODE: &str = r#"
pub fn take_service(service: string) -> u32 {
    if service == "checkout" {
        return 1
    }
    0
}
"#;

const PIPE: &str = r#"
use crate::encode::take_service

pub struct Result {
    pub code: u32,
    pub service_name: string,
}

pub fn run(label: string) -> Result {
    let mut service = "unknown"
    if label == "service=checkout" {
        service = "checkout"
    }
    let code = take_service(service)
    Result {
        code: code,
        service_name: service,
    }
}

pub fn cap() -> string {
    let r = run("service=checkout")
    r.service_name
}
"#;

fn wdb143_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("encode.wj", ENCODE);
    test.add_file("pipe.wj", PIPE);
    test
}

#[test]
fn wdb143_module_file_string_reuse_after_owned_call_must_clone() {
    let test = wdb143_fixture();
    let map = test
        .compile()
        .expect("WDB-143 multipass compile should succeed (codegen may still be wrong)");
    let pipe_rs = map.get("pipe.rs").expect("pipe.rs");

    // Tip bug: move into owned call then `service.to_string()` / `service` in field.
    let call_moves = pipe_rs.contains("take_service(service)")
        && !pipe_rs.contains("take_service(service.clone())");
    let field_reuses = pipe_rs.contains("service_name: service")
        || pipe_rs.contains("service_name: service.to_string()")
        || pipe_rs.contains("service_name: service.clone()");
    // Good: clone into call, or clone/to_string into field *without* prior move.
    let good = pipe_rs.contains("take_service(service.clone())")
        || (pipe_rs.contains("service_name: service.clone()") && !call_moves);

    eprintln!("WDB-143 pipe.rs:\n{pipe_rs}");
    eprintln!("call_moves={call_moves} field_reuses={field_reuses} good={good}");

    if call_moves && field_reuses {
        panic!(
            "WDB-143 RED: string moved into owned call then reused in field. \
             Tip emits take_service(service) + service_name: service.to_string(). \
             Product: document_otlp_dremel_kv_from_resource_label."
        );
    }

    test.cargo_check().expect(
        "WDB-143: reuse of string after owned call must cargo-check.",
    );
}
