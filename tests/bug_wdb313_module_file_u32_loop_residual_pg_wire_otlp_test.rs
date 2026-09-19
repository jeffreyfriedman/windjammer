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

//! WDB-313: residual `u32 = 0_usize` in pg_wire + OTLP walk (beyond wave1 CLI / timeseries).
//!
//! Twin of WDB-298/308/311 for remaining tip-out/gen sites:
//!   `let mut i: u32 = 0_usize;` in relational_pg_wire_port + observability OTLP
//! Prefer `0_u32`.

use std::path::PathBuf;

#[test]
fn wdb313_tip_out_pg_wire_otlp_must_not_emit_u32_eq_0_usize() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("relational_pg_wire_port.rs"),
        tip.join("relational/relational_pg_wire_port.rs"),
        tip.join("observability_otlp_protobuf_recursive_walk_port.rs"),
        tip.join("observability/observability_otlp_protobuf_recursive_walk_port.rs"),
        gen.join("relational/relational_pg_wire_port.rs"),
        gen.join("observability/observability_otlp_protobuf_recursive_walk_port.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("port");
        if text.contains("u32 = 0_usize") {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-313: tip-out/gen pg_wire/OTLP missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-313 RED: tip-out/product still has u32=0_usize in:\n  {}",
        bad_paths.join("\n  ")
    );
}
