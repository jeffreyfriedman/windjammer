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

//! WDB-407: owned `Vec` formal stored into a field must stay `Vec`, not `&Vec` + clone.
//!
//! Product:
//!   `FABRIKChain::new(joint_positions: &Vec<Vec3>)` then `joints: joint_positions.clone()`
//!   call site `FABRIKChain::new(&joints)`
//!   `VoxParser::new(data: &Vec<u8>)` + `VoxParser::new(&data)`
//! WJ source is `new(joint_positions: Vec<Vec3>)` / `new(data: Vec<u8>)` and
//! `Chain::new(joints)` / `VoxParser::new(data)`.
//! Distinct from WDB-398 (`&palette.copy()` into still-owned `new`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Chain {
    pub joints: Vec<i32>,
}

impl Chain {
    pub fn new(joints: Vec<i32>) -> Chain {
        Chain { joints: joints }
    }
}

pub fn make() -> Chain {
    let joints = vec![1, 2, 3]
    Chain::new(joints)
}
"#;

/// Product shape: private associated `VoxParser::new(data: Vec<u8>)` stored into a
/// field was demoted to `pub fn new(data: &Vec<u8>)` + `.clone()`.
const PRODUCT_VOX: &str = r#"
struct VoxParser {
    data: Vec<u8>,
    pos: int,
}

impl VoxParser {
    fn new(data: Vec<u8>) -> VoxParser {
        VoxParser { data: data, pos: 0 }
    }
}

pub fn parse(data: Vec<u8>) -> int {
    let parser = VoxParser::new(data)
    parser.pos
}
"#;

#[test]
fn wdb407_module_file_owned_vec_new_must_not_demote() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-407 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-407 MultiFile lib.rs:\n{rs}");
    let demoted_formal = rs.contains("fn new(joints: &Vec")
        || rs.contains("fn new(joints: &mut Vec")
        || rs.contains("new(joints: &Vec<i32>)");
    let borrowed_call = rs.contains("Chain::new(&joints)") || rs.contains("new(&joints)");
    let field_clone = rs.contains("joints: joints.clone()")
        || rs.contains("joints: joint_positions.clone()");
    assert!(
        !demoted_formal && !borrowed_call && !field_clone,
        "WDB-407 RED: owned Vec new() demoted to &Vec + clone:\n{rs}"
    );
    test.cargo_check().expect("WDB-407 cargo-check");
}

#[test]
fn wdb407_module_file_private_associated_vec_new_must_not_demote() {
    let mut test = MultiFileTest::new();
    test.add_file("vox.wj", PRODUCT_VOX);
    let map = test.compile().expect("WDB-407 product-shape compile");
    let rs = map.get("vox.rs").expect("vox.rs");
    eprintln!("WDB-407 product-shape vox.rs:\n{rs}");
    assert!(
        !rs.contains("fn new(data: &Vec") && !rs.contains("VoxParser::new(&data)"),
        "WDB-407 RED: private associated Vec new demoted:\n{rs}"
    );
    assert!(
        !rs.contains("data: data.clone()"),
        "WDB-407 RED: demoted Vec new cloned into field:\n{rs}"
    );
    test.cargo_check().expect("WDB-407 product-shape cargo-check");
}

fn wdb407_search_roots() -> Vec<PathBuf> {
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
fn wdb407_tip_out_game_core_owned_vec_new_must_not_demote() {
    let mut paths = Vec::new();
    for dir in wdb407_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/animation/ik.rs"));
        paths.push(dir.join(".agent-wip/rel_tip_out/animation/ik_test.rs"));
        paths.push(dir.join(".agent-wip/rel_tip_out/assets/vox_loader.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/animation/ik.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/animation/ik_test.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/assets/vox_loader.rs"));
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
                && (t.contains("FABRIKChain::new(&joints)")
                    || t.contains("VoxParser::new(&data)")
                    || t.contains("fn new(joint_positions: &Vec")
                    || t.contains("fn new(data: &Vec<u8>)"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-407: FABRIK/VoxParser product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-407 RED: tip/product owned Vec new() demoted in:\n  {}",
        bad_paths.join("\n  ")
    );
}
