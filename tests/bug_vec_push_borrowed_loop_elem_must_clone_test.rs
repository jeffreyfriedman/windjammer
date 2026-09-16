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

//! P3.303: `for x in map.values()` then `vec.push(x)` must clone non-Copy elems.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn vec_push_borrowed_loop_elem_must_clone_for_owned_push() {
    let mut t = MultiFileTest::new();
    t.add_file("mod.wj", "pub mod mgr\n");
    t.add_file(
        "mgr.wj",
        r#"
use std::collections::HashMap

pub struct Achievement {
    id: i32,
}

impl Achievement {
    pub fn id(self) -> i32 {
        self.id
    }
}

pub struct Manager {
    achievements: HashMap<i32, Achievement>,
}

impl Manager {
    pub fn collect(self) -> Vec<Achievement> {
        let mut result: Vec<Achievement> = Vec::new()
        for ach in self.achievements.values() {
            result.push(ach)
        }
        result
    }
}
"#,
    );

    let out = t.compile().expect("compile");
    let rs = out.get("mgr.rs").expect("mgr.rs");
    assert!(
        rs.contains(".push(ach.clone())"),
        "borrowed loop elem into owned Vec::push must clone\n{rs}"
    );
    for line in rs.lines() {
        if line.contains("push(ach") && !line.contains("ach.clone()") {
            panic!("must not push borrowed ref without clone\n{line}\n{rs}");
        }
    }
    t.cargo_check().expect("cargo check");
}
