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

//! WDB-158: read-only compare helper must not demote `Value` formal to `&mut Value`.
//!
//! Product (cold relational gen after WDB-155 tip fix):
//!   `row_matches_eq_at(row, column, expected: &mut Value)`
//!   `values_equal_int64(a: Value, b: Value)` body emits `if let … = &mut b` → E0596
//!
//! `.wj` uses owned `Value` for equality checks. Tip must keep owned (or `&Value`),
//! never `&mut Value` for non-mutating compares. WDB-156 writeback fixture can
//! false-GREEN; this gate targets the compare / match pattern.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod cell
pub mod cmp
"#;

const CELL: &str = r#"
pub enum Cell {
    Int64(i64),
    Empty,
}
"#;

const CMP: &str = r#"
use crate::cell::Cell

pub fn cells_equal_int64(a: Cell, b: Cell) -> bool {
    match a {
        Cell::Int64(x) => {
            match b {
                Cell::Int64(y) => x == y,
                Cell::Empty => false,
            }
        },
        Cell::Empty => false,
    }
}

pub fn row_matches(expected: Cell) -> bool {
    let got = Cell::Int64(7)
    cells_equal_int64(got, expected)
}
"#;

fn wdb158_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("cell.wj", CELL);
    test.add_file("cmp.wj", CMP);
    test
}

#[test]
fn wdb158_module_file_value_compare_must_not_demote_to_mut_ref() {
    let test = wdb158_fixture();
    let map = test
        .compile()
        .expect("WDB-158 multipass compile should succeed (codegen may still be wrong)");
    let cmp_rs = map.get("cmp.rs").expect("cmp.rs");

    let mut_formal = cmp_rs.contains("b: &mut Cell")
        || cmp_rs.contains("expected: &mut Cell")
        || cmp_rs.contains("a: &mut Cell");
    let mut_match = cmp_rs.contains("&mut b") || cmp_rs.contains("&mut expected");

    if mut_formal || mut_match {
        eprintln!("WDB-158 RED cmp.rs:\n{cmp_rs}");
    }

    assert!(
        !mut_formal,
        "WDB-158 RED: cells_equal_int64 / row_matches must not demote Cell formals to &mut. Product: row_matches_eq_at expected: &mut Value."
    );
    assert!(
        !mut_match,
        "WDB-158 RED: match/if-let on owned Cell must not emit &mut binding. Product: values_equal_int64 `if let … = &mut b` → E0596."
    );
}
