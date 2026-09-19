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

//! WDB-324: game-core navmesh `find_triangle` — `u32 = 0_usize` must not emit.
//!
//! Product tip gen (`windjammer-game-core/gen/ai/navmesh.rs`):
//!   `let mut i: u32 = 0_usize;` inside `find_triangle(...) -> Option<u32>`
//!   with `while i < self.triangles.len()`. Twin of WDB-308/298.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Triangle {
    pub id: u32,
    pub walkable: bool,
}

pub struct Navmesh {
    pub triangles: Vec<Triangle>,
}

pub fn find_triangle(mesh: Navmesh, _x: f32) -> Option<u32> {
    let mut i = 0
    while i < mesh.triangles.len() {
        let tri = mesh.triangles[i]
        if tri.walkable {
            return Some(tri.id)
        }
        i = i + 1
    }
    None
}
"#;

#[test]
fn wdb324_module_file_game_core_navmesh_u32_must_not_emit_0_usize() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-324 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-324 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("u32 = 0_usize") || rs.contains(": u32 = 0_usize");
    assert!(
        !bad,
        "WDB-324 RED: MultiFile emitted u32 = 0_usize (must be 0_u32):\n{rs}"
    );
    test.cargo_check().expect("WDB-324 cargo-check");
}

#[test]
fn wdb324_tip_out_game_core_navmesh_must_not_emit_u32_eq_0_usize() {
    let roots = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("windjammer-game/windjammer-game-core/gen/ai/navmesh.rs"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("windjammer-game/windjammer-game-core/ai/navmesh.rs"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("windjammer-game/windjammer-game-core/build/ai/navmesh.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &roots {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("navmesh");
        if text.contains("u32 = 0_usize") {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-324: game-core navmesh.rs missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-324 RED: game-core tip gen still has u32=0_usize in:\n  {}",
        bad_paths.join("\n  ")
    );
}
