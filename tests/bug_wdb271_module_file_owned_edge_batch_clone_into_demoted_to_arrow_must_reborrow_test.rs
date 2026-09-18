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

//! WDB-271: owned `edges.clone()` into demoted `&GraphSqlEdgeBatch` to_arrow must reborrow.
//!
//! Tip greened to `graph_sql_edge_batch_to_arrow(&edges)` after demoting
//!   `graph_sql_edge_batch_to_arrow(batch: &GraphSqlEdgeBatch)`
//! but gen still emits `graph_sql_edge_batch_to_arrow(edges.clone())` → E0308.
//! Signature-driven: pass `&edges` / bare `edges` when formal is `&GraphSqlEdgeBatch`.

use std::path::PathBuf;

#[test]
fn wdb271_tip_out_sql_must_reborrow_owned_edge_batch_clone_into_demoted_to_arrow() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");

    let pairs = [
        (
            tip.join("graph_sql_arrow_ffi_port.rs"),
            tip.join("graph_sql_datafusion_port.rs"),
        ),
        (
            gen.join("graph/graph_sql_arrow_ffi_port.rs"),
            gen.join("graph/graph_sql_datafusion_port.rs"),
        ),
    ];
    let mut saw = false;
    let mut any_bad = false;
    let mut bad_path = String::new();
    for (formal_path, call_path) in &pairs {
        if !formal_path.exists() || !call_path.exists() {
            continue;
        }
        saw = true;
        let formal = std::fs::read_to_string(formal_path).expect("arrow_ffi");
        let demoted = formal.contains("fn graph_sql_edge_batch_to_arrow(batch: &GraphSqlEdgeBatch");
        let call = std::fs::read_to_string(call_path).expect("datafusion");
        let bad = demoted && call.contains("graph_sql_edge_batch_to_arrow(edges.clone())");
        eprintln!(
            "WDB-271 demoted={} bad={} path={}",
            demoted,
            bad,
            call_path.display()
        );
        if bad {
            any_bad = true;
            bad_path = call_path.display().to_string();
        }
    }
    assert!(saw, "WDB-271: arrow_ffi / datafusion missing");
    assert!(
        !any_bad,
        "WDB-271 RED: tip-out/product passes edges.clone() into demoted &GraphSqlEdgeBatch to_arrow. {}",
        bad_path
    );
}
