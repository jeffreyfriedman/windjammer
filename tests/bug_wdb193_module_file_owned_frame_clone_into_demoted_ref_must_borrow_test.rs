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

//! WDB-193: owned `frame.clone()` into demoted `&PgWireFrame` must borrow.
//!
//! Product residual (~2× &PgWireFrame←PgWireFrame), tip-out/gen pg_serve:
//!   `pg_wire_decode_simple_query_sql(frame: &PgWireFrame)`
//!   call `pg_wire_decode_simple_query_sql(frame.clone())` → E0308.
//! Same for decode_parse / decode_bind / decode_sync. Signature-driven.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod wire
pub mod serve
"#;

const WIRE: &str = r#"
pub struct WireFrame {
    pub msg_type: int,
    pub payload: string,
}

/// Read-only decode — tip demotes to `&WireFrame` (product pg_wire_decode_*).
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
    // Product: decode_simple_sql(frame.clone()) into demoted & — must &frame
    match decode_simple_sql(frame.clone()) {
        Some(_sql) => decode_sync(frame.clone()),
        None => false,
    }
}
"#;

fn wdb193_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("wire.wj", WIRE);
    test.add_file("serve.wj", SERVE);
    test
}

#[test]
fn wdb193_module_file_owned_frame_clone_into_demoted_ref_must_borrow() {
    let test = wdb193_fixture();
    let map = test
        .compile()
        .expect("WDB-193 multipass compile should succeed");
    let wire_rs = map.get("wire.rs").expect("wire.rs");
    let serve_rs = map.get("serve.rs").expect("serve.rs");

    eprintln!("WDB-193 wire.rs:\n{wire_rs}\nserve.rs:\n{serve_rs}");

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
            "WDB-193 RED: demoted &Frame received frame.clone(). \
             Product: pg_wire_decode_*(frame.clone()). Got:\n{serve_rs}"
        );
    }
    if demoted {
        assert!(
            good || !bad,
            "WDB-193: demoted &Frame must borrow. Got:\n{serve_rs}"
        );
    }
}

#[test]
fn wdb193_tip_out_pg_serve_must_borrow_frame_into_demoted_decode() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let serve = if tip.join("relational_pg_serve_port.rs").exists() {
        tip.join("relational_pg_serve_port.rs")
    } else {
        gen.join("relational/relational_pg_serve_port.rs")
    };
    let wire = gen.join("relational/relational_pg_wire_port.rs");
    if !serve.exists() {
        eprintln!("WDB-193: skip — pg_serve missing");
        return;
    }
    let serve_text = std::fs::read_to_string(&serve).expect("serve");
    let wire_text = if wire.exists() {
        std::fs::read_to_string(&wire).unwrap_or_default()
    } else {
        String::new()
    };
    let demoted = wire_text.contains("fn pg_wire_decode_simple_query_sql(frame: &PgWireFrame")
        || wire_text.contains("fn pg_wire_decode_sync(frame: &PgWireFrame");
    let bad = serve_text.contains("pg_wire_decode_simple_query_sql(frame.clone())")
        || serve_text.contains("pg_wire_decode_parse(frame.clone())")
        || serve_text.contains("pg_wire_decode_sync(frame.clone())");
    eprintln!(
        "WDB-193 tip-out demoted={} bad={} path={}",
        demoted,
        bad,
        serve.display()
    );
    assert!(
        !(demoted && bad),
        "WDB-193 RED: tip-out/product still passes frame.clone() into demoted decode. {}",
        serve.display()
    );
}
