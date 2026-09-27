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

//! WDB-415: owned `Vec` formal moved into a callee inside `if` must not `.clone()`.
//!
//! Product `unified_renderer.rs`:
//!   `UnifiedRenderer::materials_to_palette(materials.clone())`
//! WJ is `UnifiedRenderer::materials_to_palette(materials)` — one use, then done.
//! Distinct from WDB-412 (field.clone() into for-loop formal), WDB-414 (`new(self.scene)`),
//! and WDB-407 (`Vec` `new` demote).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Mat {
    pub name: string,
}

pub struct Pal {
    pub n: i32,
}

pub fn materials_to_palette(materials: Vec<Mat>) -> Pal {
    Pal { n: materials.len() as i32 }
}

pub fn upload_materials(test_mode: bool, materials: Vec<Mat>) -> i32 {
    if !test_mode {
        let palette = materials_to_palette(materials)
        palette.n
    } else {
        0
    }
}
"#;

#[test]
fn wdb415_module_file_owned_vec_formal_inside_if_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-415 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-415 MultiFile lib.rs:\n{rs}");
    let cloned = rs.contains("materials.clone()");
    assert!(
        !cloned,
        "WDB-415 RED: owned Vec formal cloned inside if:\n{rs}"
    );
    test.cargo_check().expect("WDB-415 cargo-check");
}

fn wdb415_search_roots() -> Vec<PathBuf> {
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
fn wdb415_tip_out_game_core_materials_to_palette_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb415_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/rendering/unified_renderer.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/rendering/unified_renderer.rs"));
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
            !t.starts_with("//") && t.contains("materials_to_palette(materials.clone())")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-415: unified_renderer product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-415 RED: tip/product materials_to_palette cloned materials in:\n  {}",
        bad_paths.join("\n  ")
    );
}
