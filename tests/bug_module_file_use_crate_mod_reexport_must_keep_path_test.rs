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
))]

//! Regression: module file use crate mod reexport must keep path.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod graph_algorithm_host
pub mod graph
pub mod graph_cdlp_test
"#;

const HOST: &str = r#"
pub fn cdlp_same_community(vertex_a: i64, vertex_b: i64) -> bool {
    vertex_a == vertex_b
}
"#;

const GRAPH: &str = r#"
pub use crate::graph_algorithm_host::cdlp_same_community
"#;

const LEAF_TEST: &str = r#"
use crate::graph::cdlp_same_community

pub fn test_cdlp_same_community_via_graph_reexport() {
    if !cdlp_same_community(1, 1) {
        panic("cdlp_same_community must be reachable via crate::graph::cdlp_same_community")
    }
}
"#;

#[test]
fn module_file_use_crate_mod_reexport_must_keep_path_or_cargo_check() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("graph_algorithm_host.wj", HOST);
    test.add_file("graph.wj", GRAPH);
    test.add_file("graph_cdlp_test.wj", LEAF_TEST);

    let map = test
        .compile()
        .expect("multipass compile should succeed (codegen may still be wrong)");
    let leaf = map
        .get("graph_cdlp_test.rs")
        .expect("graph_cdlp_test.rs must be generated");

    let kept_path = leaf.contains("use crate::graph::cdlp_same_community");
    let dropped_to_root = leaf.contains("use crate::cdlp_same_community") && !kept_path;

    if dropped_to_root || !kept_path {
        eprintln!("RED emit graph_cdlp_test.rs:\n{leaf}");
    }

    assert!(
        kept_path,
        "RED: use crate::graph::cdlp_same_community must not become use crate::cdlp_same_community."
    );

    test.cargo_check().expect(
        "RED: reexport import path must cargo-check.",
    );
}
