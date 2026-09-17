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

//! P3.336: `int` `% 10` and `digit == 0` inside `while n > 0` must not emit i64 vs i32 split.
//!
//! Product tip `make api-check` (LedgerKit `audit_events.wj`, `postgres_invoice_repository.wj`):
//! P3.323 regressed to `n as i64 % 10_i32`, `while n > (0_i32 as i32)` (~289 rustc errors).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod audit
"#;

const AUDIT: &str = r#"
pub fn int_to_string(value: int) -> string {
    if value == 0 {
        return "0"
    }
    let mut n = value
    let mut digits = ""
    while n > 0 {
        let digit = n % 10
        if digit == 0 {
            digits = "0" + digits
        } else {
            digits = "9" + digits
        }
        n = n / 10
    }
    digits
}
"#;

fn int_mod_literal_width_split(rs: &str) -> bool {
    rs.contains(" as i64 % 10_i32")
        || rs.contains(" as i64 % 10i32")
        || rs.contains("while n > (0_i32")
        || (rs.contains("== 0_i32") && rs.contains("as i64"))
}

#[test]
fn int_mod_literal_zero_compare_must_not_split_i64_i32() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("audit.wj", AUDIT);
    let map = test.compile().expect("P3.336 compile");
    let rs = map.get("audit.rs").expect("audit.rs");
    let split = int_mod_literal_width_split(rs);
    if split {
        eprintln!("P3.336 RED:\n{rs}");
    }
    assert!(
        !split,
        "P3.336: int `%` / `== 0` in while loop must not mix i64 lhs with i32 literal peers:\n{rs}"
    );
    test.cargo_check().expect("P3.336 cargo-check");
}
