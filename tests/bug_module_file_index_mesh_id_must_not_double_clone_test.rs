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
//!   `rendering/pbr_pipeline.rs`: `self.levels[pi].clone().mesh_id.clone()`
//!   `event/dispatcher.rs`: `self.event_log[i].clone().event_description.clone()`

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct LodLevel {
    pub mesh_id: string,
}

pub struct LodChain {
    pub levels: Vec<LodLevel>,
}

impl LodChain {
    pub fn primary(self, i: usize) -> string {
        self.levels[i].mesh_id
    }
}
"#;

#[test]
fn module_file_index_mesh_id_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone().mesh_id");
    assert!(
        !bad,
        "RED: indexed mesh_id double-cloned:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

