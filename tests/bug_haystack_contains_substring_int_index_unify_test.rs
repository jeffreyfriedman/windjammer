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

//! FAILING REPRO — `strings.substring(h, i + j, i + j + 1)` with `int` loop
//! counters must cast both start and end to `usize` (or keep both as i64).
//! Tip emits `i + j + 1_usize` (E0277 i64 + usize). LedgerKit
//! `domain/string_contains.wj` still uses `h + ""` / index workarounds around
//! this class (`make api-check` WJ0003 / E0277).

#[path = "common/test_utils.rs"]
mod test_utils;

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SOURCE: &str = include_str!("fixtures/library_multipass/haystack_contains_int_index.wj");

fn assert_substring_indices_unified(rs: &str) {
    assert!(
        rs.contains("substring"),
        "must emit substring; got:\n{rs}"
    );
    // Reject mixed i64 + usize end expressions.
    assert!(
        !rs.contains("+ 1_usize") && !rs.contains("+1_usize"),
        "RED: substring end must not mix i64 + 1_usize. Got:\n{rs}"
    );
}

#[test]
fn haystack_contains_substring_int_indices_must_unify_usize() {
    let (rs, ok) = test_utils::compile_single_check(SOURCE);
    assert_substring_indices_unified(&rs);
    assert!(
        ok,
        "RED: substring(int, int+1) with strings.len must cargo-check. Generated:\n{rs}"
    );
}

#[test]
fn hexagonal_haystack_contains_substring_int_indices_must_unify_usize() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod domain
pub mod adapters
"#,
    );
    project.add_file("domain/mod.wj", "pub mod contains\n");
    project.add_file("domain/contains.wj", SOURCE);
    project.add_file("adapters/mod.wj", "pub mod check\n");
    project.add_file(
        "adapters/check.wj",
        r#"
use super::super::domain::contains::haystack_contains

pub fn has_needle(hay: string, needle: string) -> bool {
    haystack_contains(hay, needle)
}
"#,
    );

    let map = project
        .compile()
        .expect("hexagonal haystack compile should succeed");
    let key = map
        .keys()
        .find(|k| k.ends_with("contains.rs"))
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "missing contains.rs; keys: {:?}",
                map.keys().collect::<Vec<_>>()
            )
        });
    assert_substring_indices_unified(map.get(&key).expect("contains.rs"));
}
