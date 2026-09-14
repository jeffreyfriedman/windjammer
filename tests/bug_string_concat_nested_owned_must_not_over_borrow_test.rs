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

//! FAILING REPRO — nested `concat2` / overlay_row helpers must keep owned
//! string formals at call sites (LedgerKit `domain/string_concat.wj`).
//!
//! Product tip api-check (2026-09-13/14): ~17× WJ0003 on string_concat
//! "arguments to this function are incorrect" / "consider removing the borrow".

#[path = "common/test_utils.rs"]
mod test_utils;

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SOURCE: &str =
    include_str!("fixtures/library_multipass/string_concat_nested_owned_must_not_over_borrow.wj");

fn assert_no_over_borrow_into_concat2(rs: &str) {
    // Tip must not pass &String / &str into owned concat/overlay formals (call sites).
    let over_borrow = rs.contains("concat2(&")
        || rs.contains("append_overlay_row(&")
        || rs.contains("overlay_row2(&")
        || rs.contains("overlay_row3(&")
        || rs.contains("overlay_row4(&")
        || rs.contains("overlay_row2(&String::from")
        || rs.contains("append_overlay_row(&a")
        || rs.contains("append_overlay_row(&b");
    assert!(
        !over_borrow,
        "RED P3.264: nested concat/overlay helpers must not over-borrow owned formals. Generated:\n{rs}"
    );
}

#[test]
fn string_concat_nested_owned_must_not_over_borrow() {
    let (rs, ok) = test_utils::compile_single_check(SOURCE);
    assert_no_over_borrow_into_concat2(&rs);
    assert!(
        ok,
        "RED P3.264: nested concat2/overlay_row must cargo-check. Generated:\n{rs}"
    );
}

#[test]
fn hexagonal_string_concat_nested_owned_must_not_over_borrow() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod domain
pub mod adapters
"#,
    );
    project.add_file("domain/mod.wj", "pub mod string_concat\n");
    project.add_file("domain/string_concat.wj", SOURCE);
    project.add_file("adapters/mod.wj", "pub mod overlay\n");
    project.add_file(
        "adapters/overlay.wj",
        r#"
use crate::domain::string_concat::{append_overlay_row, overlay_row2}

pub fn remember(line_id: string, je: string) -> string {
    append_overlay_row("" + "", overlay_row2(line_id, je))
}
"#,
    );

    let map = project
        .compile()
        .expect("hexagonal string_concat compile should succeed");
    for (key, rs) in &map {
        if key.ends_with("string_concat.rs") || key.ends_with("overlay.rs") {
            assert_no_over_borrow_into_concat2(rs);
        }
    }
}
