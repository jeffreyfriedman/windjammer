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

//! P3.520: same-field replace after a consuming helper must stay `&mut self`.
//!
//! Product `wj-notes-api` `check_rate`:
//!   `let checked = check_key(..., buckets_for_limit(self.buckets), ...)`
//!   `self.buckets = buckets_from_limit(checked.0)`
//! Caller `handle_method` continues with `self.dispatch(...)`.
//! Distinct from WDB-414 (`Vox::new(self.scene)` then write a *different* field).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
use std::collections::HashMap

pub struct App {
    pub buckets: HashMap<string, i32>,
}

fn take_map(buckets: HashMap<string, i32>) -> HashMap<string, i32> {
    buckets
}

fn put_map(buckets: HashMap<string, i32>) -> HashMap<string, i32> {
    buckets
}

impl App {
    fn check_rate(self, key: string) -> bool {
        let next = put_map(take_map(self.buckets))
        self.buckets = next
        key.len() == 0
    }

    fn dispatch(self) -> i32 {
        1
    }

    pub fn handle_method(self, key: string) -> i32 {
        if self.check_rate(key) {
            return 429
        }
        self.dispatch()
    }
}
"#;

#[test]
fn check_rate_field_replace_must_not_move_self() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("P3.520 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("P3.520 MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("fn check_rate(mut self") && !rs.contains("fn check_rate(self,"),
        "P3.520 RED: check_rate must be &mut self (field replace + later self use):\n{rs}"
    );
    assert!(
        rs.contains("fn check_rate(&mut self"),
        "P3.520 RED: expected &mut self check_rate:\n{rs}"
    );
    test.cargo_check().expect("P3.520 cargo-check");
}
