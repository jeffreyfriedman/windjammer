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

//!

#[path = "common/test_utils.rs"]
mod test_utils;

const FIXTURE: &str =
    include_str!("fixtures/library_multipass/mut_ref_graph_into_owned_must_clone.wj");

#[test]
fn codegen_demoted_mut_ref_into_owned_graph_must_clone() {
    let (rs, ok) = test_utils::compile_single_check(FIXTURE);
    let callee_owned = rs.contains("fn out_neighbors(graph: TypedGraph")
        || rs.contains("out_neighbors(graph: TypedGraph");
    let caller_demoted = rs.contains("fn query(graph: &mut TypedGraph")
        || rs.contains("query(graph: &mut TypedGraph");
    let bare = rs.contains("out_neighbors(graph,") && !rs.contains("out_neighbors(graph.clone(),");
    let bad = callee_owned && caller_demoted && bare;
    eprintln!(
        "codegen owned={} demoted={} bare={} bad={}\n{rs}",
        callee_owned, caller_demoted, bare, bad
    );
    assert!(
        !bad,
        "demoted &mut graph into owned out_neighbors must clone. Generated:\n{rs}"
    );
    assert!(ok, "fixture must cargo-check. Generated:\n{rs}");
}

