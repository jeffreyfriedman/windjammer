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

//! WDB-281 class: live Vec into an *owned* Vec formal must `.clone()` (not `&items`).
//! Tip may demote a read-only `contains(items: Vec)` to `&Vec` and borrow at the
//! call site — that is correct (no E0308). RED only when formal stays owned and
//! the call site reborrows.

#[path = "common/test_utils.rs"]
mod test_utils;

const SRC: &str = r#"
fn contains(items: Vec<i64>, value: i64) -> bool {
    for item in items {
        if item == value {
            return true
        }
    }
    false
}

fn push_unique(items: Vec<i64>, value: i64) -> Vec<i64> {
    if contains(items, value) {
        return items
    }
    let mut next = items
    next.push(value)
    next
}

fn main() {}
"#;

#[test]
fn owned_vec_reuse_into_owned_callee_must_clone_not_reborrow() {
    let (rs, ok) = test_utils::compile_single_check(SRC);
    let formal_owned = rs.contains("fn contains(items: Vec<i64>")
        || rs.contains("fn contains(items: Vec<i64>,");
    let formal_borrowed = rs.contains("fn contains(items: &Vec<i64>")
        || rs.contains("fn contains(items: &[i64]");
    if formal_owned {
        assert!(
            !rs.contains("contains(&items,"),
            "must not reborrow live Vec into owned formal. Generated:\n{rs}"
        );
        assert!(
            rs.contains("contains(items.clone(),")
                || (rs.contains("contains(items,") && !rs.contains("return items")),
            "expected items.clone() into owned contains (or move if unused). Generated:\n{rs}"
        );
    } else {
        assert!(
            formal_borrowed,
            "contains formal should be owned Vec or demoted &Vec. Generated:\n{rs}"
        );
        assert!(
            rs.contains("contains(&items,") || rs.contains("contains(items,"),
            "demoted contains should borrow or move. Generated:\n{rs}"
        );
    }
    assert!(ok, "fixture must cargo-check. Generated:\n{rs}");
}
