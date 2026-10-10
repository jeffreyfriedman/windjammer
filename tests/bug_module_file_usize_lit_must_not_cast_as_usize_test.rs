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

//! WDB-364: `usize` literals must not emit redundant `N_usize as usize`.
//!
//! Tip output for a game project cluster:
//!   `0_usize as usize`, `1_usize as usize`, `20_usize as usize`
//! Prefer bare `0_usize` / `1`. Twin of WDB-361 (`i as usize`) / WDB-352 (`_i32 as i32`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn clamp_idx(i: usize, parent: usize) -> bool {
    if parent >= 0 {
        true
    } else {
        i + 1 < parent
    }
}
"#;

/// Product shape (physics_world): nested for-range `(i + 1)..bodies.len()`.
const SRC_RANGE: &str = r#"
pub struct World {
    pub bodies: Vec<i32>,
}

impl World {
    pub fn step(self) {
        for i in 0..self.bodies.len() {
            for j in (i + 1)..self.bodies.len() {
                let _ = self.bodies[j]
            }
        }
    }
}
"#;

#[test]
fn module_file_usize_lit_must_not_cast_as_usize() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("_usize as usize") || rs.contains("0_usize as usize");
    assert!(
        !bad,
        "usize lit emitted as usize:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

#[test]
fn module_file_range_start_must_not_cast_usize_lit() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC_RANGE);
    let map = test.compile().expect("range compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("range lib.rs:\n{rs}");
    let bad = rs.contains("_usize as usize");
    assert!(
        !bad,
        "range start `_usize as usize`:\n{rs}"
    );
    test.cargo_check().expect("range cargo-check");
}
