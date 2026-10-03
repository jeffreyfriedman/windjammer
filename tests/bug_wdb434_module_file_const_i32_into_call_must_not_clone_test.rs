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

//! WDB-434: Copy `const i32` into owned call formals must not `.clone()`.
//!
//! Product `scene/station_builder.rs`:
//!   `VoxelGrid::new(GRID.clone(), GRID.clone(), GRID.clone())`
//! WJ is `VoxelGrid::new(GRID, GRID, GRID)`.
//! Distinct from WDB-426 (`LEAF_FLAG.clone()` on **assign**), WDB-428 (local in
//! println), and WDB-385 (`f32::MAX.clone()` assoc path).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub const GRID: i32 = 64

pub struct VoxelGrid {
    pub w: i32,
    pub h: i32,
    pub d: i32,
}

impl VoxelGrid {
    pub fn new(w: i32, h: i32, d: i32) -> VoxelGrid {
        VoxelGrid { w: w, h: h, d: d }
    }
}

pub fn make_grid() -> VoxelGrid {
    VoxelGrid::new(GRID, GRID, GRID)
}
"#;

#[test]
fn wdb434_module_file_const_i32_into_call_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-434 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-434 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("GRID.clone()");
    assert!(
        !bad,
        "WDB-434 RED: const i32 cloned into call formals:\n{rs}"
    );
    test.cargo_check().expect("WDB-434 cargo-check");
}

fn wdb434_search_roots() -> Vec<PathBuf> {
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
fn wdb434_tip_out_game_core_grid_const_into_new_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb434_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/scene/station_builder.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/scene/station_builder.rs"));
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
            !t.starts_with("//") && t.contains("GRID.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-434: station_builder product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-434 RED: tip/product cloned const GRID into call:\n  {}",
        bad_paths.join("\n  ")
    );
}
