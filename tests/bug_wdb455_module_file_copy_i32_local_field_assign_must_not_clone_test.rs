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

//! WDB-455: Copy `i32` **local** into **field assign** must not `.clone()`.
//!
//! Product `csg/scene.rs` add_*:
//!   `self.root_id = id.clone()`
//! WJ uses bare `id` (`let id = self.next_id`).
//! Distinct from WDB-393 (i32 **formal** field assign), WDB-446 (i32 local into **typed let**),
//! and WDB-437 (i32 formal into **let**).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct CsgScene {
    pub root_id: i32,
    pub next_id: i32,
}

impl CsgScene {
    pub fn new() -> CsgScene {
        CsgScene { root_id: -1, next_id: 0 }
    }

    pub fn add_sphere(self, cx: f32, cy: f32, cz: f32, radius: f32) -> i32 {
        let id = self.next_id
        self.next_id = self.next_id + 1
        if self.root_id == -1 {
            self.root_id = id
        }
        let _ = cx + cy + cz + radius
        id
    }
}
"#;

#[test]
fn wdb455_module_file_copy_i32_local_field_assign_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-455 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-455 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("id.clone()");
    assert!(
        !bad,
        "WDB-455 RED: Copy i32 local cloned into field assign:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb455_search_roots() -> Vec<PathBuf> {
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
fn wdb455_tip_out_game_core_csg_scene_i32_local_field_assign_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb455_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/csg/scene.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/csg/scene.rs"));
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
            !t.starts_with("//") && t.contains("root_id = id.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-455: csg/scene product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-455 RED: tip/product Copy i32 local into field assign clone:\n  {}",
        bad_paths.join("\n  ")
    );
}
