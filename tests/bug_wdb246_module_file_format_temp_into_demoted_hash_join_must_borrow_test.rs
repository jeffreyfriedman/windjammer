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

//! WDB-246: `format!` String temps into demoted `&str` `hash_join_semi_i64` must borrow.
//!
//! Product residual tip-out/gen graph_sql_datafusion_port:
//!   `left_ab.hash_join_semi_i64(right_ab, _temp0, _temp1)` where `_temp0`/`_temp1`
//!   are `format!(...)` `String`s but formals are `&str` → E0308.
//! Twin of WDB-244 (`.to_string()` lits into `sql_exec`). Signature-driven: `&_temp0`, `&_temp1`.

use std::path::PathBuf;

#[test]
fn wdb246_tip_out_datafusion_must_borrow_format_temps_into_demoted_hash_join() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let columnar = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-types/gen/columnar_batch_arrow.rs");
    let demoted = if columnar.exists() {
        let text = std::fs::read_to_string(&columnar).expect("columnar");
        text.contains("fn hash_join_semi_i64(") && text.contains("left_col: &str")
    } else {
        true
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
        let bad = text.contains("hash_join_semi_i64(right_ab, _temp0, _temp1)")
            && !text.contains("hash_join_semi_i64(right_ab, &_temp0, &_temp1)");
        eprintln!(
            "WDB-246 demoted={} bad={} path={}",
            demoted,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-246 RED: tip-out/product passes owned format temps into demoted &str hash_join. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-246: graph_sql_datafusion_port missing");
}
