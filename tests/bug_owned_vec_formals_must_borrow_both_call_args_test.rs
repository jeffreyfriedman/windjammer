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

//! P3.739: if both owned `Vec` formals are demoted to `&Vec`, both call args borrow.
//!
//! `half_edge.wj` declares `from_triangle_mesh(positions: Vec<Vec3>, indices: Vec<u32>)`.
//! Tip-out emits `positions: &Vec<Vec3>, indices: &Vec<u32>` but
//! `mesh_ops.rs` calls `from_triangle_mesh(positions, &indices)`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn mixed_borrow(body: &str) -> bool {
    body.contains("from_triangle_mesh(positions, &indices)")
        || body.contains("from_triangle_mesh(positions,&indices)")
}

#[test]
fn owned_vec_formals_must_borrow_both_call_args() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod mesh
pub mod ops
"#,
    );
    test.add_file(
        "mesh.wj",
        r#"
pub struct Vec3 {
    pub x: f32,
}
pub struct Mesh {}
impl Mesh {
    pub fn from_triangle_mesh(positions: Vec<Vec3>, indices: Vec<u32>) -> Mesh {
        Mesh {}
    }
}
"#,
    );
    test.add_file(
        "ops.wj",
        r#"
use crate::mesh::Vec3
use crate::mesh::Mesh
pub fn build(positions: Vec<Vec3>, indices: Vec<u32>) -> Mesh {
    Mesh::from_triangle_mesh(positions, indices)
}
"#,
    );
    let map = test
        .compile()
        .expect("P3.739: owned Vec formals fixture must transpile");
    let body = map
        .get("ops.rs")
        .unwrap_or_else(|| panic!("ops.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !mixed_borrow(body),
        "both Vec args must be passed the same way; got:\n{body}"
    );
}

fn search_roots() -> Vec<std::path::PathBuf> {
    let mut roots = Vec::new();
    let mut walked = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..6 {
        roots.push(walked.clone());
        if let Some(parent) = walked.parent() {
            walked = parent.to_path_buf();
        } else {
            break;
        }
    }
    roots
}

#[test]
fn owned_vec_formals_tip_out_mesh_ops() {
    let mut paths = Vec::new();
    for dir in search_roots() {
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/editor/mesh_ops.rs"));
    }
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("mesh_ops.rs");
        if mixed_borrow(&text) {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "P3.739: mesh_ops.rs product file missing");
    assert!(
        bad_paths.is_empty(),
        "P3.739 RED: tip-out borrows only the second Vec:\n  {}",
        bad_paths.join("\n  ")
    );
}
