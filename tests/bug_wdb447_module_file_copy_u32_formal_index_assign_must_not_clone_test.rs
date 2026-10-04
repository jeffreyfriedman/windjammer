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

//! WDB-447: Copy `u32` **formal** into indexed assign must not `.clone()`.
//!
//! Product `ffi_tilemap/tilemap.rs` `clear`:
//!   `self.tiles[i] = tile_id.clone()`
//! WJ uses bare `tile_id`.
//! Distinct from WDB-441 (i64 **local** into index assign) and WDB-442 (u32 local into let).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct TileMap {
    pub tiles: Vec<u32>,
    pub width: u32,
    pub height: u32,
}

impl TileMap {
    pub fn clear(self, tile_id: u32) {
        let mut i: u32 = 0
        let n = self.width * self.height
        while i < n {
            self.tiles[i as int] = tile_id
            i = i + 1
        }
    }
}
"#;

#[test]
fn wdb447_module_file_copy_u32_formal_index_assign_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-447 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-447 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("tile_id.clone()");
    assert!(
        !bad,
        "WDB-447 RED: Copy u32 formal cloned into index assign:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb447_search_roots() -> Vec<PathBuf> {
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
fn wdb447_tip_out_game_core_tilemap_u32_formal_index_assign_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb447_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/ffi_tilemap/tilemap.rs"));
        paths.push(dir.join(".agent-wip/rel_tip_out/tilemap/tilemap.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/ffi_tilemap/tilemap.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/tilemap/tilemap.rs"));
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
                && t.contains("tiles[")
                && t.contains("= tile_id.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-447: tilemap product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-447 RED: tip/product Copy u32 formal into index assign clone:\n  {}",
        bad_paths.join("\n  ")
    );
}
