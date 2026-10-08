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

//! P3.732: a local `Vec3` passed to an owned `Vec3` formal must not be borrowed.
//!
//! Engine tip-out `gen/camera/fps_camera.rs` emits
//! `collides_aabb(grid.clone(), &test_x, ...)` while the formal is `pos: Vec3`
//! (expected `Vec3`, found `&Vec3` — 13 of 272 E0308s, 2026-10-07).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn owned_vec3_formal_must_not_borrow_local() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x: x, y: y, z: z }
    }
}
pub struct Grid {}
pub fn collides(grid: Grid, pos: Vec3) -> bool {
    pos.x > 0.0
}
pub fn move_it(grid: Grid) {
    let test_x = Vec3::new(1.0, 2.0, 3.0)
    collides(grid, test_x)
}
"#,
    );
    let map = test
        .compile()
        .expect("P3.732: owned Vec3 formal fixture must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !body.contains("&test_x"),
        "owned Vec3 formal must not borrow the local; got:\n{body}"
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
fn owned_vec3_formal_tip_out_fps_camera_must_not_borrow_local() {
    let mut paths = Vec::new();
    for dir in search_roots() {
        paths.push(dir.join(
            "windjammer-game/windjammer-game-core/gen/camera/fps_camera.rs",
        ));
    }
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("fps_camera.rs");
        if text.contains("&test_x") || text.contains("&test_y") || text.contains("&test_z") {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "P3.732: fps_camera.rs product file missing");
    assert!(
        bad_paths.is_empty(),
        "P3.732 RED: tip-out borrows Vec3 local into owned formal:\n  {}",
        bad_paths.join("\n  ")
    );
}
