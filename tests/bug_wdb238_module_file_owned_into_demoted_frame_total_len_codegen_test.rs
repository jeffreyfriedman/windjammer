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
    feature = "codegen_tests",
))]

//! WDB-238 multipass: owned inbound into demoted `&Vec` frame_total_len must borrow.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod wire
pub mod serve
"#;

const WIRE: &str = r#"
pub fn frame_total_len(bytes: Vec<u8>, offset: int) -> int {
    let _ = offset
    bytes.len() as int
}
"#;

const SERVE: &str = r#"
use crate::wire::frame_total_len

pub fn feed(inbound: Vec<u8>, offset: int) -> int {
    frame_total_len(inbound, offset)
}
"#;

#[test]
fn wdb238_codegen_owned_into_demoted_frame_total_len_must_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("wire.wj", WIRE);
    test.add_file("serve.wj", SERVE);
    let map = test.compile().expect("WDB-238 multipass compile");
    let wire = map.get("wire.rs").expect("wire.rs");
    let serve = map.get("serve.rs").expect("serve.rs");
    let demoted = wire.contains("fn frame_total_len(bytes: &Vec<u8>")
        || wire.contains("fn frame_total_len(bytes: &[u8]");
    let bad = demoted
        && serve.contains("frame_total_len(inbound.clone()")
        && !serve.contains("frame_total_len(&inbound");
    eprintln!("WDB-238 demoted={demoted} bad={bad}\nwire:\n{wire}\nserve:\n{serve}");
    assert!(
        !bad,
        "WDB-238: owned into demoted &Vec must borrow, not clone"
    );
    test.cargo_check().expect("WDB-238 cargo-check");
}
