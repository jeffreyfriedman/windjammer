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

//! FAILING REPRO — `while idx < vec.len()` must unify int index vs usize len.
//!
//! Product tip api-check (LedgerKit seed/postgres repos):
//!   `while (idx as i64) < validated.lines.len()` → WJ0003 expected int, found uint.

#[path = "common/test_utils.rs"]
mod test_utils;

const SOURCE: &str =
    include_str!("fixtures/library_multipass/while_idx_lt_vec_len_must_unify_int_uint.wj");

#[test]
fn while_idx_lt_vec_len_must_unify_int_uint() {
    let (rs, ok) = test_utils::compile_single_check(SOURCE);
    let mixed = rs.contains("as i64") && rs.contains(".len()") && rs.contains('<');
    // Acceptable: both sides usize, or both i64, or for-in rewrite.
    let bad = rs.contains("as i64) <") && rs.contains(".len()");
    if bad {
        eprintln!("RED P3.265 while idx < len mixed compare:\n{rs}");
    }
    assert!(
        !bad,
        "RED P3.265: while idx < vec.len() must not compare i64 to usize len. Got:\n{rs}"
    );
    assert!(
        ok,
        "RED P3.265: while idx < vec.len() must cargo-check. Generated:\n{rs}"
    );
    let _ = mixed;
}
