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

//! WDB-144: when tip demotes `string` → `&str`, call sites must pass `&str`
//! (not `String` / `.clone()` of a `String` binding).
//!
//! Product CQ-C5 (tip-synced `document_otlp_dremel_kv_pipeline_port`):
//!   `label: &str` but test emits
//!   `document_otlp_dremel_kv_from_resource_label(..., label.clone())` → E0308.
//! Body also emits `strings::contains(label.clone(), …)` on `&str`.
//!
//! Minimal fixture mirrors the multi-`contains` demotion trigger used in product.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod pipe
pub mod cap
"#;

const PIPE: &str = r#"
use std::strings

pub struct Parsed {
    pub code: u32,
    pub service_name: string,
}

pub fn parse_label(label: string) -> Parsed {
    let mut service = "unknown"
    if strings::contains(label, "service=checkout") {
        service = "checkout"
    }
    let mut code: u32 = 0
    if strings::contains(label, "attr=42") {
        code = code + 1
    }
    if strings::contains(label, "attr=99") {
        code = code + 1
    }
    Parsed {
        code: code,
        service_name: service,
    }
}
"#;

const CAP: &str = r#"
use crate::pipe::parse_label

pub fn cap_binding() -> string {
    let label = "service=checkout;attr=42;attr=99"
    let result = parse_label(label)
    result.service_name
}
"#;

fn wdb144_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("pipe.wj", PIPE);
    test.add_file("cap.wj", CAP);
    test
}

#[test]
fn wdb144_module_file_demoted_str_formal_must_not_receive_owned_string() {
    let test = wdb144_fixture();
    let map = test
        .compile()
        .expect("WDB-144 multipass compile should succeed (codegen may still be wrong)");
    let pipe_rs = map.get("pipe.rs").expect("pipe.rs");
    let cap_rs = map.get("cap.rs").expect("cap.rs");

    let demoted = {
        let i = pipe_rs.find("fn parse_label").unwrap_or(0);
        let sl = &pipe_rs[i..pipe_rs.len().min(i + 100)];
        sl.contains("label: &str")
    };
    let bad_call = demoted
        && (cap_rs.contains("parse_label(label.clone())")
            || (cap_rs.contains("parse_label(label)") && !cap_rs.contains("parse_label(&label)"))
            || cap_rs.contains("parse_label(String::from")
            || cap_rs.contains(".to_string())"));
    let bad_body_clone = demoted && pipe_rs.contains("label.clone()");

    eprintln!("WDB-144 pipe.rs:\n{pipe_rs}\ncap.rs:\n{cap_rs}");
    eprintln!("demoted={demoted} bad_call={bad_call} bad_body_clone={bad_body_clone}");

    if demoted && (bad_call || bad_body_clone) {
        panic!(
            "WDB-144 RED: demoted &str must be passed/used as &str (not String/clone). \
             Product: document_otlp_dremel_kv_from_resource_label + lib-test &str←String."
        );
    }

    // If tip no longer demotes, still require cargo-check of the fixture.
    test.cargo_check().expect(
        "WDB-144: string formal / demoted &str call sites must cargo-check.",
    );
}
