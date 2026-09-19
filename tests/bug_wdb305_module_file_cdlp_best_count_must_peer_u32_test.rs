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

//! WDB-305: CDLP majority `best_count` must peer `u32` (not `0_i64` vs `u32` count).
//!
//! Product tip-out/gen:
//!   WJ: `let mut best_count = 0` + `count` from `graph_vertex_u32_get` → u32
//!   tip: `let mut best_count = 0_i64` then `count > best_count` → E0277/E0308 u32 vs i64.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn majority_label(neighbor_labels: Vec<i64>, counts: Vec<u32>) -> i64 {
    let mut best_label = neighbor_labels[0]
    let mut best_count = 0
    let mut i = 0
    while i < neighbor_labels.len() {
        let label = neighbor_labels[i]
        let count = counts[i]
        if count > best_count || (count == best_count && label < best_label) {
            best_count = count
            best_label = label
        }
        i = i + 1
    }
    best_label
}
"#;

#[test]
fn wdb305_module_file_u32_count_accum_must_not_init_i64() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-305 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-305 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("best_count = 0_i64")
        || (rs.contains("0_i64") && rs.contains("best_count"));
    assert!(
        !bad,
        "WDB-305 RED: MultiFile initialized best_count as i64 against u32 count:\n{rs}"
    );
    test.cargo_check().expect("WDB-305 cargo-check");
}

#[test]
fn wdb305_tip_out_cdlp_best_count_must_not_be_i64() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("graph_cdlp_engine.rs"),
        tip.join("graph/graph_cdlp_engine.rs"),
        gen.join("graph/graph_cdlp_engine.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("cdlp");
        if text.contains("best_count = 0_i64") {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-305: tip-out/gen CDLP engine missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-305 RED: tip-out/product emits best_count = 0_i64 (must peer u32) in:\n  {}",
        bad_paths.join("\n  ")
    );
}
