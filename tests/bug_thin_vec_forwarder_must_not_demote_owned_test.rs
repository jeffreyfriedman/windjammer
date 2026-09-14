#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
    feature = "integration_tests",
))]

//! FAILING REPRO — thin Vec forwarder must not demote to `&Vec` while callee stays owned.
//!
//! Product tip api-check (LedgerKit request_context.wj):
//!   `post_request(headers: &Vec)` calling `post_request_query(..., Vec)` → E0308.

#[path = "common/test_utils.rs"]
mod test_utils;

const SOURCE: &str =
    include_str!("fixtures/library_multipass/thin_vec_forwarder_must_not_demote_owned.wj");

#[test]
fn thin_vec_forwarder_must_not_demote_owned() {
    let (rs, ok) = test_utils::compile_single_check(SOURCE);
    let demoted_forwarder = rs.contains("fn post_simple")
        && (rs.contains("headers: &Vec") || rs.contains("headers: &std::vec::Vec"));
    if demoted_forwarder || !ok {
        eprintln!("RED P3.266 thin Vec forwarder demotion:\n{rs}");
    }
    assert!(
        ok,
        "RED P3.266: thin Vec forwarder must cargo-check with owned formals. Generated:\n{rs}"
    );
    assert!(
        !demoted_forwarder,
        "RED P3.266: thin Vec forwarder must keep owned Vec formal. Generated:\n{rs}"
    );
}
