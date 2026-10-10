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

//!
//!   `self.last_poll_time = current_time.clone()`
//! WJ uses bare `current_time`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct LiveReloadSystem {
    pub last_poll_time: i64,
    pub poll_interval_ms: i32,
}

impl LiveReloadSystem {
    pub fn poll(self, current_time: i64) -> i32 {
        if current_time - self.last_poll_time < self.poll_interval_ms as i64 {
            return 0
        }
        self.last_poll_time = current_time
        1
    }
}
"#;

#[test]
fn module_file_copy_i64_formal_field_assign_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("current_time.clone()");
    assert!(
        !bad,
        "RED: Copy i64 formal cloned into field assign:\n{rs}"
    );
    let _ = test.cargo_check();
}

