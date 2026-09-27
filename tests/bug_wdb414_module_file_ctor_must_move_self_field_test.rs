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

//! WDB-414: `Type::new(self.scene)` must move the field, not `self.scene.clone()`.
//!
//! Product:
//!   `CsgVoxelizer::new(self.scene.clone())` in rifter_quarter / cathedral / humanoid_demo
//! WJ is `CsgVoxelizer::new(self.scene)` — scene is not used after.
//! Distinct from WDB-360 (`encode(self.grid.clone())`), WDB-410 (wither reconstruct),
//! and WDB-407 (`Vec` `new` demote).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Scene {
    pub n: i32,
}

pub struct Vox {
    pub scene: Scene,
}

impl Vox {
    pub fn new(scene: Scene) -> Vox {
        Vox { scene: scene }
    }
}

pub struct Demo {
    pub scene: Scene,
    pub grid: i32,
}

impl Demo {
    pub fn initialize(self) {
        let v = Vox::new(self.scene)
        self.grid = v.scene.n
    }
}
"#;

#[test]
fn wdb414_module_file_ctor_must_move_self_field() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-414 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-414 MultiFile lib.rs:\n{rs}");
    let cloned = rs.contains("self.scene.clone()");
    assert!(
        !cloned,
        "WDB-414 RED: constructor cloned self.scene:\n{rs}"
    );
    test.cargo_check().expect("WDB-414 cargo-check");
}

fn wdb414_search_roots() -> Vec<PathBuf> {
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
fn wdb414_tip_out_game_core_ctor_must_not_clone_self_scene() {
    let mut paths = Vec::new();
    for dir in wdb414_search_roots() {
        for demo in ["rifter_quarter.rs", "cathedral.rs", "humanoid_demo.rs"] {
            paths.push(dir.join(".agent-wip/rel_tip_out/demos").join(demo));
            paths.push(
                dir.join("windjammer-game/windjammer-game-core/gen/demos")
                    .join(demo),
            );
        }
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
            !t.starts_with("//") && t.contains("CsgVoxelizer::new(self.scene.clone())")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-414: demo product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-414 RED: tip/product CsgVoxelizer::new cloned self.scene in:\n  {}",
        bad_paths.join("\n  ")
    );
}
