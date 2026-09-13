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

//! WDB-175: demoted `&Vec<u8>` into owned `Vec<u8>` formal must clone (inverse WDB-171).
//!
//! Product residual after tip-out sync (~12× Vec←&Vec), e.g. relational_pg_serve_port:
//!   `pg_wire_decode_startup(startup)` while `startup: &Vec<u8>` and formal is `Vec<u8>`
//! → E0308 expected `Vec<u8>`, found `&Vec<u8>`.
//!
//! Signature-driven. Prefer tip greens over dogfood.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod wire
pub mod serve
"#;

const WIRE: &str = r#"
/// Owns the startup bytes (product pg_wire_decode_startup keeps `Vec<u8>`).
pub fn decode_startup(buf: Vec<u8>) -> int {
    buf.len() as int
}
"#;

const SERVE: &str = r#"
use crate::wire::decode_startup

/// Multi-use read-only probe demotes formal toward `&Vec<u8>`.
pub fn buf_len(buf: Vec<u8>) -> int {
    buf.len() as int
}

pub fn on_startup(buf: Vec<u8>) -> int {
    let _n = buf_len(buf)
    // Product: decode_startup(startup) while demoted — must clone into owned Vec.
    decode_startup(buf)
}
"#;

fn wdb175_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("wire.wj", WIRE);
    test.add_file("serve.wj", SERVE);
    test
}

#[test]
fn wdb175_module_file_demoted_vec_into_owned_vec_must_clone() {
    let test = wdb175_fixture();
    let map = test
        .compile()
        .expect("WDB-175 multipass compile should succeed");
    let wire_rs = map.get("wire.rs").expect("wire.rs");
    let serve_rs = map.get("serve.rs").expect("serve.rs");

    eprintln!("WDB-175 wire.rs:\n{wire_rs}\nserve.rs:\n{serve_rs}");

    let callee_owned = {
        let i = wire_rs.find("fn decode_startup").unwrap_or(0);
        let sl = &wire_rs[i..wire_rs.len().min(i + 120)];
        (sl.contains("buf: Vec<u8>") || sl.contains("buf:Vec<u8>"))
            && !(sl.contains("buf: &Vec") || sl.contains("buf:&Vec"))
    };
    let caller_demoted = {
        let i = serve_rs.find("fn on_startup").unwrap_or(0);
        let sl = &serve_rs[i..serve_rs.len().min(i + 100)];
        sl.contains("buf: &Vec<u8>") || sl.contains("buf:&Vec<u8>")
    };
    let call_ok = serve_rs.contains("decode_startup(buf.clone()")
        || serve_rs.contains("decode_startup((*buf).clone()")
        || serve_rs.contains("decode_startup(buf.to_vec()")
        || serve_rs.contains("decode_startup((*buf).to_vec()");
    let call_bad = serve_rs.contains("decode_startup(buf)")
        && !serve_rs.contains("decode_startup(buf.clone()")
        && !serve_rs.contains("decode_startup(buf.to_vec()");

    if callee_owned && caller_demoted && call_bad && !call_ok {
        panic!(
            "WDB-175 RED: demoted &Vec<u8> passed into owned decode_startup without clone. \
             Product: pg_wire_decode_startup(startup) with startup: &Vec<u8>. Got:\n{serve_rs}\n{wire_rs}"
        );
    }

    if callee_owned && caller_demoted {
        assert!(
            call_ok || !call_bad,
            "WDB-175: demoted &Vec into owned Vec must clone. Got:\n{serve_rs}"
        );
    }
}

#[test]
fn wdb175_product_pg_serve_must_clone_demoted_startup_into_owned_decode() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let serve = if tip.join("relational_pg_serve_port.rs").exists() {
        tip.join("relational_pg_serve_port.rs")
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("windjammerdb/crates/wdb-layers/gen/relational/relational_pg_serve_port.rs")
    };
    let wire = if tip.join("relational_pg_wire_port.rs").exists() {
        tip.join("relational_pg_wire_port.rs")
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("windjammerdb/crates/wdb-layers/gen/relational/relational_pg_wire_port.rs")
    };
    if !serve.exists() || !wire.exists() {
        eprintln!("WDB-175: skip product gate — serve/wire missing");
        return;
    }
    let serve_text = std::fs::read_to_string(&serve).expect("serve");
    let wire_text = std::fs::read_to_string(&wire).expect("wire");
    let decode_owned = wire_text.contains("fn pg_wire_decode_startup(buf: Vec<u8>")
        || wire_text.contains("fn pg_wire_decode_startup(startup: Vec<u8>");
    // Look for bare demoted pass: decode_startup(startup) without clone when startup is &Vec
    let bad = decode_owned
        && (serve_text.contains("pg_wire_decode_startup(startup)")
            || serve_text.contains("pg_wire_decode_startup(&startup)"))
        && !serve_text.contains("pg_wire_decode_startup(startup.clone()")
        && !serve_text.contains("pg_wire_decode_startup((*startup).clone()");
    // Only RED if serve formal for that path is demoted — check nearby signature
    let serve_demotes = serve_text.contains("startup: &Vec<u8>")
        || serve_text.contains("buf: &Vec<u8>");
    eprintln!(
        "WDB-175 product decode_owned={} serve_demotes={} bad={}",
        decode_owned, serve_demotes, bad
    );
    assert!(
        !(decode_owned && serve_demotes && bad),
        "WDB-175 RED: product pg_serve still passes &Vec into owned decode_startup."
    );
}
