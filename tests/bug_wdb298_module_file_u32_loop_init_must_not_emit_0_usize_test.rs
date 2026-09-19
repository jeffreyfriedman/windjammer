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

//! WDB-298: `u32` loop counter init must emit `0_u32`, never `0_usize`.
//!
//! Product tip-out/gen (~17×): `let mut i: u32 = 0_usize` (adjacency / CDLP / LCC / wave1)
//! → E0308 expected `u32`, found `usize`. WJ source is `let mut i = 0` with `i` used as `u32`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn walk_degree(degree: u32) -> u32 {
    let mut i = 0
    let mut acc: u32 = 0
    while i < degree {
        acc = acc + i
        i = i + 1
    }
    acc
}
"#;

#[test]
fn wdb298_module_file_u32_loop_init_must_not_emit_0_usize() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-298 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-298 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("u32 = 0_usize") || rs.contains(": u32 = 0_usize");
    assert!(
        !bad,
        "WDB-298 RED: MultiFile emitted u32 = 0_usize (must be 0_u32):\n{rs}"
    );
    test.cargo_check().expect("WDB-298 cargo-check");
}

#[test]
fn wdb298_tip_out_must_not_emit_u32_eq_0_usize() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("graph_adjacency_port.rs"),
        tip.join("graph/graph_adjacency_port.rs"),
        tip.join("graph_cdlp_engine.rs"),
        tip.join("graph/graph_cdlp_engine.rs"),
        tip.join("graph_lcc_engine.rs"),
        tip.join("graph/graph_lcc_engine.rs"),
        gen.join("graph/graph_adjacency_port.rs"),
        gen.join("graph/graph_cdlp_engine.rs"),
        gen.join("graph/graph_lcc_engine.rs"),
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
    assert!(saw, "WDB-298: tip-out/gen adjacency/cdlp/lcc missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-298 RED: tip-out/product emits `u32 = 0_usize` in:\n  {}",
        bad_paths.join("\n  ")
    );
}
