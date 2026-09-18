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

//! WDB-281 class: owned Vec still live after call into owned Vec formal must `.clone()`,
//! never `&items` (E0308).

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
    assert!(
        !rs.contains("contains(&items,"),
        "must not reborrow live Vec into owned formal. Generated:\n{rs}"
    );
    assert!(
        rs.contains("contains(items.clone(),")
            || (rs.contains("contains(items,") && !rs.contains("return items")),
        "expected items.clone() into owned contains (or move if unused). Generated:\n{rs}"
    );
    assert!(ok, "fixture must cargo-check. Generated:\n{rs}");
}
