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

//! Regression: module file lod config mesh id must not clone element.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct MeshId {
    pub id: i32,
}

pub struct Level {
    pub id: MeshId,
}

impl Level {
    pub fn mesh_id(self) -> MeshId {
        self.id
    }

    pub fn is_active(self, _d: f32) -> bool {
        true
    }
}

pub struct Lod {
    pub levels: Vec<Level>,
}

impl Lod {
    pub fn get_level_for_distance(self, distance: f32) -> Option<MeshId> {
        let mut i = 0
        while i < self.levels.len() {
            if self.levels[i].is_active(distance) {
                return Some(self.levels[i].mesh_id())
            }
            i = i + 1
        }
        None
    }
}
"#;

#[test]
fn module_file_lod_config_mesh_id_must_not_clone_element() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("].clone().mesh_id()"),
        "RED: indexed mesh_id cloned the element:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
