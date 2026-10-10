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
//!   `fn capacity() { self.capacity.clone() }`
//!   `fn in_use_count() { self.in_use.clone() }`
//!   `PoolStats { capacity: self.capacity.clone(), … }`
//! WJ returns bare `self.capacity` / `self.in_use`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct PoolStats {
    pub capacity: usize,
    pub in_use: usize,
    pub peak_usage: usize,
}

pub struct ObjectPool<T> {
    available: Vec<T>,
    capacity: usize,
    in_use: usize,
    peak_usage: usize,
}

impl ObjectPool<T> {
    pub fn capacity(self) -> usize {
        self.capacity
    }

    pub fn in_use_count(self) -> usize {
        self.in_use
    }

    pub fn stats(self) -> PoolStats {
        PoolStats {
            capacity: self.capacity,
            in_use: self.in_use,
            peak_usage: self.peak_usage,
        }
    }
}
"#;

#[test]
fn module_file_copy_usize_field_return_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("self.capacity.clone()")
        || rs.contains("self.in_use.clone()")
        || rs.contains("self.peak_usage.clone()");
    assert!(
        !bad,
        "RED: Copy usize field return/struct-lit cloned:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

