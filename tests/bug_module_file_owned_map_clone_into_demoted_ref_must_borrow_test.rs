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

//! owned `GraphVertexI64Map` / `.clone()` into demoted `&GraphVertexI64Map` must borrow.
#[path = "common/test_utils.rs"]
mod test_utils;

const FIXTURE: &str = include_str!(
    "fixtures/library_multipass/wdb222_owned_map_clone_into_demoted_ref_must_borrow.wj"
);

#[test]
fn codegen_owned_map_into_demoted_ref_must_borrow() {
    let (rs, ok) = test_utils::compile_single_check(FIXTURE);
    let demoted = rs.contains("fn get(map: &GraphVertexI64Map")
        || rs.contains("get(map: &GraphVertexI64Map");
    if demoted {
        assert!(
            rs.contains("get(&map") || rs.contains("get(&map.clone()"),
            "demoted &Map formal must borrow at call site. Generated:\n{rs}"
        );
        assert!(
            !rs.contains("get(map.clone()"),
            "no owned clone into &Map. Generated:\n{rs}"
        );
    }
    assert!(ok, "fixture must cargo-check. Generated:\n{rs}");
}
