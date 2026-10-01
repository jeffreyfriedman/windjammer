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

//! WDB-427: Copy `i32` struct field into local must not `.clone()`.
//!
//! Product `rendering/mesh_generator.rs`:
//!   `let size = chunk.size.clone();`
//! WJ is `let size = chunk.size`.
//! Distinct from WDB-393 (local assign `x.clone()`), WDB-346 (`color.r.clone()`),
//! WDB-426 (const `u32`), and WDB-370 (indexed Copy field).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Chunk {
    pub size: i32,
}

pub fn extent(chunk: Chunk) -> i32 {
    let size = chunk.size
    size * size
}
"#;

#[test]
fn wdb427_module_file_copy_i32_field_let_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-427 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-427 MultiFile lib.rs:\n{rs}");
    let cloned = rs.contains(".clone()");
    assert!(
        !cloned,
        "WDB-427 RED: Copy i32 field cloned into local:\n{rs}"
    );
    test.cargo_check().expect("WDB-427 cargo-check");
}

fn wdb427_search_roots() -> Vec<PathBuf> {
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
fn wdb427_tip_out_game_core_chunk_size_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb427_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/rendering/mesh_generator.rs"));
        paths.push(dir.join(".agent-wip/rel_tip_out/mesh_generator.rs"));
        paths.push(
            dir.join("windjammer-game/windjammer-game-core/gen/rendering/mesh_generator.rs"),
        );
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
            !t.starts_with("//") && t.contains("chunk.size.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-427: mesh_generator product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-427 RED: tip/product cloned Copy chunk.size in:\n  {}",
        bad_paths.join("\n  ")
    );
}
