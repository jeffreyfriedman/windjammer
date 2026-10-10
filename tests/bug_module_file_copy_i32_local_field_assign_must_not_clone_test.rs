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
//!   `self.root_id = id.clone()`
//! WJ uses bare `id` (`let id = self.next_id`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct CsgScene {
    pub root_id: i32,
    pub next_id: i32,
}

impl CsgScene {
    pub fn new() -> CsgScene {
        CsgScene { root_id: -1, next_id: 0 }
    }

    pub fn add_sphere(self, cx: f32, cy: f32, cz: f32, radius: f32) -> i32 {
        let id = self.next_id
        self.next_id = self.next_id + 1
        if self.root_id == -1 {
            self.root_id = id
        }
        let _ = cx + cy + cz + radius
        id
    }
}
"#;

#[test]
fn module_file_copy_i32_local_field_assign_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("id.clone()");
    assert!(
        !bad,
        "RED: Copy i32 local cloned into field assign:\n{rs}"
    );
    let _ = test.cargo_check();
}

