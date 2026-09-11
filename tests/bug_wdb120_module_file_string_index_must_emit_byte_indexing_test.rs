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

//! WDB-120: `string[i]` / `s[i] as u8` must emit byte indexing, not `str` Index.
//!
//! WindjammerDB CQ-C5: `observability_http_tcp_server_exchange` had
//!   `while i < resp.body.len() { wire.push(resp.body[i] as u8) }`
//! tip/cargo module-file emit `resp.body[i as usize]` on `String`/`str` →
//! rustc E0277 (`str` cannot be indexed by `usize`).
//!
//! Expected: emit `as_bytes()[i]` (or equivalent) so Cap UTF-8 body→`Vec<u8>` works.
//! Gate: multipass cargo-check (RED until fixed).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod wire
"#;

const WIRE: &str = r#"
pub fn body_to_wire(body: string) -> Vec<u8> {
    let mut wire: Vec<u8> = Vec::new()
    let mut i = 0
    while i < body.len() {
        wire.push(body[i] as u8)
        i = i + 1
    }
    wire
}

pub fn cap() -> Vec<u8> {
    body_to_wire("ok")
}
"#;

fn wdb120_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("wire.wj", WIRE);
    test
}

#[test]
fn wdb120_module_file_string_index_must_emit_byte_indexing() {
    let mut test = wdb120_fixture();
    let map = test
        .compile()
        .expect("WDB-120 multipass compile should succeed (codegen may still be wrong)");
    let wire_rs = map.get("wire.rs").expect("wire.rs must be generated");

    let indexes_str_directly = (wire_rs.contains("body[") || wire_rs.contains("body ["))
        && !wire_rs.contains("as_bytes()")
        && !wire_rs.contains(".bytes()");

    if indexes_str_directly {
        eprintln!("WDB-120 RED emit wire.rs:\n{wire_rs}");
    }

    test.cargo_check().expect(
        "WDB-120 RED: string[i] as u8 must cargo-check via byte indexing. Product: observability_http_tcp_server_port E0277.",
    );
}
