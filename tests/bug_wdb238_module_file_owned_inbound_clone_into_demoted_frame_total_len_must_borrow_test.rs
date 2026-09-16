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

//! WDB-238: owned `inbound.clone()` into demoted `&Vec<u8>` `pg_wire_frame_total_len`
//! must borrow.
//!
//! Twin of WDB-205 (`pg_wire_frame_decode`). Product residual tip-out/gen
//! relational_pg_serve_port (~4× &Vec←Vec overall):
//!   `pg_wire_frame_total_len(bytes: &Vec<u8>, …)`
//!   called with `inbound.clone()` → E0308 expected `&Vec<_>`, found `Vec<_>`.
//! Signature-driven: pass `&inbound` (or demote call site borrow).

use std::path::PathBuf;

#[test]
fn wdb238_tip_out_pg_serve_must_borrow_inbound_into_demoted_frame_total_len() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let wire_paths = [
        tip.join("relational_pg_wire_port.rs"),
        gen.join("relational/relational_pg_wire_port.rs"),
    ];
    let mut demoted = false;
    for path in &wire_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("wire");
        if text.contains("fn pg_wire_frame_total_len(bytes: &Vec<u8>")
            || text.contains("fn pg_wire_frame_total_len(inbound: &Vec<u8>")
        {
            demoted = true;
            break;
        }
    }
    assert!(
        demoted,
        "WDB-238: demoted &Vec formal for pg_wire_frame_total_len missing"
    );

    let paths = [
        tip.join("relational_pg_serve_port.rs"),
        gen.join("relational/relational_pg_serve_port.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("serve");
        let bad = text.contains("pg_wire_frame_total_len(inbound.clone()")
            && !text.contains("pg_wire_frame_total_len(&inbound");
        eprintln!("WDB-238 demoted={} bad={} path={}", demoted, bad, path.display());
        assert!(
            !bad,
            "WDB-238 RED: tip-out/product passes inbound.clone() into demoted &Vec frame_total_len. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-238: relational_pg_serve_port missing");
}
