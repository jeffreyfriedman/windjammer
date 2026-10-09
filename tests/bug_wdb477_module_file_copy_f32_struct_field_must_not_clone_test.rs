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

//! WDB-477: a Copy `f32` struct field must not `.clone()` when read into another struct.
//!
//! Product `voxel/material.rs` `from_material_data`:
//!   `color_r: m.albedo.x.clone()` and `roughness: m.roughness.clone()`
//! WJ is `color_r: m.albedo.x` and `roughness: m.roughness` (`f32`).
//! Distinct from WDB-466 (Copy tuple field `.0` / `.1`) and WDB-344 (a named `f32` local).

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

pub struct Src {
    pub albedo: Vec3,
    pub roughness: f32,
}

pub struct Mat {
    pub color_r: f32,
    pub roughness: f32,
}

pub fn pack(m: Src) -> Mat {
    Mat { color_r: m.albedo.x, roughness: m.roughness }
}
"#;

#[test]
fn wdb477_module_file_copy_f32_struct_field_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-477 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-477 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".clone()");
    assert!(
        !bad,
        "WDB-477 RED: Copy f32 field cloned:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb477_search_roots() -> Vec<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut roots = vec![manifest.clone()];
    let git = manifest.join(".git");
    if git.is_file() {
        if let Ok(text) = std::fs::read_to_string(&git) {
            if let Some(line) = text.lines().find(|l| l.starts_with("gitdir:")) {
                let gitdir = PathBuf::from(line.trim_start_matches("gitdir:").trim());
                if let Some(repo) = gitdir.ancestors().nth(3) {
                    roots.push(repo.to_path_buf());
                    if let Some(src_wj) = repo.parent() {
                        roots.push(src_wj.to_path_buf());
                    }
                }
            }
        }
    }
    let mut walked = manifest;
    for _ in 0..8 {
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
fn wdb477_tip_out_voxel_material_f32_field_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb477_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/voxel/material.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/voxel/material.rs"));
    }
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("product");
        let bad = text.lines().any(|line| {
            let t = line.trim_start();
            !t.starts_with("//")
                && (t.contains("albedo.x.clone()") || t.contains("roughness.clone()"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-477: voxel material product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-477 RED: tip/product Copy f32 field cloned:\n  {}",
        bad_paths.join("\n  ")
    );
}
