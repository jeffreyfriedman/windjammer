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

//! DB-442: Copy `u32` local into `let` must not `.clone()`.
//!
//! Product `world/async_loader.rs`:
//!   `let mut step = budget.clone()`
//! WJ is `let mut step = budget`.
//! Distinct from DB-437 (i32 **formal** into let), DB-441 (i64 into **index assign**),
//! and DB-440 (f32 into **struct lit**).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn pump_budget(max_bytes: u32, remaining: u32) -> u32 {
    let mut budget = max_bytes
    let mut loaded: u32 = 0
    while budget > 0 {
        let mut step = budget
        if step > remaining {
            step = remaining
        }
        loaded = loaded + step
        budget = budget - step
        if remaining == 0 {
            break
        }
    }
    loaded
}
"#;

#[test]
fn module_file_copy_u32_local_into_let_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("budget.clone()") || rs.contains("step.clone()");
    assert!(
        !bad,
        "Copy u32 local cloned into let:\n{rs}"
    );
    let _ = test.cargo_check();
}
