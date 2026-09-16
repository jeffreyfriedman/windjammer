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

//! WDB-230: tip-out `map.put(&(ids[i]), &(vals[i]))` into owned Copy formals.
//!
//! WDB-138 multipass fixture is tip GREEN; product tip-out/gen still RED (~7×):
//!   `map.put(&(csr.vertex_ids[i as usize]), &(distances[i as usize]))`
//! while `put(&mut self, vertex: i64, value: f64)` → E0308 i64←&i64 / f64←&f64.
//! Signature-driven: Copy index/value exprs must pass by value.

use std::path::PathBuf;

#[test]
fn wdb230_tip_out_pagerank_must_not_borrow_copy_put_args() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("graph_pagerank_engine.rs"),
        tip.join("graph_batch_engine.rs"),
        gen.join("graph/graph_pagerank_engine.rs"),
        gen.join("graph/graph_batch_engine.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("engine");
        let bad = text.contains("map.put(&(csr.vertex_ids[")
            || text.contains(".put(&(csr.vertex_ids[");
        eprintln!("WDB-230 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-230 RED: tip-out/product wraps Copy put args in &. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-230: pagerank/batch engines missing");
}
