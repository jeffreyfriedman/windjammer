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

//! WDB-239: owned `Vec` into demoted `&Vec` materialize formals must borrow.
//!
//! Product residual tip-out/gen graph_sql_query_port (~4× &Vec←Vec):
//!   `graph_materialize_from_edge_lists(srcs: Vec<i64>, dsts: &Vec<i64>, weights: &Vec<f64>, …)`
//!   called as `graph_materialize_from_edge_lists(srcs, dsts, weights, false)` → E0308.
//! Signature-driven: pass `&dsts`, `&weights`. Twin of WDB-205/238 (owned into demoted Vec).
//!
//! Tip truth (2026-09-17): multipass keeps all three Vec formals Owned — call sites move.
//! Prefer tip-out over stale gen/; pair formal shape with call site per root.

use std::path::PathBuf;

fn materialize_formal_shape(text: &str) -> Option<&'static str> {
    if text.contains("fn graph_materialize_from_edge_lists(srcs: Vec<i64>, dsts: &Vec<i64>")
        || text.contains("dsts: &Vec<i64>, weights: &Vec<f64>")
    {
        Some("demoted")
    } else if text.contains(
        "fn graph_materialize_from_edge_lists(srcs: Vec<i64>, dsts: Vec<i64>, weights: Vec<f64>",
    ) {
        Some("owned")
    } else {
        None
    }
}

#[test]
fn wdb239_tip_out_sql_must_borrow_owned_vecs_into_demoted_materialize() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");

    // Prefer tip-out when present (gen may lag behind tip multipass).
    let roots: Vec<(PathBuf, PathBuf)> = if tip.join("graph_materialize_port.rs").exists() {
        vec![(
            tip.join("graph_materialize_port.rs"),
            tip.join("graph_sql_query_port.rs"),
        )]
    } else {
        vec![(
            gen.join("graph/graph_materialize_port.rs"),
            gen.join("graph/graph_sql_query_port.rs"),
        )]
    };

    let mut saw = false;
    for (mat_path, sql_path) in &roots {
        if !mat_path.exists() || !sql_path.exists() {
            continue;
        }
        saw = true;
        let mat = std::fs::read_to_string(mat_path).expect("materialize");
        let sql = std::fs::read_to_string(sql_path).expect("sql");
        let shape = materialize_formal_shape(&mat)
            .expect("WDB-239: materialize edge-list formals missing (demoted or owned)");
        let bad = shape == "demoted"
            && sql.contains("graph_materialize_from_edge_lists(srcs, dsts, weights,")
            && !sql.contains("graph_materialize_from_edge_lists(srcs, &dsts, &weights,")
            && !sql.contains("graph_materialize_from_edge_lists(srcs, &dsts, weights,");
        eprintln!(
            "WDB-239 shape={} bad={} mat={} sql={}",
            shape,
            bad,
            mat_path.display(),
            sql_path.display()
        );
        assert!(
            !bad,
            "WDB-239 RED: tip-out/product passes owned dsts/weights into demoted &Vec materialize. {}",
            sql_path.display()
        );
    }
    assert!(saw, "WDB-239: graph_sql_query_port / materialize missing");
}
