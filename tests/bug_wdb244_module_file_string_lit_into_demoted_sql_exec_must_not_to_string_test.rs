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

//! WDB-244: string lit / `.to_string()` into demoted `&str` columnar `sql_exec` must stay `&str`.
//!
//! Product residual tip-out/gen graph_sql_datafusion_port (~46× &str←String class):
//!   `sql_exec(..., left_table: &str, right_table: &str, …)` called with
//!   `"props".to_string()`, `"edges".to_string()` → E0308.
//! Twin of WDB-225 (join_path String::from). Signature-driven: bare lit / no `.to_string()`.

use std::path::PathBuf;

#[test]
fn wdb244_tip_out_datafusion_must_not_to_string_lits_into_demoted_sql_exec() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    // Columnar formals live in wdb-types gen (cross-crate)
    let columnar = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-types/gen/columnar_batch_arrow.rs");
    let demoted = if columnar.exists() {
        let text = std::fs::read_to_string(&columnar).expect("columnar");
        text.contains("fn sql_exec(") && text.contains("left_table: &str")
    } else {
        true // tip-out call-site gate still valid
    };

    let paths = [
        tip.join("graph_sql_datafusion_port.rs"),
        gen.join("graph/graph_sql_datafusion_port.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("df");
        let bad = text.contains("sql_exec(right_ab, \"props\".to_string(), \"edges\".to_string()")
            || text.contains("sql_exec_csr_edges(\"props\".to_string(), \"edges\".to_string()");
        eprintln!(
            "WDB-244 demoted={} bad={} path={}",
            demoted,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-244 RED: tip-out/product passes .to_string() lits into demoted &str sql_exec. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-244: graph_sql_datafusion_port missing");
}
