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

//! Regression: module file self method must not clone receiver.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Streamer {
    pub active: Vec<u32>,
}

impl Streamer {
    pub fn is_tile_active(self, tile_id: u32) -> bool {
        let mut i: usize = 0
        while i < self.active.len() {
            if self.active[i] == tile_id {
                return true
            }
            i = i + 1
        }
        false
    }

    pub fn needs_center(self, center_id: u32) -> bool {
        !self.is_tile_active(center_id)
    }
}

// Shape: &self caller + &self sibling + later field index.
pub struct MeshEntry {
    pub uploaded: bool,
}

pub struct MeshRenderer {
    pub mesh_registry: Vec<MeshEntry>,
}

impl MeshRenderer {
    pub fn find_mesh_index(self, name: string) -> i32 {
        let mut i: i32 = 0
        while (i as usize) < self.mesh_registry.len() {
            i = i + 1
        }
        -1
    }

    pub fn is_mesh_uploaded(self, name: string) -> bool {
        let idx = self.find_mesh_index(name)
        if idx < 0 {
            return false
        }
        self.mesh_registry[idx as usize].uploaded
    }
}
"#;

#[test]
fn module_file_self_method_must_not_clone_receiver() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("self.clone()");
    assert!(
        !bad,
        "RED: self.clone() before &self method:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
