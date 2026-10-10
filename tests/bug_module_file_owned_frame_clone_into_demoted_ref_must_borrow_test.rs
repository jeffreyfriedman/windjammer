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

//!
//!   `pg_wire_decode_simple_query_sql(frame: &PgWireFrame)`
//!   call `pg_wire_decode_simple_query_sql(frame.clone())` → E0308.
//! Same for decode_parse / decode_bind / decode_sync. Signature-driven.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod wire
pub mod serve
"#;

const WIRE: &str = r#"
pub struct WireFrame {
    pub msg_type: int,
    pub payload: string,
}

pub fn decode_simple_sql(frame: WireFrame) -> Option<string> {
    if frame.msg_type > 0 {
        return Some(frame.payload)
    }
    None
}

pub fn decode_sync(frame: WireFrame) -> bool {
    frame.msg_type > 0
}
"#;

const SERVE: &str = r#"
use crate::wire::WireFrame
use crate::wire::decode_simple_sql
use crate::wire::decode_sync

pub fn feed_ready_frame(frame: WireFrame) -> bool {
    match decode_simple_sql(frame.clone()) {
        Some(_sql) => decode_sync(frame.clone()),
        None => false,
    }
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("wire.wj", WIRE);
    test.add_file("serve.wj", SERVE);
    test
}

#[test]
fn module_file_owned_frame_clone_into_demoted_ref_must_borrow() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let wire_rs = map.get("wire.rs").expect("wire.rs");
    let serve_rs = map.get("serve.rs").expect("serve.rs");

    eprintln!("wire.rs:\n{wire_rs}\nserve.rs:\n{serve_rs}");

    let demoted = {
        let i = wire_rs.find("fn decode_simple_sql").unwrap_or(0);
        let sl = &wire_rs[i..wire_rs.len().min(i + 100)];
        sl.contains("frame: &WireFrame") || sl.contains("frame:&WireFrame")
    };
    let bad = serve_rs.contains("decode_simple_sql(frame.clone())")
        && !serve_rs.contains("decode_simple_sql(&frame");
    let good = serve_rs.contains("decode_simple_sql(&frame")
        || serve_rs.contains("decode_simple_sql(&frame.clone()");

    if demoted && bad && !good {
        panic!(
            "RED: demoted &Frame received frame.clone(). \
             Got:\n{serve_rs}"
        );
    }
    if demoted {
        assert!(
            good || !bad,
            "demoted &Frame must borrow. Got:\n{serve_rs}"
        );
    }
}

