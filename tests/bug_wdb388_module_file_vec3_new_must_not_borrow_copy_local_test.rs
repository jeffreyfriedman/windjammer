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

//! WDB-388: Copy locals into `Vec3::new(x: f32, …)` must not emit `Vec3::new(&x, …)`.
//!
//! P3.442 isolate + navmesh tip exist; product `gen/` still over-borrows the first
//! f32 ident in 30+ files (cameras, mesh_generator, voxel_mesh, terrain, …).
//! WJ source is `Vec3::new(x, y, z)`. Twin of P3.442 with full-product scan.

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

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x: x, y: y, z: z }
    }
}

pub fn point(a: f32, b: f32, c: f32) -> Vec3 {
    Vec3::new(a, b, c)
}
"#;

#[test]
fn wdb388_module_file_vec3_new_must_not_borrow_copy_local() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-388 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-388 MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("Vec3::new(&a"),
        "WDB-388 RED: Copy local borrowed into Vec3::new:\n{rs}"
    );
    test.cargo_check().expect("WDB-388 cargo-check");
}

#[test]
fn wdb388_tip_out_game_core_must_not_vec3_new_borrow() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let gen = game.join("gen");
    let roots = [tip.as_path(), gen.as_path()];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for root in &roots {
        if !root.exists() {
            continue;
        }
        let walker = walkdir_rs(root);
        for path in walker {
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            saw = true;
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            let bad = text.lines().any(|line| {
                !line.trim_start().starts_with("//") && line.contains("Vec3::new(&")
            });
            if bad {
                bad_paths.push(path.display().to_string());
            }
        }
    }
    assert!(saw, "WDB-388: tip/gen product trees missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-388 RED: Vec3::new(& in:\n  {}",
        bad_paths.join("\n  ")
    );
}

fn walkdir_rs(root: &std::path::Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for ent in rd.flatten() {
            let p = ent.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                out.push(p);
            }
        }
    }
    out
}
