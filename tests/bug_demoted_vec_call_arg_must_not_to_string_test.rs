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

//! P3.390 Product: `bt_validation.wj` — same-file private index-only `Vec` formals demote
//! to `&Vec`, then IR rewrites reuse `.clone()` → `.to_string()` via the non-text
//! `emitted_rust_ref_formals` fallback (E0599).
//!
//! Cross-module `pub fn` isolates stay owned and do not hit this path.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod bt_validation
"#;

const BT: &str = r#"
fn path_extend(base: Vec<i32>, value: i32) -> Vec<i32> {
    let mut out: Vec<i32> = Vec::new()
    let mut i = 0
    while i < base.len() {
        out.push(base[i])
        i = i + 1
    }
    out.push(value)
    out
}

fn vec_contains(vals: Vec<i32>, needle: i32) -> bool {
    let mut i = 0
    while i < vals.len() {
        if vals[i] == needle {
            return true
        }
        i = i + 1
    }
    false
}

fn visit_cycle(cur: i32, ancestors: Vec<i32>) -> bool {
    if vec_contains(ancestors, cur) {
        return true
    }
    let extended = path_extend(ancestors, cur)
    let _ = extended.len()
    false
}

pub fn entry(doc_root: i32) -> bool {
    visit_cycle(doc_root, Vec::new())
}
"#;

#[test]
fn demoted_vec_call_arg_must_not_to_string() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("bt_validation.wj", BT);

    let map = test.compile().expect("P3.390 compile");
    let rs = map.get("bt_validation.rs").expect("bt_validation.rs");

    assert!(
        !rs.contains("ancestors.to_string()"),
        "P3.390 RED: demoted &Vec must not .to_string():\n{rs}"
    );
    assert!(
        !rs.contains("ancestors.clone()"),
        "P3.390 RED: demoted &Vec into shared-ref callee must not .clone():\n{rs}"
    );
    assert!(
        rs.contains("vec_contains(ancestors,") || rs.contains("vec_contains(&ancestors,"),
        "P3.390: expected bare/borrowed ancestors into vec_contains:\n{rs}"
    );

    test.cargo_check().expect("P3.390 cargo-check");
}
