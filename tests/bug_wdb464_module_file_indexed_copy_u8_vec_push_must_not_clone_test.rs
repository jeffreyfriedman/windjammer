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

//! WDB-464: indexed Copy `u8` into `Vec::push` must not `.clone()`.
//!
//! Product `ecs/world.rs` `serialize_vec3`:
//!   `bytes.push(x_bits[0].clone())` … (from `f32::to_le_bytes()`)
//! WJ uses bare `bytes.push(x_bits[0])`.
//! Distinct from WDB-363 (indexed Copy **field**), WDB-431 (Copy u64 **field**),
//! WDB-350 (Copy usize **cast**).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn serialize_f32_bytes(x: f32) -> Vec<u8> {
    let mut bytes: Vec<u8> = Vec::new()
    let x_bits = x.to_le_bytes()
    bytes.push(x_bits[0])
    bytes.push(x_bits[1])
    bytes.push(x_bits[2])
    bytes.push(x_bits[3])
    bytes
}
"#;

#[test]
fn wdb464_module_file_indexed_copy_u8_vec_push_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-464 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-464 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("x_bits[0].clone()")
        || rs.contains("x_bits[1].clone()")
        || rs.contains("x_bits[2].clone()")
        || rs.contains("x_bits[3].clone()");
    assert!(
        !bad,
        "WDB-464 RED: indexed Copy u8 cloned into Vec::push:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb464_search_roots() -> Vec<PathBuf> {
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
fn wdb464_tip_out_game_core_serialize_vec3_indexed_u8_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb464_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/ecs/world.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/ecs/world.rs"));
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
                && (t.contains("x_bits[") && t.contains("].clone()")
                    || t.contains("y_bits[") && t.contains("].clone()")
                    || t.contains("z_bits[") && t.contains("].clone()"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-464: ecs/world product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-464 RED: tip/product indexed Copy u8 into Vec::push clone:\n  {}",
        bad_paths.join("\n  ")
    );
}
