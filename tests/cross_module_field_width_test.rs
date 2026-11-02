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

//! P0 (financial correctness): an `int` accumulator over a field of a struct defined in ANOTHER module is
//! 64-bit. Truncating it to `i32` lets an unbalanced journal entry validate as balanced. Widths come from
//! the field's declared type through the cross-module resolver; compiled and interpreted runs agree.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn cross_module_int_field_accumulator_is_64_bit() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "model.wj",
        r#"
pub struct Line {
    pub account: string,
    pub amount_cents: int,
}

pub struct Draft {
    pub lines: Vec<Line>,
}
"#,
    );
    test.add_file(
        "check.wj",
        r#"
use crate::model::Draft

pub fn validate(draft: Draft) -> Result<Draft, string> {
    let mut total = 0
    for idx in 0..draft.lines.len() {
        total = total + draft.lines[idx].amount_cents
    }
    if total != 0 {
        return Err("must balance")
    }
    Ok(draft)
}
"#,
    );
    let map = test.compile().expect("multipass compile should succeed");
    let check = map.get("check.rs").expect("check.rs");
    assert!(
        !check.contains("i32"),
        "an int accumulator over a cross-module int field must not be i32:\n{check}"
    );
    assert!(check.contains("i64"), "expected an i64 accumulator:\n{check}");
}
