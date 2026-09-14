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

//! FAILING REPRO — owned trait string formals must not receive `&String` at call sites.
//!
//! Product tip api-check (LedgerKit):
//!   `self.get(&_temp0, _temp1)` / `lock.close_period(&tenant_slug, …)` after tip kept
//!   trait-impl formals Owned while call sites still borrow.

#[path = "common/test_utils.rs"]
mod test_utils;

const SOURCE: &str =
    include_str!("fixtures/library_multipass/trait_owned_string_call_must_not_over_borrow.wj");

#[test]
fn trait_owned_string_call_must_not_over_borrow() {
    let (rs, ok) = test_utils::compile_single_check(SOURCE);
    let bad = rs.contains(".get(&") || rs.contains("get(&_");
    if bad || !ok {
        eprintln!("RED P3.267 trait owned string over-borrow:\n{rs}");
    }
    assert!(
        !bad,
        "RED P3.267: trait owned string formal must not receive &arg. Generated:\n{rs}"
    );
    assert!(
        ok,
        "RED P3.267: trait owned string call must cargo-check. Generated:\n{rs}"
    );
}
