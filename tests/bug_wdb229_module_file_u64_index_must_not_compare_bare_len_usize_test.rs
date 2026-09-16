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

//! WDB-229: `u64` loop index must not compare against bare `len()` (`usize`).
//!
//! Product residual (~14× u64←usize), tip-out/gen graph_sql_query_port:
//!   `while u < csr.vertex_ids.len()` with `u: u64` → E0308 expected u64, found usize.
//! Related to WDB-215 (len as i64) but distinct width: cast `len() as u64`.

use std::path::PathBuf;

#[test]
fn wdb229_tip_out_graph_sql_must_cast_len_to_u64_for_u64_index() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("graph_sql_query_port.rs"),
        gen.join("graph/graph_sql_query_port.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("graph_sql");
        // Bad: while u < …len() without as u64 (and not as usize on left)
        let bad = text.contains("while u < csr.vertex_ids.len()")
            && !text.contains("while u < csr.vertex_ids.len() as u64");
        eprintln!("WDB-229 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-229 RED: tip-out/product compares u64 index to bare len() usize. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-229: graph_sql_query_port missing");
}
