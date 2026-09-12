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

//! FAILING REPRO — LedgerKit `apply_seed_bank_*_overlay` without `+ ""`.
//! Module `const string` (`LINE_STATUS_MATCHED`) into owned `BankLineView.status`
//! must emit `.to_string()`, not `.clone()` on `&str` (E0308).
//!
//! Tip probe 2026-09-12: `status: LINE_STATUS_MATCHED.clone()` → cargo-check FAIL.

#[path = "common/test_utils.rs"]
mod test_utils;

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SOURCE: &str =
    include_str!("fixtures/library_multipass/seed_overlay_apply_bank_line_no_plus_empty.wj");

fn assert_module_const_owns_into_string_field(rs: &str) {
    assert!(
        rs.contains("LINE_STATUS_MATCHED") || rs.contains("status:"),
        "must emit status assignment; got:\n{rs}"
    );
    let bad_clone = rs.contains("LINE_STATUS_MATCHED.clone()")
        || rs.contains("status: LINE_STATUS_MATCHED.clone()");
    assert!(
        !bad_clone,
        "RED: module const string into owned field must .to_string(), not .clone(). Got:\n{rs}"
    );
}

#[test]
fn seed_overlay_apply_bank_line_must_cargo_check_without_plus_empty() {
    let (rs, ok) = test_utils::compile_single_check(SOURCE);
    assert_module_const_owns_into_string_field(&rs);
    assert!(
        ok,
        "RED P3.253: apply overlay without + \"\" must cargo-check (module const → owned field). Generated:\n{rs}"
    );
}

#[test]
fn hexagonal_seed_overlay_apply_bank_line_must_cargo_check_without_plus_empty() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod domain
pub mod adapters
"#,
    );
    project.add_file("domain/mod.wj", "pub mod bank\n");
    project.add_file("domain/bank.wj", SOURCE);
    project.add_file("adapters/mod.wj", "pub mod overlay\n");
    project.add_file(
        "adapters/overlay.wj",
        r#"
use super::super::domain::bank::{BankLineView, apply_seed_bank_match_overlay}

pub fn apply(lines: Vec<BankLineView>) -> Vec<BankLineView> {
    apply_seed_bank_match_overlay(lines)
}
"#,
    );

    let map = project
        .compile()
        .expect("hexagonal apply overlay compile should succeed");
    let key = map
        .keys()
        .find(|k| k.ends_with("bank.rs"))
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "missing bank.rs; keys: {:?}",
                map.keys().collect::<Vec<_>>()
            )
        });
    let rs = map.get(&key).expect("bank.rs");
    assert_module_const_owns_into_string_field(rs);
}
