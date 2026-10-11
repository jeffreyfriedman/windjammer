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

//! indexed Copy u32 field must not emit `].clone().buffer_id` / `.buffer_id.clone()`.
//!
//! Tip output for a game project `rendering/shader_graph_compiler.rs`:
//!   `lifetimes[li].clone().buffer_id`
//! WJ source is `lifetimes[li].buffer_id` (Copy u32). Twin of .

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Lifetime {
    pub buffer_id: u32,
    pub name: string,
}

pub fn takes_id(id: u32) -> u32 {
    id
}

pub struct Analysis {
    pub lifetimes: Vec<Lifetime>,
}

impl Analysis {
    pub fn first_id(self, i: usize) -> u32 {
        takes_id(self.lifetimes[i].buffer_id)
    }
}
"#;

#[test]
fn module_file_copy_u32_index_field_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone().buffer_id") || rs.contains(".buffer_id.clone()");
    assert!(
        !bad,
        "indexed Copy u32 field cloned:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
