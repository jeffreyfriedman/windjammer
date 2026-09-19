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

//! WDB-300: cast expr must not grow a trailing `.clone()` (`n as usize.clone()`).
//!
//! Product tip-out/gen PageRank / pg_serve:
//!   `graph_pagerank_buffers_new(n as usize.clone(), …)` → "cast cannot be followed by a method call".
//! WJ source: `graph_pagerank_buffers_new(n, 1.0 / n as f64)` — clone coercion must not attach after `as`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn buffers_new(n: usize, init: f64) -> f64 {
    init + (n as f64)
}

pub fn buffers_seeded(n: usize, prior_len: usize) -> f64 {
    if prior_len != n {
        return buffers_new(n, 1.0 / n as f64)
    }
    0.0
}
"#;

#[test]
fn wdb300_module_file_cast_must_not_emit_trailing_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-300 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-300 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("as usize.clone()")
        || rs.contains("as u32.clone()")
        || rs.contains("as i64.clone()")
        || rs.contains("as f64.clone()");
    assert!(
        !bad,
        "WDB-300 RED: MultiFile emitted cast-then-.clone() (illegal Rust):\n{rs}"
    );
    test.cargo_check().expect("WDB-300 cargo-check");
}

#[test]
fn wdb300_tip_out_must_not_emit_cast_trailing_clone() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("graph_pagerank_engine.rs"),
        tip.join("graph/graph_pagerank_engine.rs"),
        tip.join("relational_pg_serve_port.rs"),
        tip.join("relational/relational_pg_serve_port.rs"),
        gen.join("graph/graph_pagerank_engine.rs"),
        gen.join("relational/relational_pg_serve_port.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("port");
        if text.contains("as usize.clone()")
            || text.contains("as u32.clone()")
            || text.contains("as i64.clone()")
        {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-300: tip-out/gen pagerank/pg_serve missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-300 RED: tip-out/product emits `as T.clone()` in:\n  {}",
        bad_paths.join("\n  ")
    );
}
