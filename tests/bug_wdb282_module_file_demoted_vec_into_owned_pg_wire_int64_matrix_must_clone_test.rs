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

//! WDB-282: demoted `&Vec` into owned `pg_wire_encode_select_int64_matrix` must clone.
//!
//! Twin of WDB-241/224. Tip-out pg execute / sf1 multicol still emits:
//!   `pg_wire_encode_select_int64_matrix(&fields, &matrix)`
//! while formals are owned `Vec<…>` → E0308.
//! Signature-driven: clone/move fields and rows.

use std::path::PathBuf;

#[test]
fn wdb282_tip_out_pg_wire_must_clone_ref_vec_into_owned_int64_matrix() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let wire_paths = [
        tip.join("relational_pg_wire_port.rs"),
        gen.join("relational/relational_pg_wire_port.rs"),
        gen.join("graph/relational_pg_wire_port.rs"),
    ];
    let mut owned = false;
    for path in &wire_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("pg_wire");
        if text.contains("fn pg_wire_encode_select_int64_matrix(fields: Vec<PgWireFieldDesc>") {
            owned = true;
            break;
        }
    }
    assert!(
        owned,
        "WDB-282: owned Vec pg_wire_encode_select_int64_matrix formal missing"
    );

    let call_paths = [
        tip.join("relational_pg_execute_port.rs"),
        tip.join("relational_pg_serve_sf1_multicol_port.rs"),
        gen.join("relational/relational_pg_execute_port.rs"),
        gen.join("graph/relational_pg_execute_port.rs"),
    ];
    let mut saw = false;
    let mut any_bad = false;
    let mut bad_path = String::new();
    for path in &call_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("pg call");
        let bad = text.contains("pg_wire_encode_select_int64_matrix(&fields,")
            || text.contains(
                "pg_wire_encode_select_int64_matrix(&pg_wire_serve_sf1_multicol_q6_fields(),",
            );
        eprintln!("WDB-282 bad={} path={}", bad, path.display());
        if bad {
            any_bad = true;
            bad_path = path.display().to_string();
        }
    }
    assert!(saw, "WDB-282: pg execute/serve ports missing");
    assert!(
        !any_bad,
        "WDB-282 RED: tip-out/product passes &Vec into owned pg_wire_encode_select_int64_matrix. {}",
        bad_path
    );
}
