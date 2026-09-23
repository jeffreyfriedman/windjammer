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

//! WDB-371: indexed Copy Vec3 must not emit `].clone().position.clone()`.
//!
//! Product tip game-core `editor/mesh_ops.rs` / `half_edge.rs`:
//!   `positions.push(mesh.vertices[vi].clone().position.clone())`
//! Prefer `mesh.vertices[vi].position` (Copy Vec3). Twin of WDB-370 / WDB-355.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct Vertex {
    pub position: Vec3,
}

pub struct Mesh {
    pub vertices: Vec<Vertex>,
}

impl Mesh {
    pub fn collect_positions(self) -> Vec<Vec3> {
        let mut out = Vec::new()
        let mut i = 0
        while i < self.vertices.len() {
            out.push(self.vertices[i].position)
            i = i + 1
        }
        out
    }
}
"#;

#[test]
fn wdb371_module_file_copy_vec3_field_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-371 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-371 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone().position") || rs.contains(".position.clone()");
    assert!(
        !bad,
        "WDB-371 RED: indexed Copy Vec3 double-cloned:\n{rs}"
    );
    test.cargo_check().expect("WDB-371 cargo-check");
}

#[test]
fn wdb371_tip_out_game_core_mesh_must_not_double_clone_position() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("editor/mesh_ops.rs"),
        tip.join("mesh_ops.rs"),
        tip.join("editor/half_edge.rs"),
        tip.join("half_edge.rs"),
        game.join("gen/editor/mesh_ops.rs"),
        game.join("gen/editor/half_edge.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("product");
        let bad = text.lines().any(|line| {
            !line.trim_start().starts_with("//")
                && (line.contains("].clone().position.clone()")
                    || line.contains("].clone().position"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-371: mesh_ops/half_edge product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-371 RED: tip/product indexed position double-clone in:\n  {}",
        bad_paths.join("\n  ")
    );
}
