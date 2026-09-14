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

//! WDB-205: owned `inbound.clone()` into demoted `&Vec<u8>` decode must borrow.
//!
//! Product residual (~4× &Vec<u8>←Vec), tip-out/gen pg_serve:
//!   `pg_wire_frame_decode(inbound: &Vec<u8>)`
//!   call `pg_wire_frame_decode(inbound.clone())` → E0308.
//! Same class as WDB-193 (frame) for byte buffers.

use std::path::PathBuf;

#[test]
fn wdb205_tip_out_pg_serve_must_borrow_inbound_into_demoted_frame_decode() {
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
        eprintln!("WDB-205: skip — pg_serve missing");
        return;
    }
    let serve_text = std::fs::read_to_string(&serve).expect("serve");
    let wire_text = if wire.exists() {
        std::fs::read_to_string(&wire).unwrap_or_default()
    } else {
        String::new()
    };
    let demoted = wire_text.contains("fn pg_wire_frame_decode(inbound: &Vec<u8>")
        || wire_text.contains("inbound: &Vec<u8>");
    let bad = serve_text.contains("pg_wire_frame_decode(inbound.clone())")
        && !serve_text.contains("pg_wire_frame_decode(&inbound");
    eprintln!(
        "WDB-205 tip-out demoted={} bad={} path={}",
        demoted,
        bad,
        serve.display()
    );
    assert!(
        !(demoted && bad),
        "WDB-205 RED: tip-out/product passes inbound.clone() into demoted &Vec decode. {}",
        serve.display()
    );
}
