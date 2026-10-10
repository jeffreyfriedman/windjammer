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
//!   `self.evaluate_node(node_a, clips.clone().clone(), time, bone_count)`
//! WJ passes `clips` once per call. One `.clone()` is enough when the formal
//! stays owned and the value is used again. A second `.clone()` is redundant.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Clip {
    pub id: int,
}

fn use_clips(clips: Vec<Clip>) -> int {
    clips.len()
}

pub fn split(clips: Vec<Clip>) -> int {
    let a = use_clips(clips)
    let b = use_clips(clips)
    a + b
}
"#;

#[test]
fn module_file_reused_vec_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains(".clone().clone()"),
        "RED: reused Vec double-cloned:\n{rs}"
    );
    let _ = test.cargo_check();
}

