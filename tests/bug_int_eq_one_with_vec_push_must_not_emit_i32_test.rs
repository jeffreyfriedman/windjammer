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

//! P3.705 / LedgerKit `home.wj` `tasks_from_kpis`:
//! `if unmatched_bank_lines == 1` beside `Vec::push` + format interp must not
//! emit `1_i32` when the formal is `i64` (E0277). Bare `n == 1` without push
//! already emits `1_i64` (P3.682); push/format context regresses the lit width.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r##"
pub struct TaskFields {
    pub title: string,
    pub href: string,
}

pub fn tasks_from_kpis(unmatched_bank_lines: int) -> Vec<TaskFields> {
    let mut tasks: Vec<TaskFields> = Vec::new()
    if unmatched_bank_lines > 0 {
        let plural = if unmatched_bank_lines == 1 { "" } else { "s" }
        let title = "Reconcile: match ${unmatched_bank_lines} bank line${plural}"
        tasks.push(TaskFields {
            title: title,
            href: "#/bank",
        })
    }
    tasks
}
"##;

#[test]
fn int_eq_one_with_vec_push_must_not_emit_i32() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("P3.705 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("P3.705 MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("== 1_i32") && !rs.contains("==1_i32"),
        "P3.705 RED: int == 1 beside Vec::push must not emit 1_i32:\n{rs}"
    );
    assert!(
        rs.contains("== 1_i64") || rs.contains("== 1"),
        "P3.705 RED: expected i64-width lit for int == 1:\n{rs}"
    );
    test.cargo_check().expect("P3.705 cargo-check");
}
